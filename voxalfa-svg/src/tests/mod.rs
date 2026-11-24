use voxalfa_core::validation::Validator;

use crate::{
    fonts::{FontFaces, Fonts},
    model::MarkerKind,
    options::RenderOptions,
    renderer::{
        markers::PlacedMarker,
        systems::{LayoutContext, SystemLayout, layout_system},
    },
    resolver::resolve,
};

fn build_layout(source: &str) -> (Fonts, Vec<SystemLayout>) {
    let mut validator = Validator::init().unwrap();
    let output = validator.analyze(source);

    assert!(!output.has_error(), "{:?}", output.diagnostics);

    let options = RenderOptions::default();
    let fonts = Fonts::with_faces(&options, &FontFaces::default()).unwrap();
    let score = resolve(&output, &fonts, &options, options.printable_width()).unwrap();

    let layouts = score
        .systems
        .iter()
        .map(|system| {
            layout_system(&LayoutContext {
                system,
                fonts: &fonts,
                lead: score.lead_width,
                options: &options,
            })
        })
        .collect();

    (fonts, layouts)
}

fn marker(layout: &SystemLayout, kind: MarkerKind) -> &PlacedMarker {
    layout
        .markers
        .iter()
        .find(|marker| marker.kind == kind)
        .unwrap()
}

mod layout;
mod render;
