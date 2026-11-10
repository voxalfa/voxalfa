use voxalfa_core::{
    ast::solfa::PulseAccent,
    data_types::{TimeSignature, Touch, Voice},
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
    pub tempo: Option<String>,
    pub is_bpm: bool,
    pub meter: Option<String>,
}

pub struct PulseStack<'a> {
    pub time: TimeSignature,
    pub accent: PulseAccent,
    pub measure_start: bool,
    pub section_end: bool,
    pub pulse_offset: usize,
    pub stack: Vec<PulseStackItem<'a>>,
    pub markers: Vec<FlatMarker>,
    pub volta: Option<FlatVolta>,
    pub max_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlatVolta {
    pub number: usize,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FlatMarker {
    pub kind: MarkerKind,
    pub glyph: MarkerGlyph,
    pub index: f32, // pulse relative index (0-1)
    pub voice: Option<Voice>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerKind {
    TimeSignature,
    Key,
    Tempo,
    Bpm,
    Dynamic,
    Touch,
    Jump,
    Mark,
}

#[derive(Debug, Clone)]
pub enum MarkerGlyph {
    Text(String),
    Touch(Touch),
    Segno,
}

pub struct PulseStackItem<'a> {
    pub voices: Vec<(Voice, &'a PulseIr)>,
    pub lyric_voices: Vec<Voice>,
    pub lyrics: Vec<ShiftedLyrics>,
}

#[derive(Debug)]
pub struct LyricLine {
    pub tokens: Vec<LyricToken>,
}

#[derive(Debug)]
pub struct ShiftedLyrics {
    pub tokens: Vec<LyricToken>, // tokens with absolute pulse indices
}

#[derive(Debug, Clone)]
pub struct LyricToken {
    pub start_pulse: usize,
    pub start_unit: f32,
    pub end_pulse: usize,
    pub end_unit: f32,
    pub text: String,
    pub underline: bool,
}

impl LyricToken {
    pub fn apply_offset(&self, pulse_offset: usize) -> Self {
        let mut token = self.clone();
        token.start_pulse += pulse_offset;
        token.end_pulse += pulse_offset;
        token
    }
}

#[derive(Debug)]
pub struct Measure {
    pub start: usize,
    pub end: usize,
}

pub struct System<'a> {
    pub pulses: Vec<SystemPulse<'a>>,
    pub time: TimeSignature,
    pub voices: Vec<Voice>,
    pub lyric_groups: Vec<SystemLyricGroup>,
    pub markers: Vec<SystemMarker>,
    pub dynamic_spans: Vec<SystemDynamicSpan>,
    pub voltas: Vec<SystemVolta>,
    pub pulse_offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicSpanKind {
    Crescendo,
    Decrescendo,
}

#[derive(Debug, Clone, Copy)]
pub struct FlatDynamicSpan {
    pub kind: DynamicSpanKind,
    pub start_pulse: usize,
    pub start_unit: f32,
    pub end_pulse: usize,
    pub end_unit: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct SystemDynamicSpan {
    pub kind: DynamicSpanKind,
    pub start: usize,
    pub start_unit: f32,
    pub end: usize,
    pub end_unit: f32,
    pub fits: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemVolta {
    pub start: usize, // fist pulse
    pub end: usize,   // last pulse + 1
    pub number: usize,
    pub label: Option<String>,
}

impl SystemVolta {
    pub fn new(start: usize, end: usize, volta: FlatVolta) -> Self {
        SystemVolta {
            start,
            end,
            number: volta.number,
            label: volta.label,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SystemMarker {
    pub pulse: usize,
    pub unit: f32,
    pub kind: MarkerKind,
    pub glyph: MarkerGlyph,
    pub voice: Option<Voice>,
}

pub struct SystemPulse<'a> {
    pub accent: PulseAccent,
    pub measure_start: bool,
    pub section_end: bool,
    pub pulse_width: f32,
    pub stack: Vec<SystemPulseStackItem<'a>>,
}

pub struct SystemPulseStackItem<'a> {
    pub voices: Vec<(Voice, &'a PulseIr)>,
}

pub struct SystemLyricGroup {
    pub voices: Vec<Voice>,
    pub verses: Vec<Vec<SystemLyricToken>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SystemLyricToken {
    pub start: usize,
    pub start_unit: f32,
    pub end: usize,
    pub end_unit: f32,
    pub text: String,
    pub underline: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SystemVoiceLyrics {
    pub verses: Vec<Vec<SystemLyricToken>>,
}

impl SystemVoiceLyrics {
    pub fn new(verse_count: usize) -> Self {
        Self {
            verses: vec![Vec::new(); verse_count],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.verses.iter().all(Vec::is_empty)
    }
}

pub struct ResolvedScore<'a> {
    pub header: HeaderView,
    pub systems: Vec<System<'a>>,
    pub lead_width: f32,
}
