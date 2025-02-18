use std::io::{Read, Write};
use std::sync::Arc;
use iced::{Task, widget::{column, text}, Theme, Background, Color, Border, Shadow};
use iced::futures::AsyncReadExt;
use iced::widget::{button, rich_text, span};
use iced::widget::button::{Status, Style};
use rustls::ClientConfig;
use url::Url;
use protocol::gemini_protocol::parse_response;
use protocol::gemini_protocol::response::{OkResponse, Response};
use protocol::gemtext::gemtext_body::Line;
use protocol::gemtext::parse_gemtext;
use crate::network::tls_client::TlsClient;

#[derive(Debug, Clone)]
pub enum LoadStatus {
    Success(DocumentData),
    Error(Response),
}

#[derive(Debug, Clone)]
pub enum DocumentMessage {
    LoadComplete((Url, Result<LoadStatus, String>)),
    FIX_THIS,
    LinkPressed(Url),
}

#[derive(Debug, Clone)]
pub struct DocumentData {
    url: Url,
    content: OkResponse
}

#[derive(Debug)]
pub enum Document {
    Loading,
    Error(Url, Response),
    Loaded(DocumentData)
}

impl Document {
    pub fn new(tls_client: Arc<ClientConfig>, url: Url) -> (Self, Task<DocumentMessage>) {
        (Self::Loading, Task::perform(Self::load_document(tls_client, url.clone()), DocumentMessage::LoadComplete))
    }

    pub fn update(&mut self, message: DocumentMessage) -> Task<DocumentMessage> {
        match self {
            Self::Loading => {
                match message {
                    DocumentMessage::LoadComplete((url, Ok(data))) => {
                        match data {
                            LoadStatus::Success(data) => {
                                *self = Self::Loaded(data);
                            },
                            LoadStatus::Error(response) => {
                                *self = Self::Error(url, response);
                            }
                        }
                    },
                    DocumentMessage::LoadComplete((url, Err(error))) => {
                        eprintln!("Failed to load document: {}", error);

                        *self = Self::Error(url, Response::PermanentFailure(Some(format!("Failed to load document: {}", error))));
                    },
                    _ => ()
                };

                Task::none()
            },
            Self::Error(_, _) => Task::none(),
            Self::Loaded(doc) => {
                match message {
                    DocumentMessage::FIX_THIS => {
                        Task::none()
                    },
                    DocumentMessage::LinkPressed(url) => {
                        log::info!("Link pressed: {}", url);

                        Task::none()
                    }
                    _ => {
                        Task::none()
                    }
                }
            }
        }
    }

    pub fn view(&self) -> iced::Element<DocumentMessage> {
        match self {
            Self::Loading => text("Loading...").into(),
            Self::Error(url, response) => text(format!("Error: {}: {:?}", url, response)).into(),
            Self::Loaded(data) =>  {
                let mut columns = column![
                    text(data.url.to_string()),
                ];

                for line in &data.content.body.0 {
                    columns = match line {
                        Line::Text(val) => columns.push(text(val)),
                        Line::Link { url, description } => {
                            let description = description.as_ref().cloned().unwrap_or_default();

                            let b = button(text(description))
                                .on_press(DocumentMessage::LinkPressed(url.clone()))
                                .style(link_style);

                            columns.push(b)
                        }

                        Line::Heading { text: t, depth } => {
                            let head = rich_text!(span(t))
                                .size(10.0 + (10.0 * *depth as f32));

                            columns.push(head)
                        }
                        Line::ListItem(value) => {
                            let item = rich_text!(span(value))
                                .size(10.0);

                            columns.push(item)
                        }
                        Line::Quote(value) => columns.push(text(value)),
                        Line::PreformatToggleOn => columns.push(text("```")),
                        Line::PreformatToggleOff => columns.push(text("```")),
                    };
                }

                columns.into()
            }
        }
    }

    async fn load_document(tls: Arc<ClientConfig>, url: Url) -> (Url, Result<LoadStatus, String>) {
        let r = match url.scheme() {
            "gemini" => Self::load_gemini(tls, &url).await,
            "file" => Self::load_file(&url).await,
            _ => Err(format!("Unsupported scheme: {}", url.scheme()))
        };

        (url, r)
    }

    async fn load_gemini(tls: Arc<ClientConfig>, url: &Url) -> Result<LoadStatus, String> {

        const DEFAULT_PORT: u16 = 1965;

        let host = url.host_str().ok_or("No host found")?;
        let port = url.port().unwrap_or(DEFAULT_PORT);

        let mut conn = TlsClient::new_from_host((host, port), tls).
            map_err(|e| format!("Failed to connect: {}", e))?;

        write!(conn, "{}\r\n", url.to_string()).unwrap();

        let mut pt = vec![];
        conn.read_to_end(&mut pt).unwrap();
        let pt = String::from_utf8_lossy(&pt).to_string();
        println!("{}", &pt);

        let r = parse_response(&url, &pt).unwrap();

        if let Response::Success(r) = r {
            Ok(LoadStatus::Success(DocumentData {
                url: url.clone(),
                content: r,
            }))
        } else {
            Ok(LoadStatus::Error(r))
        }
    }

    async fn load_file(url: &Url) -> Result<LoadStatus, String> {
        use async_std::fs::File;

        let path = url.path().strip_prefix("/").unwrap();
        dbg!(&path);
        let mut file = File::open(path).await.map_err(|e| format!("Failed to open file: {}", e))?;

        let mut content = String::new();
        file.read_to_string(&mut content).await.map_err(|e| format!("Failed to read file: {}", e))?;
        let r = match parse_gemtext(&url, content) {
            Ok(r) => r,
            Err(e) => return Err(format!("Failed to parse gemtext: {}", e))
        };

        Ok(LoadStatus::Success(DocumentData {
            url: url.clone(),
            content: OkResponse {
                mime: Default::default(),
                body: r
            }
        }))
    }
}

fn link_style(theme: &Theme, status: Status) -> Style {
    let text = theme.palette().primary;

    let style = Style {
        background: Background::Color(Color::TRANSPARENT).into(),
        text_color: text,
        border: Border::default(),
        shadow: Shadow::default(),
    };

    match status {
        Status::Pressed => {
            let mut text = text.clone();
            text.r = text.r * 0.9;
            text.g = text.g * 0.9;
            text.b = text.b * 0.9;

            let mut style = style.clone();

            style.text_color = text;

            style
        },
        Status::Hovered => {
            let mut text = text.clone();
            text.r = text.r * 1.1;
            text.g = text.g * 1.1;
            text.b = text.b * 1.1;

            let mut style = style.clone();

            style.text_color = text;

            style
        },
        _ => style
    }
}

