use super::DiscidiumApp;

pub fn central_panel(data: &mut DiscidiumApp, ctx: &eframe::egui::Context) {
    eframe::egui::CentralPanel::default().show(ctx, |ui| {
        ui.horizontal(|ui| {
            // servers_ui(data, ui);
            ui.separator();
            // if data.cur_guild_id.is_some() {
            //     ui.vertical(|ui| {
            //         ui.label("channels");
            //         ui.label("todo");
            //     });
            // } else {
            //     dm_channels_ui(data, ui);
            // }
            ui.separator();
            // if data.cur_channel_id.is_some() {
            //     messages_ui(data, ui);
            // } else {
            //     ui.vertical(|ui| {
            //         ui.label("friends");
            //         ui.label("todo");
            //     });
            // }
        });
    });
}
