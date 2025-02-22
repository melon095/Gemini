use crate::gemtext::gemtext_body::{GemTextBody, Line};
use crate::gemtext::{GemTextError, GemTextErrorKind};
use url::Url;

const LINK_START: &'static str = "=>";
const PREFORMAT_TOGGLE: &'static str = "```";
const HEADING_START: &'static str = "#";
const LIST_ITEM: &'static str = "*";
const QUOTE_START: &'static str = ">";

const WSP: &[char; 2] = &[' ', '\t'];

#[derive(Debug, Eq, PartialEq)]
pub enum ParserMode {
    Normal,
    Preformat,
}

#[derive(Debug)]
pub struct GemTextParser<'a> {
    line_iter: std::str::Lines<'a>,
    url_path: &'a Url,
    pub body: Vec<Line>,
    cursor: &'a str,
    pub line_num: usize,
    pub mode: ParserMode,
}

impl<'a> GemTextParser<'a> {
    pub(super) fn new(url_path: &'a Url, str: &'a str) -> GemTextParser<'a> {
        GemTextParser {
            line_iter: str.lines(),
            body: Vec::new(),
            url_path: url_path,
            cursor: "",
            line_num: 0,
            mode: ParserMode::Normal,
        }
    }

    pub(super) fn gemtext_document(&mut self) -> Result<GemTextBody, GemTextError> {
        let mut b = GemTextBody(vec![]);

        // FIXME: Remove clone
        for line in self.line_iter.clone() {
            self.line_num += 1;
            self.cursor = line;

            b.0.push(self.gemtext_line(line)?);
        }

        Ok(b)
    }

    fn gemtext_line(&mut self, line: &'a str) -> Result<Line, GemTextError> {
        if line.starts_with(LINK_START) {
            self.link_line()
        } else if line.starts_with(PREFORMAT_TOGGLE) {
            self.preformat_toggle()
        } else if line.starts_with(HEADING_START) {
            self.heading()
        } else if line.starts_with(LIST_ITEM) {
            self.list_item()
        } else if line.starts_with(QUOTE_START) {
            self.quote_line()
        } else {
            self.text_line(line)
        }
    }

    fn text_line(&self, line: &'a str) -> Result<Line, GemTextError> {
        Ok(Line::Text(line.to_string()))
    }

    fn link_line(&mut self) -> Result<Line, GemTextError> {
        const START: usize = "=>".len();

        let line = self
            .cursor
            .chars()
            .skip(START)
            .skip_while(|c| c.is_whitespace())
            .collect::<String>();

        let split = line
            .splitn(2, WSP)
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>();

        if split.len() == 0 {
            return Err(self.make_err(GemTextErrorKind::LinkLineMissingUrl));
        }

        let url = split[0];
        if split.len() == 1 {
            return Ok(Line::Link {
                url: self.make_url(url)?,
                description: None,
            });
        }

        let text = Some(split[1..].join(" "));

        Ok(Line::Link {
            url: self.make_url(url)?,
            description: text,
        })
    }

    fn preformat_toggle(&mut self) -> Result<Line, GemTextError> {
        match self.mode {
            ParserMode::Normal => {
                self.mode = ParserMode::Preformat;
                Ok(Line::PreformatToggleOn)
            }
            ParserMode::Preformat => {
                self.mode = ParserMode::Normal;
                Ok(Line::PreformatToggleOff)
            }
        }
    }

    fn heading(&mut self) -> Result<Line, GemTextError> {
        let depth = self.cursor.chars().take_while(|c| c == &'#').count();

        Ok(Line::Heading {
            text: self.cursor[depth..].trim().to_string(),
            depth: depth as u8,
        })
    }

    fn list_item(&mut self) -> Result<Line, GemTextError> {
        const START: usize = "* ".len();

        let line = self.cursor.chars().skip(START).collect::<String>();

        Ok(Line::ListItem(line))
    }

    fn quote_line(&mut self) -> Result<Line, GemTextError> {
        const START: usize = ">".len();

        let line = self.take_cursor_whitespace(START);

        Ok(Line::Quote(line))
    }

    fn take_cursor_whitespace(&mut self, start: usize) -> String {
        self.cursor
            .chars()
            .skip(start)
            .take_while(|c| c.is_whitespace())
            .collect::<String>()
    }

    fn make_err(&self, kind: GemTextErrorKind) -> GemTextError {
        GemTextError {
            line: self.line_num,
            kind,
        }
    }

    fn make_url(&self, input: &str) -> Result<Url, GemTextError> {
        let url = Url::parse(input);
        match url {
            Ok(url) => Ok(url),
            Err(_) => {
                let url = self.url_path.join(input);
                match url {
                    Ok(url) => Ok(url),
                    Err(err) => Err(self.make_err(GemTextErrorKind::InvalidUrl(err))),
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::gemtext::gemtext_body::Line::{Heading, Link, Text};
    use crate::gemtext::{gemtext_body::Line, parse_gemtext, GemTextErrorKind};
    use url::Url;

    #[test]
    fn test_link_line_description() {
        let url = Url::parse("gemini://gemini.circumlunar.space/docs/faq.gmi").unwrap();
        let input = "=> gemini://gemini.circumlunar.space/docs/faq.gmi The Gemini FAQ".to_string();
        let parsed = parse_gemtext(&url, input);
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        assert_eq!(parsed.0.len(), 1);
        assert_eq!(
            parsed.0.get(0).unwrap(),
            &Line::Link {
                url: Url::parse("gemini://gemini.circumlunar.space/docs/faq.gmi").unwrap(),
                description: Some("The Gemini FAQ".to_string())
            }
        )
    }

    #[test]
    fn test_link_line() {
        let url = Url::parse("gemini://gemini.circumlunar.space/docs/faq.gmi").unwrap();
        let input = "=>   \t\r gemini://gemini.circumlunar.space/docs/faq.gmi ".to_string();
        let parsed = parse_gemtext(&url, input);
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        assert_eq!(parsed.0.len(), 1);
        assert_eq!(
            parsed.0.get(0).unwrap(),
            &Line::Link {
                url: Url::parse("gemini://gemini.circumlunar.space/docs/faq.gmi").unwrap(),
                description: None
            }
        )
    }

    #[test]
    fn test_link_line_missing_url() {
        let url = Url::parse("gemini://gemini.circumlunar.space/docs/faq.gmi").unwrap();
        let input = "=> ".to_string();

        let parsed = parse_gemtext(&url, input);
        assert!(parsed.is_err());
        let parsed = parsed.unwrap_err();
        assert_eq!(parsed.line, 1);
        assert_eq!(parsed.kind, GemTextErrorKind::LinkLineMissingUrl);
    }

    #[test]
    fn test_homepage() {
        let url = Url::parse("gemini://geminiprotocol.net/").unwrap();
        const INPUT: &'static str = r#"# Project Gemini

## Gemini in 100 words

Gemini is a new internet technology supporting an electronic library of interconnected text documents.  That's not a new idea, but it's not old fashioned either.  It's timeless, and deserves tools which treat it as a first class concept, not a vestigial corner case.  Gemini isn't about innovation or disruption, it's about providing some respite for those who feel the internet has been disrupted enough already.  We're not out to change the world or destroy other technologies.  We are out to build a lightweight online space where documents are just documents, in the interests of every reader's privacy, attention and bandwidth.

=> docs/faq.gmi If you'd like to know more, read our FAQ
=> https://www.youtube.com/watch?v=DoEI6VzybDk  Or, if you'd prefer, here's a video overview

## Official resources

=> news/        Project Gemini news
=> docs/        Project Gemini documentation
=> history/     Project Gemini history
=> software/    Known Gemini software

All content at geminiprotocol.net is CC BY-NC-ND 4.0 licensed unless stated otherwise:
=> https://creativecommons.org/licenses/by-nc-nd/4.0/   CC Attribution-NonCommercial-NoDerivs 4.0 International
"#;

        let parsed = parse_gemtext(&url, INPUT.to_string());
        assert!(parsed.is_ok());
        let parsed = parsed.unwrap();
        assert_eq!(parsed.0.len(), 18);

        assert_eq!(parsed.0, vec![
            Heading {
                text: "Project Gemini".to_string(),
                depth: 1
            },
            Text("".to_string()),
            Heading {
                text: "Gemini in 100 words".to_string(),
                depth: 2
            },
            Text("".to_string()),
            Text("Gemini is a new internet technology supporting an electronic library of interconnected text documents.  That's not a new idea, but it's not old fashioned either.  It's timeless, and deserves tools which treat it as a first class concept, not a vestigial corner case.  Gemini isn't about innovation or disruption, it's about providing some respite for those who feel the internet has been disrupted enough already.  We're not out to change the world or destroy other technologies.  We are out to build a lightweight online space where documents are just documents, in the interests of every reader's privacy, attention and bandwidth.".to_string()),
            Text("".to_string()),
            Link {
                url: Url::parse("gemini://geminiprotocol.net/docs/faq.gmi").unwrap(),
                description: Some("If you'd like to know more, read our FAQ".to_string())
            },
            Link {
                url: Url::parse("https://www.youtube.com/watch?v=DoEI6VzybDk").unwrap(),
                description: Some(" Or, if you'd prefer, here's a video overview".to_string())
            },
            Text("".to_string()),
            Heading {
                text: "Official resources".to_string(),
                depth: 2
            },
            Text("".to_string()),
            Link {
                url: Url::parse("gemini://geminiprotocol.net/news/").unwrap(),
                description: Some("       Project Gemini news".to_string())
            },
            Link {
                url: Url::parse("gemini://geminiprotocol.net/docs/").unwrap(),
                description: Some("       Project Gemini documentation".to_string())
            },
            Link {
                url: Url::parse("gemini://geminiprotocol.net/history/").unwrap(),
                description: Some("    Project Gemini history".to_string())
            },
            Link {
                url: Url::parse("gemini://geminiprotocol.net/software/").unwrap(),
                description: Some("   Known Gemini software".to_string())
            },
            Text("".to_string()),
            Text("All content at geminiprotocol.net is CC BY-NC-ND 4.0 licensed unless stated otherwise:".to_string()),
            Link {
                url: Url::parse("https://creativecommons.org/licenses/by-nc-nd/4.0/").unwrap(),
                description: Some("  CC Attribution-NonCommercial-NoDerivs 4.0 International".to_string())
            }
        ])
    }
}
