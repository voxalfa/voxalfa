use std::collections::BTreeSet;

use voxalfa_core::{ast::directives::ElementStyle, output::FinalOutput};
use voxalfa_svg::fonts::{FontFace, FontFaces, default_font_style};

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
