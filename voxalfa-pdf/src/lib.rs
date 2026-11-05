use std::{collections::BTreeSet, sync::Arc};

use krilla::{Document, geom::Size, page::PageSettings};
use krilla_svg::{SurfaceExt, SvgSettings};
use thiserror::Error;
use voxalfa_core::{ast::directives::ElementStyle, output::FinalOutput};
use voxalfa_svg::{FontFace, FontFaces, RenderOptions, default_font_style};

const PDF_PX_TO_PT: f32 = 72.0 / 96.0;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Svg(#[from] voxalfa_svg::Error),

    #[error("invalid PDF page size")]
    PageSize,

    #[error("failed to parse SVG: {0}")]
    Parse(#[from] usvg::Error),

    #[error("failed to draw SVG on the PDF page")]
    Render,

    #[error("failed to finish PDF: {0}")]
    Pdf(#[from] krilla::error::KrillaError),
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn render(data: &FinalOutput, options: &RenderOptions) -> Result<Vec<u8>> {
    let faces = build_font_faces(data);
    render_with_fonts(data, options, &faces)
}

#[cfg(feature = "fs")]
pub fn build_font_faces(data: &FinalOutput) -> FontFaces {
    let requests = font_requests(data);

    if requests.is_empty() {
        return FontFaces::default();
    }

    let mut db = fontdb::Database::new();

    db.load_system_fonts();
    resolve_requests(&db, requests)
}

#[cfg(not(feature = "fs"))]
pub fn build_font_faces(_data: &FinalOutput) -> FontFaces {
    FontFaces::default()
}

pub fn resolve_font_faces(db: &fontdb::Database, data: &FinalOutput) -> FontFaces {
    resolve_requests(db, font_requests(data))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct FontRequest<'a> {
    family: &'a str,
    weight: u16,
    italic: bool,
}

impl<'a> FontRequest<'a> {
    fn new(element: &str, style: &'a ElementStyle) -> Option<Self> {
        let family = style.font_family.as_deref()?.trim();

        if family.is_empty() {
            return None;
        }

        let (default_weight, default_italic) = default_font_style(element).unwrap_or((400, false));

        Some(Self {
            family,
            weight: style.font_weight.unwrap_or(default_weight),
            italic: style.italic.unwrap_or(default_italic),
        })
    }
}

fn font_requests(data: &FinalOutput) -> BTreeSet<FontRequest<'_>> {
    data.header
        .directives
        .elements
        .iter()
        .filter_map(|(element, style)| FontRequest::new(element, &style.value))
        .collect()
}

fn resolve_requests(db: &fontdb::Database, requests: BTreeSet<FontRequest<'_>>) -> FontFaces {
    let mut faces = FontFaces::default();

    for request in requests {
        if let Some(face) = resolve_request(db, request) {
            faces.add(face);
        }
    }

    faces
}

fn resolve_request(db: &fontdb::Database, request: FontRequest<'_>) -> Option<FontFace> {
    let families = [fontdb::Family::Name(request.family)];

    let query = fontdb::Query {
        families: &families,
        weight: fontdb::Weight(request.weight),
        style: if request.italic {
            fontdb::Style::Italic
        } else {
            fontdb::Style::Normal
        },
        ..Default::default()
    };

    let id = db.query(&query)?;
    let info = db.face(id)?;
    let (data, index) = db.with_face_data(id, |data, index| (data.to_vec(), index))?;

    Some(
        FontFace::new(request.family, data)
            .with_weight(info.weight.0)
            .with_italic(matches!(
                info.style,
                fontdb::Style::Italic | fontdb::Style::Oblique
            ))
            .with_index(index),
    )
}

pub fn render_with_fonts(
    data: &FinalOutput,
    options: &RenderOptions,
    faces: &FontFaces,
) -> Result<Vec<u8>> {
    let mut options = options.clone();

    options.encode_font = false;

    let pages = voxalfa_svg::render_pages_with_fonts(data, &options, faces)?;

    let mut fontdb = fontdb::Database::new();

    for face in faces.iter() {
        fontdb.load_font_data(face.data.to_vec());
    }

    let usvg_options = usvg::Options {
        fontdb: Arc::new(fontdb),
        ..Default::default()
    };

    let mut document = Document::new();

    for svg in &pages {
        let tree = usvg::Tree::from_data(svg.as_bytes(), &usvg_options)?;

        let page_size = Size::from_wh(
            tree.size().width() * PDF_PX_TO_PT,
            tree.size().height() * PDF_PX_TO_PT,
        )
        .ok_or(Error::PageSize)?;

        let page_settings =
            PageSettings::from_wh(page_size.width(), page_size.height()).ok_or(Error::PageSize)?;

        let mut page = document.start_page_with(page_settings);
        let mut surface = page.surface();

        surface
            .draw_svg(
                &tree,
                page_size,
                SvgSettings {
                    embed_text: true,
                    ..Default::default()
                },
            )
            .ok_or(Error::Render)?;

        surface.finish();
        page.finish();
    }

    Ok(document.finish()?)
}

#[cfg(test)]
mod tests {
    use voxalfa_core::validation::Validator;
    use voxalfa_svg::RenderOptions;

    use crate::render;

    #[test]
    fn renders_a_pdf_document() {
        let source = r#"
;; @version 0.1.0-alpha

[#] title="PDF test"

[$] key={C} | time={4,4} | tempo={100} | voices={S}

---

[S] |d :r !m :f ||
"#;

        let mut validator = Validator::init().unwrap();
        let output = validator.analyze(source);

        assert!(!output.has_error(), "{:?}", output.diagnostics);

        let pdf = render(&output, &RenderOptions::default()).unwrap();

        assert!(pdf.starts_with(b"%PDF"));
    }
}
