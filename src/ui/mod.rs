mod components;

use components::Login;
use dioxus::prelude::*;
use keyring::Entry;

use crate::api::{Connection, client::Client, state::State};

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/styling/main.css");

pub fn create_ui() {
    // DiscidiumData::delete_token();
    dioxus::launch(App);
}

async fn data_thread(mut reciever: UnboundedReceiver<Message>, mut state: Signal<Option<State>>) {
    let mut data = DiscidiumData::init();
    if data.is_some() {
        state.set(Some(data.as_ref().unwrap().state.clone()));
    }
    loop {
        use futures::StreamExt;
        if let Some(message) = reciever.next().await {
            match message {
                Message::Login(new_data) => {
                    let _ = data.insert(new_data);
                    state.set(Some(data.as_ref().unwrap().state.clone()));
                }
            };
        } else {
            break;
        }
    }
}

enum Message {
    Login(DiscidiumData),
}

#[component]
fn App() -> Element {
    let state = use_signal(|| None);
    let sender =
        use_coroutine(move |reciever: UnboundedReceiver<Message>| data_thread(reciever, state));
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        if state.read().is_none() {
            Login { sender }
        } else {
            "logged in!"
        }
    }
}

#[derive(Debug)]
struct DiscidiumData {
    client: Client,
    connection: Connection,
    state: State,
}

impl DiscidiumData {
    fn init() -> Option<Self> {
        let entry = Entry::new("discidium", &whoami::username());
        if entry.is_err() || entry.as_ref().unwrap().get_password().is_err() {
            return None;
        }
        let token = entry.unwrap().get_password().unwrap();
        let client = Client::from_user_token(token.into());
        let (connection, ready) = match client.connect() {
            Ok(a) => a,
            Err(err) => {
                return None;
                // panic!("error connecting, Err: {:?}", err) // TODO if token doesnt work
            }
        };
        let state = State::new(ready);
        Some(Self {
            client,
            connection,
            state,
        })
    }

    pub fn set_token(token: String) {
        let entry = Entry::new("discidium", &whoami::username()).unwrap();
        entry.set_password(&token).unwrap();
    }

    pub fn delete_token() {
        let _ = Entry::new("discidium", &whoami::username()).and_then(|x| x.delete_credential());
    }

    pub fn from_token(token: String) -> Option<Self> {
        Self::set_token(token);
        Self::init()
    }
}
