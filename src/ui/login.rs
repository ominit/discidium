use ratatui::{
    layout::{Constraint, Layout},
    text::{Text, ToText},
    widgets::{Block, Paragraph},
    Frame,
};
use tui_textarea::TextArea;

use super::{DiscidiumData, TUIApp};

pub fn login_ui(app: &mut TUIApp, frame: &mut Frame) {
    let [area] = Layout::horizontal([Constraint::Length(20)])
        .flex(ratatui::layout::Flex::Center)
        .areas(frame.area());
    let vertical = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .flex(ratatui::layout::Flex::Center);
    let [login_area, p_area, q_area, enter_area] = vertical.areas(area);
    frame.render_widget(Text::raw("login").centered(), login_area);
    frame.render_widget(Text::raw("p - paste token"), p_area);
    frame.render_widget(Text::raw("q - quit"), q_area);
    if app.text_edit.contains_key("login_token") {
        frame.render_widget(Text::raw("enter - login with pasted token"), enter_area);
    }
}
