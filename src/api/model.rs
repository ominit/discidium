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
use ureq::serde_json::{Map, Value};

use super::CDN_URL;

fn remove_value(map: &mut Map<String, Value>, key: &str) -> Option<Value> {
    map.remove(key)
}

fn decode_string(value: Option<Value>) -> Option<String> {
    if value.as_ref()?.is_null() {
        return None;
    }
    Some(value?.as_str()?.to_owned())
}

fn decode_bool(value: Option<Value>) -> Option<bool> {
    if value.as_ref()?.is_null() {
        return None;
    }
    Some(value?.as_bool()?)
}

fn decode_u64(value: Option<Value>) -> Option<u64> {
    if value.as_ref()?.is_null() {
        return None;
    }
    Some(value?.as_u64()?)
}

fn decode_array<T, F: Fn(Value) -> T>(value: Option<Value>, f: F) -> Option<Vec<T>> {
    Some(
        value?
            .as_array()?
            .iter()
            .map(|x| f(x.clone()))
            .collect::<Vec<_>>(),
    )
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
pub struct UserId(
    #[serde(deserialize_with = "serde_aux::prelude::deserialize_number_from_string")] pub usize,
);

impl UserId {
    fn decode(value: Option<Value>) -> Result<Self> {
        Ok(Self(decode_string(value).unwrap().parse()?))
    }

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

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Deserialize)]
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

#[derive(Clone, Debug)]
pub struct Emoji(pub String);

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

#[derive(Deserialize, Debug, Clone)]
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
    pub avatar: String,
    pub avatar_decoration_data: Option<String>, // TODO
    pub clan: Option<String>,
    pub discriminator: u16,
    pub global_name: Option<String>,
    pub id: UserId,
    pub public_flags: u64,
    pub username: String,
}

impl User {
    fn decode(mut value: Option<Value>) -> Self {
        let value = value.as_mut().unwrap().as_object_mut().unwrap();
        let avatar = decode_string(remove_value(value, "avatar")).unwrap();
        let avatar_decoration_data = decode_string(remove_value(value, "avatar_decoration_data"));
        let clan = decode_string(remove_value(value, "clan"));
        let discriminator = remove_value(value, "discriminator")
            .unwrap()
            .as_str()
            .unwrap()
            .parse::<u16>()
            .unwrap();
        let global_name = decode_string(remove_value(value, "global_name"));
        let id = UserId::decode(remove_value(value, "id")).unwrap();
        let public_flags = decode_u64(remove_value(value, "public_flags")).unwrap();
        let username = decode_string(remove_value(value, "username")).unwrap();
        if !value.is_empty() {
            panic!("value not taken out of User: {:?}", value);
        }
        Self {
            avatar,
            avatar_decoration_data,
            clan,
            discriminator,
            global_name,
            id,
            public_flags,
            username,
        }
    }

    // #[inline(always)]
    // pub fn mention(&self) -> Mention {
    //     self.id.mention()
    // }

    // pub fn avatar_url(&self) -> Option<String> {
    //     self.avatar
    //         .as_ref()
    //         .map(|x| format!("{}/avatars/{}/{}.jpg", CDN_URL, self.id.0, x))
    // }
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
    // pub fn display_name(&self) -> &str {
    //     if let Some(name) = self.nick.as_ref() {
    //         name
    //     } else {
    //         &self.user.username
    //     }
    // }
}

#[derive(Debug, Clone)]
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

#[derive(Deserialize, Debug, Clone)]
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
    // pub fn name(&self) -> Cow<str> {
    //     match self.name {
    //         Some(ref name) => Cow::Borrowed(name),
    //         None => {
    //             if self.recipients.is_empty() {
    //                 return Cow::Borrowed("Empty Group");
    //             }
    //             Cow::Owned(
    //                 self.recipients
    //                     .iter()
    //                     .map(|x| x.username.clone())
    //                     .collect::<Vec<String>>()
    //                     .join(", "),
    //             )
    //         }
    //     }
    // }

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

