use std::fmt::{Display, Formatter};
use crate::gemtext::GemTextError;

#[derive(Debug, Eq, PartialEq)]
pub struct ParserError {
    pub line: usize,
    pub kind: ErrorKind,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ErrorKind {
    MissingStatus,
    InvalidStatus(usize),
    InvalidBody(GemTextError),
    SyntaxExpectedData,
    SyntaxMissingNewline,
    SyntaxMissingSpace,
    InvalidDigit,
}

impl Display for ParserError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error on line {}: {}", self.line, self.kind)
    }
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::MissingStatus => write!(f, "missing status code"),
            ErrorKind::InvalidStatus(s) => write!(f, "invalid status code: {}", s),
            ErrorKind::InvalidBody(e) => write!(f, "invalid body: {}", e),
            ErrorKind::SyntaxExpectedData => write!(f, "expected data"),
            ErrorKind::SyntaxMissingNewline => write!(f, "missing newline"),
            ErrorKind::SyntaxMissingSpace => write!(f, "missing space"),
            ErrorKind::InvalidDigit => write!(f, "invalid digit"),
        }
    }
}

impl From<GemTextError> for ParserError {
    fn from(value: GemTextError) -> Self {
        ParserError {
            line: value.line,
            kind: ErrorKind::InvalidBody(value),
        }
    }
}
