use std::io::{Read, Write};
use std::sync::Arc;
use iced::{Task, widget::{column, text}};
use rustls::ClientConfig;
use url::Url;
use protocol::gemini_protocol::parse_response;
use protocol::gemini_protocol::response::{OkResponse, Response};
use protocol::gemtext::gemtext_body::GemTextBody;
use crate::gemtext_iced_impl::gemtext_line_to_iced;
use crate::network::tls_client::TlsClient;

#[derive(Debug, Clone)]
pub enum LoadStatus {
    Success(DocumentData),
    Error(Url, Response),
}

#[derive(Debug, Clone)]
pub enum DocumentMessage {
    LoadComplete(Result<LoadStatus, String>),
    FIX_THIS,
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
                    DocumentMessage::LoadComplete(Ok(data)) => {
                        match data {
                            LoadStatus::Success(data) => {
                                *self = Self::Loaded(data);
                            },
                            LoadStatus::Error(url, response) => {
                                *self = Self::Error(url, response);
                            }
                        }

                        Task::none()
                    },
                    DocumentMessage::LoadComplete(Err(error)) => {
                        eprintln!("Failed to load document: {}", error);

                        Task::none()
                    },
                    _ => Task::none()
                }
            },
            Self::Error(_, _) => Task::none(),
            Self::Loaded(_) => Task::none()
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
                    columns = columns.push(gemtext_line_to_iced(&line));
                }

                columns.into()
            }
        }
    }

    async fn load_document(tls: Arc<ClientConfig>, url: Url) -> Result<LoadStatus, String> {
        const DEFAULT_PORT: u16 = 1965;

        let host = url.host_str().ok_or("No host found")?;
        let port = url.port().unwrap_or(DEFAULT_PORT);

        let mut conn = TlsClient::new_from_host((host, port), tls).
            map_err(|e| format!("Failed to connect: {}", e))?;

        write!(conn, "gemini://geminiprotocol.net/\r\n").unwrap();

        let mut pt = vec![];
        conn.read_to_end(&mut pt).unwrap();
        let pt = String::from_utf8_lossy(&pt).to_string();
        println!("{}", &pt);

        let r = parse_response(&pt).unwrap();

        if let Response::Success(r) = r {
            Ok(LoadStatus::Success(DocumentData {
                url,
                content: r,
            }))
        } else {
            Ok(LoadStatus::Error(url, r))
        }
    }
}
