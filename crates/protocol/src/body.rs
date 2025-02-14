use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};

#[derive(Debug, Eq, PartialEq)]
pub struct Body {
    pub body: String // FIXME: Parse!
}

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