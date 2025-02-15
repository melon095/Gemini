use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};

// TODO: Pre format data should be baked into lines
#[derive(Debug, Eq, PartialEq)]
pub enum Line {
    Text(String),
    Link { url: String, description: Option<String> },
    Heading { text: String, depth: u8 },
    ListItem(String),
    Quote(String),
    PreformatToggleOn,
    PreformatToggleOff,
}

#[derive(Debug, Eq, PartialEq)]
pub struct GemTextBody(pub Vec<Line>);

#[derive(Eq, PartialEq)]
pub struct MimeType {
    pub typ: String,
    pub sub: String,
    pub parameters: Option<HashMap<String, String>>
}

impl Debug for MimeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.typ, self.sub)?;
        if let Some(parameters) = &self.parameters {
            write!(f, "; ")?;
            for (key, value) in parameters {
                write!(f, "{}={}; ", key, value)?;
            }
        }
        Ok(())
    }
}

impl Display for MimeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}
