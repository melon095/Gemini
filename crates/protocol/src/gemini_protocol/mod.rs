use url::Url;
use crate::error::ParserError;
use crate::gemini_protocol::parser::Parser;
use crate::gemini_protocol::response::Response;

pub mod response;
pub mod parser;

pub fn parse_response(url: &Url, response: &str) -> Result<Response, ParserError> {
    let mut r = Parser::new(url, response);

    r.reply()
}
