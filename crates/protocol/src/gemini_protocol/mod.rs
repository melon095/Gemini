use crate::error::ParserError;
use crate::gemini_protocol::parser::Parser;
use crate::gemini_protocol::response::Response;

pub mod response;
pub mod parser;

pub fn parse_response(response: &str) -> Result<Response, ParserError> {
    let mut r = Parser {
        iter: response.chars(),
        line: 1,
    };

    r.reply()
}
