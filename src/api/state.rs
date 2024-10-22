use std::collections::BTreeMap;

use super::model::{
    CurrentUser, Event, Group, PrivateChannel, PublicChannel, ReadyEvent, Relationship, ServerId,
    UserId,
};

pub struct State {
    user: CurrentUser,
    // servers: Vec<LiveServer>,
    dead_servers: Vec<ServerId>,
    // private_channels: Vec<PrivateChannel>,
    // groups: BTreeMap<ChannelId, Group>,
    // calls: BTreeMap<ChannelId, Call>,
    // presences: Vec<Presence>,
    relationships: Vec<Relationship>,
    // settings: UserSettings,
    // server_settings: Vec<ServersSettings>,
    notes: Option<BTreeMap<UserId, Option<String>>>,
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
            notes: Some(ready.notes),
        }
    }

    pub fn update(&mut self, event: &Event) {
        match *event {
            Event::Ready(ref ready) => *self = State::new(ready.clone()),
            _ => {}
        }
    }

    #[inline]
    pub fn user(&self) -> &CurrentUser {
        &self.user
    }

    #[inline]
    pub fn relationships(&self) -> &[Relationship] {
        &self.relationships
    }

    #[inline]
    pub fn notes(&self) -> Option<&BTreeMap<UserId, Option<String>>> {
        self.notes.as_ref()
    }
}

pub enum ChannelRef<'a> {
    Private(&'a PrivateChannel),
    Group(&'a Group),
    // Public(&'a LiveServer, &'a PublicChannel),
}
