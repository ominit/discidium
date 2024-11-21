#![allow(dead_code)] // TODO :(

use std::{
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

#[derive(Debug, Clone)]
pub struct WrappedMap(Map<String, Value>);

impl WrappedMap {
    fn get(&mut self, key: &str) -> Option<WrappedValue> {
        let value = self.0.remove(key)?;
        if value.is_null() {
            return None;
        }
        Some(WrappedValue(value))
    }

    /// type_name is used in the panic message
    fn check_empty_panic(self, type_name: &str) {
        if !self.0.is_empty() {
            panic!("value not taken out of {}: {:?}", type_name, self.0);
        }
    }
}

#[derive(Debug, Clone)]
pub struct WrappedValue(Value);

impl WrappedValue {
    fn to_string(self) -> Result<String> {
        if !self.0.is_string() {
            return Err(Error::msg(format!(
                "{:?} is not a string",
                self.0.to_string()
            )));
        }
        Ok(self.0.as_str().unwrap().to_string())
    }

    fn to_bool(self) -> Result<bool> {
        if !self.0.is_boolean() {
            return Err(Error::msg(format!(
                "{:?} is not a bool",
                self.0.to_string()
            )));
        }
        Ok(self.0.as_bool().unwrap())
    }

    fn to_u64(self) -> Result<u64> {
        if !self.0.is_u64() {
            return Err(Error::msg(format!("{:?} is not a u64", self.0.to_string())));
        }
        Ok(self.0.as_u64().unwrap())
    }

    fn to_decoder<T, F: FnOnce(WrappedMap) -> Result<T>>(self, decode: F) -> Result<T> {
        if !self.0.is_object() {
            return Err(Error::msg(format!(
                "{:?} is not an object",
                self.0.to_string()
            )));
        }
        decode(self.to_map()?)
    }

    fn to_value_decoder<T, F: FnOnce(WrappedValue) -> Result<T>>(self, decode: F) -> Result<T> {
        decode(self)
    }

    fn to_map(self) -> Result<WrappedMap> {
        if !self.0.is_object() {
            return Err(Error::msg(format!(
                "{:?} is not an object",
                self.0.to_string()
            )));
        }
        Ok(WrappedMap(self.0.as_object().unwrap().clone()))
    }

    fn to_array(self) -> Result<Vec<WrappedValue>> {
        if !self.0.is_array() {
            return Err(Error::msg(format!(
                "{:?} is not an array",
                self.0.to_string()
            )));
        }
        Ok(self
            .0
            .as_array()
            .unwrap()
            .iter()
            .map(|x| WrappedValue(x.clone()))
            .collect())
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
pub struct UserId(
    #[serde(deserialize_with = "serde_aux::prelude::deserialize_number_from_string")] pub usize,
);

impl UserId {
    fn decode(value: WrappedValue) -> Result<Self> {
        Ok(Self(value.to_string()?.parse::<usize>()?))
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
    fn decode(value: WrappedValue) -> Result<Self> {
        Ok(Self(value.to_string()?.parse::<usize>()?))
    }

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

impl ApplicationId {
    fn decode(value: WrappedValue) -> Result<Self> {
        Ok(Self(value.to_string()?.parse::<usize>()?))
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Deserialize)]
pub struct ServerId(pub usize);

impl ServerId {
    fn decode(value: WrappedValue) -> Result<Self> {
        Ok(Self(value.to_string()?.parse::<usize>()?))
    }

    pub fn everyone(self) -> RoleId {
        RoleId(self.0)
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Deserialize)]
pub struct MessageId(pub usize);

impl MessageId {
    fn decode(value: WrappedValue) -> Result<Self> {
        Ok(Self(value.to_string()?.parse::<usize>()?))
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug, Deserialize)]
pub struct RoleId(pub usize);

impl RoleId {
    fn decode(value: WrappedValue) -> Result<Self> {
        Ok(Self(value.to_string()?.parse::<usize>()?))
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
    fn decode(value: WrappedValue) -> Result<Self> {
        Ok(Self(value.to_string()?.parse::<usize>()?))
    }
}

#[derive(Clone, Debug)]
pub struct Emoji(pub String);

impl Emoji {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        Ok(Self(map.get("name").unwrap().to_string()?))
    }
}

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

impl ChannelType {
    fn decode(value: WrappedValue) -> Result<Self> {
        Ok(match value.to_u64()? {
            0 => ChannelType::Text,
            1 => ChannelType::Private,
            2 => ChannelType::Voice,
            3 => ChannelType::Group,
            4 => ChannelType::Category,
            5 => ChannelType::News,
            6 => ChannelType::Store,
            10 => ChannelType::NewsThread,
            11 => ChannelType::PublicThread,
            12 => ChannelType::PrivateThread,
            13 => ChannelType::StageVoice,
            14 => ChannelType::Directory,
            15 => ChannelType::Forum,
            other => panic!("unknown channel type {:?}", other),
        })
    }
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
    pub avatar_decoration_data: Option<String>,
    pub clan: Option<String>,
    pub discriminator: u16,
    pub global_name: Option<String>,
    pub id: UserId,
    pub primary_guild: Option<String>,
    pub public_flags: u64,
    pub username: String,
}

impl User {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let avatar = map.get("avatar").unwrap().to_string()?;
        let avatar_decoration_data = map
            .get("avatar_decoration_data")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let clan = map
            .get("clan")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let discriminator = map
            .get("discriminator")
            .unwrap()
            .to_string()?
            .parse::<u16>()?;
        let global_name = map
            .get("global_name")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let id = map.get("id").unwrap().to_value_decoder(UserId::decode)?;
        let primary_guild = map
            .get("primary_guild")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let public_flags = map.get("public_flags").unwrap().to_u64()?;
        let username = map.get("username").unwrap().to_string()?;
        map.check_empty_panic("User");
        Ok(Self {
            avatar,
            avatar_decoration_data,
            clan,
            discriminator,
            global_name,
            id,
            primary_guild,
            public_flags,
            username,
        })
    }

    #[inline(always)]
    pub fn mention(&self) -> Mention {
        self.id.mention()
    }

    pub fn avatar_url(&self) -> Option<String> {
        Some(format!(
            "{}/avatars/{}/{}.jpg",
            CDN_URL, self.id.0, self.avatar
        ))
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
    pub fn decode(mut map: WrappedMap) -> Result<Self> {
        Ok(
            match map
                .get("type")
                .unwrap()
                .to_value_decoder(ChannelType::decode)?
            {
                ChannelType::Private => Channel::Private(PrivateChannel::decode(map)?),
                ChannelType::Group => Channel::Group(Group::decode(map)?),
                other => todo!("{:?}", other),
            },
        )
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Group {
    pub blocked_user_warning_dismissed: bool,
    pub flags: u64,
    pub icon: Option<String>,
    pub id: ChannelId,
    pub last_message_id: MessageId,
    pub name: Option<String>,
    pub owner_id: UserId,
    pub recipient_flags: u64,
    pub recipients: Vec<User>,
}

impl Group {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let blocked_user_warning_dismissed = map
            .get("blocked_user_warning_dismissed")
            .unwrap()
            .to_bool()?;
        let flags = map.get("flags").unwrap().to_u64()?;
        let icon = map
            .get("icon")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let id = map.get("id").unwrap().to_value_decoder(ChannelId::decode)?;
        let last_message_id = map
            .get("last_message_id")
            .unwrap()
            .to_value_decoder(MessageId::decode)?;
        let name = map
            .get("name")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let owner_id = map
            .get("owner_id")
            .unwrap()
            .to_value_decoder(UserId::decode)?;
        let recipient_flags = map.get("recipient_flags").unwrap().to_u64()?;
        let recipients = map
            .get("recipients")
            .unwrap()
            .to_array()?
            .into_iter()
            .map(|x| x.to_decoder(User::decode))
            .collect::<Result<Vec<_>>>()?;
        map.check_empty_panic("Group");
        Ok(Self {
            blocked_user_warning_dismissed,
            flags,
            icon,
            id,
            last_message_id,
            name,
            owner_id,
            recipient_flags,
            recipients,
        })
    }

    pub fn name(&self) -> std::borrow::Cow<str> {
        match self.name {
            Some(ref name) => std::borrow::Cow::Borrowed(name),
            None => {
                if self.recipients.is_empty() {
                    return std::borrow::Cow::Borrowed("Empty Group");
                }
                std::borrow::Cow::Owned(
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
            .map(|x| format!("{}/channel-icons/{}/{}.jpg", CDN_URL, self.id.0, x))
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
    pub flags: u64,
    pub id: ChannelId,
    pub is_message_request: bool,
    pub is_message_request_timestamp: Option<String>,
    pub is_spam: bool,
    pub last_message_id: MessageId,
    pub recipient_flags: u64,
    pub recipient: User,
    pub safety_warnings: Vec<String>,
}

impl PrivateChannel {
    pub fn decode(mut map: WrappedMap) -> Result<Self> {
        let flags = map.get("flags").unwrap().to_u64()?;
        let id = map.get("id").unwrap().to_value_decoder(ChannelId::decode)?;
        let is_message_request = map.get("is_message_request").unwrap().to_bool()?;
        let is_message_request_timestamp = map
            .get("is_message_request_timestamp")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let is_spam = map.get("is_spam").unwrap().to_bool()?;
        let last_message_id = map
            .get("last_message_id")
            .unwrap()
            .to_value_decoder(MessageId::decode)?;
        let recipient_flags = map.get("recipient_flags").unwrap().to_u64()?;
        let recipient = map
            .get("recipients")
            .unwrap()
            .to_array()?
            .first()
            .unwrap()
            .clone() // TODO
            .to_decoder(User::decode)?;
        let safety_warnings = map
            .get("safety_warnings")
            .unwrap()
            .to_array()?
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Result<Vec<_>>>()?;
        map.check_empty_panic("PrivateChannel");
        Ok(Self {
            flags,
            id,
            is_message_request,
            is_message_request_timestamp,
            is_spam,
            last_message_id,
            recipient_flags,
            recipient,
            safety_warnings,
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
    pub accent_color: Option<String>,
    pub avatar: String,
    pub avatar_decoration_data: Option<String>,
    pub banner: Option<String>,
    pub banner_color: Option<String>,
    pub bio: String,
    pub clan: Option<String>,
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
    pub primary_guild: Option<String>,
    pub pronouns: String,
    pub purchased_flags: u64,
    pub username: String,
    pub verified: bool,
}

impl CurrentUser {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let accent_color = map
            .get("accent_color")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let avatar = map.get("avatar").unwrap().to_string()?;
        let avatar_decoration_data = map
            .get("avatar_decoration_data")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let banner = map
            .get("banner")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let banner_color = map
            .get("banner_color")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let bio = map.get("bio").unwrap().to_string()?;
        let clan = map
            .get("clan")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let desktop = map.get("desktop").unwrap().to_bool()?;
        let discriminator = map
            .get("discriminator")
            .unwrap()
            .to_string()?
            .parse::<u16>()?;
        let email = map.get("email").unwrap().to_string()?;
        let flags = map.get("flags").unwrap().to_u64()?;
        let global_name = map.get("global_name").unwrap().to_string()?;
        let id = map.get("id").unwrap().to_value_decoder(UserId::decode)?;
        let mfa_enabled = map.get("mfa_enabled").unwrap().to_bool()?;
        let mobile = map.get("mobile").unwrap().to_bool()?;
        let nsfw_allowed = map.get("nsfw_allowed").unwrap().to_bool()?;
        let phone = map.get("phone").unwrap().to_string()?;
        let premium = map.get("premium").unwrap().to_bool()?;
        let premium_type = map.get("premium_type").unwrap().to_u64()?;
        let primary_guild = map
            .get("primary_guild")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let pronouns = map.get("pronouns").unwrap().to_string()?;
        let purchased_flags = map.get("purchased_flags").unwrap().to_u64()?;
        let username = map.get("username").unwrap().to_string()?;
        let verified = map.get("verified").unwrap().to_bool()?;
        map.check_empty_panic("CurrentUser");
        Ok(Self {
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
            primary_guild,
            pronouns,
            purchased_flags,
            username,
            verified,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Relationship {
    // pub id: UserId,
    // pub is_spam_request: bool,
    // pub nickname: Option<String>,
    // pub since: String,
    // pub type_relationship: RelationshipType,
    // pub user: User,
    // pub user_ignored: bool,
}

impl Relationship {
    fn decode(mut map: WrappedMap) -> Self {
        // let id = UserId::decode(remove_value(value, "id")).unwrap();
        // let is_spam_request = decode_bool(remove_value(value, "is_spam_request")).unwrap();
        // let nickname = decode_string(remove_value(value, "nickname"));
        // let since = decode_string(remove_value(value, "since")).unwrap();
        // let type_relationship = match decode_u64(remove_value(value, "type")).unwrap() {
        //     0 => RelationshipType::Ignored,
        //     1 => RelationshipType::Friends,
        //     2 => RelationshipType::Blocked,
        //     3 => RelationshipType::IncomingRequest,
        //     4 => RelationshipType::OutgoingRequest,
        //     other => panic!("unknown type, {:?}", other),
        // };
        // let user = User::decode(remove_value(value, "user"));
        // let user_ignored = decode_bool(remove_value(value, "user_ignored")).unwrap();
        map.check_empty_panic("Relationship");
        Self {
            // id,
            // is_spam_request,
            // nickname,
            // since,
            // type_relationship,
            // user,
            // user_ignored,
        }
    }
}

#[derive(Debug, Clone)]
pub enum RelationshipType {
    Ignored,
    Friends,
    Blocked,
    IncomingRequest,
    OutgoingRequest,
}

#[derive(Debug, Clone)]
pub struct Presence {
    pub activities: Vec<PresenceActivity>,
    pub client_status: PresenceClientStatus,
    pub last_modified: u64,
    pub status: Status,
    pub user: User,
}

impl Presence {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let activities = map
            .get("activities")
            .unwrap()
            .to_array()?
            .into_iter()
            .map(|x| x.to_decoder(PresenceActivity::decode))
            .collect::<Result<Vec<_>>>()?;
        let client_status = map
            .get("client_status")
            .unwrap()
            .to_decoder(PresenceClientStatus::decode)?;
        let last_modified = map.get("last_modified").unwrap().to_u64()?;
        let status = map
            .get("status")
            .unwrap()
            .to_value_decoder(Status::decode)?;
        let user = map.get("user").unwrap().to_decoder(User::decode)?;
        map.check_empty_panic("Presence");
        Ok(Self {
            activities,
            client_status,
            last_modified,
            status,
            user,
        })
    }
}

#[derive(Debug, Clone)]
pub enum Status {
    DND,
}

impl Status {
    fn decode(value: WrappedValue) -> Result<Self> {
        Ok(match value.to_string()?.as_str() {
            "dnd" => Status::DND,
            other => panic!("unknown status: {:?}", other),
        })
    }
}

#[derive(Debug, Clone)]
pub struct PresenceClientStatus {
    pub desktop: Option<Status>,
    pub mobile: Option<Status>,
    pub web: Option<Status>,
}

impl PresenceClientStatus {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let desktop = map
            .get("desktop")
            .and_then(|x| Some(x.to_value_decoder(Status::decode)))
            .transpose()?;
        let mobile = map
            .get("mobile")
            .and_then(|x| Some(x.to_value_decoder(Status::decode)))
            .transpose()?;
        let web = map
            .get("web")
            .and_then(|x| Some(x.to_value_decoder(Status::decode)))
            .transpose()?;
        map.check_empty_panic("PresenceClientStatus");
        Ok(Self {
            desktop,
            mobile,
            web,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PresenceActivity {
    // pub assets: Option<PresenceActivityAsset>,
    pub created_at: u64,
    // pub details: Option<String>,
    pub emoji: Option<Emoji>,
    // pub flags: Option<u64>,
    pub id: String,
    pub name: String,
    // pub party: Option<PresenceActivityParty>,
    // pub session_id: Option<String>,
    pub state: String,
    // pub sync_id: Option<String>,
    // pub timestamp_end: Option<String>,
    // pub timestamp_start: Option<String>,
    pub type_activity: u64,
}

impl PresenceActivity {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        // let assets = PresenceActivityAsset::decode(remove_value(value, "assets"));
        let created_at = map.get("created_at").unwrap().to_u64()?;
        // let details = decode_string(remove_value(value, "details"));
        let emoji = map
            .get("emoji")
            .and_then(|x| Some(x.to_decoder(Emoji::decode)))
            .transpose()?;
        // let flags = decode_u64(remove_value(value, "flags"));
        let id = map.get("id").unwrap().to_string()?;
        let name = map.get("name").unwrap().to_string()?;
        // let party = PresenceActivityParty::decode(remove_value(value, "party"));
        // let session_id = decode_string(remove_value(value, "session_id"));
        let state = map.get("state").unwrap().to_string()?;
        // let sync_id = decode_string(remove_value(value, "sync_id"));
        let type_activity = map.get("type").unwrap().to_u64()?;
        // let timestamp = remove_value(value, "timestamps").clone();
        // let mut timestamp_end = None;
        // let mut timestamp_start = None;
        // if timestamp.is_some() {
        //     timestamp_end = decode_string(remove_value(
        //         &mut timestamp.clone().unwrap().as_object_mut().unwrap(),
        //         "end",
        //     ));
        //     timestamp_start = decode_string(remove_value(
        //         &mut timestamp.unwrap().as_object_mut().unwrap(),
        //         "start",
        //     ));
        // }
        map.check_empty_panic("PresenceActivity");
        Ok(Self {
            // assets,
            created_at,
            // details,
            emoji,
            // flags,
            id,
            name,
            // party,
            // session_id,
            state,
            // sync_id,
            // timestamp_end,
            // timestamp_start,
            type_activity,
        })
    }
}

#[derive(Debug, Clone)]
struct PresenceActivityParty {
    // pub id: String,
}

impl PresenceActivityParty {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        // let id = decode_string(remove_value(value, "id")).unwrap();
        map.check_empty_panic("PresenceActivityParty");
        Ok(Self {
            // id
        })
    }
}

#[derive(Debug, Clone)]
struct PresenceActivityAsset {
    // pub large_image: String,
    // pub large_text: String,
}

impl PresenceActivityAsset {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        // let large_image = decode_string(remove_value(value, "large_image")).unwrap();
        // let large_text = decode_string(remove_value(value, "large_text")).unwrap();
        map.check_empty_panic("PresenceActivityAsset");
        Ok(Self {
            // large_image,
            // large_text,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ReadyEvent {
    pub presences: Vec<Presence>,
    pub private_channels: Vec<Channel>,
    // pub relationships: Vec<Relationship>,
    pub session_id: String,
    pub user: CurrentUser,
    pub v: u64,
}

impl ReadyEvent {
    pub fn decode(mut map: WrappedMap) -> Result<Self> {
        map.get("_trace");
        map.get("analytics_token");
        map.get("api_code_version");
        map.get("auth");
        map.get("auth_session_id_hash");
        map.get("broadcaster_user_ids");
        map.get("connected_accounts");
        map.get("consents");
        map.get("country_code");
        map.get("experiments");
        map.get("explicit_content_scan_version");
        map.get("friend_suggestion_count");
        map.get("game_relationships");
        map.get("geo_ordered_rtc_regions");
        map.get("guild_experiments");
        map.get("guild_join_requests");
        map.get("guilds");
        map.get("notes");
        map.get("notification_settings");
        let presences = map
            .get("presences")
            .unwrap()
            .to_array()?
            .into_iter()
            .map(|x| x.to_decoder(Presence::decode))
            .collect::<Result<Vec<_>>>()?;
        let private_channels = map
            .get("private_channels")
            .unwrap()
            .to_array()?
            .into_iter()
            .map(|x| x.to_decoder(Channel::decode))
            .collect::<Result<Vec<_>>>()?;
        map.get("read_state");
        // let relationships =
        // decode_array(remove_value(value, "relationships"), Relationship::decode).unwrap();
        map.get("relationships");
        map.get("resume_gateway_url");
        let session_id = map.get("session_id").unwrap().to_string()?;
        map.get("session_type");
        map.get("sessions");
        map.get("static_client_session_id");
        map.get("tutorial");
        let user = map.get("user").unwrap().to_decoder(CurrentUser::decode)?;
        map.get("user_guild_settings");
        map.get("user_settings");
        map.get("user_settings_proto");
        let v = map.get("v").unwrap().to_u64()?;
        map.check_empty_panic("ReadyEvent");
        Ok(Self {
            presences,
            private_channels,
            // relationships,
            session_id,
            user,
            v,
        })
    }
}

#[derive(Debug)]
pub enum Event {
    Ready(ReadyEvent),
    Unknown(String, WrappedValue),
}

impl Event {
    pub fn decode(kind: &str, value: WrappedValue) -> Result<Self> {
        match kind {
            "READY" => Ok(Event::Ready(value.to_decoder(ReadyEvent::decode)?)),
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
    pub fn decode(mut map: WrappedMap) -> Result<Self> {
        Ok(
            match map.get("op").expect("op is null in GatewayEvent").to_u64() {
                Ok(0) => GatewayEvent::Dispatch(
                    map.get("s")
                        .expect("s not found in websocket message")
                        .to_u64()
                        .expect("unable to convert websocket message to u64")
                        as usize,
                    Event::decode(
                        &map.get("t")
                            .expect("t not found in websocket message")
                            .to_string()
                            .expect("could not convert to a string"),
                        map.get("d").expect("d not found in websocket message"),
                    )?,
                ),
                Ok(1) => GatewayEvent::Heartbeat(
                    map.get("s")
                        .expect("s not found in websocket message")
                        .to_u64()
                        .expect("unable to convert websocket message to u64")
                        as usize,
                ),
                Ok(7) => GatewayEvent::Reconnect,
                Ok(9) => GatewayEvent::InvalidateSession,
                Ok(10) => GatewayEvent::Hello(
                    map.get("d")
                        .expect("d not found in websocket message")
                        .to_map()
                        .expect("unable to convert d to a map")
                        .get("heartbeat_interval")
                        .expect("heartbeat_interval not found in websocket message")
                        .to_u64()
                        .expect("unable to convert websocket message to u64")
                        as usize,
                ),
                Ok(11) => Self::HeartbeatAck,
                _ => return Err(Error::msg("unexpected opcode")),
            },
        )
    }
}

pub fn receive_json<F, T>(
    websocket: &mut Arc<Mutex<WebSocket<MaybeTlsStream<TcpStream>>>>,
    decode: F,
) -> Result<T>
where
    F: FnOnce(WrappedMap) -> Result<T>,
{
    let message = websocket.lock().unwrap().read()?;
    match message {
        tungstenite::Message::Text(text) => ureq::serde_json::from_str(&text)
            .map_err(From::from)
            .and_then(|x| WrappedValue(x).to_decoder(decode))
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
                .and_then(|x| WrappedValue(x).to_decoder(decode))
                .map_err(|e| e)
        }
        _ => {
            todo!("websocket message not text or binary")
        }
    }
}
