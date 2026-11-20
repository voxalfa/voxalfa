use voxalfa_core::{ast::directives::ElementStyle, data_types::Touch};

use crate::{
    fonts::{FontMetrics, Fonts},
    model::{MarkerGlyph, MarkerKind, System, SystemMarker, TextAnchor, VoiceInstance},
    options::RenderOptions,
};

use super::{
    constants::*,
    geometry::{FLOAT_ERROR, SystemGeometry, segno_width},
    voltas::PlacedVolta,
};

pub fn layout_markers(
    system: &System,
    fonts: &Fonts,
    lead: f32,
    options: &RenderOptions,
    voltas: &[PlacedVolta],
    volta_height: f32,
) -> MarkerLayout {
    MarkerStacker::new(system, fonts, lead, options, voltas, volta_height).layout()
}

fn marker_rank(kind: MarkerKind) -> usize {
    match kind {
        MarkerKind::Touch => 0,
        MarkerKind::Dynamic => 1,
        MarkerKind::Mark | MarkerKind::Jump => 2,
        MarkerKind::TimeSignature | MarkerKind::Key | MarkerKind::Tempo | MarkerKind::Bpm => 3,
    }
}

fn touch_width(glyph: &MarkerGlyph, fonts: &Fonts) -> f32 {
    let size = fonts.touch.size() * TOUCH_SYMBOL_SCALE;

    match glyph {
        MarkerGlyph::Touch(Touch::Fermata) => size * FERMATA_WIDTH_SCALE,
        MarkerGlyph::Touch(Touch::Staccato) => size * STACCATO_WIDTH_SCALE,
        MarkerGlyph::Touch(Touch::Marcato) => size * MARCATO_WIDTH_SCALE,
        _ => 0.0,
    }
}

