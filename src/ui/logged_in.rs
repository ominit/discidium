use std::sync::{mpsc::Sender, Arc};

use cushy::{
    context::Trackable,
    figures::IntoComponents,
    value::{Dynamic, DynamicRead, IntoValue, Source},
    widget::{MakeWidget, MakeWidgetList, WidgetInstance},
    widgets::Canvas,
};
use parking_lot::Mutex;

use crate::api::{
    model::{Channel, ChannelId, ServerId},
    state::State,
};

use super::{DiscidiumData, Message};

pub fn logged_in_ui(state: Arc<Mutex<State>>, sender: Sender<Message>) -> WidgetInstance {
    let state = Dynamic::new(state);
    let cur_server_id: Dynamic<Option<ServerId>> = Dynamic::new(None);
    let cur_channel_id: Dynamic<Option<ChannelId>> = Dynamic::new(None);

    let widget = servers_list(state.clone(), sender.clone())
        .and(channels_list(
            state.clone(),
            sender.clone(),
            cur_server_id.clone(),
            cur_channel_id.clone(),
        ))
        .into_columns();
    widget.make_widget()
}

fn servers_list(state: Dynamic<Arc<Mutex<State>>>, sender: Sender<Message>) -> WidgetInstance {
    "servers".make_widget()
}

fn channels_list(
    state: Dynamic<Arc<Mutex<State>>>,
    sender: Sender<Message>,
    cur_server_id: Dynamic<Option<ServerId>>,
    cur_channel_id: Dynamic<Option<ChannelId>>,
) -> WidgetInstance {
    let channels = cur_server_id.map_each(move |id| -> Dynamic<Vec<Channel>> {
        match *id {
            Some(_) => {
                todo!();
            }
            None => state.map_each(|x| x.lock().private_channels.clone()),
        }
    });
    "channels"
        .and(
            channels
                .map_ref(|x| {
                    x.map_ref(|x| {
                        x.iter()
                            .map(|x| match x.clone() {
                                Channel::Group(channel) => channel.id.0.to_string().make_widget(),
                                Channel::Private(channel) => {
                                    channel.recipient.username.make_widget()
                                }
                                Channel::Public(_) => todo!(),
                                Channel::Category(_) => todo!(),
                                Channel::News => todo!(),
                                Channel::Store => todo!(),
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .into_rows(),
        )
        .into_rows()
        .make_widget()
}
