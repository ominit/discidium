mod logged_in;
mod login;

use std::{
    collections::BTreeMap,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread::{self, Thread},
    time::Duration,
};

use cushy::{
    value::{Destination, Dynamic, DynamicRead, IntoReader, Source, Switchable},
    widget::{MakeWidget, WidgetInstance},
    widgets::Label,
    InputState, Run,
};
use keyring::Entry;
use logged_in::logged_in_ui;
use login::login_ui;
use parking_lot::Mutex;

use crate::api::{
    client::Client,
    model::{ChannelId, ServerId},
    state::State,
    Connection,
};

pub fn create_ui() {
    // DiscidiumData::delete_token();
    let data = DiscidiumData::init();
    let (sender, reciever) = mpsc::channel::<Message>();
    let (state_sender, state_reciever) = mpsc::channel::<Arc<Mutex<State>>>();
    let is_logged_in = data.as_ref().is_some();
    if data.as_ref().is_some() {
        sender.send(Message::Login(data.unwrap())).unwrap();
    }
    thread::spawn(move || data_thread(reciever, state_sender));
    ui(state_reciever, sender, is_logged_in).run().unwrap();
}

fn data_thread(reciever: Receiver<Message>, state_sender: Sender<Arc<Mutex<State>>>) {
    let mut data = None;
    loop {
        let message = reciever.recv().unwrap();
        match message {
            Message::Login(new_data) => {
                let _ = data.insert(new_data);
                state_sender
                    .send(data.as_ref().unwrap().state.clone())
                    .unwrap();
            }
        };
    }
}

enum Message {
    Login(DiscidiumData),
}

fn ui(
    state_reciever: Receiver<Arc<Mutex<State>>>,
    sender: Sender<Message>,
    logged_in: bool,
) -> WidgetInstance {
    if logged_in {
        println!("arstarstsara");
        let state = state_reciever.recv().unwrap();
        println!("arstarstsaraaaaaa");
        return logged_in_ui(state.clone(), sender.clone());
    }
    let is_logged_in = Dynamic::new(false);
    is_logged_in
        .switcher(move |current: &bool, x| {
            if !current {
                return login_ui(sender.clone(), x.clone());
            }
            let state = state_reciever.recv().unwrap();
            logged_in_ui(state.clone(), sender.clone())
        })
        .make_widget()
}

#[derive(Debug)]
struct DiscidiumData {
    client: Client,
    connection: Connection,
    state: Arc<Mutex<State>>,
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
        let state = Arc::new(Mutex::new(State::new(ready)));
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
