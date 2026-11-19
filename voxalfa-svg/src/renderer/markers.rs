use voxalfa_core::{
    ast::directives::ElementStyle,
    data_types::{Touch, Voice},
    ir::solfa::{NoteKind, PulseIr},
};

use crate::{
    fonts::{FontMetrics, Fonts},
    model::{MarkerGlyph, MarkerKind, System, SystemMarker, SystemPulse, TextAnchor},
    options::RenderOptions,
};

use super::{
    constants::*,
    geometry::{prolonged_leading, pulse_offsets, pulse_usable_width, segno_width},
    text::note_text,
};

const FLOAT_ERROR: f32 = 0.001;

pub fn marker_rank(kind: MarkerKind) -> usize {
    match kind {
        MarkerKind::Touch => 0,
        MarkerKind::Dynamic => 1,
        MarkerKind::Mark | MarkerKind::Jump => 2,
        MarkerKind::TimeSignature | MarkerKind::Key | MarkerKind::Tempo | MarkerKind::Bpm => 3,
    }
}

pub fn touch_width(glyph: &MarkerGlyph, fonts: &Fonts) -> f32 {
    let size = fonts.touch.size() * TOUCH_SYMBOL_SCALE;

    match glyph {
        MarkerGlyph::Touch(Touch::Fermata) => size * FERMATA_WIDTH_SCALE,
        MarkerGlyph::Touch(Touch::Staccato) => size * STACCATO_WIDTH_SCALE,
        MarkerGlyph::Touch(Touch::Marcato) => size * MARCATO_WIDTH_SCALE,
        _ => 0.0,
    }
}

pub fn marker_font(kind: MarkerKind, fonts: &Fonts) -> &FontMetrics {
    match kind {
        MarkerKind::Key => &fonts.label.key,
        MarkerKind::Jump => &fonts.label.jump,
        MarkerKind::Tempo => &fonts.label.tempo,
        MarkerKind::Bpm => &fonts.label.bpm,
        MarkerKind::TimeSignature => &fonts.label.time,
        MarkerKind::Touch => &fonts.touch,
        MarkerKind::Mark => &fonts.label.mark,
        MarkerKind::Dynamic => &fonts.label.dynamic,
    }
}

pub fn spread_markers(markers: &mut [PlacedMarker], gap: f32) {
    markers.sort_by(|lhs, rhs| lhs.x.total_cmp(&rhs.x));

    let mut right_edge = f32::NEG_INFINITY;

    for marker in markers {
        let left = match marker.anchor {
            TextAnchor::Start => marker.x,
            TextAnchor::Middle => marker.x - marker.width / 2.0,
            TextAnchor::End => marker.x - marker.width,
        };

        let shift = (right_edge + gap - left).max(0.0);

        marker.x += shift;
        right_edge = left + marker.width + shift;
    }
}

pub fn marker_style(kind: MarkerKind, options: &RenderOptions) -> Option<&ElementStyle> {
    match kind {
        MarkerKind::Key => Some(&options.elements.key_label),
        MarkerKind::Jump => Some(&options.elements.jump),
        MarkerKind::Tempo => Some(&options.elements.tempo),
        MarkerKind::Bpm => Some(&options.elements.bpm),
        MarkerKind::Mark => Some(&options.elements.mark),
        MarkerKind::Dynamic => Some(&options.elements.dynamic),
        MarkerKind::TimeSignature => Some(&options.elements.time_label),
        MarkerKind::Touch => None,
    }
}

pub fn anchor_x(x: f32, anchor: TextAnchor, style: &ElementStyle) -> f32 {
    match anchor {
        TextAnchor::Start => x + style.left(),
        TextAnchor::Middle => x + (style.left() - style.right()) / 2.0,
        TextAnchor::End => x - style.right(),
    }
}

pub fn label_at(
    markers: &[PlacedMarker],
    pulse: usize,
    unit: f32,
    voice: Option<Voice>,
) -> Option<&PlacedMarker> {
    markers.iter().find(|marker| {
        marker.kind == MarkerKind::Dynamic
            && marker.voice == voice
            && marker.pulse == pulse
            && (marker.unit - unit).abs() < FLOAT_ERROR
    })
}

