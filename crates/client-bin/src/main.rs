mod document;
mod network;

use crate::document::{Document, DocumentMessage};
use crate::network::tls_config::make_tls_config;
use iced::widget::{button, column, row, scrollable, span, text, text_input, Button, Row, Text};
use iced::{Background, Center, Color, Element, Length, Task};
use log::info;
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
    DocumentGoBack,
    DocumentGoForward,
    DebugPrintDocument,
}

#[derive(Debug)]
pub struct GeminiRootWindow {
    document_cursor: usize,
    search_box: String,
    tls_config: Arc<ClientConfig>,
    documents: Vec<Document>,
}

impl GeminiRootWindow {
    fn new() -> (Self, Task<GeminiRootMessage>) {
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
                tls_config,
                documents,
            },
            Task::batch(tasks),
        )
    }

    fn update(&mut self, message: GeminiRootMessage) -> Task<GeminiRootMessage> {
        match message {
            GeminiRootMessage::Search => {
                info!("Search button pressed");
                let url = if self.search_box.starts_with("gemini://") {
                    Url::parse(&self.search_box).unwrap()
                } else {
                    Url::parse(&format!("gemini://{}", self.search_box)).unwrap()
                };
                let (document, task) = Document::new(self.tls_config.clone(), url);
                self.documents.push(document);

                let index = self.documents.len() - 1;
                task.map(move |d| {
                    return GeminiRootMessage::DocumentHasLoaded(index, d);
                })
            }
            GeminiRootMessage::SearchBoxChanged(s) => {
                info!("Search box changed to {}", s);
                self.search_box = s;

                Task::none()
            }
            GeminiRootMessage::DocumentMessage(index, msg) => {
                if let Some(document) = self.documents.get_mut(index) {
                    document
                        .update(msg)
                        .map(move |msg| GeminiRootMessage::DocumentMessage(index, msg))
                } else {
                    Task::none()
                }
            }
            GeminiRootMessage::DocumentHasLoaded(index, msg) => {
                if let Some(document) = self.documents.get_mut(index) {
                    self.document_cursor = index;

                    document
                        .update(msg)
                        .map(move |msg| GeminiRootMessage::DocumentMessage(index, msg))
                } else {
                    Task::none()
                }
            }
            GeminiRootMessage::ViewDocument(index) => {
                self.document_cursor = index;
                Task::none()
            }
            GeminiRootMessage::DocumentGoBack => {
                if let Some(document) = self.documents.get_mut(self.document_cursor) {
                    let cursor = self.document_cursor;
                    document
                        .go_back()
                        .map(move |msg| GeminiRootMessage::DocumentMessage(cursor, msg))
                } else {
                    info!("No document to go back");
                    Task::none()
                }
            }
            GeminiRootMessage::DocumentGoForward => {
                todo!();
            }
            GeminiRootMessage::DebugPrintDocument => {
                if let Some(document) = self.documents.get(self.document_cursor) {
                    info!("Document: {:#?}", document);
                } else {
                    info!("No document to debug print");
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> iced::Element<GeminiRootMessage> {
        let controls = self.view_controls();

        let mut document_tabs = Row::new();
        for (index, document) in self.documents.iter().enumerate() {
            let url = document.title();
            let b = Button::new(Text::new(url).width(Length::FillPortion(1)))
                .on_press(GeminiRootMessage::ViewDocument(index));
            let c = column![b];

            document_tabs = document_tabs.push(c);
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
            button("Search").on_press(GeminiRootMessage::Search),
            text_input("Enter a URL", &self.search_box)
                .on_input(GeminiRootMessage::SearchBoxChanged)
                .on_submit(GeminiRootMessage::Search),
            back_button,
            button("Debug Print Document").on_press(GeminiRootMessage::DebugPrintDocument)
        ]
        .spacing(10)
        .align_y(Center)
    }

    fn view_document(&self) -> iced::Element<GeminiRootMessage> {
        match self.documents.get(self.document_cursor) {
            None => text!("THIS SHOULD NEVER HAPPEN ({})", self.document_cursor).into(),
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
}

fn main() {
    env_logger::builder()
        .filter_module("wgpu_hal", log::LevelFilter::Off)
        .filter_module("gemini", log::LevelFilter::Debug)
        .try_init()
        .unwrap();

    // let mut window_size = Vec2::new(800.0, 600.0);
    iced::application(
        "Gemini Browser",
        GeminiRootWindow::update,
        GeminiRootWindow::view,
    )
    .run_with(GeminiRootWindow::new)
    .unwrap();
}
