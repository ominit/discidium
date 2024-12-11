mod login;

use std::{collections::BTreeMap, sync::Arc};

use cushy::{
    value::{Destination, Dynamic, DynamicRead, IntoReader, Source, Switchable},
    widget::{MakeWidget, WidgetInstance},
    widgets::Label,
    InputState, Run,
};
use keyring::Entry;
use login::login_ui;
use parking_lot::Mutex;

use crate::api::{
    client::Client,
    model::{ChannelId, ServerId},
    state::State,
    Connection,
};

pub fn create_ui() {
    DiscidiumData::delete_token();
    let data = Arc::new(Mutex::new(DiscidiumData::init()));
    ui(data).run().unwrap();
}

fn ui(data: Arc<Mutex<Option<DiscidiumData>>>) -> WidgetInstance {
    let is_logged_in = Dynamic::new(data.lock().is_some());
    is_logged_in
        .switcher(move |current: &bool, x| {
            if !current {
                return login_ui(data.clone(), x.clone());
            }
            "a".make_widget()
        })
        .make_widget()
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
            Err(err) => panic!("error connecting, Err: {:?}", err), // TODO if token doesnt work
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
