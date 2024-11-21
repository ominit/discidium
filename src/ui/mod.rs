mod central_panel;
mod login;

use std::sync::{Arc, Mutex};

use central_panel::central_panel;
use keyring::Entry;
use login::login_ui;
use secrecy::{ExposeSecret, SecretString};
use widget::node::BoxedUiNode;
use zng::prelude::*;

use crate::api::{client::Client, state::State, Connection};

pub fn create_ui() {
    zng::env::init!();
    let data = Arc::new(Mutex::new(DiscidiumData::init()));

    APP.defaults().run_window(async move {
        Window! {
            title = "discidium";
            child = ui(data);
        }
    });
}

struct DiscidiumData {
    token: Option<SecretString>,
    client: Option<Client>,
    connection: Option<Connection>,
    state: Option<State>,
}

impl DiscidiumData {
    fn init() -> Self {
        let mut data = Self {
            token: None,
            client: None,
            connection: None,
            state: None,
        };

        let entry = Entry::new("discidium", &whoami::username());
        if entry.is_ok() && entry.as_ref().unwrap().get_password().is_ok() {
            let token = entry.unwrap().get_password().unwrap();
            data.update_from_token(SecretString::from(token));
        }

        data
    }

    pub fn update_from_token(&mut self, token: SecretString) {
        let client = Client::from_user_token(token.clone());
        let (connection, ready) = match client.connect() {
            Ok(a) => a,
            Err(err) => panic!("token wrong, Err: {:?}", err), // TODO if token doesnt work
        };
        self.client = Some(client);
        self.connection = Some(connection);
        self.state = Some(State::new(ready));
        self.token = Some(token);
        let entry = Entry::new("discidium", &whoami::username()).unwrap();
        entry
            .set_password(self.token.clone().unwrap().expose_secret())
            .unwrap();
    }
}

fn ui(data: Arc<Mutex<DiscidiumData>>) -> BoxedUiNode {
    if data.lock().unwrap().token.is_none() {
        return login_ui(data);
    }
    central_panel(data)
}

// fn servers_ui(data: &mut Data, ui: &mut Ui) {
//     ui.vertical(|ui| {
//         if ui.button("home").clicked() {
//             data.cur_guild_id = None;
//             data.cur_channel_id = None;
//         }
//         ui.label("servers");
//     });
// }

// fn messages_ui(data: &mut Data, ui: &mut Ui) {
//     ui.vertical(|ui| {
//         let channel = data
//             .dm_channels
//             .get(&data.cur_channel_id.clone().unwrap())
//             .unwrap();
//         ui.label(channel.get_dm_name());
//         ui.separator();
//         if let Some(messages) = data.channel_messages.get(&channel.id) {
//             ScrollArea::vertical()
//                 .auto_shrink([false, false])
//                 .show(ui, |ui| {
//                     for m in messages {
//                         ui.label(m.author.username.clone() + ": " + &m.content.clone());
//                     }
//                 });
//             let mut send_message = data.text_edit.remove("send_message").unwrap_or_default();
//             ui.text_edit_singleline(&mut send_message);
//             data.text_edit
//                 .insert("send_message".to_string(), send_message);
//             if ui.button("send").clicked() {
//                 let token = data.token.clone().unwrap();
//                 let channel_id = data.cur_channel_id.clone().unwrap();
//                 let message = data.text_edit.remove("send_message").unwrap_or_default();
//                 run_async(
//                     move || {
//                         SendMessage::SendMessage(Client::send_message(
//                             channel_id.clone(),
//                             message.clone(),
//                             token.clone(),
//                         ))
//                     },
//                     data.ratelimit.clone(),
//                     data.sender.clone(),
//                 )
//             }
//         } else {
//             ui.label("loading");
//         }
//     });
// }

// fn dm_channels_ui(data: &mut Data, ui: &mut Ui) {
//     ui.vertical(|ui| {
//         ScrollArea::vertical()
//             .id_salt("dm_channels_ui")
//             .max_width(200.)
//             .auto_shrink([false, false])
//             .max_height(f32::INFINITY)
//             .show(ui, |ui| {
//                 for dm in &data.dm_channels {
//                     if ui.button(dm.1.get_dm_name().clone()).clicked() {
//                         let channel_id = dm.0.clone();
//                         let _ = data.cur_channel_id.insert(channel_id.clone());
//                         let sender = data.sender.clone();
//                         let ratelimit = &data.ratelimit;
//                         let token = data.token.clone().unwrap();
//                         run_async(
//                             move || {
//                                 SendMessage::GetMessages(
//                                     Client::get_messages(token.clone(), channel_id.clone()),
//                                     channel_id.clone(),
//                                 )
//                             },
//                             ratelimit.clone(),
//                             sender.clone(),
//                         )
//                     }
//                 }
//             });
//     });
// }

// fn collect_message(data: &mut Data) {
//     match data.receiver.try_recv() {
//         Ok(SendMessage::GetToken(token)) => {
//             println!("token acquired");
//             let _ = data.token.insert(token);
//         }
//         Ok(SendMessage::GetDms(dms)) => {
//             println!("dms acquired");
//             for dm in dms {
//                 data.dm_channels.insert(dm.id.clone(), dm);
//             }
//         }
//         Ok(SendMessage::GetMessages(messages, channel_id)) => {
//             // TODO get messages and sort by timestamp (be able to load more messages)
//             println!("messages acquired");
//             data.channel_messages.insert(channel_id, messages);
//         }
//         _ => {}
//     }
// }
