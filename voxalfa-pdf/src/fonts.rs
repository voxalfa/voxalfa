use std::collections::BTreeSet;

use voxalfa_core::{
    ast::directives::{ElementStyle, FontStretch},
    output::FinalOutput,
};
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
    stretch: FontStretch,
    italic: bool,
}

impl<'a> FontRequest<'a> {
    fn new(element: &str, style: &'a ElementStyle) -> Option<Self> {
        let family = style.font_family.as_deref()?.trim();

        if family.is_empty() {
            return None;
        }

        let (default_weight, default_italic, default_stretch) =
            default_font_style(element).unwrap_or((400, false, FontStretch::Normal));

        Some(Self {
            family,
            weight: style.font_weight.unwrap_or(default_weight),
            stretch: style.font_stretch.unwrap_or(default_stretch),
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

    let style = if request.italic {
        fontdb::Style::Italic
    } else {
        fontdb::Style::Normal
    };

    let query = fontdb::Query {
        families: &families,
        weight: fontdb::Weight(request.weight),
        stretch: to_fontdb_stretch(request.stretch),
        style,
    };

    let id = db.query(&query)?;
    let info = db.face(id)?;
    let (data, index) = db.with_face_data(id, |data, index| (data.to_vec(), index))?;
    let italic = matches!(info.style, fontdb::Style::Italic | fontdb::Style::Oblique);

    Some(
        FontFace::new(request.family, data)
            .with_weight(info.weight.0)
            .with_stretch(from_fontdb_stretch(info.stretch))
            .with_italic(italic)
            .with_index(index),
    )
}

fn to_fontdb_stretch(stretch: FontStretch) -> fontdb::Stretch {
    match stretch {
        FontStretch::UltraCondensed => fontdb::Stretch::UltraCondensed,
        FontStretch::ExtraCondensed => fontdb::Stretch::ExtraCondensed,
        FontStretch::Condensed => fontdb::Stretch::Condensed,
        FontStretch::SemiCondensed => fontdb::Stretch::SemiCondensed,
        FontStretch::Normal => fontdb::Stretch::Normal,
        FontStretch::SemiExpanded => fontdb::Stretch::SemiExpanded,
        FontStretch::Expanded => fontdb::Stretch::Expanded,
        FontStretch::ExtraExpanded => fontdb::Stretch::ExtraExpanded,
        FontStretch::UltraExpanded => fontdb::Stretch::UltraExpanded,
    }
}

fn from_fontdb_stretch(stretch: fontdb::Stretch) -> FontStretch {
    match stretch {
        fontdb::Stretch::UltraCondensed => FontStretch::UltraCondensed,
        fontdb::Stretch::ExtraCondensed => FontStretch::ExtraCondensed,
        fontdb::Stretch::Condensed => FontStretch::Condensed,
        fontdb::Stretch::SemiCondensed => FontStretch::SemiCondensed,
        fontdb::Stretch::Normal => FontStretch::Normal,
        fontdb::Stretch::SemiExpanded => FontStretch::SemiExpanded,
        fontdb::Stretch::Expanded => FontStretch::Expanded,
        fontdb::Stretch::ExtraExpanded => FontStretch::ExtraExpanded,
        fontdb::Stretch::UltraExpanded => FontStretch::UltraExpanded,
    }
}

#[cfg(test)]
mod tests {
    use fontdb::{Database, Stretch, Style};
    use voxalfa_svg::fonts::FontFaces;

    #[test]
    fn test_builtin_font_queries() {
        let faces = FontFaces::default();
        let mut db = Database::new();

        for face in faces.iter() {
            db.load_font_data(face.data.to_vec());
        }

        let cases = [
            ("Fira Sans", 700, Stretch::Normal, Style::Normal),
            ("Noto Sans", 400, Stretch::Normal, Style::Normal),
            ("Noto Sans", 700, Stretch::Normal, Style::Normal),
            ("Noto Sans", 400, Stretch::Condensed, Style::Normal),
            ("Noto Serif", 400, Stretch::Normal, Style::Normal),
            ("Noto Serif", 400, Stretch::Normal, Style::Italic),
            ("Noto Serif", 700, Stretch::Normal, Style::Italic),
            ("Noto Sans Symbols", 700, Stretch::Normal, Style::Normal),
        ];

        for (family, weight, stretch, style) in cases {
            let query = fontdb::Query {
                families: &[fontdb::Family::Name(family)],
                weight: fontdb::Weight(weight),
                stretch,
                style,
            };

            let id = db
                .query(&query)
                .unwrap_or_else(|| panic!("missing font {family} {weight} {stretch:?} {style:?}"));
            let face = db.face(id).expect("missing face info");

            assert_eq!(face.weight.0, weight);
            assert_eq!(face.stretch, stretch);
            assert_eq!(face.style, style);
        }
    }
}
