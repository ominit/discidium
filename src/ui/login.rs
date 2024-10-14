use eframe::egui::TextEdit;
use secrecy::SecretString;

use super::DiscidiumApp;

pub fn login_ui(app: &mut DiscidiumApp, ctx: &eframe::egui::Context) {
    eframe::egui::CentralPanel::default().show(ctx, |ui| {
        ui.label("login");
        // ui.horizontal(|ui| {
        //     ui.label("email");
        //     let mut text = data.text_edit.remove("login_email").unwrap_or_default();

        //     ui.text_edit_singleline(&mut text);
        //     data.text_edit.insert("login_email".to_string(), text);
        // });
        // ui.horizontal(|ui| {
        //     ui.label("password");
        //     let mut text = data.text_edit.remove("login_password").unwrap_or_default();

        //     ui.add(TextEdit::singleline(&mut text).password(true));
        //     data.text_edit.insert("login_password".to_string(), text);
        // });
        // if data.text_edit.contains_key("login_password")
        //     && data.text_edit.contains_key("login_email")
        //     && ui.button("submit").clicked()
        // {
        //     let email = data.text_edit.remove("login_email").unwrap();
        //     let password = data.text_edit.remove("login_password").unwrap();
        //     let sender = data.sender.clone();
        //     let ratelimit = &data.ratelimit;
        //     run_async(
        //         move || Message::GetToken(Client::login_user(email.clone(), password.clone())),
        //         ratelimit,
        //         sender,
        //     );
        // }
        ui.horizontal(|ui| {
            ui.label("token");
            let mut text = app.text_edit.remove("login_token").unwrap_or_default();

            ui.add(TextEdit::singleline(&mut text).password(true));
            app.text_edit.insert("login_token".to_string(), text);
        });
        if app.text_edit.contains_key("login_token") && ui.button("submit").clicked() {
            let token = app.text_edit.remove("login_token").unwrap();
            app.update_from_token(SecretString::from(token));
        }
    });
}
