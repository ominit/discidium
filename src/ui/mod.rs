mod login;

use std::{collections::BTreeMap, sync::Arc};

use crossterm::event::{Event, KeyEvent, KeyModifiers};
use keyring::Entry;
use login::login_ui;
use parking_lot::Mutex;
use ratatui::{text::Text, DefaultTerminal, Frame};

use crate::api::{client::Client, state::State, Connection};

pub fn create_ui() {
    // DiscidiumData::delete_token();
    let terminal = ratatui::init();
    TUIApp::new().run(terminal);
    ratatui::restore();
}

pub(super) struct TUIApp {
    pub data: Option<DiscidiumData>,
    pub text_edit: BTreeMap<String, String>,
}

impl TUIApp {
    fn new() -> Self {
        Self {
            data: DiscidiumData::init(),
            text_edit: BTreeMap::new(),
        }
    }

    fn run(mut self, mut terminal: DefaultTerminal) {
        loop {
            terminal
                .draw(|frame| self.ui(frame))
                .expect("unable to draw frame");
            let event = crossterm::event::read().expect("failed to read event");
            if self.data.is_none() {
                if event
                    == Event::Key(KeyEvent::new(
                        crossterm::event::KeyCode::Char('q'),
                        KeyModifiers::empty(),
                    ))
                {
                    break;
                }
                if event
                    == Event::Key(KeyEvent::new(
                        crossterm::event::KeyCode::Char('p'),
                        KeyModifiers::empty(),
                    ))
                {
                    let token = arboard::Clipboard::new().unwrap().get_text();
                    if token.is_ok() {
                        self.text_edit
                            .insert("login_token".to_owned(), token.unwrap());
                    }
                }
                if event
                    == Event::Key(KeyEvent::new(
                        crossterm::event::KeyCode::Enter,
                        KeyModifiers::empty(),
                    ))
                    && self.text_edit.contains_key("login_token")
                {
                    self.data =
                        DiscidiumData::from_token(self.text_edit.remove("login_token").unwrap());
                }
            }
            if event
                == Event::Key(KeyEvent::new(
                    crossterm::event::KeyCode::Char('q'),
                    KeyModifiers::empty(),
                ))
            {
                break;
            }
        }
    }

    fn ui(&mut self, frame: &mut Frame) {
        if self.data.is_none() {
            login_ui(self, frame);
            return;
        }
        frame.render_widget(Text::raw("hi"), frame.area());
    }
}

#[derive(Debug)]
struct DiscidiumData {
    client: Arc<Mutex<Client>>,
    connection: Arc<Mutex<Connection>>,
    state: Arc<Mutex<State>>,
}

impl DiscidiumData {
    fn init() -> Option<Self> {
        let entry = Entry::new("discidium", &whoami::username());
        if entry.is_err() || entry.as_ref().unwrap().get_password().is_err() {
            return None;
        }
        let token = entry.unwrap().get_password().unwrap();
        let client = Arc::new(Mutex::new(Client::from_user_token(token.into())));
        let (_connection, ready) = match client.lock().connect() {
            Ok(a) => a,
            Err(err) => panic!("error connecting, Err: {:?}", err), // TODO if token doesnt work
        };
        let connection = Arc::new(Mutex::new(_connection));
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
