use std::fmt::{Display, Formatter};
use crate::gemtext::gemtext_body::{GemTextBody};
use crate::gemtext::gemtext_parser::GemTextParser;

pub mod gemtext_body;
pub mod gemtext_parser;

#[derive(Debug, Eq, PartialEq)]
pub struct GemTextError {
    pub line: usize,
    pub kind: GemTextErrorKind
}

#[derive(Debug, Eq, PartialEq)]
pub enum GemTextErrorKind {
    LinkLineMissingUrl,
}

impl Display for GemTextError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error on line {}: {}", self.line, self.kind)
    }
}

impl Display for GemTextErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GemTextErrorKind::LinkLineMissingUrl => write!(f, "Link line missing URL"),
        }
    }
}

pub fn parse_gemtext(str: String) -> Result<GemTextBody, GemTextError> {
    let mut parser = GemTextParser::new(&str);

    parser.gemtext_document()
}
