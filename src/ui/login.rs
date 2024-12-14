use std::sync::mpsc::Sender;

use cushy::{
    value::{Destination, Dynamic, DynamicRead},
    widget::{MakeWidget, WidgetInstance},
    widgets::input::InputValue,
};

use super::{DiscidiumData, Message};

pub fn login_ui(sender: Sender<Message>, is_logged_in: Dynamic<bool>) -> WidgetInstance {
    let token_input: Dynamic<String> = Dynamic::default();
    token_input
        .to_input()
        .mask_symbol("•")
        .placeholder("token")
        .centered()
        .and(
            "login"
                .into_button()
                .on_click(move |_| {
                    let data = DiscidiumData::from_token(token_input.read().to_string());
                    if data.is_some() {
                        sender.send(Message::Login(data.unwrap())).unwrap();
                        is_logged_in.set(true);
                    }
                })
                .centered(),
        )
        .into_rows()
        .make_widget()
}
