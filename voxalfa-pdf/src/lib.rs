mod fonts;

pub mod error;

use std::sync::Arc;

use krilla::{Document, geom::Size, page::PageSettings};
use krilla_svg::{SurfaceExt, SvgSettings};
use voxalfa_core::output::FinalOutput;
use voxalfa_svg::{RenderOptions, fonts::FontFaces};

pub use error::{Error, Result};

use crate::fonts::build_font_faces;

const PDF_PX_TO_PT: f32 = 72.0 / 96.0;

pub fn render(data: &FinalOutput, options: &RenderOptions) -> Result<Vec<u8>> {
    let faces = build_font_faces(data);
    render_with_fonts(data, options, &faces)
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
    fn test_pdf_rendering() {
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
