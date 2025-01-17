mod components;

use std::sync::mpsc::{self, Receiver};

use crate::api::{client::Client, state::State, Connection};

use components::Login;
use keyring::Entry;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

pub fn create_ui() {
    // DiscidiumData::delete_token();
    yew::Renderer::<App>::new().render();
}

async fn data_thread(mut reciever: Receiver<Message>, mut state: UseStateHandle<Option<State>>) {
    let mut data = DiscidiumData::init().await;
    if data.is_some() {
        state.set(Some(data.as_ref().unwrap().state.clone()));
    }
    loop {
        use futures::StreamExt;
        if let Ok(message) = reciever.recv() {
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

#[function_component(App)]
fn app() -> Html {
    // let state = Mutable::new(None);
    let state = use_state(|| None);
    let (sender, receiver) = mpsc::channel();
    spawn_local(data_thread(receiver, state.clone()));
    // rsx! {
    //     document::Link { rel: "icon", href: FAVICON }
    //     document::Link { rel: "stylesheet", href: MAIN_CSS }
    //     if state.read().is_none() {
    //         Login { sender }
    //     } else {
    //         "logged in!"
    //     }
    // }
    if state.as_ref().is_none() {
        return html! {
            <Login  />
        };
    }
    html! {
        <main class="container">
            <h1>{"Welcome to Tauri + Yew"}</h1>

            <div class="row">
                <a href="https://tauri.app" target="_blank">
                </a>
                <a href="https://yew.rs" target="_blank">
                </a>
            </div>
            <p>{"Click on the Tauri and Yew logos to learn more."}</p>
        </main>
    }
}

struct DiscidiumData {
    client: Client,
    connection: Connection,
    state: State,
}

impl DiscidiumData {
    async fn init() -> Option<Self> {
        let entry = Entry::new("discidium", &whoami::username());
        if entry.is_err() || entry.as_ref().unwrap().get_password().is_err() {
            return None;
        }
        let token = entry.unwrap().get_password().unwrap();
        let client = Client::from_user_token(token.into());
        let (connection, ready) = match client.connect().await {
            Ok(a) => a,
            Err(err) => {
                eprintln!("error connecting, Err: {:?}", err); // TODO if token doesnt work
                return None;
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

    pub async fn from_token(token: String) -> Option<Self> {
        Self::set_token(token.clone());
        let client = Client::from_user_token(token.into());
        let (connection, ready) = match client.connect().await {
            Ok(a) => a,
            Err(err) => {
                eprintln!("error connecting, Err: {:?}", err); // TODO if token doesnt work
                return None;
            }
        };
        let state = State::new(ready);
        Some(Self {
            client,
            connection,
            state,
        })
    }
}