pub fn marker_bounds(marker: &PlacedMarker) -> (f32, f32) {
    match marker.anchor {
        TextAnchor::Start => (marker.x, marker.x + marker.width),
        TextAnchor::Middle => (marker.x - marker.width / 2.0, marker.x + marker.width / 2.0),
        TextAnchor::End => (marker.x - marker.width, marker.x),
    }
}

pub fn find_voice<'a>(pulse: &SystemPulse<'a>, voice: Voice) -> Option<&'a PulseIr> {
    pulse.stack.iter().find_map(|sub| {
        sub.voices
            .iter()
            .find(|(current, _)| *current == voice)
            .map(|(_, voice_pulse)| *voice_pulse)
    })
}

impl MarkerColumn {
    fn new(column: Vec<PlacedMarker>) -> Self {
        let mut ranks: [Vec<PlacedMarker>; 4] = std::array::from_fn(|_| Vec::new());

        for marker in column {
            ranks[marker_rank(marker.kind)].push(marker);
        }

        Self { ranks }
    }

    fn spread(&mut self, fonts: &Fonts) {
        for row in &mut self.ranks {
            if row.is_empty() {
                continue;
            }

            let font = row
                .iter()
                .map(|marker| marker_font(marker.kind, fonts))
                .max_by(|lhs, rhs| lhs.size().total_cmp(&rhs.size()))
                .unwrap_or(&fonts.label.key);

            spread_markers(row, font.size() * MARKER_HORIZONTAL_GAP);
        }
    }

    fn below_height(&self, rank_heights: &[f32; 4]) -> f32 {
        self.ranks
            .iter()
            .take(2)
            .enumerate()
            .filter(|(_, row)| !row.is_empty())
            .map(|(rank, _)| rank_heights[rank])
            .sum()
    }

    fn above_height(&self, rank_heights: &[f32; 4]) -> f32 {
        self.ranks
            .iter()
            .skip(2)
            .enumerate()
            .filter(|(_, row)| !row.is_empty())
            .map(|(rank, _)| rank_heights[rank + 2])
            .sum()
    }

    fn place(&mut self, fonts: &Fonts, rank_heights: &[f32; 4], bands: &MarkerBands) {
        for rank in 0..self.ranks.len() {
            if self.ranks[rank].is_empty() {
                continue;
            }

            let height = rank_heights[rank];

            let offset: f32 = if bands.has_volta {
                let start = if rank < 2 { 0 } else { 2 };

                self.ranks[start..rank]
                    .iter()
                    .enumerate()
                    .filter(|(_, row)| !row.is_empty())
                    .map(|(index, _)| rank_heights[start + index])
                    .sum()
            } else {
                self.ranks[..rank]
                    .iter()
                    .enumerate()
                    .filter(|(_, row)| !row.is_empty())
                    .map(|(index, _)| rank_heights[index])
                    .sum()
            };

            let base = match (bands.has_volta, rank < 2) {
                (true, true) => bands.below,
                (true, false) => bands.above,
                (false, _) => bands.all,
            };

            let top = base - offset - height;

            for marker in &mut self.ranks[rank] {
                let font = marker_font(marker.kind, fonts);
                marker.y = top + (height - font.line_height()) / 2.0 + font.ascent();
            }
        }
    }

    fn into_markers(self) -> impl Iterator<Item = PlacedMarker> {
        self.ranks.into_iter().flatten()
    }
}

impl<'a, 'b> MarkerLayout<'a, 'b> {
    pub fn new(
        system: &'a System<'b>,
        fonts: &'a Fonts,
        lead: f32,
        options: &'a RenderOptions,
    ) -> Self {
        let xs = pulse_offsets(system);
        let usable_widths = system
            .pulses
            .iter()
            .map(|pulse| pulse_usable_width(pulse, fonts, lead))
            .collect();

        Self {
            system,
            fonts,
            options,
            lead,
            xs,
            usable_widths,
        }
    }

