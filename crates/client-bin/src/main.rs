mod network;
mod document;

use std::sync::Arc;
use iced::{Center, Task};
use iced::widget::{button, column, row, scrollable, text_input, Row};
use log::info;
use rustls::ClientConfig;
use url::Url;
use crate::document::{Document, DocumentMessage};
use crate::network::tls_config::make_tls_config;

#[derive(Debug, Clone)]
pub enum GeminiRootMessage {
    Search,
    SearchBoxChanged(String),
    DocumentMessage(usize, DocumentMessage)
}

#[derive(Debug)]
pub struct GeminiRootWindow {
    tls_config: Arc<ClientConfig>,
    search_box: String,
    documents: Vec<Document>
}

impl GeminiRootWindow {
    fn new() -> (Self, Task<GeminiRootMessage>) {
        // let url = Url::parse("gemini://geminiprotocol.net/").unwrap();
        let url = Url::parse(&format!("file://{}/../../files/test.gemini", env!("CARGO_MANIFEST_DIR"))).unwrap();

        let tls_config = make_tls_config().unwrap();
        let (document, task) = Document::new(tls_config.clone(), url);

        // let tasks = vec![task];

        (Self {
            tls_config,
            search_box: String::new(),
            // documents: Vec::new()
            documents: vec![
                document
            ]
            // }, Task::batch(tasks))
        }, task.map(|d| GeminiRootMessage::DocumentMessage(0, d)))
    }

    fn update(&mut self, message: GeminiRootMessage) -> Task<GeminiRootMessage> {
        match message {
            GeminiRootMessage::Search => {
                info!("Searching");

                Task::none()
            }
            GeminiRootMessage::SearchBoxChanged(s) => {
                info!("Search box changed to {}", s);
                self.search_box = s;

                Task::none()
            }
            GeminiRootMessage::DocumentMessage(index, msg) =>
            {
                if let Some(document) = self.documents.get_mut(index) {
                    document
                        .update(msg)
                        .map(move |msg| GeminiRootMessage::DocumentMessage(index, msg))
                } else {
                    Task::none()
                }
            }
        }
    }

    fn view(&self) -> iced::Element<GeminiRootMessage> {
        let controls = self.view_controls();

        // TODO: Multiple
        let document = self.view_document();

        column![
            controls,
            document
        ]
            .spacing(10)
            .padding(10)
            .into()
    }

    fn view_controls(&self) -> Row<GeminiRootMessage> {
        row![
            button("Search")
                .on_press(GeminiRootMessage::Search),
            text_input("Enter a URL", &self.search_box)
                .on_input(GeminiRootMessage::SearchBoxChanged)
                .on_submit(GeminiRootMessage::Search)
        ]
            .spacing(10)
            .align_y(Center)
    }

    fn view_document(&self) -> iced::Element<GeminiRootMessage> {
        let document_views = self.documents
            .iter()
            .enumerate()
            .map(|(index, document)| {
                let view = document.view();
                view.map(move |msg| GeminiRootMessage::DocumentMessage(index, msg))
            })
            .collect::<Vec<_>>();

        scrollable(
            column!()
                .padding(20)
                .align_x(Center)
                .extend(document_views)
        ).into()
    }
}

fn main() {
    env_logger::builder()
        .filter_module("wgpu_hal", log::LevelFilter::Off)
        .filter_module("gemini", log::LevelFilter::Debug)
        .try_init()
        .unwrap();

    // let mut window_size = Vec2::new(800.0, 600.0);
    iced::application("Gemini Browser", GeminiRootWindow::update, GeminiRootWindow::view)
        .run_with(GeminiRootWindow::new)
        .unwrap();
}
