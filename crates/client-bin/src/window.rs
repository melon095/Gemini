use crate::document::{Document, DocumentMessage};
use crate::network::tls_config::make_tls_config;
use iced::widget::{button, column, row, scrollable, text, text_input, Button, Row, Text};
use iced::{Background, Center, Color, Length, Task};
use iced_aw::ContextMenu;
use log::{debug, error, info};
use rustls::ClientConfig;
use std::sync::Arc;
use url::Url;

#[derive(Debug, Clone)]
pub enum GeminiRootMessage {
    Search,
    SearchBoxChanged(String),
    DocumentMessage(usize, DocumentMessage),
    DocumentHasLoaded(usize, DocumentMessage),
    ViewDocument(usize),
    CloseDocument(usize),
    DocumentGoBack,
    DocumentGoForward,
    DebugPrintDocument,
    CurrentDocumentURLPotentialChange(String),
    UserWishesToNavigateDocument,
}

#[derive(Debug)]
pub struct GeminiRootWindow {
    document_cursor: usize,
    search_box: String,
    displayed_document_url: String,
    tls_config: Arc<ClientConfig>,
    documents: Vec<Document>,
}

impl GeminiRootWindow {
    pub fn new() -> (Self, Task<GeminiRootMessage>) {
        let urls = vec![
            Url::parse("gemini://geminiprotocol.net/").unwrap(),
            Url::parse(&format!(
                "file://{}/../../files/test.gemini",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap(),
        ];

        let tls_config = make_tls_config().unwrap();

        let mut documents = Vec::new();
        let mut tasks = Vec::new();

        for (index, url) in urls.iter().enumerate() {
            let (document, task) = Document::new(tls_config.clone(), url.clone());
            documents.push(document);

            tasks.push(task.map(move |d| {
                return GeminiRootMessage::DocumentHasLoaded(index, d);
            }));
        }

        (
            Self {
                document_cursor: 0,
                search_box: String::new(),
                displayed_document_url: String::new(),
                tls_config,
                documents,
            },
            Task::batch(tasks),
        )
    }

    pub fn update(&mut self, message: GeminiRootMessage) -> Task<GeminiRootMessage> {
        match message {
            GeminiRootMessage::Search => {
                info!("Search button pressed");
                let url = canonicalize_url(&self.search_box);

                let (document, task) = Document::new(self.tls_config.clone(), url);
                self.documents.push(document);

                let index = self.documents.len() - 1;
                task.map(move |d| {
                    return GeminiRootMessage::DocumentHasLoaded(index, d);
                })
            }
            GeminiRootMessage::SearchBoxChanged(s) => {
                debug!("Search box changed to {}", s);
                self.search_box = s;

                Task::none()
            }
            GeminiRootMessage::DocumentMessage(index, msg) => match self.documents.get_mut(index) {
                Some(document) => document
                    .update(msg)
                    .map(move |msg| GeminiRootMessage::DocumentMessage(index, msg)),
                None => {
                    error!("[DocumentMessage] Document index out of bounds: {}", index);

                    Task::none()
                }
            },
            GeminiRootMessage::DocumentHasLoaded(index, msg) => {
                match self.documents.get_mut(index) {
                    Some(document) => {
                        self.document_cursor = index;

                        document
                            .update(msg)
                            .map(move |msg| GeminiRootMessage::DocumentMessage(index, msg))
                    }
                    None => {
                        error!(
                            "[DocumentHasLoaded] Document index out of bounds: {}",
                            index
                        );

                        Task::none()
                    }
                }
            }
            GeminiRootMessage::ViewDocument(index) => {
                if index < self.documents.len() {
                    self.document_cursor = index;

                    self.displayed_document_url = self.current_document_url().unwrap().to_string();
                }
                Task::none()
            }
            GeminiRootMessage::CloseDocument(index) => {
                self.documents.remove(index);
                if self.document_cursor >= self.documents.len() {
                    self.document_cursor = self.documents.len().saturating_sub(1);
                }
                Task::none()
            }
            GeminiRootMessage::DocumentGoBack => {
                match self.documents.get_mut(self.document_cursor) {
                    Some(document) => {
                        let cursor = self.document_cursor;
                        document
                            .update(DocumentMessage::NavigateBack)
                            .map(move |msg| GeminiRootMessage::DocumentMessage(cursor, msg))
                    }
                    None => Task::none(),
                }
            }
            GeminiRootMessage::DocumentGoForward => {
                todo!();
            }
            GeminiRootMessage::DebugPrintDocument => {
                match self.documents.get(self.document_cursor) {
                    Some(document) => debug!("Document: {:#?}", document),
                    None => debug!("No document to debug print"),
                }

                Task::none()
            }
            GeminiRootMessage::CurrentDocumentURLPotentialChange(s) => {
                debug!("Current document URL potential change: {}", s);
                self.displayed_document_url = s;
                Task::none()
            }
            GeminiRootMessage::UserWishesToNavigateDocument => {
                match self.documents.get_mut(self.document_cursor) {
                    Some(document) => {
                        let url = canonicalize_url(&self.displayed_document_url);
                        let cursor = self.document_cursor;

                        document
                            .update(DocumentMessage::NavigateUrl(url))
                            .map(move |msg| GeminiRootMessage::DocumentMessage(cursor, msg))
                    }
                    None => Task::none(),
                }
            }
        }
    }

    pub fn view(&self) -> iced::Element<GeminiRootMessage> {
        let controls = self.view_controls();

        let mut document_tabs = Row::new();
        for (index, document) in self.documents.iter().enumerate() {
            let url = document.title();
            let b = Button::new(Text::new(url).width(Length::FillPortion(1)))
                .on_press(GeminiRootMessage::ViewDocument(index));
            let c = column![b];

            let menu = ContextMenu::new(c, move || {
                column(vec![
                    Button::new(Text::new("Close"))
                        .on_press(GeminiRootMessage::CloseDocument(index))
                        .style(button::secondary)
                        .into(),
                ])
                .spacing(10)
                .into()
            });

            document_tabs = document_tabs.push(menu);
        }

        let document_tabs = scrollable(document_tabs.spacing(10));

        let document = self.view_document();

        column![controls, document_tabs, document]
            .spacing(10)
            .padding(10)
            .into()
    }

    fn view_controls(&self) -> Row<GeminiRootMessage> {
        let back_button = if self
            .documents
            .get(self.document_cursor)
            .map_or(false, |d| d.can_go_back())
        {
            button("Back").on_press(GeminiRootMessage::DocumentGoBack)
        } else {
            button("Back").style(|_, _| button::Style {
                background: Some(Background::Color(Color::from_rgb8(0x80, 0x80, 0x80))),
                ..Default::default()
            })
        };

        row![
            text_input("Current Document", &self.displayed_document_url.to_string())
                .width(Length::Fill)
                .padding(10)
                .on_input(GeminiRootMessage::CurrentDocumentURLPotentialChange)
                .on_submit(GeminiRootMessage::UserWishesToNavigateDocument),
            text_input("Enter a URL", &self.search_box)
                .padding(10)
                .on_input(GeminiRootMessage::SearchBoxChanged)
                .on_submit(GeminiRootMessage::Search),
            button("Search").on_press(GeminiRootMessage::Search),
            back_button,
            button("Debug Print Document").on_press(GeminiRootMessage::DebugPrintDocument)
        ]
        .spacing(10)
        .align_y(Center)
    }

    fn view_document(&self) -> iced::Element<GeminiRootMessage> {
        match self.documents.get(self.document_cursor) {
            None => text("No document to display").into(),
            Some(document) => {
                let view = document
                    .view()
                    .map(move |msg| GeminiRootMessage::DocumentMessage(self.document_cursor, msg));

                scrollable(view)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .spacing(10)
                    .into()
            }
        }
    }

    fn current_document_url(&self) -> Option<Url> {
        self.documents.get(self.document_cursor).map(|d| d.url())
    }
}

fn canonicalize_url(url: &str) -> Url {
    let url = if url.starts_with("gemini://") {
        Url::parse(url)
    } else {
        Url::parse(&format!("gemini://{}", url))
    };

    // FIXME: Handle invalid URLs better
    url.unwrap_or_else(|e| {
        error!("Invalid URL: {}", e);
        Url::parse("gemini://geminiprotocol.net/").unwrap()
    })
}