fn marker_font(kind: MarkerKind, fonts: &Fonts) -> &FontMetrics {
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

fn spread_markers(markers: &mut [PlacedMarker], gap: f32) {
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

fn marker_style(kind: MarkerKind, options: &RenderOptions) -> Option<&ElementStyle> {
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

pub fn label_at(
    markers: &[PlacedMarker],
    pulse: usize,
    unit: f32,
    voice: Option<VoiceInstance>,
) -> Option<&PlacedMarker> {
    markers.iter().find(|marker| {
        marker.kind == MarkerKind::Dynamic
            && marker.voice.map(|voice| voice.id) == voice.map(|voice| voice.id)
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

#[derive(Debug)]
enum MarkerStack {
    Continuous {
        bottom: f32,
    },
    Split {
        above_bottom: f32,
        below_bottom: f32,
    },
}

#[derive(Debug)]
struct MarkerColumn {
    ranks: [Vec<PlacedMarker>; 4],
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

    fn band_height(&self, ranks: std::ops::Range<usize>, rank_heights: &[f32; 4]) -> f32 {
        self.ranks[ranks.clone()]
            .iter()
            .zip(&rank_heights[ranks])
            .filter(|(row, _)| !row.is_empty())
            .map(|(_, height)| *height)
            .sum()
    }

    fn overlaps(&self, spans: &[(f32, f32)]) -> bool {
        self.ranks.iter().flatten().any(|marker| {
            let (left, right) = match marker.anchor {
                TextAnchor::Start => (marker.x, marker.x + marker.width),
                TextAnchor::Middle => {
                    (marker.x - marker.width / 2.0, marker.x + marker.width / 2.0)
                }
                TextAnchor::End => (marker.x - marker.width, marker.x),
            };

            spans
                .iter()
                .any(|(start, end)| left < *end && right > *start)
        })
    }

    fn place(&mut self, fonts: &Fonts, rank_heights: &[f32; 4], stack: &MarkerStack) {
        for rank in 0..self.ranks.len() {
            if self.ranks[rank].is_empty() {
                continue;
            }

            let height = rank_heights[rank];

            let (group_start, base) = match stack {
                MarkerStack::Continuous { bottom } => (0, *bottom),
                MarkerStack::Split { above_bottom, .. } if rank >= 2 => (2, *above_bottom),
                MarkerStack::Split { below_bottom, .. } => (0, *below_bottom),
            };

            let offset = self.band_height(group_start..rank, rank_heights);
            let top = base - offset - height;

            for marker in &mut self.ranks[rank] {
                let font = marker_font(marker.kind, fonts);
                marker.y = font.centered_baseline(top, height);
            }
        }
    }

    fn into_markers(self) -> impl Iterator<Item = PlacedMarker> {
        self.ranks.into_iter().flatten()
    }
}

struct MarkerStacker<'a, 'b> {
    system: &'a System<'b>,
    fonts: &'a Fonts,
    options: &'a RenderOptions,
    geometry: SystemGeometry<'a, 'b>,
    volta_height: f32,
    volta_line: f32,
    volta_spans: Vec<(f32, f32)>,
}

impl<'a, 'b> MarkerStacker<'a, 'b> {
    fn new(
        system: &'a System<'b>,
        fonts: &'a Fonts,
        lead: f32,
        options: &'a RenderOptions,
        voltas: &[PlacedVolta],
        volta_height: f32,
    ) -> Self {
        let volta_line = voltas.first().map(|volta| volta.line_y).unwrap_or(0.0);

        let volta_spans = voltas
            .iter()
            .map(|volta| (volta.start_x, volta.end_x))
            .collect();

        Self {
            system,
            fonts,
            options,
            geometry: SystemGeometry::new(system, fonts, lead),
            volta_height,
            volta_line,
            volta_spans,
        }
    }

    fn layout(&self) -> MarkerLayout {
        if self.system.markers.is_empty() {
            return MarkerLayout {
                markers: Vec::new(),
                height: self.volta_height,
                volta_offset: 0.0,
            };
        }

        let global_voice = self.system.voices.first().map(|voice| voice.id);

        let (voice_dynamics, stacked): (Vec<_>, Vec<_>) =
            self.system.markers.iter().partition(|marker| {
                marker.kind == MarkerKind::Dynamic
                    && marker
                        .voice
                        .is_some_and(|voice| Some(voice.id) != global_voice)
            });

        let mut layout = if stacked.is_empty() {
            MarkerLayout {
                markers: Vec::new(),
                height: self.volta_height,
                volta_offset: 0.0,
            }
        } else {
            self.stack(self.columns(stacked))
        };

        layout
            .markers
            .extend(voice_dynamics.into_iter().map(|marker| self.place(marker)));

        layout
    }

    fn columns(&self, mut markers: Vec<&SystemMarker>) -> Vec<Vec<PlacedMarker>> {
        markers.sort_by(|lhs, rhs| {
            lhs.pulse
                .cmp(&rhs.pulse)
                .then(lhs.unit.total_cmp(&rhs.unit))
        });

        let mut columns: Vec<Vec<PlacedMarker>> = Vec::new();

        for marker in markers {
            let placed = self.place(marker);

            match columns.last_mut() {
                Some(column) if column[0].same_note(&placed) => column.push(placed),
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

        let fallback_x = self.geometry.note_x(marker.pulse, marker.unit);

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
            .map(|style| anchor.align_x(x, style))
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
        self.geometry.note_center(pulse_id, unit, None)
    }

    fn stack(&self, columns: Vec<Vec<PlacedMarker>>) -> MarkerLayout {
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
            .map(|column| column.band_height(0..2, &rank_heights))
            .fold(0.0, f32::max);

        let max_above = columns
            .iter()
            .map(|column| column.band_height(2..4, &rank_heights))
            .fold(0.0, f32::max);

        let volta_below = columns
            .iter()
            .filter(|column| column.overlaps(&self.volta_spans))
            .map(|column| column.band_height(0..2, &rank_heights))
            .fold(0.0, f32::max);

        let has_volta = self.volta_height > 0.0;
        let stack_height = max_above + max_below + if has_volta { self.volta_height } else { 0.0 };
        let volta_gap = self.fonts.label.volta.size() * VOLTA_MARKER_GAP;

        let volta_shift = if has_volta && max_above > 0.0 {
            self.volta_line - volta_gap
        } else {
            0.0
        };

        let marker_bottom = if has_volta {
            stack_height - volta_shift
        } else {
            stack_height
        };

        let stack = if has_volta {
            MarkerStack::Split {
                above_bottom: max_above,
                below_bottom: marker_bottom,
            }
        } else {
            MarkerStack::Continuous {
                bottom: marker_bottom,
            }
        };

        let mut markers = Vec::new();

        for mut column in columns {
            column.place(self.fonts, &rank_heights, &stack);
            markers.extend(column.into_markers());
        }

        let height = marker_bottom + self.fonts.label.key.size() * MARKER_BOTTOM_GAP;

        let volta_offset = if has_volta {
            max_above + max_below - volta_below - volta_shift
        } else {
            0.0
        };

        MarkerLayout {
            markers,
            height,
            volta_offset,
        }
    }

    fn rank_heights(&self, columns: &[Vec<PlacedMarker>]) -> [f32; 4] {
        let mut heights: [f32; 4] = [0.0; 4];

        for marker in columns.iter().flatten() {
            let font = marker_font(marker.kind, self.fonts);
            let rank = marker_rank(marker.kind);

            let padding = marker_style(marker.kind, self.options)
                .map(|style| style.top() + style.bottom())
                .unwrap_or(0.0);

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
    pub voice: Option<VoiceInstance>,
}

impl PlacedMarker {
    fn same_note(&self, other: &Self) -> bool {
        self.pulse == other.pulse && (self.unit - other.unit).abs() < FLOAT_ERROR
    }
}

#[derive(Debug)]
pub struct MarkerLayout {
    pub markers: Vec<PlacedMarker>,
    pub height: f32,
    pub volta_offset: f32,
}
