use thiserror::Error;

use crate::ast::symbols::Range;

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
    #[error("expected {0}, got {1}")]
    ExpectedType(&'static str, &'static str),
    #[error("invalid {0}")]
    InvalidType(&'static str),
    #[error("invalid time signature, expected two integers")]
    InvalidTimeSignature,
    #[error("invalid assignment")]
    InvalidAssignment,
    #[error("reassignment of key '{name}'")]
    KeyReassignment { name: String, range: Range },
}

impl DiagnosticKind {
    pub fn get_code(&self) -> &'static str {
        match self {
            DiagnosticKind::SyntaxError => "E001",
            DiagnosticKind::Missing(_) => "E002",
            DiagnosticKind::InvalidUTF8(_) => "E003",
            DiagnosticKind::ExpectedType(_, _) => "E004",
            DiagnosticKind::InvalidType(_) => "E005",
            DiagnosticKind::InvalidTimeSignature => "E006",
            DiagnosticKind::InvalidAssignment => "E007",
            DiagnosticKind::KeyReassignment { .. } => "E008",
        }
    }

    pub fn get_label(&self) -> Option<String> {
        match self {
            DiagnosticKind::KeyReassignment { name, .. } => {
                Some(format!("'{name}' has been reassigned here"))
            }
            _ => None,
        }
    }

    pub fn get_extra_info(&self) -> Vec<DiagnosticRelatedInfo> {
        match self {
            DiagnosticKind::KeyReassignment { name, range } => vec![DiagnosticRelatedInfo {
                message: format!("'{name}' has been assigned here"),
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
