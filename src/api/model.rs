use std::{
    borrow::Cow,
    collections::BTreeMap,
    io::Read,
    net::TcpStream,
    sync::{Arc, Mutex},
};

use anyhow::{Error, Result};
use chrono::{DateTime, FixedOffset};
use serde::{
    de::{MapAccess, Visitor},
    Deserialize,
};
use tungstenite::{stream::MaybeTlsStream, WebSocket};

use super::CDN_URL;

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
pub struct UserId(
    #[serde(deserialize_with = "serde_aux::prelude::deserialize_number_from_string")] pub usize,
);

impl UserId {
    #[inline(always)]
    pub fn mention(&self) -> Mention {
        Mention {
            prefix: "<@",
            id: self.0,
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Deserialize)]
pub struct ChannelId(pub usize);

impl ChannelId {
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

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Deserialize)]
pub struct ServerId(pub usize);

impl ServerId {
    pub fn everyone(self) -> RoleId {
        RoleId(self.0)
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Deserialize)]
pub struct MessageId(pub usize);

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Deserialize)]
pub struct RoleId(pub usize);

impl RoleId {
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

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug, Deserialize)]
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

#[derive(Deserialize)]
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

pub struct Server {
    pub id: ServerId,
    pub name: String,
    pub afk_timeout: usize,
    pub afk_channel_id: Option<ChannelId>,
    pub icon: Option<String>,
    pub roles: Vec<Role>,
    pub region: String,
    pub embed_enabled: bool,
    pub embed_channel_id: Option<ChannelId>,
    pub owner_id: UserId,
    // pub verification_level: VereficationLevel,
    // pub emojis: Vec<Emoji>,
    pub features: Vec<String>,
    pub splash: Option<String>,
    pub default_message_notifications: usize,
    pub mfa_level: usize,
}

impl Server {
    pub fn icon_url(&self) -> Option<String> {
        self.icon
            .as_ref()
            .map(|x| format!("{}/icons/{}/{}.jpg", CDN_URL, self.id.0, x))
    }
}

/// number of members removed by server prune
pub struct ServerPrune {
    pub pruned: usize,
}

pub struct Role {
    pub id: RoleId,
    pub name: String,
    /// Color in 0xRRGGBB form
    pub color: usize,
    pub hoist: bool,
    pub managed: bool,
    pub position: isize,
    pub mentionable: bool,
    // pub permissions: Permissions,
}

impl Role {
    #[inline(always)]
    pub fn mention(&self) -> Mention {
        self.id.mention()
    }
}

pub struct Ban {
    reason: Option<String>,
    user: User,
}

#[derive(Deserialize, Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub username: String,
    #[serde(deserialize_with = "serde_aux::prelude::deserialize_number_from_string")]
    pub discriminator: u16,
    pub avatar: Option<String>,
    #[serde(default)]
    pub bot: bool,
}

impl User {
    #[inline(always)]
    pub fn mention(&self) -> Mention {
        self.id.mention()
    }

    pub fn avatar_url(&self) -> Option<String> {
        self.avatar
            .as_ref()
            .map(|x| format!("{}/avatars/{}/{}.jpg", CDN_URL, self.id.0, x))
    }
}

pub struct Member {
    pub user: User,
    pub roles: Vec<RoleId>,
    pub nick: Option<String>,
    pub joined_at: String,
    pub mute: bool,
    pub deaf: bool,
}

impl Member {
    pub fn display_name(&self) -> &str {
        if let Some(name) = self.nick.as_ref() {
            name
        } else {
            &self.user.username
        }
    }
}

pub enum Channel {
    Group(Group),
    Private(PrivateChannel),
    Public(PublicChannel),
    Category(ChannelCategory),
    News,
    Store,
}

impl Channel {
    pub fn decode(value: ureq::serde_json::Value) -> Result<Channel> {
        match value
            .get("type")
            .and_then(|x| x.as_u64())
            .expect("channel doesnt contain a type")
        {
            // 0 | 2 => Public
            // 1 => Private
            // 3 => Group
            4 => Ok(Channel::Category(ureq::serde_json::from_value::<
                ChannelCategory,
            >(value)?)),
            5 => Ok(Channel::News),
            6 => Ok(Channel::Store),
            other => panic!("unexpected channel type {:?}", other),
        }
    }
}

