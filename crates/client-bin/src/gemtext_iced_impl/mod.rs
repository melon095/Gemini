use iced::Element;
use iced::widget::text;
use protocol::gemtext::gemtext_body::Line;
use crate::document::DocumentMessage;
// TODO: This should be moved to allow Into<Line>.

pub fn gemtext_line_to_iced(line: &Line) -> impl Into<Element<'_, DocumentMessage>> {
    match line {
        Line::Text(val) => text(val),
        Line::Link { url, ..} => text(url),
        Line::Heading { text: t, .. } => text(t),
        Line::ListItem(value) => text(value),
        Line::Quote(value) => text(value),
        Line::PreformatToggleOn => text("```"),
        Line::PreformatToggleOff => text("```"),
    }
}