use std::sync::{Arc, Mutex};

use widget::node::BoxedUiNode;
use zng::prelude::*;

use super::DiscidiumData;

pub fn login_ui(data: Option<Arc<Mutex<DiscidiumData>>>) -> BoxedUiNode {
    let token = var(Txt::from_static(""));
    Box::new(Stack! {
        spacing = 25;
        direction = StackDirection::top_to_bottom();
        children_align = Align::CENTER;
        children = ui_vec![
            Text!("login"),
            Stack!{
                spacing = 10;
                direction = StackDirection::left_to_right();
                children = ui_vec![
                    Text!("token"),
                    TextInput!{
                        txt = token.clone();
                        obscure_txt = true;
                    },
                ];
            },
            Button!{
                on_click = hn!(token, mut data, |_| {
                    let _ = data.insert(DiscidiumData::from_token(token.get_string()));
                    token.set("");
                });
                child = Text!("submit");
            },
        ];
    })
}
