use std::sync::Arc;

use cushy::{
    value::{Destination, Dynamic, DynamicRead, Source},
    widget::{MakeWidget, WidgetInstance},
    widgets::input::InputValue,
};
use parking_lot::Mutex;

use super::DiscidiumData;

pub fn login_ui(
    data: Arc<Mutex<Option<DiscidiumData>>>,
    is_logged_in: Dynamic<bool>,
) -> WidgetInstance {
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
                    let test = DiscidiumData::from_token(token_input.read().to_string());
                    if test.is_some() {
                        is_logged_in.set(true);
                        let _ = data.clone().lock().insert(test.unwrap());
                    }
                })
                .centered(),
        )
        .into_rows()
        .make_widget()
}
