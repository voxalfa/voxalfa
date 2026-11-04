use voxalfa_core::{
    ast::solfa::PulseAccent,
    data_types::{TimeSignature, Voice},
    ir::solfa::PulseIr,
};

#[derive(Debug, Default)]
pub struct HeaderView {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub composers: Vec<String>,
    pub language: Option<String>,
    #[allow(unused)]
    pub description: Option<String>,
    pub key: Option<String>,
    pub time: Option<String>,
    #[allow(unused)]
    pub tempo: Option<String>,
    pub meter: Option<String>,
}

pub(crate) struct FlatPulse<'a> {
    pub time: TimeSignature,
    pub accent: PulseAccent,
    pub measure_start: bool,
    pub section_end: bool,
    pub grid_offset: usize, // number of pulse before strong pulse
    pub subsections: Vec<FlatSubPulse<'a>>,
    pub markers: Vec<FlatMarker>,
    pub volta: Option<FlatVolta>,
    pub max_columns: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlatVolta {
    pub number: usize,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct FlatMarker {
    pub kind: MarkerKind,
    pub glyph: MarkerGlyph,
    /// Position within the pulse, from 0 (start) to 1 (end).
    pub unit: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarkerKind {
    TimeSignature,
    Key,
    Tempo,
    TempoNumber,
    Dynamic,
    Touch,
    Jump,
    Mark,
}

#[derive(Debug, Clone)]
pub(crate) enum MarkerGlyph {
    Text(String),
    Segno,
    Fermata,
    Staccato,
    Marcato,
}

pub(crate) struct FlatSubPulse<'a> {
    pub voices: Vec<(Voice, &'a PulseIr)>,
    pub lyric_voices: Vec<Voice>,
    pub lyrics: Vec<Vec<LyricToken>>,
}

#[derive(Debug, Clone)]
pub(crate) struct LyricToken {
    pub start_pulse: usize,
    pub start_unit: f32,
    pub end_pulse: usize,
    pub end_unit: f32,
    pub text: String,
    pub underline: bool,
}

pub(crate) struct System<'a> {
    pub pulses: Vec<SystemPulse<'a>>,
    pub voices: Vec<Voice>,
    pub lyric_groups: Vec<SystemLyricGroup>,
    pub markers: Vec<SystemMarker>,
    pub dynamic_spans: Vec<SystemDynamicSpan>,
    pub voltas: Vec<SystemVolta>,
    pub grid_offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DynamicSpanKind {
    Crescendo,
    Decrescendo,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FlatDynamicSpan {
    pub kind: DynamicSpanKind,
    pub start_pulse: usize,
    pub start_unit: f32,
    pub end_pulse: usize,
    pub end_unit: f32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SystemDynamicSpan {
    pub kind: DynamicSpanKind,
    pub start: usize,
    pub start_unit: f32,
    pub end: usize,
    pub end_unit: f32,
    pub fits: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SystemVolta {
    /// First pulse of the bracket.
    pub start: usize,
    /// One past the last pulse of the bracket.
    pub end: usize,
    pub number: usize,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct SystemMarker {
    pub pulse: usize,
    pub unit: f32,
    pub kind: MarkerKind,
    pub glyph: MarkerGlyph,
}

pub(crate) struct SystemPulse<'a> {
    pub accent: PulseAccent,
    pub measure_start: bool,
    pub section_end: bool,
    pub pulse_width: f32,
    pub subsections: Vec<SystemSubPulse<'a>>,
}

pub(crate) struct SystemSubPulse<'a> {
    pub voices: Vec<(Voice, &'a PulseIr)>,
}

pub(crate) struct SystemLyricGroup {
    pub voices: Vec<Voice>,
    pub verses: Vec<Vec<SystemLyricToken>>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SystemLyricToken {
    pub start: usize,
    pub start_unit: f32,
    pub end: usize,
    pub end_unit: f32,
    pub text: String,
    pub underline: bool,
}

pub(crate) struct ResolvedScore<'a> {
    pub header: HeaderView,
    pub systems: Vec<System<'a>>,
    pub lead_width: f32,
}
