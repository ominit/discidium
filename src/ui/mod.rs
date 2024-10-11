use std::{collections::HashMap, sync::Arc, thread, time::Duration};

use anyhow::Result;
use crossbeam_channel::{bounded, Receiver, Sender};
use eframe::{
    egui::{FontFamily, FontId, Layout, ScrollArea, TextEdit, TextStyle, Ui},
    App,
};

use crate::api::{client::Client, state::State};

pub fn create_ui() -> Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "discidium",
        native_options,
        Box::new(|cc| Ok(Box::new(Data::new(cc)?))),
    )
    .unwrap();
    Ok(())
}

struct Data {}

impl Data {
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

        let client;
        let connection;
        let state;
        if let Some(storage) = cc.storage {
            let token =
                eframe::get_value::<Option<String>>(storage, eframe::APP_KEY).unwrap_or(None);
            // token = None;
            if token.is_some() {
                client = Client::from_user_token(token.unwrap());
                let ready;
                (connection, ready) = client.connect()?;
                state = State::new(ready);
            }
        } else {
        }

        Ok(Self {})
    }
}

impl App for Data {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        // ui(self, ctx, frame);
        ctx.request_repaint();
    }

    // fn save(&mut self, storage: &mut dyn eframe::Storage) {
    //     eframe::set_value(storage, eframe::APP_KEY, &self.token);
    // }
}

// fn ui(data: &mut Data, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
//     if data.token.is_none() {
//         login_ui(data, ctx, frame);
//         return;
//     }
//     central_panel(data, ctx, frame);
// }

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

// fn login_ui(data: &mut Data, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
//     eframe::egui::CentralPanel::default().show(ctx, |ui| {
//         ui.label("login");
//         // ui.horizontal(|ui| {
//         //     ui.label("email");
//         //     let mut text = data.text_edit.remove("login_email").unwrap_or_default();

//         //     ui.text_edit_singleline(&mut text);
//         //     data.text_edit.insert("login_email".to_string(), text);
//         // });
//         // ui.horizontal(|ui| {
//         //     ui.label("password");
//         //     let mut text = data.text_edit.remove("login_password").unwrap_or_default();

//         //     ui.add(TextEdit::singleline(&mut text).password(true));
//         //     data.text_edit.insert("login_password".to_string(), text);
//         // });
//         // if data.text_edit.contains_key("login_password")
//         //     && data.text_edit.contains_key("login_email")
//         //     && ui.button("submit").clicked()
//         // {
//         //     let email = data.text_edit.remove("login_email").unwrap();
//         //     let password = data.text_edit.remove("login_password").unwrap();
//         //     let sender = data.sender.clone();
//         //     let ratelimit = &data.ratelimit;
//         //     run_async(
//         //         move || Message::GetToken(Client::login_user(email.clone(), password.clone())),
//         //         ratelimit,
//         //         sender,
//         //     );
//         // }
//         ui.horizontal(|ui| {
//             ui.label("token");
//             let mut text = data.text_edit.remove("login_token").unwrap_or_default();

//             ui.add(TextEdit::singleline(&mut text).password(true));
//             data.text_edit.insert("login_token".to_string(), text);
//         });
//         if data.text_edit.contains_key("login_token") && ui.button("submit").clicked() {
//             let _ = data
//                 .token
//                 .insert(data.text_edit.remove("login_token").unwrap());
//         }
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