    pub fn layout(&self, volta_height: f32, volta_line: f32) -> MarkerLayoutResult {
        if self.system.markers.is_empty() {
            return MarkerLayoutResult {
                markers: Vec::new(),
                height: volta_height,
                volta_offset: 0.0,
            };
        }

        self.stack(self.columns(), volta_height, volta_line)
    }

    fn columns(&self) -> Vec<Vec<PlacedMarker>> {
        let mut markers = self.system.markers.iter().collect::<Vec<_>>();

        markers.sort_by(|lhs, rhs| {
            lhs.pulse
                .cmp(&rhs.pulse)
                .then(lhs.unit.total_cmp(&rhs.unit))
        });

        let same_note = |lhs: &PlacedMarker, rhs: &PlacedMarker| {
            lhs.pulse == rhs.pulse && (lhs.unit - rhs.unit).abs() < FLOAT_ERROR
        };

        let mut columns: Vec<Vec<PlacedMarker>> = Vec::new();

        for marker in markers {
            let placed = self.place(marker);

            match columns.last_mut() {
                Some(column) if same_note(&column[0], &placed) => column.push(placed),
                _ => columns.push(vec![placed]),
            }
        }

        columns
    }

    fn place(&self, marker: &SystemMarker) -> PlacedMarker {
        let font = marker_font(marker.kind, self.fonts);

        let width = match &marker.glyph {
            MarkerGlyph::Text(text) => font.width(text),
            MarkerGlyph::Segno => segno_width(self.fonts),
            MarkerGlyph::Touch(_) => touch_width(&marker.glyph, self.fonts),
        };

        let fallback_x =
            self.xs[marker.pulse] + marker.unit * self.usable_widths[marker.pulse] + self.lead;

        let x = match marker.kind {
            MarkerKind::Dynamic | MarkerKind::Touch => self
                .note_center(marker.pulse, marker.unit)
                .unwrap_or(fallback_x),
            MarkerKind::Mark if matches!(&marker.glyph, MarkerGlyph::Segno) => self
                .note_center(marker.pulse, marker.unit)
                .unwrap_or(fallback_x),
            _ => fallback_x,
        };

        let anchor = match marker.kind {
            MarkerKind::TimeSignature => TextAnchor::Middle,
            MarkerKind::Key | MarkerKind::Tempo | MarkerKind::Bpm => TextAnchor::Start,
            MarkerKind::Dynamic | MarkerKind::Touch => TextAnchor::Middle,
            MarkerKind::Jump => TextAnchor::End,
            MarkerKind::Mark if marker.unit >= 0.5 => TextAnchor::End,
            MarkerKind::Mark => TextAnchor::Middle,
        };

        let x = marker_style(marker.kind, self.options)
            .map(|style| anchor_x(x, anchor, style))
            .unwrap_or(x);

        PlacedMarker {
            kind: marker.kind,
            glyph: marker.glyph.clone(),
            width,
            x,
            y: 0.0,
            anchor,
            pulse: marker.pulse,
            unit: marker.unit,
            voice: marker.voice,
        }
    }

    fn note_center(&self, pulse_id: usize, unit: f32) -> Option<f32> {
        let pulse = self.system.pulses.get(pulse_id)?;
        let usable_width = self.usable_widths[pulse_id];

        for voice in &self.system.voices {
            let Some(voice_pulse) = find_voice(pulse, *voice) else {
                continue;
            };

            let total = voice_pulse
                .columns
                .iter()
                .map(|column| column.duration as u32)
                .sum::<u32>()
                .max(1);

            let mut cursor = 0u32;

            for (index, column) in voice_pulse.columns.iter().enumerate() {
                let start = cursor as f32 / total as f32;

                cursor += column.duration as u32;

                let end = cursor as f32 / total as f32;
                let is_last = index + 1 == voice_pulse.columns.len();

                if unit + FLOAT_ERROR >= start
                    && (unit < end - FLOAT_ERROR || (is_last && unit <= end + FLOAT_ERROR))
                {
                    if matches!(column.note, NoteKind::EmptyNote) {
                        break;
                    }

                    let (text, _) = note_text(&column.note);
                    let glyph_width = self.fonts.text.note.width(&text);
                    let leading = match column.note {
                        NoteKind::ProlongedNote => self.lead + prolonged_leading(self.fonts),
                        _ => self.lead,
                    };

                    return Some(
                        self.xs[pulse_id] + start * usable_width + leading + glyph_width / 2.0,
                    );
                }
            }
        }

        None
    }

