use std::{
    collections::BTreeMap,
    io::Read,
    net::TcpStream,
    sync::{Arc, Mutex},
};

use anyhow::{Error, Result};
use serde::{Deserialize, Serialize};
use tungstenite::{stream::MaybeTlsStream, WebSocket};

fn decode_id(value: ureq::serde_json::Value) -> usize {
    match value {
        ureq::serde_json::Value::Number(num) => {
            num.as_u64().expect("expected numeric id: {num}") as usize
        }
        ureq::serde_json::Value::String(str) => {
            str.parse::<usize>().expect("expected numeric id: {str}")
        }
        _ => panic!("expected numeric id: {}", value),
    }
}

pub struct Mention {
    prefix: &'static str,
    id: usize,
}

impl std::fmt::Display for Mention {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(self.prefix)?;
        std::fmt::Display::fmt(&self.id, f)?;
        std::fmt::Write::write_char(f, '>')
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Deserialize)]
pub struct UserId(pub usize);

impl UserId {
    fn decode(value: ureq::serde_json::Value) -> Self {
        Self(decode_id(value))
    }

    #[inline(always)]
    pub fn mention(&self) -> Mention {
        Mention {
            prefix: "<@",
            id: self.0,
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct ChannelId(pub usize);

impl ChannelId {
    fn decode(value: ureq::serde_json::Value) -> Self {
        Self(decode_id(value))
    }

    #[inline(always)]
    pub fn mention(&self) -> Mention {
        Mention {
            prefix: "<#",
            id: self.0,
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct ApplicationId(pub usize);

impl ApplicationId {
    fn decode(value: ureq::serde_json::Value) -> Self {
        Self(decode_id(value))
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct ServerId(pub usize);

impl ServerId {
    fn decode(value: ureq::serde_json::Value) -> Self {
        Self(decode_id(value))
    }

    pub fn everyone(self) -> RoleId {
        RoleId(self.0)
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct MessageId(pub usize);

impl MessageId {
    fn decode(value: ureq::serde_json::Value) -> Self {
        Self(decode_id(value))
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct RoleId(pub usize);

impl RoleId {
    fn decode(value: ureq::serde_json::Value) -> Self {
        Self(decode_id(value))
    }

    #[inline(always)]
    pub fn mention(&self) -> Mention {
        Mention {
            prefix: "<&",
            id: self.0,
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct EmojiId(pub usize);

impl EmojiId {
    fn decode(value: ureq::serde_json::Value) -> Self {
        Self(decode_id(value))
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub enum ChannelType {
    Group,
    Private,
    Text,
    Voice,
    Category,
    News,
    Store,
    NewsThread,
    PublicThread,
    PrivateThread,
    StageVoice,
    Directory,
    Forum,
}

pub struct ChannelCategory {
    pub name: String,
    pub parent_id: Option<ChannelId>,
    pub nsfw: bool,
    pub position: isize,
    pub server_id: Option<ServerId>,
    pub id: ChannelId,
}

pub struct ServerInfo {
    pub id: ServerId,
    pub name: String,
    pub icon: Option<String>,
    pub owner: bool,
    // pub permissions: Permissions,
}

#[derive(Debug, Deserialize)]
pub struct ReadyEvent {
    // pub version: usize, // missing
    // pub user: CurrentUser,
    pub session_id: String,
    // pub user_settings: Option<UserSettings>,
    // pub read_state: Option<Vec<ReadState>>,
    // pub private_channels: Vec<Channel>,
    // pub presences: Vec<Presence>,
    // pub relationships: Vec<Relationship>,
    // pub servers: Vec<PossibleServer<LiveServer>>,
    // pub user_server_settings: Option<Vec<UserServerSettings>>,
    // pub tutorial: Option<Tutorial>,
    // pub trace: Vec<Option<String>>, // missing
    pub notes: Option<BTreeMap<UserId, Option<String>>>,
    pub shard: Option<[u8; 2]>,
}

#[derive(Debug)]
pub enum Event {
    Ready(ReadyEvent),
    Unknown(String, ureq::serde_json::Value),
}

impl Event {
    pub fn decode(kind: &str, value: ureq::serde_json::Value) -> Result<Self> {
        match kind.clone() {
            "READY" => Ok(Event::Ready(ureq::serde_json::from_value::<ReadyEvent>(
                value,
            )?)),
            other => {
                println!("unknown event: {:?}", kind.clone());
                Ok(Event::Unknown(kind.to_string(), value))
            }
        }
    }
}

#[derive(Debug)]
pub enum GatewayEvent {
    Dispatch(usize, Event),
    Heartbeat(usize),
    Reconnect,
    InvalidateSession,
    Hello(usize),
    HeartbeatAck,
}

impl GatewayEvent {
    pub fn decode(value: ureq::serde_json::Value) -> Result<Self> {
        Ok(match value.get("op").and_then(|x| x.as_u64()) {
            Some(0) => GatewayEvent::Dispatch(
                value
                    .get("s")
                    .expect("s not found in websocket message")
                    .as_u64()
                    .expect("unable to convert websocket message to u64") as usize,
                Event::decode(
                    value
                        .get("t")
                        .expect("t not found in websocket message")
                        .as_str()
                        .expect("could not convert to a string"),
                    value
                        .get("d")
                        .expect("d not found in websocket message")
                        .clone(),
                )?,
            ),
            Some(1) => GatewayEvent::Heartbeat(
                value
                    .get("s")
                    .expect("s not found in websocket message")
                    .as_u64()
                    .expect("unable to convert websocket message to u64") as usize,
            ),
            Some(7) => GatewayEvent::Reconnect,
            Some(9) => GatewayEvent::InvalidateSession,
            Some(10) => GatewayEvent::Hello(
                value
                    .get("d")
                    .expect("d not found in websocket message")
                    .get("heartbeat_interval")
                    .expect("heartbeat_interval not found in websocket message")
                    .as_u64()
                    .expect("unable to convert websocket message to u64") as usize,
            ),
            Some(11) => Self::HeartbeatAck,
            _ => return Err(Error::msg("unexpected opcode")),
        })
    }
}

pub fn receive_json<F, T>(
    websocket: &mut Arc<Mutex<WebSocket<MaybeTlsStream<TcpStream>>>>,
    decode: F,
) -> Result<T>
where
    F: FnOnce(ureq::serde_json::Value) -> Result<T>,
{
    let message = websocket.lock().unwrap().read()?;
    match message {
        tungstenite::Message::Text(text) => ureq::serde_json::from_str(&text)
            .map_err(From::from)
            .and_then(decode)
            .map_err(|e| e),
        tungstenite::Message::Binary(bin) => {
            let mut vec;
            let text = {
                vec = Vec::new();
                flate2::read::ZlibDecoder::new(&bin[..]).read_to_end(&mut vec)?;
                &vec[..]
            };
            ureq::serde_json::from_reader(text)
                .map_err(From::from)
                .and_then(decode)
                .map_err(|e| e)
        }
        _ => {
            todo!("websocket message not text or binary")
        }
    }
}
