use crate::{fonts::Fonts, model::System, options::RenderOptions};

use super::constants::*;

pub fn layout_voltas(
    system: &System,
    fonts: &Fonts,
    lead: f32,
    options: &RenderOptions,
) -> (Vec<PlacedVolta>, f32) {
    if system.voltas.is_empty() {
        return (Vec::new(), 0.0);
    }

    let style = &options.elements.volta;
    let text_y = style.top() + fonts.label.volta.ascent();

    let line_y =
        style.top() + fonts.label.volta.line_height() + fonts.label.volta.size() * VOLTA_TEXT_GAP;

    let mut voltas = Vec::new();

    for volta in &system.voltas {
        if volta.start >= volta.end {
            continue;
        }

        let start_x = system.pulse_offsets[volta.start] + lead;
        let end_x = system.pulse_offsets[volta.end - 1] + system.pulses[volta.end - 1].pulse_width;
        let text_x = start_x + style.left() + fonts.label.volta.size() * VOLTA_TEXT_INSET;

        let text = match &volta.label {
            Some(label) => format!("{}. {label}", volta.number),
            None => format!("{}.", volta.number),
        };

        voltas.push(PlacedVolta {
            start_x,
            end_x,
            text,
            text_x,
            text_y,
            line_y,
            open_end: volta.open_end,
        });
    }

    if voltas.is_empty() {
        return (Vec::new(), 0.0);
    }

    let height = line_y
        + fonts.label.volta.size() * VOLTA_END_TICK
        + fonts.label.volta.size() * VOLTA_BOTTOM_GAP
        + style.bottom();

    (voltas, height)
}

#[derive(Debug)]
pub struct PlacedVolta {
    pub start_x: f32,
    pub end_x: f32,
    pub text: String,
    pub text_x: f32,
    pub text_y: f32,
    pub line_y: f32,
    pub open_end: bool,
}
