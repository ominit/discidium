mod central_panel;
mod login;

use std::sync::{Arc, Mutex};

use central_panel::central_panel;
use keyring::Entry;
use login::login_ui;
use widget::node::BoxedUiNode;
use zng::prelude::*;

use crate::api::{client::Client, state::State, Connection};

pub fn create_ui() {
    zng::env::init!();
    let data = DiscidiumData::init();

    let app_data = data.clone(); // prevents the heartbeat sender from getting dropped
    APP.defaults().run_window(async move {
        Window! {
            title = "discidium";
            child = ui(app_data.clone());
        }
    });
}

struct DiscidiumData {
    client: Client,
    connection: Connection,
    state: State,
}

impl DiscidiumData {
    fn init() -> Option<Arc<Mutex<Self>>> {
        let entry = Entry::new("discidium", &whoami::username());
        if entry.is_ok() && entry.as_ref().unwrap().get_password().is_ok() {
            return Some(Self::from_keyring());
        }

        None
    }

    pub fn set_token(token: String) {
        let entry = Entry::new("discidium", &whoami::username()).unwrap();
        entry.set_password(&token).unwrap();
    }

    pub fn from_keyring() -> Arc<Mutex<Self>> {
        let entry = Entry::new("discidium", &whoami::username());
        let token = entry.unwrap().get_password().unwrap();
        let client = Client::from_user_token(token.into());
        let (connection, ready) = match client.connect() {
            Ok(a) => a,
            Err(err) => panic!("error connecting, Err: {:?}", err), // TODO if token doesnt work
        };
        let state = State::new(ready);
        Arc::new(Mutex::new(Self {
            client,
            connection,
            state,
        }))
    }

    pub fn from_token(token: String) -> Arc<Mutex<Self>> {
        Self::set_token(token);
        Self::from_keyring()
    }
}

fn ui(data: Option<Arc<Mutex<DiscidiumData>>>) -> BoxedUiNode {
    if data.is_none() {
        return login_ui(data);
    }
    central_panel(data.unwrap())
}