#[derive(Deserialize)]
pub struct Group {
    #[serde(rename = "id")]
    pub channel_id: ChannelId,
    pub icon: Option<String>,
    pub last_message_id: Option<MessageId>,
    pub last_pin_timestamp: Option<DateTime<FixedOffset>>,
    pub name: Option<String>,
    pub owner_id: UserId,
    #[serde(default)]
    pub recipients: Vec<User>,
}

impl Group {
    pub fn name(&self) -> Cow<str> {
        match self.name {
            Some(ref name) => Cow::Borrowed(name),
            None => {
                if self.recipients.is_empty() {
                    return Cow::Borrowed("Empty Group");
                }
                Cow::Owned(
                    self.recipients
                        .iter()
                        .map(|x| x.username.clone())
                        .collect::<Vec<String>>()
                        .join(", "),
                )
            }
        }
    }

    pub fn icon_url(&self) -> Option<String> {
        self.icon
            .as_ref()
            .map(|x| format!("{}/channel-icons/{}/{}.jpg", CDN_URL, self.channel_id.0, x))
    }
}

#[derive(Deserialize)]
pub struct Call {
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub region: String,
    pub ringring: Vec<UserId>,
    pub unavailable: bool,
    // pub voice_states: Vec<VoiceState>,
}

pub struct PrivateChannel {
    pub id: ChannelId,
    pub channel_type: ChannelType,
    pub recipient: User,
    pub last_message_id: Option<MessageId>,
    pub owner_id: Option<UserId>,
    pub application_id: Option<ApplicationId>,
    pub last_pin_timestamp: Option<DateTime<FixedOffset>>,
}

impl PrivateChannel {
    pub fn decode(value: ureq::serde_json::Value) -> Result<Self> {
        let mut recipients = ureq::serde_json::from_value::<Vec<User>>(
            value
                .get("recipients")
                .expect("recipients value not found")
                .clone(),
        )?;
        if recipients.len() != 1 {
            panic!("expected 1 recipient, found {:?}", recipients);
        }
        let id = ChannelId(
            value
                .get("id")
                .expect("unable to find id")
                .as_u64()
                .expect("unable to parse id") as usize,
        );
        let channel_type = ureq::serde_json::from_value::<ChannelType>(
            value.get("type").expect("unable to find type").clone(),
        )?;
        let mut last_message_id = None;
        if let Some(message_id) = value.get("last_message_id") {
            last_message_id = Some(MessageId(
                message_id.as_u64().expect("unable to parse id") as usize
            ));
        }
        let mut owner_id = None;
        if let Some(id) = value.get("owner_id") {
            owner_id = Some(UserId(id.as_u64().expect("unable to parse id") as usize));
        }
        let mut application_id = None;
        if let Some(id) = value.get("application_id") {
            application_id = Some(ApplicationId(
                id.as_u64().expect("unable to parse id") as usize
            ));
        }
        let mut last_pin_timestamp = None;
        if let Some(timestamp) = value.get("last_pin_timestamp") {
            last_pin_timestamp = Some(DateTime::parse_from_rfc3339(
                timestamp.as_str().expect("unable to parse timestamp"),
            )?);
        }

        Ok(PrivateChannel {
            id,
            channel_type,
            recipient: recipients.remove(0),
            last_message_id,
            owner_id,
            application_id,
            last_pin_timestamp,
        })
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct PublicChannel {
    pub id: ChannelId,
    pub name: String,
    #[serde(rename = "guild_id")]
    pub server_id: ServerId,
    #[serde(rename = "type")]
    pub channel_type: ChannelType,
    pub permision_overwrites: Vec<PermissionOverwrite>,
    pub topic: Option<String>,
    pub position: isize,
    pub last_message_id: Option<MessageId>,
    pub bitrate: Option<usize>,
    pub user_limit: Option<usize>,
    pub last_pin_timestamp: Option<DateTime<FixedOffset>>,
    pub nsfw: bool,
    pub parent_id: Option<ChannelId>,
}

impl PublicChannel {
    #[inline(always)]
    pub fn mention(&self) -> Mention {
        self.id.mention()
    }
}

#[derive(Clone, Debug)]
pub enum PermissionOverwriteType {
    Member(UserId),
    Role(RoleId),
}

#[derive(Clone, Debug)]
pub struct PermissionOverwrite {
    pub permission_overwrite_type: PermissionOverwriteType,
    // pub allow: Permissions,
    // pub deny: Permissions,
}

impl<'de> Deserialize<'de> for PermissionOverwrite {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(PermissionOverwriteVisitor {})
    }
}

struct PermissionOverwriteVisitor {}

impl<'de> Visitor<'de> for PermissionOverwriteVisitor {
    type Value = PermissionOverwrite;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("PermissionOverwrite")
    }

