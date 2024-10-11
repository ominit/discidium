use std::collections::BTreeMap;

use super::model::{CurrentUser, ReadyEvent, Relationship, ServerId, UserId};

pub struct State {
    user: CurrentUser,
    // servers: Vec<LiveServer>,
    dead_servers: Vec<ServerId>,
    // private_channels: Vec<PrivateChannels>,
    // groups: BTreeMap<ChannelId, Group>,
    // calls: BTreeMap<ChannelId, Call>,
    // presences: Vec<Presence>,
    relationships: Vec<Relationship>,
    // settings: UserSettings,
    // server_settings: Vec<ServersSettings>,
    notes: BTreeMap<UserId, Option<String>>,
}

impl State {
    pub fn new(ready: ReadyEvent) -> Self {
        // let mut servers = Vec::new();
        let mut unavailable = Vec::new();
        // for server in ready.servers {}

        Self {
            user: ready.user,
            dead_servers: unavailable,
            relationships: ready.relationships,
            notes: ready.notes,
        }
    }
}