    fn stack(
        &self,
        columns: Vec<Vec<PlacedMarker>>,
        volta_height: f32,
        volta_line: f32,
    ) -> MarkerLayoutResult {
        let rank_heights = self.rank_heights(&columns);
        let mut columns = columns
            .into_iter()
            .map(MarkerColumn::new)
            .collect::<Vec<_>>();

        for column in &mut columns {
            column.spread(self.fonts);
        }

        let max_below = columns
            .iter()
            .map(|column| column.below_height(&rank_heights))
            .fold(0.0, f32::max);
        let max_above = columns
            .iter()
            .map(|column| column.above_height(&rank_heights))
            .fold(0.0, f32::max);

        let has_volta = volta_height > 0.0;

        let stack_height = if has_volta {
            max_above + volta_height + max_below
        } else {
            max_below + max_above
        };

        let volta_gap = self.fonts.label.volta.size() * VOLTA_MARKER_GAP;

        let volta_shift = if has_volta && max_above > 0.0 {
            volta_line - volta_gap
        } else {
            0.0
        };

        let above_bottom = max_above + volta_shift;

        let mut markers = Vec::new();

        let bands = MarkerBands {
            below: max_above + volta_height + max_below,
            above: above_bottom,
            all: stack_height,
            has_volta,
        };

        for mut column in columns {
            column.place(self.fonts, &rank_heights, &bands);
            markers.extend(column.into_markers());
        }

        if volta_shift != 0.0 {
            for marker in &mut markers {
                marker.y -= volta_shift;
            }
        }

        let height = if stack_height == 0.0 {
            volta_height
        } else {
            stack_height + self.fonts.label.key.size() * MARKER_BOTTOM_GAP - volta_shift
        };

        let volta_offset = if has_volta {
            max_above - volta_shift
        } else {
            0.0
        };

        MarkerLayoutResult {
            markers,
            height,
            volta_offset,
        }
    }

    fn rank_heights(&self, columns: &[Vec<PlacedMarker>]) -> [f32; 4] {
        let mut heights: [f32; 4] = [0.0; 4];

        for marker in columns.iter().flatten() {
            let font = marker_font(marker.kind, self.fonts);
            let padding = marker_style(marker.kind, self.options)
                .map(|style| style.top() + style.bottom())
                .unwrap_or(0.0);

            let rank = marker_rank(marker.kind);
            heights[rank] = heights[rank].max(font.line_height() + padding);
        }

        heights
    }
}

#[derive(Debug, Clone)]
pub struct PlacedMarker {
    pub kind: MarkerKind,
    pub glyph: MarkerGlyph,
    pub width: f32,
    pub anchor: TextAnchor,
    pub x: f32,
    pub y: f32,
    pub pulse: usize,
    pub unit: f32,
    pub voice: Option<Voice>,
}

#[derive(Debug)]
pub struct MarkerBands {
    pub below: f32,
    pub above: f32,
    pub all: f32,
    pub has_volta: bool,
}

#[derive(Debug)]
pub struct MarkerColumn {
    pub ranks: [Vec<PlacedMarker>; 4],
}

#[derive(Debug)]
pub struct MarkerLayoutResult {
    pub markers: Vec<PlacedMarker>,
    pub height: f32,
    pub volta_offset: f32,
}

pub struct MarkerLayout<'a, 'b> {
    pub system: &'a System<'b>,
    pub fonts: &'a Fonts,
    pub options: &'a RenderOptions,
    pub lead: f32,
    pub xs: Vec<f32>,
    pub usable_widths: Vec<f32>,
}
