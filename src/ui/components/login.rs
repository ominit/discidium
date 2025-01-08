use dioxus::prelude::*;

use crate::ui::{DiscidiumData, Message};

#[component]
pub fn Login(sender: Coroutine<Message>) -> Element {
    let mut token = use_signal(|| "".to_string());
    rsx! {
        div {
            "token: "
            input {
                oninput: move |event| token.set(event.value()),
                value: "{token}"
            }
        }
        button {
            onclick: move |_| async move { // TODO make button wait for current event to finish before allowing another click
                let data = DiscidiumData::from_token(token.read().to_string()).await;
                if data.is_some() {
                    token.set("".to_string());
                    sender.send(Message::Login(data.unwrap()));
                }
            },
            "submit"
        }
    }
}
