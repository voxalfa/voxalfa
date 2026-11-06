use voxalfa_core::validation::Validator;
use voxalfa_midi::Converter;
use voxalfa_svg::render_pages;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = "createMIDI")]
pub fn create_midi(source: &str) -> Result<Vec<u8>, JsError> {
    let mut validator = Validator::init()?;
    let mut buffer = Vec::new();

    let output = validator.analyze(source);
    let converter = Converter::new(&output);
    let midi = converter.convert()?;

    midi.write_std(&mut buffer)?;

    Ok(buffer)
}

#[wasm_bindgen(js_name = "renderSVG")]
pub fn render_svg(source: &str) -> Result<Vec<String>, JsError> {
    let mut validator = Validator::init()?;

    let output = validator.analyze(source);
    let pages = render_pages(&output)?;

    Ok(pages)
}
