use super::model::{Channel, CurrentUser, Event, Presence, ReadyEvent};

#[derive(Debug, Clone)]
pub struct State {
    presences: Vec<Presence>,
    pub private_channels: Vec<Channel>,
    user: CurrentUser,
}

impl State {
    pub fn new(ready: ReadyEvent) -> Self {
        Self {
            presences: ready.presences,
            private_channels: ready.private_channels,
            user: ready.user,
        }
    }

    pub fn update(&mut self, event: &Event) {
        match *event {
            Event::Ready(ref ready) => *self = State::new(ready.clone()),
            _ => {}
        }
    }
}
