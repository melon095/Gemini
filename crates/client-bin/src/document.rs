use crate::network::tls_client::TlsClient;
use iced::futures::AsyncReadExt;
use iced::widget::button::{Status, Style};
use iced::widget::{button, rich_text, span, Column};
use iced::{widget::text, Background, Border, Color, Shadow, Task, Theme};
use protocol::gemini_protocol::parse_response;
use protocol::gemini_protocol::response::{OkResponse, Response};
use protocol::gemtext::gemtext_body::Line;
use protocol::gemtext::parse_gemtext;
use rustls::ClientConfig;
use std::collections::LinkedList;
use std::io::{Read, Write};
use std::sync::Arc;
use url::Url;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ShouldSaveHistory {
    Yes,
    No,
}

#[derive(Debug, Clone)]
pub enum LoadStatus {
    Success(DocumentData),
    Error(Response),
}

#[derive(Debug, Clone)]
pub enum DocumentMessage {
    LoadComplete((Url, Result<LoadStatus, String>)),
    LinkPressed(Url),
}

#[derive(Debug, Clone)]
pub struct DocumentData {
    url: Url,
    content: OkResponse,
}

#[derive(Debug)]
pub struct Document {
    tls_config: Arc<ClientConfig>,
    pub history: LinkedList<Url>,
    pub state: DocumentState,
}

#[derive(Debug)]
pub enum DocumentState {
    Loading,
    Error(Url, Response),
    Loaded(DocumentData),
}

impl Document {
    pub fn new(tls_client: Arc<ClientConfig>, url: Url) -> (Self, Task<DocumentMessage>) {
        let mut doc = Self {
            tls_config: tls_client.clone(),
            history: LinkedList::new(),
            state: DocumentState::Loading,
        };
        let task = doc.load_new_page(url.clone(), ShouldSaveHistory::Yes);

        (doc, task)
    }

    pub fn title(&self) -> String {
        match &self.state {
            DocumentState::Loading => "Loading...".to_string(),
            DocumentState::Error(url, ..) => format!("Error {}", url),
            DocumentState::Loaded(data) => data.url.to_string(),
        }
    }

    pub fn can_go_back(&self) -> bool {
        self.history.len() > 1 && !matches!(self.state, DocumentState::Loading)
    }

    pub fn go_back(&mut self) -> Task<DocumentMessage> {
        assert!(self.can_go_back());

        if self.history.len() > 1 {
            self.history.pop_back();
            let url = self.history.back().unwrap().clone();

            self.load_new_page(url, ShouldSaveHistory::No)
        } else {
            Task::none()
        }
    }

    pub fn update(&mut self, message: DocumentMessage) -> Task<DocumentMessage> {
        match &self.state {
            DocumentState::Loading => {
                match message {
                    DocumentMessage::LoadComplete((url, Ok(data))) => match data {
                        LoadStatus::Success(data) => {
                            self.state = DocumentState::Loaded(data);
                        }
                        LoadStatus::Error(response) => {
                            self.state = DocumentState::Error(url, response);
                        }
                    },
                    DocumentMessage::LoadComplete((url, Err(error))) => {
                        eprintln!("Failed to load document: {}", error);

                        self.state = DocumentState::Error(
                            url,
                            Response::PermanentFailure(Some(format!(
                                "Failed to load document: {}",
                                error
                            ))),
                        );
                    }
                    _ => (),
                };

                Task::none()
            }
            DocumentState::Error(_, _) => Task::none(),
            DocumentState::Loaded(..) => match message {
                DocumentMessage::LinkPressed(url) => {
                    log::info!("Link pressed: {}", url);

                    self.load_new_page(url, ShouldSaveHistory::Yes)
                }
                _ => Task::none(),
            },
        }
    }

    pub fn view(&self) -> iced::Element<DocumentMessage> {
        match &self.state {
            DocumentState::Loading => text("Loading...").into(),
            DocumentState::Error(url, response) => {
                text(format!("Error: {}: {:?}", url, response)).into()
            }
            DocumentState::Loaded(data) => {
                let mut columns = Column::new();

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
                            let head = rich_text!(span(t)).size(10.0 + (10.0 * *depth as f32));

                            columns.push(head)
                        }
                        Line::ListItem(value) => {
                            let item = rich_text!(span(value)).size(10.0);

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

    fn load_new_page(
        &mut self,
        url: Url,
        should_save_history: ShouldSaveHistory,
    ) -> Task<DocumentMessage> {
        self.state = DocumentState::Loading;
        if should_save_history == ShouldSaveHistory::Yes {
            self.history.push_back(url.clone());
        }

        Task::perform(
            Self::load_document(self.tls_config.clone(), url.clone()),
            DocumentMessage::LoadComplete,
        )
    }

    async fn load_document(tls: Arc<ClientConfig>, url: Url) -> (Url, Result<LoadStatus, String>) {
        let r = match url.scheme() {
            "gemini" => Self::load_gemini(tls, &url).await,
            "file" => Self::load_file(&url).await,
            _ => Err(format!("Unsupported scheme: {}", url.scheme())),
        };

        (url, r)
    }

    async fn load_gemini(tls_config: Arc<ClientConfig>, url: &Url) -> Result<LoadStatus, String> {
        const DEFAULT_PORT: u16 = 1965;

        let host = url.host_str().ok_or("No host found")?;
        let port = url.port().unwrap_or(DEFAULT_PORT);

        let mut conn = TlsClient::new_from_host((host, port), tls_config.clone())
            .map_err(|e| format!("Failed to connect: {}", e))?;

        write!(conn, "{}\r\n", url.to_string()).unwrap();

        let mut pt = vec![];
        conn.read_to_end(&mut pt).unwrap();
        let pt = String::from_utf8_lossy(&pt).to_string();

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

        let mut file = File::open(path)
            .await
            .map_err(|e| format!("Failed to open file: {}", e))?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .await
            .map_err(|e| format!("Failed to read file: {}", e))?;
        let r = match parse_gemtext(&url, content) {
            Ok(r) => r,
            Err(e) => return Err(format!("Failed to parse gemtext: {}", e)),
        };

        Ok(LoadStatus::Success(DocumentData {
            url: url.clone(),
            content: OkResponse {
                mime: Default::default(),
                body: r,
            },
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
        }
        Status::Hovered => {
            let mut text = text.clone();
            text.r = text.r * 1.1;
            text.g = text.g * 1.1;
            text.b = text.b * 1.1;

            let mut style = style.clone();

            style.text_color = text;

            style
        }
        _ => style,
    }
}