#[derive(Debug, Clone)]
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
    pub accent_color: Option<String>, // TODO idk what type
    pub avatar: Option<String>,
    pub avatar_decoration_data: Option<String>, // TODO
    pub banner: Option<String>,                 // TODO
    pub banner_color: Option<String>,           // TODO
    pub bio: String,
    pub clan: Option<String>, // TODO
    pub desktop: bool,
    pub discriminator: u16,
    pub email: String,
    pub flags: u64,
    pub global_name: String,
    pub id: UserId,
    pub mfa_enabled: bool,
    pub mobile: bool,
    pub nsfw_allowed: bool,
    pub phone: String,
    pub premium: bool,
    pub premium_type: u64,
    pub pronouns: String,
    pub purchased_flags: u64,
    pub username: String,
    pub verified: bool,
}

impl CurrentUser {
    fn decode(mut value: Option<Value>) -> Self {
        let value = value.as_mut().unwrap().as_object_mut().unwrap();
        let accent_color = decode_string(remove_value(value, "accent_color"));
        let avatar = decode_string(remove_value(value, "avatar"));
        let avatar_decoration_data = decode_string(remove_value(value, "avatar_decoration_data"));
        let banner = decode_string(remove_value(value, "banner"));
        let banner_color = decode_string(remove_value(value, "banner_color"));
        let bio = decode_string(remove_value(value, "bio")).unwrap();
        let clan = decode_string(remove_value(value, "clan"));
        let desktop = remove_value(value, "desktop").unwrap().as_bool().unwrap();
        let discriminator = remove_value(value, "discriminator")
            .unwrap()
            .as_str()
            .unwrap()
            .parse::<u16>()
            .unwrap();
        let email = decode_string(remove_value(value, "email")).unwrap();
        let flags = remove_value(value, "flags").unwrap().as_u64().unwrap();
        let global_name = decode_string(remove_value(value, "global_name")).unwrap();
        let id = UserId::decode(remove_value(value, "id")).unwrap();
        let mfa_enabled = remove_value(value, "mfa_enabled")
            .unwrap()
            .as_bool()
            .unwrap();
        let mobile = remove_value(value, "mobile").unwrap().as_bool().unwrap();
        let nsfw_allowed = remove_value(value, "nsfw_allowed")
            .unwrap()
            .as_bool()
            .unwrap();
        let phone = decode_string(remove_value(value, "phone")).unwrap();
        let premium = remove_value(value, "premium").unwrap().as_bool().unwrap();
        let premium_type = remove_value(value, "premium_type")
            .unwrap()
            .as_u64()
            .unwrap();
        let pronouns = decode_string(remove_value(value, "pronouns")).unwrap();
        let purchased_flags = remove_value(value, "purchased_flags")
            .unwrap()
            .as_u64()
            .unwrap();
        let username = decode_string(remove_value(value, "username")).unwrap();
        let verified = remove_value(value, "verified").unwrap().as_bool().unwrap();
        if !value.is_empty() {
            panic!("value not taken out of CurrentUser: {:?}", value);
        }
        Self {
            accent_color,
            avatar,
            avatar_decoration_data,
            banner,
            banner_color,
            bio,
            clan,
            desktop,
            discriminator,
            email,
            flags,
            global_name,
            id,
            mfa_enabled,
            mobile,
            nsfw_allowed,
            phone,
            premium,
            premium_type,
            pronouns,
            purchased_flags,
            username,
            verified,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Relationship {
    pub id: UserId,
    #[serde(rename = "type")]
    pub relationship_type: RelationshipType,
    pub user: User,
    pub is_spam_request: bool,
    pub nickname: Option<String>, // TODO
    pub since: String,
    pub user_ignored: bool,
}

serde_aux::enum_number_declare!(pub RelationshipType {
    Ignored = 0,
    Friends = 1,
    Blocked = 2,
    IncomingRequest = 3,
    OutgoingRequest = 4,
});

#[derive(Debug, Clone)]
pub struct Presence {
    pub activities: Vec<PresenceActivity>,
    pub client_status: PresenceClientStatus,
    pub last_modified: u64,
    pub status: Status,
}

impl Presence {
    fn decode(mut value: Value) -> Self {
        let value = value.as_object_mut().unwrap();
        let activities =
            decode_array(remove_value(value, "activities"), PresenceActivity::decode).unwrap();
        let client_status = PresenceClientStatus::decode(remove_value(value, "client_status"));
        let last_modified = decode_u64(remove_value(value, "last_modified")).unwrap();
        let status = Status::decode(remove_value(value, "status")).unwrap();
        let user = User::decode(remove_value(value, "user"));
        if !value.is_empty() {
            panic!("value not taken out of Presence: {:?}", value);
        }
        Self {
            activities,
            client_status,
            last_modified,
            status,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Status {
    DND,
}

impl Status {
    fn decode(value: Option<Value>) -> Option<Self> {
        Some(match decode_string(value)?.as_str() {
            "dnd" => Status::DND,
            other => panic!("unknown status: {:?}", other),
        })
    }
}

#[derive(Debug, Clone)]
pub struct PresenceClientStatus {
    pub desktop: Status,
    pub mobile: Option<Status>,
}

impl PresenceClientStatus {
    fn decode(mut value: Option<Value>) -> Self {
        let value = value.as_mut().unwrap().as_object_mut().unwrap();
        let desktop = Status::decode(remove_value(value, "desktop")).unwrap();
        let mobile = Status::decode(remove_value(value, "mobile"));
        if !value.is_empty() {
            panic!("value not taken out of PresenceClientStatus: {:?}", value);
        }
        Self { desktop, mobile }
    }
}

#[derive(Debug, Clone)]
pub struct PresenceActivity {
    pub assets: Option<PresenceActivityAsset>,
    pub created_at: u64,
    pub details: Option<String>,
    pub emoji: Option<Emoji>,
    pub flags: Option<u64>,
    pub id: String,
    pub name: String,
    pub party: Option<PresenceActivityParty>,
    pub session_id: Option<String>,
    pub state: String,
    pub sync_id: Option<String>,
    pub timestamp_end: Option<String>,
    pub timestamp_start: Option<String>,
    pub type_activity: u64,
}

impl PresenceActivity {
    fn decode(mut value: Value) -> Self {
        let value = value.as_object_mut().unwrap();
        let assets = PresenceActivityAsset::decode(remove_value(value, "assets"));
        let created_at = decode_u64(remove_value(value, "created_at")).unwrap();
        let details = decode_string(remove_value(value, "details"));
        let emoji = None;
        remove_value(value, "emoji"); // TODO
        let flags = decode_u64(remove_value(value, "flags"));
        let id = decode_string(remove_value(value, "id")).unwrap();
        let name = decode_string(remove_value(value, "name")).unwrap();
        let party = PresenceActivityParty::decode(remove_value(value, "party"));
        let session_id = decode_string(remove_value(value, "session_id"));
        let state = decode_string(remove_value(value, "state")).unwrap();
        let sync_id = decode_string(remove_value(value, "sync_id"));
        let type_activity = decode_u64(remove_value(value, "type")).unwrap();
        let timestamp = remove_value(value, "timestamps").clone();
        let timestamp_end = None;
        let timestamp_start = None;
        if timestamp.is_some() {
            let timestamp_end = decode_string(remove_value(
                &mut timestamp.clone().unwrap().as_object_mut().unwrap(),
                "end",
            ));
            let timestamp_start = decode_string(remove_value(
                &mut timestamp.unwrap().as_object_mut().unwrap(),
                "start",
            ));
        }
        if !value.is_empty() {
            panic!("value not taken out of PresenceActivity: {:?}", value);
        }
        Self {
            assets,
            created_at,
            details,
            emoji,
            flags,
            id,
            name,
            party,
            session_id,
            state,
            sync_id,
            timestamp_end,
            timestamp_start,
            type_activity,
        }
    }
}

#[derive(Debug, Clone)]
struct PresenceActivityParty {
    pub id: String,
}

impl PresenceActivityParty {
    fn decode(mut value: Option<Value>) -> Option<Self> {
        let value = value.as_mut()?.as_object_mut()?;
        let id = decode_string(remove_value(value, "id")).unwrap();
        if !value.is_empty() {
            panic!("value not taken out of PresenceActivityParty: {:?}", value);
        }
        Some(Self { id })
    }
}

#[derive(Debug, Clone)]
struct PresenceActivityAsset {
    pub large_image: String,
    pub large_text: String,
}

impl PresenceActivityAsset {
    fn decode(mut value: Option<Value>) -> Option<Self> {
        let value = value.as_mut()?.as_object_mut()?;
        let large_image = decode_string(remove_value(value, "large_image")).unwrap();
        let large_text = decode_string(remove_value(value, "large_text")).unwrap();
        if !value.is_empty() {
            panic!("value not taken out of PresenceActivityAsset: {:?}", value);
        }
        Some(Self {
            large_image,
            large_text,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ReadyEvent {
    pub presences: Vec<Presence>,
    pub session_id: String,
    pub user: CurrentUser,
    pub v: u64,
    // pub relationships: Vec<Relationship>,
    // pub notes: BTreeMap<UserId, Option<String>>,
}

impl ReadyEvent {
    pub fn decode(mut value: Value) -> Result<Self> {
        let value = value.as_object_mut().unwrap();
        // remove_value(value, "_trace");
        // remove_value(value, "analytics_token");
        // remove_value(value, "api_code_version");
        // remove_value(&mut value, "auth");
        // remove_value(value, "auth_session_id_hash");
        // remove_value(value, "broadcaster_user_ids");
        // remove_value(value, "connected_accounts");
        // remove_value(value, "consents");
        // remove_value(value, "country_code");
        // remove_value(value, "experiments");
        // remove_value(value, "explicit_content_scan_version");
        // remove_value(value, "friend_suggestion_count");
        // remove_value(value, "geo_ordered_rtc_regions");
        // remove_value(value, "guild_experiments");
        // remove_value(value, "guild_join_requests");
        // remove_value(value, "guilds");
        // remove_value(&mut value, "notes");
        // remove_value(value, "notification_settings");
        let presences = decode_array(remove_value(value, "presences"), Presence::decode).unwrap();
        // remove_value(value, "private_channels");
        // remove_value(value, "read_state");
        // remove_value(value, "relationships");
        // remove_value(value, "resume_gateway_url");
        let session_id = decode_string(remove_value(value, "session_id")).unwrap();
        // remove_value(value, "session_type");
        // remove_value(value, "sessions");
        // remove_value(value, "static_client_session_id");
        // remove_value(value, "tutorial");
        let user = CurrentUser::decode(remove_value(value, "user"));
        // remove_value(value, "user_guild_settings");
        // remove_value(value, "user_settings");
        // remove_value(value, "user_settings_proto");
        let v = decode_u64(remove_value(value, "v")).unwrap();

        if !value.is_empty() {
            panic!("value not taken out of ReadyEvent: {:?}", value);
        }
        Ok(Self {
            presences,
            session_id,
            user,
            v,
        })
    }
}

#[derive(Debug)]
pub enum Event {
    Ready(ReadyEvent),
    Unknown(String, Value),
}

impl Event {
    pub fn decode(kind: &str, value: Value) -> Result<Self> {
        match kind {
            "READY" => Ok(Event::Ready(ReadyEvent::decode(value)?)),
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
    pub fn decode(value: Value) -> Result<Self> {
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
