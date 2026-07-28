use thiserror::Error;

use crate::{
    SUPPORTED_VERSION, ast::solfa::PulseAccent, data_types::Voice, ts_utils::range::Range,
};

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub kind: DiagnosticKind,
    pub range: Range,
    pub stage: ReportStage,
}

#[derive(Debug, Error, Clone)]
pub enum DiagnosticKind {
    #[error("unsupported version {0} ({SUPPORTED_VERSION})")]
    UnsupportedVersion(String),
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
    #[error("expected a range for timestamp")]
    ExpectedTimestampRange,
    #[error("range not allowed for this event")]
    RangeNotAllowed,
    #[error("invalid voice '{0}'")]
    InvalidVoice(String),
    #[error("undefined voice '{0:?}'")]
    UndefinedVoice(Voice, Range),
    #[error("expected '{0:?}', got '{1:?}'")]
    VoiceMismatch(Voice, Voice),
    #[error("invalid note distribution")]
    InvalidNoteDistribution,
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
    #[error("voice distribution doesn't match splits")]
    InvalidVoiceDistribution(Range),
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
    #[error("missing lyrics operator and anchor")]
    ExpectedLyricJoin(Range),
    #[error("lyrics join is unused (must be followed by a section)")]
    UnusedLyricJoin(Range),
    #[error("merged section structure does not match previous")]
    InvalidSectionMerge(Range),
    #[error("timestamp is unmatched")]
    UnmatchedTimestamp,
    #[error("section metadata should be declared at the top level")]
    NonTopLevelSectionMetadata(Range),
}

impl DiagnosticKind {
    pub fn get_code(&self) -> &'static str {
        match self {
            DiagnosticKind::UnsupportedVersion(_) => "E000",
            DiagnosticKind::SyntaxError => "E001",
            DiagnosticKind::Missing(_) => "E002",
            DiagnosticKind::InvalidUTF8(_) => "E003",
            DiagnosticKind::KeyReassignment { .. } => "E004",
            DiagnosticKind::UnknownField(_) => "E005",
            DiagnosticKind::UnknownParameter(_) => "E006",
            DiagnosticKind::ExpectedType(_, _) => "E007",
            DiagnosticKind::InvalidType(_) => "E008",
            DiagnosticKind::InvalidTimeSignature => "E009",
            DiagnosticKind::ExpectedTimestampRange => "E010",
            DiagnosticKind::RangeNotAllowed => "E011",
            DiagnosticKind::InvalidVoice(_) => "E012",
            DiagnosticKind::UndefinedVoice(_, _) => "E013",
            DiagnosticKind::VoiceMismatch(_, _) => "E014",
            DiagnosticKind::InvalidNoteDistribution => "E015",
            DiagnosticKind::PulseCountMismatch(_, _, _) => "E016",
            DiagnosticKind::VoiceCountMismatch(_, _, _) => "E017",
            DiagnosticKind::UnmatchedUnderline => "E018",
            DiagnosticKind::MismatchedVerseIndex(_, _) => "E019",
            DiagnosticKind::ExpectedLyricAnchor => "E020",
            DiagnosticKind::InvalidNoteProlongation => "E021",
            DiagnosticKind::MismatchedPulseAccent(_, _, _) => "E022",
            DiagnosticKind::TrailingLyric(_) => "E023",
            DiagnosticKind::InvalidVoiceDistribution(_) => "E024",
            DiagnosticKind::VerseMismatch(_, _, _) => "E025",
            DiagnosticKind::UndefinedVersesMetadata(_) => "E026",
            DiagnosticKind::UndefinedVoiceMetadata(_) => "E027",
            DiagnosticKind::UndefinedTimeParameter(_) => "E028",
            DiagnosticKind::NonTopLevelParamsOverride(_) => "E029",
            DiagnosticKind::ExpectedLyricJoin(_) => "E030",
            DiagnosticKind::UnusedLyricJoin(_) => "E031",
            DiagnosticKind::InvalidSectionMerge(_) => "E032",
            DiagnosticKind::UnmatchedTimestamp => "E033",
            DiagnosticKind::NonTopLevelSectionMetadata(_) => "E034",
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
            DiagnosticKind::PulseCountMismatch(expected, _, range) => {
                vec![DiagnosticRelatedInfo {
                    message: format!("this line has {expected} pulses"),
                    range: *range,
                }]
            }
            DiagnosticKind::InvalidVoiceDistribution(range)
            | DiagnosticKind::VoiceCountMismatch(_, _, range) => vec![DiagnosticRelatedInfo {
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
            DiagnosticKind::VerseMismatch(_, _, range) => vec![DiagnosticRelatedInfo {
                message: "verses defined here".to_string(),
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
            DiagnosticKind::NonTopLevelSectionMetadata(range) => vec![DiagnosticRelatedInfo {
                message: "declare section metadata here".to_string(),
                range: *range,
            }],
            DiagnosticKind::ExpectedLyricJoin(range) => vec![DiagnosticRelatedInfo {
                message: "next section here".to_string(),
                range: *range,
            }],
            DiagnosticKind::UnusedLyricJoin(range) => vec![DiagnosticRelatedInfo {
                message: "consider adding a next section".to_string(),
                range: *range,
            }],
            DiagnosticKind::InvalidSectionMerge(range) => vec![DiagnosticRelatedInfo {
                message: "root section defined here".to_string(),
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

#[derive(Debug, Clone, Copy)]
pub enum ReportStage {
    Parsing,
    IRBuild,
    Validation,
}
