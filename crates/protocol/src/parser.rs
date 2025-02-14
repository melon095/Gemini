use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use crate::body::{Body, MimeType};
use crate::response::{OkResponse, Response};

#[derive(Debug)]
pub struct ParserError {
    line: usize,
    kind: ErrorKind,
}

#[derive(Debug)]
pub enum ErrorKind {
    MissingStatus,
    InvalidStatus(usize),
    Syntax(String),
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
        }
    }
}

struct Parser<'a> {
    iter: std::str::Chars<'a>,
    line: usize,
}

impl<'a> Parser<'a> {
    /// reply    = input / success / redirect / tempfail / permfail / auth
    fn reply(&mut self) -> Result<Response, ParserError> {
        let c = self.eat_char()?;
        match c {
            '1' => self.input(),
            '2' => self.success(),
            '3' => self.redirect(),
            '4' => self.tempfail(),
            '5' => self.permfail(),
            '6' => self.auth(),
            c => Err(self.make_err(ErrorKind::InvalidStatus(c.to_digit(10).unwrap_or(0) as usize))),
        }
    }

    fn input(&mut self) -> Result<Response, ParserError> {
        let c = self.eat_digit()?;

        self.eat_sp()?;

        let prompt = self.eat_until_crlf();

        match c {
            0 => Ok(Response::MustPromptForInput(prompt)),
            1 => Ok(Response::MustPromptSensitiveInput(prompt)),
            c => Err(self.make_err(ErrorKind::InvalidStatus((c + 10) as usize))),
        }
    }

    fn success(&mut self) -> Result<Response, ParserError> {
        self.eat_digit()?;
        self.eat_sp()?;

        let mimetype = self.mimetype()?;

        if self.peek() != '\n' {
            return Err(self.make_err(ErrorKind::Syntax("expected newline".to_string())));
        }

        self.eat_char()?;

        let body = self.eat_until(|_| false);

        Ok(Response::Success(OkResponse {
            mime: mimetype,
            body: Body { body },
        }))
     }

    fn redirect(&mut self) -> Result<Response, ParserError> {
        let c = self.eat_digit()?;
        self.eat_sp()?;

        let url = self.eat_until_crlf();

        match c {
            0 => Ok(Response::TemporaryRedirect(url)),
            1 => Ok(Response::PermanentRedirect(url)),
            c => Err(self.make_err(ErrorKind::InvalidStatus((c + 30) as usize))),
        }
    }

    fn tempfail(&mut self) -> Result<Response, ParserError> {
        let c = self.eat_digit()?;
        let msg = self.read_error_msg()?;

        match c {
            0 => Ok(Response::UnexpectedErrorTryAgain(msg)),
            1 => Ok(Response::ServerUnavailable(msg)),
            2 => Ok(Response::CGIError(msg)),
            3 => Ok(Response::ProxyError(msg)),
            4 => Ok(Response::SlowDown(msg)),
            c => Err(self.make_err(ErrorKind::InvalidStatus((c + 40) as usize))),
        }
    }

    fn permfail(&mut self) -> Result<Response, ParserError> {
        let c = self.eat_digit()?;
        let msg = self.read_error_msg()?;

        match c {
            0 => Ok(Response::PermanentFailure(msg)),
            1 => Ok(Response::ResourceNotFound(msg)),
            2 => Ok(Response::ResourceGone(msg)),
            3 => Ok(Response::ProxyRequestRefused(msg)),
            9 => Ok(Response::BadRequest(msg)),
            c => Err(self.make_err(ErrorKind::InvalidStatus((c + 50) as usize))),
        }
    }

    fn auth(&mut self) -> Result<Response, ParserError> {
        let c = self.eat_digit()?;
        let msg = self.read_error_msg()?;

        match c {
            0 => Ok(Response::CertificateRequired(msg)),
            1 => Ok(Response::CertificateNotAuthorized(msg)),
            2 => Ok(Response::CertificateNotValid(msg)),
            c => Err(self.make_err(ErrorKind::InvalidStatus((c + 60) as usize))),
        }
    }

    fn eat_char(&mut self) -> Result<char, ParserError> {
        let c = self.iter.next().ok_or(self.make_err(ErrorKind::Syntax("expected more data".to_string())))?;

        if c == '\n' {
            self.line += 1;
        }

        Ok(c)
    }

    fn eat_digit(&mut self) -> Result<u32, ParserError> {
        let c = self.eat_char()?;

        c.to_digit(10).ok_or(self.make_err(ErrorKind::Syntax("expected digit".to_string())))
    }

    fn eat_until_crlf(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.iter.next() {
            if c == '\n' {
                self.line += 1;
            }

            if c == '\r' {
                if let Some('\n') = self.iter.next() {
                    break;
                }
            }
            s.push(c);
        }
        s
    }

    fn eat_until<F>(&mut self, mut f: F) -> String
    where
        F: FnMut(char) -> bool,
    {
        let mut s = String::new();
        while let Some(c) = self.iter.next() {
            if c == '\n' {
                self.line += 1;
            }

            if f(c) {
                break;
            }
            s.push(c);
        }
        s
    }

    fn eat_sp(&mut self) -> Result<(), ParserError> {
        let c = self.eat_char()?;
        if c != ' ' {
            return Err(self.make_err(ErrorKind::Syntax("expected space".to_string())));
        }
        Ok(())
    }

    fn read_error_msg(&mut self) -> Result<Option<String>, ParserError> {
        if self.peek() == ' ' {
            self.eat_sp()?;

            Ok(Some(self.eat_until_crlf()))
        } else {
            Ok(None)
        }
    }

    fn peek(&self) -> char {
        self.iter.clone().next().unwrap_or('\0')
    }

    /// mimetype = type "/" subtype *(";" parameter)
    fn mimetype(&mut self) -> Result<MimeType, ParserError> {
        let t = self.eat_until(|c| c == '/');
        let s = self.eat_until(|c| c == '\r');

        // Simply check for a singular semicolon to determine if there are parameters.
        let params_idx = s.find(';');
        if let None = params_idx {
            return Ok(MimeType {
                typ: t,
                sub: s,
                parameters: None,
            });
        }

        // There are parameters so find all semicolons.
        let params = s
            .split(';')
            .collect::<Vec<&str>>()
            .iter()
            .filter_map(|s| {
                let mut parts = s.split('=');
                Some((parts.next()?.to_string(), parts.next()?.to_string()))
            })
            .collect::<HashMap<String, String>>();

        Ok(MimeType {
            typ: t,
            sub: s,
            parameters: Some(params),
        })
    }

    fn make_err(&self, kind: ErrorKind) -> ParserError {
        ParserError {
            line: self.line,
            kind,
        }
    }
}

pub fn parse_response(response: &str) -> Result<Response, ParserError> {
    let mut r = Parser {
        iter: response.chars(),
        line: 1,
    };

    r.reply()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ten() -> Result<(), ParserError> {
        let resp = "10 geminmi://localhost/foo\r\n";

        let r = parse_response(resp)?;

        assert_eq!(r, Response::MustPromptForInput("geminmi://localhost/foo".to_string()));

        Ok(())
    }

    #[test]
    fn test_twenty() -> Result<(), ParserError> {
        let resp = "20 text/gemini\r\nHello, World!\nSomeData\n";

        let r = parse_response(resp)?;

        assert_eq!(r, Response::Success(OkResponse {
            mime: MimeType {
                typ: "text".to_string(),
                sub: "gemini".to_string(),
                parameters: None,
            },
            body: Body {
                body: "Hello, World!\nSomeData\n".to_string(),
            }
        }));

        Ok(())
    }
}