    fn visit_map<A: MapAccess<'de>>(
        self,
        mut map: A,
    ) -> std::result::Result<Self::Value, A::Error> {
        let mut permission_overwrite_type = None;
        let mut id = None;
        while let Some(key) = map.next_key::<&str>()? {
            match key {
                "type" => match map.next_value::<&str>() {
                    Ok("member") => {
                        permission_overwrite_type =
                            Some(PermissionOverwriteType::Member(UserId(0)));
                    }
                    Ok("role") => {
                        permission_overwrite_type = Some(PermissionOverwriteType::Role(RoleId(0)));
                    }
                    Ok(other) => {
                        panic!("unknown return: {:?}", other)
                    }
                    Err(e) => {
                        panic!("{:?}", e)
                    }
                },
                "id" => {
                    id = Some(map.next_value::<usize>()?);
                }
                _ => {}
            }
        }

        let id = id.expect("id not found");
        let mut permission_overwrite_type =
            permission_overwrite_type.expect("permission overwrite type not found");

        match permission_overwrite_type {
            PermissionOverwriteType::Member(_) => {
                permission_overwrite_type = PermissionOverwriteType::Member(UserId(id));
            }
            PermissionOverwriteType::Role(_) => {
                permission_overwrite_type = PermissionOverwriteType::Role(RoleId(id));
            }
        }

        Ok(Self::Value {
            permission_overwrite_type,
        })
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct CurrentUser {
    pub id: UserId,
    pub username: String,
    #[serde(deserialize_with = "serde_aux::prelude::deserialize_number_from_string")]
    pub discriminator: u16,
    pub avatar: Option<String>,
    pub email: Option<String>,
    pub verified: bool,
    pub mfa_enabled: bool,
    #[serde(default)]
    pub bot: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Relationship {
    pub id: UserId,
    #[serde(rename = "type")]
    pub relationship_type: RelationshipType,
    pub user: User,
}

serde_aux::enum_number_declare!(pub RelationshipType {
    Ignored = 0,
    Friends = 1,
    Blocked = 2,
    IncomingRequest = 3,
    OutgoingRequest = 4,
});

#[derive(Debug, Clone, Deserialize)]
pub struct ReadyEvent {
    // pub version: usize, // missing
    pub user: CurrentUser,
    pub session_id: String,
    // pub user_settings: Option<UserSettings>,
    // pub read_state: Option<Vec<ReadState>>,
    // pub private_channels: Vec<Channel>,
    // pub presences: Vec<Presence>,
    pub relationships: Vec<Relationship>,
    // pub servers: Vec<PossibleServer<LiveServer>>,
    // pub user_server_settings: Option<Vec<UserServerSettings>>,
    // pub tutorial: Option<Tutorial>,
    // pub trace: Vec<Option<String>>, // missing
    pub notes: BTreeMap<UserId, Option<String>>,
    pub shard: Option<[u8; 2]>,
}

#[derive(Debug)]
pub enum Event {
    Ready(ReadyEvent),
    Unknown(String, ureq::serde_json::Value),
}

impl Event {
    pub fn decode(kind: &str, value: ureq::serde_json::Value) -> Result<Self> {
        match kind {
            "READY" => Ok(Event::Ready(ureq::serde_json::from_value::<ReadyEvent>(
                value,
            )?)),
            _ => {
                println!("unknown event: {:?}", kind);
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
