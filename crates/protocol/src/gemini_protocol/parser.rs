use std::collections::HashMap;
use crate::error::{ErrorKind, ParserError};
use crate::gemtext::gemtext_body::{MimeType};
use crate::gemini_protocol::response::{OkResponse, Response};
use crate::gemtext::parse_gemtext;

pub(super) struct Parser<'a> {
    pub(super) iter: std::str::Chars<'a>,
    pub(super) line: usize,
}

impl<'a> Parser<'a> {
    pub(super) fn reply(&mut self) -> Result<Response, ParserError> {
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
            c => Err(self.make_err(ErrorKind::InvalidStatus((10 + c) as usize))),
        }
    }

    fn success(&mut self) -> Result<Response, ParserError> {
        self.eat_digit()?;
        self.eat_sp()?;

        let mimetype = self.mimetype()?;

        if self.peek() != '\n' {
            return Err(self.make_err(ErrorKind::SyntaxMissingNewline));
        }

        self.eat_char()?;

        let body = self.eat_until(|_| false);

        Ok(Response::Success(OkResponse {
            mime: mimetype,
            body: parse_gemtext(body)?,
        }))
     }

    fn redirect(&mut self) -> Result<Response, ParserError> {
        let c = self.eat_digit()?;
        self.eat_sp()?;

        let url = self.eat_until_crlf();

        match c {
            0 => Ok(Response::TemporaryRedirect(url)),
            1 => Ok(Response::PermanentRedirect(url)),
            c => Err(self.make_err(ErrorKind::InvalidStatus((30 + c) as usize))),
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
            c => Err(self.make_err(ErrorKind::InvalidStatus((40 + c) as usize))),
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
            c => Err(self.make_err(ErrorKind::InvalidStatus((50 + c) as usize))),
        }
    }

    fn auth(&mut self) -> Result<Response, ParserError> {
        let c = self.eat_digit()?;
        let msg = self.read_error_msg()?;

        match c {
            0 => Ok(Response::CertificateRequired(msg)),
            1 => Ok(Response::CertificateNotAuthorized(msg)),
            2 => Ok(Response::CertificateNotValid(msg)),
            c => Err(self.make_err(ErrorKind::InvalidStatus((60 + c) as usize))),
        }
    }

    fn eat_char(&mut self) -> Result<char, ParserError> {
        let c = self.iter.next().ok_or(self.make_err(ErrorKind::SyntaxExpectedData))?;

        if c == '\n' {
            self.line += 1;
        }

        Ok(c)
    }

    fn eat_digit(&mut self) -> Result<u32, ParserError> {
        let c = self.eat_char()?;

        c.to_digit(10).ok_or(self.make_err(ErrorKind::InvalidDigit))
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
            return Err(self.make_err(ErrorKind::SyntaxMissingSpace));
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
        let params_idx = params_idx.unwrap();

        // There are parameters so find all semicolons.
        let params = s
            .split(';')
            .collect::<Vec<&str>>()
            .iter()
            .filter_map(|s| {
                let mut parts = s.split('=');

                // rfc2045 states that parameters may NOT contain space, so it's fine to trim.
                let key = parts.next()?.trim();
                let value = parts.next()?.trim();

                // Parameters other than "charset" and "lang" are undefined and clients MUST ignore any such parameters.
                if key != "charset" && key != "lang" {
                    return None;
                }

                Some((key.to_string(), value.to_string()))
            })
            .collect::<HashMap<String, String>>();

        let s = s[..params_idx].to_string();

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

#[cfg(test)]
mod tests {
    use crate::gemini_protocol::parse_response;
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

        if let Response::Success(OkResponse { mime, body }) = r {
            assert_eq!(mime.typ, "text");
            assert_eq!(mime.sub, "gemini");
            assert_eq!(mime.parameters.is_none(), true);
            assert_eq!(body.0.len(), 2);
        } else {
            panic!("expected success response");
        }

        Ok(())
    }

    #[test]
    fn test_mimetype() -> Result<(), ParserError> {
        let resp = "20 text/gemini; lang=zh-CN; charset=utf-8\r\n";

        let r = parse_response(resp)?;

        if let Response::Success(OkResponse { mime, .. }) = r {
            assert_eq!(mime.typ, "text");
            assert_eq!(mime.sub, "gemini");
            assert_eq!(mime.parameters.is_some(), true);

            let params = mime.parameters.unwrap();
            assert_eq!(params.get("lang").unwrap(), "zh-CN");
            assert_eq!(params.get("charset").unwrap(), "utf-8");
        } else {
            panic!("expected success response");
        }

        Ok(())
    }

    #[test]
    fn test_err_syntax_expected_data() -> Result<(), ParserError> {
        let cases = vec!["", "2"];

        for case in cases {
            let r = parse_response(case);
            assert_eq!(r.is_err(), true);
            assert_eq!(r.err() == Some(ParserError {
                line: 1,
                kind: ErrorKind::SyntaxExpectedData,
            }), true);
        }

        Ok(())
    }

    #[test]
    fn test_syntax_missing_newline() -> Result<(), ParserError> {
        let cases = vec!["20 text/gemini Hello, World!"];

        for case in cases {
            let r = parse_response(case);
            assert_eq!(r.is_err(), true);
            assert_eq!(r.err() == Some(ParserError {
                line: 1,
                kind: ErrorKind::SyntaxMissingNewline,
            }), true);
        }

        Ok(())
    }

    #[test]
    fn test_syntax_missing_space() -> Result<(), ParserError> {
        let cases = vec!["20text/gemini\r\n"];

        for case in cases {
            let r = parse_response(case);
            assert_eq!(r.is_err(), true);
            assert_eq!(r.err() == Some(ParserError {
                line: 1,
                kind: ErrorKind::SyntaxMissingSpace,
            }), true);
        }

        Ok(())
    }

    #[test]
    fn test_invalid_digit() -> Result<(), ParserError> {
        let cases = vec!["2a0 text/gemini\r\n"];

        for case in cases {
            let r = parse_response(case);
            assert_eq!(r.is_err(), true);
            assert_eq!(r.err() == Some(ParserError {
                line: 1,
                kind: ErrorKind::InvalidDigit,
            }), true);
        }

        Ok(())
    }
}
