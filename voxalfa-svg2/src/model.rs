use voxalfa_core::{ast::solfa::PulseAccent, data_types::TimeSignature, ir::solfa::PulseIr};

/// Header information that the renderer actually draws.
#[derive(Debug, Default)]
pub struct HeaderView {
    pub title: Option<String>,
    pub authors: Vec<String>,
    #[allow(dead_code)]
    pub description: Option<String>,
    pub key: Option<String>,
    pub time: Option<String>,
    pub tempo: Option<String>,
    pub meter: Option<String>,
}

/// One pulse of the body, flattened across all sections.
///
/// A section boundary is irrelevant for layout: pulses are laid out in the
/// order they appear in the source so measures can span sections.
pub(crate) struct FlatPulse<'a> {
    pub time: TimeSignature,
    pub accent: PulseAccent,
    pub measure_start: bool,
    /// True for the last pulse of a source solfa line. The renderer turns it
    /// into a double barline.
    pub section_end: bool,
    /// Number of pulses before the first strong pulse of this time-signature
    /// group. It is used to indent subsequent systems so measures stay
    /// vertically aligned when the partition starts with an anacrusis.
    pub grid_offset: usize,
    pub subsections: Vec<FlatSubPulse<'a>>,
    pub max_columns: usize,
}

/// The pulse data for one sub-section (one voice group).
pub(crate) struct FlatSubPulse<'a> {
    pub voices: Vec<(voxalfa_core::data_types::Voice, &'a PulseIr)>,
    /// Every voice declared by this sub-section, even when a voice line has
    /// fewer pulses than the others.
    pub lyric_voices: Vec<voxalfa_core::data_types::Voice>,
    /// Tokens grouped by verse. Only tokens that *start* on this pulse are
    /// stored here; a token may span into later pulses.
    pub lyrics: Vec<Vec<LyricToken>>,
}

/// A resolved lyric token.
///
/// Positions are expressed as `(pulse, unit)` where `unit` is a fraction in
/// `[0, 1]` of the pulse duration. This makes the model independent of the
/// final pixel geometry.
#[derive(Debug, Clone)]
pub(crate) struct LyricToken {
    pub start_pulse: usize,
    pub start_unit: f32,
    pub end_pulse: usize,
    pub end_unit: f32,
    pub span: usize,
    pub text: String,
    pub placeholder: bool,
    pub underline: bool,
}

/// A finished system: a horizontal line of music containing one or more
/// complete measures.
pub(crate) struct System<'a> {
    pub pulses: Vec<SystemPulse<'a>>,
    /// All voices in the system, in the order they first appear.
    pub voices: Vec<voxalfa_core::data_types::Voice>,
    pub lyric_groups: Vec<SystemLyricGroup>,
    /// Number of empty grid pulses added to the left of this system.
    pub grid_offset: usize,
}

pub(crate) struct SystemPulse<'a> {
    pub accent: PulseAccent,
    pub measure_start: bool,
    pub section_end: bool,
    pub pulse_width: f32,
    pub subsections: Vec<SystemSubPulse<'a>>,
}

pub(crate) struct SystemSubPulse<'a> {
    pub voices: Vec<(voxalfa_core::data_types::Voice, &'a PulseIr)>,
}

/// One lyric line shared by a consecutive run of voices.
///
/// A voice can collect lyrics from several sub-sections. Voices with the same
/// collected lyrics are merged into one line, which is what makes a split
/// (`++`) system collapse back onto the same voice rows.
pub(crate) struct SystemLyricGroup {
    pub voices: Vec<voxalfa_core::data_types::Voice>,
    /// One token list per verse. The renderer prints one row per verse.
    pub verses: Vec<Vec<SystemLyricToken>>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SystemLyricToken {
    pub start: usize,
    pub start_unit: f32,
    pub end: usize,
    pub end_unit: f32,
    pub text: String,
    #[allow(dead_code)]
    pub placeholder: bool,
    pub underline: bool,
}

pub(crate) struct ResolvedScore<'a> {
    pub header: HeaderView,
    pub systems: Vec<System<'a>>,
    #[allow(dead_code)]
    pub col_width: f32,
    pub lead_width: f32,
}
