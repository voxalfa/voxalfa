use thiserror::Error;

use crate::{ast::types::Voice, ts_utils::range::Range};

#[derive(Debug)]
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
    #[error("unknown parameter '{0}'")]
    UnknownParameter(String),
    #[error("expected {0}, got {1}")]
    ExpectedType(&'static str, &'static str),
    #[error("invalid {0}")]
    InvalidType(&'static str),
    #[error("invalid time signature, expected two integers")]
    InvalidTimeSignature,
    #[error("invalid dynamic identifier '{0}'")]
    InvalidDynamic(String),
    #[error("invalid dynamic parameters, expected {0} numbers")]
    InvalidDynamicParams(usize),
    #[error("invalid voice '{0}'")]
    InvalidVoice(String),
    #[error("undefined voice '{0}'")]
    UndefinedVoice(String),
    #[error("expected '{0:?}', got '{1:?}")]
    VoiceMismatch(Voice, Voice),
    #[error("invalid note distribution")]
    InvalidNoteDistribution,
    #[error("expected {0} columns, got {1}")]
    MeasureColumnMismatch(usize, usize, Range),
    #[error("expected {0} measures, got {1}")]
    MeasureCountMismatch(usize, usize, Range),
    #[error("expected {0} voices, got {1}")]
    VoiceCountMismatch(usize, usize, Range),
    #[error("unmatched underline delimiter '`'")]
    UnmatchedUnderline,
    #[error("expected {0}, got {1}")]
    MismatchedVerseIndex(usize, usize),
    #[error("trailing operator, missing '...'")]
    ExpectedLyricAnchor,
}

impl DiagnosticKind {
    pub fn get_code(&self) -> &'static str {
        match self {
            DiagnosticKind::SyntaxError => "E001",
            DiagnosticKind::Missing(_) => "E002",
            DiagnosticKind::InvalidUTF8(_) => "E003",
            DiagnosticKind::KeyReassignment { .. } => "E004",
            DiagnosticKind::UnknownParameter(_) => "E005",
            DiagnosticKind::ExpectedType(_, _) => "E006",
            DiagnosticKind::InvalidType(_) => "E007",
            DiagnosticKind::InvalidTimeSignature => "E008",
            DiagnosticKind::InvalidDynamic(_) => "E009",
            DiagnosticKind::InvalidDynamicParams(_) => "E010",
            DiagnosticKind::InvalidVoice(_) => "E011",
            DiagnosticKind::UndefinedVoice(_) => "E012",
            DiagnosticKind::VoiceMismatch(_, _) => "E013",
            DiagnosticKind::InvalidNoteDistribution => "E014",
            DiagnosticKind::MeasureColumnMismatch(_, _, _) => "E015",
            DiagnosticKind::MeasureCountMismatch(_, _, _) => "E016",
            DiagnosticKind::VoiceCountMismatch(_, _, _) => "E017",
            DiagnosticKind::UnmatchedUnderline => "E018",
            DiagnosticKind::MismatchedVerseIndex(_, _) => "E019",
            DiagnosticKind::ExpectedLyricAnchor => "E020",
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
            DiagnosticKind::MeasureColumnMismatch(_, _, range) => vec![DiagnosticRelatedInfo {
                message: "time signature defined here".to_string(),
                range: *range,
            }],
            DiagnosticKind::MeasureCountMismatch(expected, _, range) => {
                vec![DiagnosticRelatedInfo {
                    message: format!("first line has {expected} measures"),
                    range: *range,
                }]
            }
            DiagnosticKind::VoiceCountMismatch(_, _, range) => vec![DiagnosticRelatedInfo {
                message: "voices defined here".to_string(),
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
}

#[derive(Debug)]
pub struct DiagnosticRelatedInfo {
    pub message: String,
    pub range: Range,
}
