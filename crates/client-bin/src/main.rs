mod document;
mod network;

use crate::document::{Document, DocumentMessage};
use crate::network::tls_config::make_tls_config;
use iced::widget::{button, column, row, scrollable, span, text, text_input, Button, Row, Text};
use iced::{Center, Element, Length, Task};
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
}

#[derive(Debug)]
pub struct GeminiRootWindow {
    tls_config: Arc<ClientConfig>,
    search_box: String,
    documents: Vec<Document>,
    document_cursor: usize,
}

impl GeminiRootWindow {
    fn new() -> (Self, Task<GeminiRootMessage>) {
        // let url = Url::parse("gemini://geminiprotocol.net/").unwrap();
        let url = Url::parse(&format!(
            "file://{}/../../files/test.gemini",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();

        let tls_config = make_tls_config().unwrap();
        let (document, task) = Document::new(tls_config.clone(), url);

        // let tasks = vec![task];

        (
            Self {
                tls_config,
                search_box: String::new(),
                documents: vec![document],
                document_cursor: 0, // }, Task::batch(tasks))
            },
            task.map(|d| GeminiRootMessage::DocumentMessage(0, d)),
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
        row![
            button("Search").on_press(GeminiRootMessage::Search),
            text_input("Enter a URL", &self.search_box)
                .on_input(GeminiRootMessage::SearchBoxChanged)
                .on_submit(GeminiRootMessage::Search)
        ]
        .spacing(10)
        .align_y(Center)
    }

    fn view_document(&self) -> iced::Element<GeminiRootMessage> {
        match self.documents.get(self.document_cursor) {
            None => text!("THIS SHOULD NEVER HAPPEN ({})", self.document_cursor).into(),
            Some(document) => {
                let view = document.view();
                view.map(move |msg| GeminiRootMessage::DocumentMessage(self.document_cursor, msg))
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
