use std::{collections::HashMap, thread, time::Duration};

use crossbeam_channel::{bounded, Receiver, Sender};
use eframe::{
    egui::{FontFamily, FontId, TextEdit, TextStyle},
    App,
};
use ratelimit::Ratelimiter;

use crate::api::user::{Client, DMChat};

pub fn create_ui() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "rust discord",
        native_options,
        Box::new(|cc| Ok(Box::new(Data::new(cc)))),
    )
    .unwrap();
}

struct Data {
    token: Option<String>,
    cur_guild_id: Option<String>,
    cur_channel_id: Option<String>,
    sender: Sender<Message>,
    receiver: Receiver<Message>,
    ratelimit: Ratelimiter,
    text_edit: HashMap<String, String>,
}

enum Message {
    GetDms(Vec<DMChat>),
    GetToken(String),
}

impl Data {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
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
        let (sender, receiver) = bounded(1);
        let token;
        if let Some(storage) = cc.storage {
            token = eframe::get_value::<Option<String>>(storage, eframe::APP_KEY).unwrap_or(None);
        } else {
            token = None;
        }

        let ratelimit = Ratelimiter::builder(1, Duration::from_secs(5))
            .build()
            .unwrap();
        Self {
            ratelimit,
            sender,
            receiver,
            token,
            cur_guild_id: None,
            cur_channel_id: None,
            text_edit: HashMap::new(),
        }
    }
}

impl App for Data {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        ui(self, ctx, frame);
        collect_message(self);
        ctx.request_repaint();
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.token);
    }
}

fn run_async(
    message: impl Fn() -> Message + Send + 'static,
    ratelimit: &Ratelimiter,
    sender: Sender<Message>,
) {
    if ratelimit.try_wait().is_ok() {
        thread::spawn(move || {
            sender.send(message()).unwrap();
        });
    }
}

fn ui(data: &mut Data, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
    if data.token.is_none() {
        login_ui(data, ctx, frame);
        return;
    }
    central_panel(data, ctx, frame);
}

fn central_panel(data: &mut Data, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
    eframe::egui::CentralPanel::default().show(ctx, |ui| {
        ui.label("hello");
        ui.label("world");
    });
}

fn login_ui(data: &mut Data, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
    eframe::egui::CentralPanel::default().show(ctx, |ui| {
        ui.label("login");
        ui.horizontal(|ui| {
            ui.label("email");
            let mut text = data.text_edit.remove("login_email").unwrap_or_default();

            ui.text_edit_singleline(&mut text);
            data.text_edit.insert("login_email".to_string(), text);
        });
        ui.horizontal(|ui| {
            ui.label("password");
            let mut text = data.text_edit.remove("login_password").unwrap_or_default();

            ui.add(TextEdit::singleline(&mut text).password(true));
            data.text_edit.insert("login_password".to_string(), text);
        });
        if data.text_edit.contains_key("login_password")
            && data.text_edit.contains_key("login_email")
            && ui.button("submit").clicked()
        {
            let email = data.text_edit.remove("login_email").unwrap();
            let password = data.text_edit.remove("login_password").unwrap();
            let sender = data.sender.clone();
            let ratelimit = &data.ratelimit;
            run_async(
                move || Message::GetToken(Client::login_user(email.clone(), password.clone())),
                ratelimit,
                sender,
            );
        }
    });
}

fn collect_message(data: &mut Data) {
    match data.receiver.try_recv() {
        Ok(Message::GetToken(token)) => {
            println!("token acquired");
            let _ = data.token.insert(token);
        }
        _ => {}
    }
}
