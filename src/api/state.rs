use std::collections::BTreeMap;

use super::model::ServerId;

pub struct State {
    // user: CurrentUser,
    // servers: Vec<LiveServer>,
    dead_servers: Vec<ServerId>,
    // private_channels: Vec<PrivateChannels>,
    // groups: BTreeMap<ChannelId, Group>,
    // calls: BTreeMap<ChannelId, Call>,
    // presences: Vec<Presence>,
    // friends: Vec<Friend>,
    // settings: DiscordSettings,
    // server_settings: Vec<ServersSettings>,
    // notes: BTreeMap<UserId, Option<String>>,
}
