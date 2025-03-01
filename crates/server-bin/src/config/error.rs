use std::fmt::Display;
#[derive(Debug, Eq, PartialEq)]
pub enum Error<'a> {
    StringExpectedStartingQuote(&'a str),
    StringExpectedEndingQuote(&'a str),
    ExpectedIdentifier(&'a str),
    InvalidNumber(&'a str),
    MissingSemicolon,
    MissingServerBlock,
    InvalidBlockTag(&'a str),
}

impl Display for Error<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::StringExpectedStartingQuote(i) => {
                write!(f, "Expected starting quote, got: {}", i)
            }
            Error::StringExpectedEndingQuote(i) => write!(f, "Expected ending quote, got: {}", i),
            Error::ExpectedIdentifier(i) => write!(f, "Expected identifier, got: {}", i),
            Error::InvalidNumber(n) => write!(f, "Invalid number: {}", n),
            Error::MissingSemicolon => write!(f, "Missing semicolon"),
            Error::MissingServerBlock => write!(f, "Missing server block"),
            Error::InvalidBlockTag(t) => write!(f, "Invalid block tag: {}", t),
        }
    }
}
