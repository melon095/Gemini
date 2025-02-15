use std::fmt::{Display, Formatter};
use crate::gemtext::GemTextError;

#[derive(Debug)]
pub struct ParserError {
    pub line: usize,
    pub kind: ErrorKind,
}

#[derive(Debug)]
pub enum ErrorKind {
    MissingStatus,
    InvalidStatus(usize),
    Syntax(String),
    InvalidBody(GemTextError),
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
            ErrorKind::Syntax(s) => write!(f, "syntax error: {}", s),
            ErrorKind::InvalidBody(e) => write!(f, "invalid body: {}", e),
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
