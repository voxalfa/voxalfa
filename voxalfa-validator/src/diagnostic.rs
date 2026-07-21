use thiserror::Error;

use crate::{
    ast::{solfa::PulseAccent, types::Voice},
    ts_utils::range::Range,
};

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub kind: DiagnosticKind,
    pub range: Range,
}

#[derive(Debug, Error, Clone)]
pub enum DiagnosticKind {
    #[error("syntax error")]
    SyntaxError,
    #[error("missing '{0}'")]
    Missing(String),
    #[error("invalid UTF-8")]
    InvalidUTF8(#[from] std::str::Utf8Error),
    #[error("reassignment of key '{0}'")]
    KeyReassignment(String, Range),
    #[error("unknown metadata field '{0}'")]
    UnknownField(String),
    #[error("unknown parameter '{0}'")]
    UnknownParameter(String),
    #[error("expected {0}, got {1}")]
    ExpectedType(&'static str, &'static str),
    #[error("invalid {0}")]
    InvalidType(&'static str),
    #[error("invalid time signature, expected two non-null integers")]
    InvalidTimeSignature,
    #[error("invalid dynamic identifier '{0}'")]
    InvalidDynamic(String),
    #[error("invalid dynamic parameters, expected {0} numbers")]
    InvalidDynamicParams(usize),
    #[error("invalid voice '{0}'")]
    InvalidVoice(String),
    #[error("undefined voice '{0:?}'")]
    UndefinedVoice(Voice, Range),
    #[error("expected '{0:?}', got '{1:?}'")]
    VoiceMismatch(Voice, Voice),
    #[error("invalid note distribution")]
    InvalidNoteDistribution,
    #[error("expected {0} pulses, got {1}")]
    MeasureColumnMismatch(usize, usize, Range),
    #[error("expected {0} pulses, got {1}")]
    PulseCountMismatch(usize, usize, Range),
    #[error("expected {0} voices, got {1}")]
    VoiceCountMismatch(usize, usize, Range),
    #[error("unmatched underline delimiter '`'")]
    UnmatchedUnderline,
    #[error("expected {0}, got {1}")]
    MismatchedVerseIndex(usize, usize),
    #[error("trailing operator, missing '...'")]
    ExpectedLyricAnchor,
    #[error("invalid note prolongation")]
    InvalidNoteProlongation,
    #[error("expected '{0}', got '{1}'")]
    MismatchedPulseAccent(PulseAccent, PulseAccent, Range),
    #[error("trailing lyric, no solfa column matched")]
    TrailingLyric(Vec<Range>),
    #[error("splits contains a null value")]
    NullSplitValue,
    #[error("splits don't match voices")]
    SplitVoiceMismatch(Option<Range>),
    #[error("voice distribution doesn't match splits")]
    InvalidVoiceDistribution(Range),
    #[error("'splits' have not been defined")]
    UndefinedSplitsMetadata(Range),
    #[error("expected {0} verses, got {1}")]
    VerseMismatch(usize, usize, Range),
    #[error("'verses' metadata has not been defined")]
    UndefinedVersesMetadata(Range),
    #[error("'voices' metadata has not been defined")]
    UndefinedVoiceMetadata(Range),
    #[error("'time' parameter has not been defined")]
    UndefinedTimeParameter(Range),
    #[error("parameter override should be done at the top level")]
    NonTopLevelParamsOverride(Range),
}

impl DiagnosticKind {
    pub fn get_code(&self) -> &'static str {
        match self {
            DiagnosticKind::SyntaxError => "E001",
            DiagnosticKind::Missing(_) => "E002",
            DiagnosticKind::InvalidUTF8(_) => "E003",
            DiagnosticKind::KeyReassignment { .. } => "E004",
            DiagnosticKind::UnknownField(_) => "E005",
            DiagnosticKind::UnknownParameter(_) => "E006",
            DiagnosticKind::ExpectedType(_, _) => "E007",
            DiagnosticKind::InvalidType(_) => "E008",
            DiagnosticKind::InvalidTimeSignature => "E009",
            DiagnosticKind::InvalidDynamic(_) => "E010",
            DiagnosticKind::InvalidDynamicParams(_) => "E011",
            DiagnosticKind::InvalidVoice(_) => "E012",
            DiagnosticKind::UndefinedVoice(_, _) => "E013",
            DiagnosticKind::VoiceMismatch(_, _) => "E014",
            DiagnosticKind::InvalidNoteDistribution => "E015",
            DiagnosticKind::MeasureColumnMismatch(_, _, _) => "E016",
            DiagnosticKind::PulseCountMismatch(_, _, _) => "E017",
            DiagnosticKind::VoiceCountMismatch(_, _, _) => "E018",
            DiagnosticKind::UnmatchedUnderline => "E019",
            DiagnosticKind::MismatchedVerseIndex(_, _) => "E020",
            DiagnosticKind::ExpectedLyricAnchor => "E021",
            DiagnosticKind::InvalidNoteProlongation => "E022",
            DiagnosticKind::MismatchedPulseAccent(_, _, _) => "E023",
            DiagnosticKind::TrailingLyric(_) => "E024",
            DiagnosticKind::NullSplitValue => "E025",
            DiagnosticKind::SplitVoiceMismatch(_) => "E026",
            DiagnosticKind::InvalidVoiceDistribution(_) => "E027",
            DiagnosticKind::UndefinedSplitsMetadata(_) => "E028",
            DiagnosticKind::VerseMismatch(_, _, _) => "E029",
            DiagnosticKind::UndefinedVersesMetadata(_) => "E030",
            DiagnosticKind::UndefinedVoiceMetadata(_) => "E031",
            DiagnosticKind::UndefinedTimeParameter(_) => "E032",
            DiagnosticKind::NonTopLevelParamsOverride(_) => "E033",
        }
    }

    pub fn get_label(&self) -> Option<String> {
        match self {
            DiagnosticKind::KeyReassignment(name, _) => {
                Some(format!("'{name}' has been reassigned here"))
            }
            _ => None,
        }
    }

    pub fn get_extra_info(&self) -> Vec<DiagnosticRelatedInfo> {
        match self {
            DiagnosticKind::KeyReassignment(name, range) => vec![DiagnosticRelatedInfo {
                message: format!("'{name}' has been assigned here"),
                range: *range,
            }],
            DiagnosticKind::MeasureColumnMismatch(_, _, range)
            | DiagnosticKind::MismatchedPulseAccent(_, _, range) => vec![DiagnosticRelatedInfo {
                message: "time signature defined here".to_string(),
                range: *range,
            }],
            DiagnosticKind::PulseCountMismatch(expected, _, range) => {
                vec![DiagnosticRelatedInfo {
                    message: format!("first line has {expected} pulses"),
                    range: *range,
                }]
            }
            DiagnosticKind::VoiceCountMismatch(_, _, range) => vec![DiagnosticRelatedInfo {
                message: "voices defined here".to_string(),
                range: *range,
            }],
            DiagnosticKind::TrailingLyric(ranges) => ranges
                .iter()
                .map(|r| DiagnosticRelatedInfo {
                    message: "add more columns here".to_string(),
                    range: *r,
                })
                .collect(),
            DiagnosticKind::SplitVoiceMismatch(Some(range))
            | DiagnosticKind::InvalidVoiceDistribution(range) => {
                vec![DiagnosticRelatedInfo {
                    message: "splits defined here".to_string(),
                    range: *range,
                }]
            }
            DiagnosticKind::VerseMismatch(_, _, range) => vec![DiagnosticRelatedInfo {
                message: "verses defined here".to_string(),
                range: *range,
            }],
            DiagnosticKind::UndefinedSplitsMetadata(range) => vec![DiagnosticRelatedInfo {
                message: "consider adding 'splits' metadata".to_string(),
                range: *range,
            }],
            DiagnosticKind::UndefinedVersesMetadata(range) => vec![DiagnosticRelatedInfo {
                message: "consider adding 'verses' metadata".to_string(),
                range: *range,
            }],
            DiagnosticKind::UndefinedVoiceMetadata(range) => vec![DiagnosticRelatedInfo {
                message: "consider adding 'voice' metadata".to_string(),
                range: *range,
            }],
            DiagnosticKind::UndefinedTimeParameter(range) => vec![DiagnosticRelatedInfo {
                message: "consider adding 'time' parameter".to_string(),
                range: *range,
            }],
            DiagnosticKind::NonTopLevelParamsOverride(range) => vec![DiagnosticRelatedInfo {
                message: "set parameter override here".to_string(),
                range: *range,
            }],
            _ => Vec::default(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Info,
    Help,
}

impl Diagnostic {
    pub fn byte_range(&self) -> std::ops::Range<usize> {
        self.range.start_byte..self.range.end_byte
    }

    pub fn is_error(&self) -> bool {
        matches!(self.level, DiagnosticLevel::Error)
    }
}

#[derive(Debug)]
pub struct DiagnosticRelatedInfo {
    pub message: String,
    pub range: Range,
}
