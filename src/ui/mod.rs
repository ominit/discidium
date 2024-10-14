mod login;

use std::collections::BTreeMap;

use aes_gcm::{
    aead::{Aead, OsRng},
    AeadCore, Aes256Gcm, Key, KeyInit,
};
use anyhow::Result;
use eframe::{
    egui::{FontFamily, FontId, TextStyle},
    App,
};
use login::login_ui;
use secrecy::{ExposeSecret, SecretString};

use crate::api::{client::Client, state::State, Connection};

pub fn create_ui() -> Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "discidium",
        native_options,
        Box::new(|cc| Ok(Box::new(DiscidiumApp::new(cc)?))),
    )
    .unwrap();
    Ok(())
}

struct DiscidiumApp {
    token: Option<SecretString>,
    client: Option<Client>,
    connection: Option<Connection>,
    state: Option<State>,
    text_edit: BTreeMap<String, String>,
}

impl DiscidiumApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Result<Self> {
        let mut style = (*cc.egui_ctx.style()).clone();
        style.text_styles = [
            (TextStyle::Body, FontId::new(23.0, FontFamily::Proportional)),
            (
                TextStyle::Heading,
                FontId::new(23.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Button,
                FontId::new(23.0, FontFamily::Proportional),
            ),
        ]
        .into();

        cc.egui_ctx.set_style(style);

        let mut app = Self {
            token: None,
            client: None,
            connection: None,
            state: None,
            text_edit: BTreeMap::new(),
        };

        if let Some(storage) = cc.storage {
            if let Some(Some((text, nonce))) =
                eframe::get_value::<Option<(Vec<u8>, Vec<u8>)>>(storage, eframe::APP_KEY)
            {
                let sys = sysinfo::System::new_all();
                let key = [
                    [
                        sys.total_memory().to_le_bytes(),
                        sys.physical_core_count().unwrap().to_le_bytes(),
                    ]
                    .concat(),
                    [0, 0, 0].to_vec(),
                    sysinfo::System::distribution_id().into_bytes(),
                    sysinfo::System::cpu_arch().unwrap().into_bytes().into(),
                ]
                .concat();
                let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
                let token =
                    String::from_utf8(cipher.decrypt((&nonce[..]).into(), &text[..]).unwrap())
                        .unwrap();
                app.update_from_token(SecretString::from(token))
            }
        }

        Ok(app)
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
        self.token = Some(token)
    }
}

impl App for DiscidiumApp {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        ui(self, ctx, frame);
        ctx.request_repaint();
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if self.token.is_none() {
            eframe::set_value(storage, eframe::APP_KEY, &None::<(Vec<u8>, Vec<u8>)>);
            return;
        }
        let sys = sysinfo::System::new_all();
        let key = [
            [
                sys.total_memory().to_le_bytes(),
                sys.physical_core_count().unwrap().to_le_bytes(),
            ]
            .concat(),
            [0, 0, 0].to_vec(),
            sysinfo::System::distribution_id().into_bytes(),
            sysinfo::System::cpu_arch().unwrap().into_bytes().into(),
        ]
        .concat();
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let text = cipher
            .encrypt(
                &nonce,
                self.token.clone().unwrap().expose_secret().as_bytes(),
            )
            .unwrap();
        println!("{:?}", text);

        eframe::set_value(storage, eframe::APP_KEY, &Some((text, nonce.to_vec())));
    }
}

fn ui(app: &mut DiscidiumApp, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
    if app.token.is_none() {
        login_ui(app, ctx);
        return;
    }
    // central_panel(app, ctx, frame);
}

// fn central_panel(data: &mut Data, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
//     eframe::egui::CentralPanel::default().show(ctx, |ui| {
//         ui.horizontal(|ui| {
//             servers_ui(data, ui);
//             ui.separator();
//             if data.cur_guild_id.is_some() {
//                 ui.vertical(|ui| {
//                     ui.label("channels");
//                     ui.label("todo");
//                 });
//             } else {
//                 dm_channels_ui(data, ui);
//             }
//             ui.separator();
//             if data.cur_channel_id.is_some() {
//                 messages_ui(data, ui);
//             } else {
//                 ui.vertical(|ui| {
//                     ui.label("friends");
//                     ui.label("todo");
//                 });
//             }
//         });
//     });
// }

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
