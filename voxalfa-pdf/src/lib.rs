use std::sync::Arc;

use fontdb::{Database, Family, Query, Style, Weight};
use krilla::{Document, geom::Size, page::PageSettings};
use krilla_svg::{SurfaceExt, SvgSettings};
use thiserror::Error;
use voxalfa_core::output::FinalOutput;
use voxalfa_svg::{FontFaces, RenderOptions};

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
    render_with_fonts(data, options, &FontFaces::default())
}

pub fn build_font_faces(db: &Database, data: &FinalOutput) -> Result<FontFaces> {
    let mut faces = FontFaces::default();

    for (key, style) in data.header.directives.elements.iter() {
        let Some(font_family) = style.value.font_family.as_ref() else {
            continue;
        };

        // let default = match key.as_str() {
        //     "note-text" => faces.solfa,
        //     _ => continue,
        // };
        //
        // let Some(id) = db.query(&Query {
        //     families: &[Family::Name(font_family)],
        //     ..Default::default()
        // }) else {
        //     continue;
        // };
    }

    Ok(faces)
}

pub fn render_with_fonts(
    data: &FinalOutput,
    options: &RenderOptions,
    faces: &FontFaces,
) -> Result<Vec<u8>> {
    let mut fontdb = fontdb::Database::new();

    fontdb.load_system_fonts();

    // fontdb.set_sans_serif_family("NotoSans");

    // for face in fontdb.faces() {
    //     println!("{}", face.families[0].0);
    // }
    //
    // todo!()
    let id = fontdb.query(&Query {
        families: &[Family::Name("Noto Sans")],
        weight: Weight::NORMAL,
        style: Style::Italic,
        ..Default::default()
    });

    if let Some(face) = id.and_then(|id| fontdb.face(id)) {
        // match face.source {
        //     fontdb::Source::Binary(as_ref) => todo!(),
        //     fontdb::Source::File(path_buf) => todo!(),
        //     fontdb::Source::SharedFile(path_buf, as_ref) => todo!(),
        // }
        println!("{:?}", face.source);
    }

    todo!("Font id is: {id:?}")

    // let pages = voxalfa_svg::render_pages_with_fonts(data, options, faces)?;
    //
    // for face in faces.iter() {
    //     fontdb.load_font_data(face.data.to_vec());
    // }
    //
    // let usvg_options = usvg::Options {
    //     fontdb: Arc::new(fontdb),
    //     ..Default::default()
    // };
    //
    // let mut document = Document::new();
    //
    // for svg in &pages {
    //     let tree = usvg::Tree::from_data(svg.as_bytes(), &usvg_options)?;
    //
    //     let page_size = Size::from_wh(
    //         tree.size().width() * PDF_PX_TO_PT,
    //         tree.size().height() * PDF_PX_TO_PT,
    //     )
    //     .ok_or(Error::PageSize)?;
    //
    //     let page_settings =
    //         PageSettings::from_wh(page_size.width(), page_size.height()).ok_or(Error::PageSize)?;
    //
    //     let mut page = document.start_page_with(page_settings);
    //     let mut surface = page.surface();
    //
    //     surface
    //         .draw_svg(
    //             &tree,
    //             page_size,
    //             SvgSettings {
    //                 embed_text: true,
    //                 ..Default::default()
    //             },
    //         )
    //         .ok_or(Error::Render)?;
    //
    //     surface.finish();
    //     page.finish();
    // }
    //
    // Ok(document.finish()?)
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
