mod constants;
mod defs;
mod geometry;
mod header;
mod layout;
mod markers;
mod primitives;
mod system;
mod text;

use std::fmt::Write;

use voxalfa_core::{data_types::TimeSignature, output::FinalOutput};

use crate::{
    error::Result,
    fonts::{FontFaces, Fonts},
    model::System,
    options::RenderOptions,
    resolver,
};

use defs::emit_defs;
use geometry::system_width;
use header::{emit_header, layout_header};
use layout::{SystemLayout, layout_system};
use system::{SystemPlacement, emit_system};

use constants::{BRACE_PADDING_LEFT, BRACE_PADDING_RIGHT};

const INITIAL_SVG_CAPACITY: usize = 32 * 1024;

pub fn render_pages(
    data: &FinalOutput,
    options: &RenderOptions,
    faces: &FontFaces,
) -> Result<Vec<String>> {
    let mut options = options.clone();

    options.apply_directives(&data.header.directives);

    let options = &options;
    let fonts = Fonts::with_faces(options, faces)?;
    let brace_padding = brace_padding(&fonts);
    let content_width = options.printable_width() - brace_padding.0 - brace_padding.1;
    let score = resolver::resolve(data, &fonts, options, content_width)?;
    let header = layout_header(&score.header, &fonts, options);

    let system_layouts = score
        .systems
        .iter()
        .map(|system| layout_system(system, &fonts, score.lead_width, options))
        .collect::<Vec<_>>();

    let pages = paginate(&system_layouts, header.height, options);
    let page_count = pages.len().max(1);
    let max_pulses_per_line = max_pulses_per_line(&score.systems, content_width);

    let mut result = Vec::with_capacity(pages.len());
    let mut current_offset = 0.0;
    let mut currente_time: Option<TimeSignature> = None;

    for (page_id, page) in pages.iter().enumerate() {
        let mut svg = String::with_capacity(INITIAL_SVG_CAPACITY);

        writeln!(
            svg,
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {:.2} {:.2}" width="{:.2}px" height="{:.2}px">"#,
            options.page.width, options.page.height, options.page.width, options.page.height,
        )?;

        emit_defs(&mut svg, &fonts, options)?;

        writeln!(
            svg,
            r##"<rect x="0" y="0" width="{:.2}" height="{:.2}" fill="#ffffff"/>"##,
            options.page.width, options.page.height,
        )?;

        if page_id == 0 {
            emit_header(&mut svg, &header, options)?;
        }

        for placed in &page.systems {
            let system = &score.systems[placed.index];
            let measure_pulses = system.time.top as usize;

            if currente_time != Some(system.time) {
                let overflow = system.pulses.len() + measure_pulses > max_pulses_per_line;

                if overflow {
                    let width = system_width(system);
                    current_offset = (content_width - width) / 2.0;
                }

                currente_time = Some(system.time);
            }

            let system = &score.systems[placed.index];

            let placement = SystemPlacement {
                system,
                layout: &system_layouts[placed.index],
                y: placed.y,
                body_offset: current_offset,
                brace_padding_left: brace_padding.0,
                is_last_system: placed.index + 1 == score.systems.len(),
            };

            emit_system(&mut svg, &fonts, &score, placement, options)?;
        }

        if page_count > 1 {
            let style = &options.elements.text.footer;
            let x = options.page.width / 2.0 + (style.left() - style.right()) / 2.0;
            let y = options.page.height - options.page_margin.bottom / 2.0 - style.bottom()
                + style.top();

            writeln!(
                svg,
                r#"<text x="{x:.2}" y="{y:.2}" text-anchor="middle" class="footer-text">{}</text>"#,
                page_id + 1,
            )?;
        }

        writeln!(svg, "</svg>")?;
        result.push(svg);
    }

    Ok(result)
}

fn brace_padding(fonts: &Fonts) -> (f32, f32) {
    let size = fonts.text.note.size();
    (size * BRACE_PADDING_LEFT, size * BRACE_PADDING_RIGHT)
}

fn max_pulses_per_line(systems: &[System<'_>], printable_width: f32) -> usize {
    let pulse_width = systems
        .first()
        .and_then(|system| system.pulses.first())
        .map(|pulse| pulse.pulse_width)
        .unwrap_or(0.0);

    if pulse_width <= 0.0 {
        0
    } else {
        (printable_width / pulse_width).floor() as usize
    }
}

fn paginate(layouts: &[SystemLayout], header_height: f32, options: &RenderOptions) -> Vec<Page> {
    let bottom = options.page.height - options.page_margin.bottom;

    let mut pages = vec![Page::default()];
    let mut y = options.page_margin.top;

    y += header_height;

    if header_height > 0.0 {
        y += options.header_gap;
    }

    for (index, layout) in layouts.iter().enumerate() {
        if y + layout.height > bottom && !pages.last().unwrap().systems.is_empty() {
            pages.push(Page::default());
            y = options.page_margin.top;
        }

        pages
            .last_mut()
            .unwrap()
            .systems
            .push(PlacedSystem { index, y });

        y += layout.height + options.system_gap;
    }

    pages
}

#[derive(Debug, Default)]
struct Page {
    systems: Vec<PlacedSystem>,
}

#[derive(Debug)]
struct PlacedSystem {
    index: usize,
    y: f32,
}
