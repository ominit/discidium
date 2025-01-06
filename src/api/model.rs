#![allow(dead_code)] // TODO :(

use std::{
    io::Read,
    net::TcpStream,
    sync::{Arc, Mutex},
};

use anyhow::{Error, Result};
use chrono::{DateTime, FixedOffset};
use tungstenite::{WebSocket, stream::MaybeTlsStream};
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

    fn to_array_decoder<T, F: Clone + Fn(WrappedMap) -> Result<T>>(
        self,
        decode: F,
    ) -> Result<Vec<T>> {
        if !self.0.is_array() {
            return Err(Error::msg(format!(
                "{:?} is not an array",
                self.0.to_string()
            )));
        }
        self.0
            .as_array()
            .unwrap()
            .iter()
            .map(|x| WrappedValue(x.clone()).to_decoder(decode.clone()))
            .collect::<Result<Vec<_>>>()
    }

    fn to_array_value_decoder<T, F: Clone + Fn(WrappedValue) -> Result<T>>(
        self,
        decode: F,
    ) -> Result<Vec<T>> {
        if !self.0.is_array() {
            return Err(Error::msg(format!(
                "{:?} is not an array",
                self.0.to_string()
            )));
        }
        self.0
            .as_array()
            .unwrap()
            .iter()
            .map(|x| WrappedValue(x.clone()).to_value_decoder(decode.clone()))
            .collect::<Result<Vec<_>>>()
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

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct UserId(pub usize);

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

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
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

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct ApplicationId(pub usize);

impl ApplicationId {
    fn decode(value: WrappedValue) -> Result<Self> {
        Ok(Self(value.to_string()?.parse::<usize>()?))
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct ServerId(pub usize);

impl ServerId {
    fn decode(value: WrappedValue) -> Result<Self> {
        Ok(Self(value.to_string()?.parse::<usize>()?))
    }

    pub fn everyone(self) -> RoleId {
        RoleId(self.0)
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
pub struct MessageId(pub usize);

impl MessageId {
    fn decode(value: WrappedValue) -> Result<Self> {
        Ok(Self(value.to_string()?.parse::<usize>()?))
    }
}

#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Ord, Eq, Debug)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct Emoji(pub String);

impl Emoji {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        Ok(Self(map.get("name").unwrap().to_string()?))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChannelCategory {
    pub flags: u64,
    pub id: ChannelId,
    pub name: String,
    pub parent_id: Option<ChannelId>,
    pub permission_overwrites: Vec<PermissionOverwrite>,
    pub position: u64,
    pub version: u64,
}

impl ChannelCategory {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let flags = map.get("flags").unwrap().to_u64()?;
        let id = map.get("id").unwrap().to_value_decoder(ChannelId::decode)?;
        let name = map.get("name").unwrap().to_string()?;
        let parent_id = map
            .get("parent_id")
            .and_then(|x| Some(x.to_value_decoder(ChannelId::decode)))
            .transpose()?;
        let permission_overwrites = map
            .get("permission_overwrites")
            .unwrap()
            .to_array_decoder(PermissionOverwrite::decode)?;
        let position = map.get("position").unwrap().to_u64()?;
        let version = map.get("version").unwrap().to_u64()?;
        map.check_empty_panic("ChannelCategory");
        Ok(Self {
            flags,
            id,
            name,
            parent_id,
            permission_overwrites,
            position,
            version,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PermissionOverwrite {
    Member(PermissionOverwriteMember),
    Role(PermissionOverwriteRole),
}

impl PermissionOverwrite {
    pub fn decode(mut map: WrappedMap) -> Result<Self> {
        Ok(match map.get("type").unwrap().to_string()?.as_str() {
            "member" => PermissionOverwrite::Member(PermissionOverwriteMember::decode(map)?),
            "role" => PermissionOverwrite::Role(PermissionOverwriteRole::decode(map)?),
            other => todo!("{:?}", other),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PermissionOverwriteMember {
    allow: Vec<Permission>,
    allow_new: Vec<Permission>,
    deny: Vec<Permission>,
    deny_new: Vec<Permission>,
    id: UserId,
}

impl PermissionOverwriteMember {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let allow = map
            .get("allow")
            .unwrap()
            .to_value_decoder(Permission::decode)?;
        let allow_new = map
            .get("allow_new")
            .unwrap()
            .to_value_decoder(Permission::decode)?;
        let deny = map
            .get("deny")
            .unwrap()
            .to_value_decoder(Permission::decode)?;
        let deny_new = map
            .get("deny_new")
            .unwrap()
            .to_value_decoder(Permission::decode)?;
        let id = map.get("id").unwrap().to_value_decoder(UserId::decode)?;
        map.check_empty_panic("PermissionOverwriteMember");
        Ok(Self {
            allow,
            allow_new,
            deny,
            deny_new,
            id,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PermissionOverwriteRole {
    allow: Vec<Permission>,
    allow_new: Vec<Permission>,
    deny: Vec<Permission>,
    deny_new: Vec<Permission>,
    id: RoleId,
}

impl PermissionOverwriteRole {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let allow = map
            .get("allow")
            .unwrap()
            .to_value_decoder(Permission::decode)?;
        let allow_new = map
            .get("allow_new")
            .unwrap()
            .to_value_decoder(Permission::decode)?;
        let deny = map
            .get("deny")
            .unwrap()
            .to_value_decoder(Permission::decode)?;
        let deny_new = map
            .get("deny_new")
            .unwrap()
            .to_value_decoder(Permission::decode)?;
        let id = map.get("id").unwrap().to_value_decoder(RoleId::decode)?;
        map.check_empty_panic("PermissionOverwriteMember");
        Ok(Self {
            allow,
            allow_new,
            deny,
            deny_new,
            id,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Permission {
    ViewChannels,
    ManageChannels,
    ManageRoles,
    CreateExpressions,
    ManageExpressinos,
    ViewAuditLog,
    ManagePermissions,
    ManageWebhooks,
    ManageServer,
    CreateInvite,
    ChangeNickname,
    ManageNicknames,
    KickMembers,
    BanMembers,
    TimeoutMembers,
    SendMessages,
    SendMessagesThreads,
    CreatePublicThreads,
    CreatePrivateThreads,
    EmbedLinks,
    AttachFiles,
    AddReactions,
    UseExternalEmoji,
    UseExternalStickers,
    MentionEveryone,
    ManageMessages,
    ManageThreads,
    ReadMessageHistory,
    SendTTSMessages,
    SendVoiceMessages,
    CreatePolls,
    Connect,
    Speak,
    Video,
    UseSoundboard,
    UseExternalSounds,
    UseVoiceActivity,
    PrioritySpeaker,
    MuteMembers,
    DeafenMembers,
    MoveMembers,
    SetVoiceChannelStatus,
    UseApplicationCommands,
    UseActivities,
    UseExternalApps,
    CreateEvents,
    ManageEvents,
    Administrator,
}

impl Permission {
    fn decode(value: WrappedValue) -> Result<Vec<Self>> {
        let mut value = value
            .clone()
            .to_u64()
            .unwrap_or_else(|_| value.to_string().unwrap().parse().unwrap());
        let mut vec = vec![];
        if value >= 2_u64.pow(50) {
            vec.push(Self::UseExternalApps);
            value -= 2_u64.pow(50);
        }
        if value >= 2_u64.pow(49) {
            vec.push(Self::CreatePolls);
            value -= 2_u64.pow(49);
        }
        if value >= 2_u64.pow(48) {
            vec.push(Self::SetVoiceChannelStatus);
            value -= 2_u64.pow(48);
        }
        if value >= 2_u64.pow(46) {
            vec.push(Self::SendVoiceMessages);
            value -= 2_u64.pow(46);
        }
        if value >= 2_u64.pow(45) {
            vec.push(Self::UseExternalSounds);
            value -= 2_u64.pow(45);
        }
        if value >= 2_u64.pow(44) {
            vec.push(Self::CreateEvents);
            value -= 2_u64.pow(44);
        }
        if value >= 2_u64.pow(42) {
            vec.push(Self::UseSoundboard);
            value -= 2_u64.pow(42);
        }
        if value >= 2_u64.pow(39) {
            vec.push(Self::UseActivities);
            value -= 2_u64.pow(39);
        }
        if value >= 2_u64.pow(38) {
            vec.push(Self::SendMessagesThreads);
            value -= 2_u64.pow(38);
        }
        if value >= 2_u64.pow(37) {
            vec.push(Self::UseExternalStickers);
            value -= 2_u64.pow(37);
        }
        if value >= 2_u64.pow(36) {
            vec.push(Self::CreatePrivateThreads);
            value -= 2_u64.pow(36);
        }
        if value >= 2_u64.pow(35) {
            vec.push(Self::CreatePublicThreads);
            value -= 2_u64.pow(35);
        }
        if value >= 2_u64.pow(34) {
            vec.push(Self::ManageThreads);
            value -= 2_u64.pow(34);
        }
        if value >= 2_u64.pow(33) {
            vec.push(Self::ManageEvents);
            value -= 2_u64.pow(33);
        }
        if value >= 2_u64.pow(31) {
            vec.push(Self::UseApplicationCommands);
            value -= 2_u64.pow(31);
        }
        if value >= 2_u64.pow(29) {
            vec.push(Self::ManageWebhooks);
            value -= 2_u64.pow(29);
        }
        if value >= 2_u64.pow(28) {
            vec.push(Self::ManagePermissions);
            value -= 2_u64.pow(28);
        }
        if value >= 2_u64.pow(25) {
            vec.push(Self::UseVoiceActivity);
            value -= 2_u64.pow(25);
        }
        if value >= 2_u64.pow(24) {
            vec.push(Self::MoveMembers);
            value -= 2_u64.pow(24);
        }
        if value >= 2_u64.pow(23) {
            vec.push(Self::DeafenMembers);
            value -= 2_u64.pow(23);
        }
        if value >= 2_u64.pow(22) {
            vec.push(Self::MuteMembers);
            value -= 2_u64.pow(22);
        }
        if value >= 2_u64.pow(21) {
            vec.push(Self::Speak);
            value -= 2_u64.pow(21);
        }
        if value >= 2_u64.pow(20) {
            vec.push(Self::Connect);
            value -= 2_u64.pow(20);
        }
        if value >= 2_u64.pow(18) {
            vec.push(Self::UseExternalEmoji);
            value -= 2_u64.pow(18);
        }
        if value >= 2_u64.pow(17) {
            vec.push(Self::MentionEveryone);
            value -= 2_u64.pow(17);
        }
        if value >= 2_u64.pow(16) {
            vec.push(Self::ReadMessageHistory);
            value -= 2_u64.pow(16);
        }
        if value >= 2_u64.pow(15) {
            vec.push(Self::AttachFiles);
            value -= 2_u64.pow(15);
        }
        if value >= 2_u64.pow(14) {
            vec.push(Self::EmbedLinks);
            value -= 2_u64.pow(14);
        }
        if value >= 2_u64.pow(13) {
            vec.push(Self::ManageMessages);
            value -= 2_u64.pow(13);
        }
        if value >= 2_u64.pow(12) {
            vec.push(Self::SendTTSMessages);
            value -= 2_u64.pow(12);
        }
        if value >= 2_u64.pow(11) {
            vec.push(Self::SendMessages);
            value -= 2_u64.pow(11);
        }
        if value >= 2_u64.pow(10) {
            vec.push(Self::ViewChannels);
            value -= 2_u64.pow(10);
        }
        if value >= 2_u64.pow(9) {
            vec.push(Self::Video);
            value -= 2_u64.pow(9);
        }
        if value >= 2_u64.pow(8) {
            vec.push(Self::PrioritySpeaker);
            value -= 2_u64.pow(8);
        }
        if value >= 2_u64.pow(6) {
            vec.push(Self::AddReactions);
            value -= 2_u64.pow(6);
        }
        if value >= 2_u64.pow(4) {
            vec.push(Self::ManageChannels);
            value -= 2_u64.pow(4);
        }
        if value >= 1 {
            vec.push(Self::CreateInvite);
            value -= 1;
        }
        if value != 0 {
            panic!("error decoding permission, {:?}", value)
        }
        Ok(vec)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Server {
    afk_channel_id: Option<ChannelId>,
    afk_timeout: u64,
    channels: Vec<Channel>,
}

impl Server {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let afk_channel_id = map
            .get("afk_channel_id")
            .and_then(|x| Some(x.to_value_decoder(ChannelId::decode)))
            .transpose()?;
        let afk_timeout = map.get("afk_timeout").unwrap().to_u64()?;
        let channels = map
            .get("channels")
            .unwrap()
            .to_array_decoder(Channel::decode)?;
        map.check_empty_panic("Server");
        Ok(Self {
            afk_channel_id,
            afk_timeout,
            channels,
        })
    }

    // pub fn icon_url(&self) -> Option<String> {
    //     self.icon
    //         .as_ref()
    //         .map(|x| format!("{}/icons/{}/{}.jpg", CDN_URL, self.id.0, x))
    // }
}

pub struct Role {
    // pub id: RoleId,
    // pub name: String,
    // /// Color in 0xRRGGBB form
    // pub color: usize,
    // pub hoist: bool,
    // pub managed: bool,
    // pub position: isize,
    // pub mentionable: bool,
    // pub permissions: Permissions,
}

impl Role {
    // #[inline(always)]
    // pub fn mention(&self) -> Mention {
    //     self.id.mention()
    // }
}

pub struct Ban {
    // reason: Option<String>,
    // user: User,
}

#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub avatar: Option<String>,
    pub avatar_decoration_data: Option<AvatarDecorationData>,
    pub bot: Option<bool>,
    pub clan: Option<Clan>,
    pub discriminator: u16,
    pub global_name: Option<String>,
    pub id: UserId,
    pub primary_guild: Option<Clan>,
    pub public_flags: Option<u64>,
    pub system: Option<bool>,
    pub username: String,
}

impl User {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let avatar = map
            .get("avatar")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let avatar_decoration_data = map
            .get("avatar_decoration_data")
            .and_then(|x| Some(x.to_decoder(AvatarDecorationData::decode)))
            .transpose()?;
        let bot = map.get("bot").and_then(|x| Some(x.to_bool())).transpose()?;
        let clan = map
            .get("clan")
            .and_then(|x| Some(x.to_decoder(Clan::decode)))
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
            .and_then(|x| Some(x.to_decoder(Clan::decode)))
            .transpose()?;
        let public_flags = map
            .get("public_flags")
            .and_then(|x| Some(x.to_u64()))
            .transpose()?;
        let system = map
            .get("system")
            .and_then(|x| Some(x.to_bool()))
            .transpose()?;
        let username = map.get("username").unwrap().to_string()?;
        map.check_empty_panic("User");
        Ok(Self {
            avatar,
            avatar_decoration_data,
            bot,
            clan,
            discriminator,
            global_name,
            id,
            primary_guild,
            public_flags,
            system,
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
            CDN_URL,
            self.id.0,
            self.avatar.as_ref()?
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Clan {
    pub badge: Option<String>,
    pub identity_enabled: bool,
    pub identity_guild_id: Option<ServerId>,
    pub tag: Option<String>,
}

impl Clan {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let badge = map
            .get("badge")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let identity_enabled = map.get("identity_enabled").unwrap().to_bool()?;
        let identity_guild_id = map
            .get("identity_guild_id")
            .and_then(|x| Some(x.to_value_decoder(ServerId::decode)))
            .transpose()?;
        let tag = map
            .get("tag")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        map.check_empty_panic("Clan");
        Ok(Self {
            badge,
            identity_enabled,
            identity_guild_id,
            tag,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AvatarDecorationData {
    pub asset: String,
    pub expires_at: Option<u64>,
    pub sku_id: String,
}

impl AvatarDecorationData {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let asset = map.get("asset").unwrap().to_string()?;
        let expires_at = map
            .get("expires_at")
            .and_then(|x| Some(x.to_u64()))
            .transpose()?;
        let sku_id = map.get("sku_id").unwrap().to_string()?;
        map.check_empty_panic("AvatarDecorationData");
        Ok(Self {
            asset,
            expires_at,
            sku_id,
        })
    }
}

pub struct Member {
    // pub user: User,
    // pub roles: Vec<RoleId>,
    // pub nick: Option<String>,
    // pub joined_at: String,
    // pub mute: bool,
    // pub deaf: bool,
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

#[derive(Debug, Clone, PartialEq)]
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
                ChannelType::Public => Channel::Public(PublicChannel::decode(map)?),
                ChannelType::Private => Channel::Private(PrivateChannel::decode(map)?),
                ChannelType::Group => Channel::Group(Group::decode(map)?),
                ChannelType::Category => Channel::Category(ChannelCategory::decode(map)?),
                other => todo!("{:?}", other),
            },
        )
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
pub enum ChannelType {
    Group,
    Private,
    Public,
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
            0 => ChannelType::Public,
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

#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub blocked_user_warning_dismissed: bool,
    pub flags: u64,
    pub icon: Option<String>,
    pub id: ChannelId,
    pub last_message_id: MessageId,
    pub last_pin_timestamp: Option<String>,
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
        let last_pin_timestamp = map
            .get("last_pin_timestamp")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
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
            .to_array_decoder(User::decode)?;
        map.check_empty_panic("Group");
        Ok(Self {
            blocked_user_warning_dismissed,
            flags,
            icon,
            id,
            last_message_id,
            last_pin_timestamp,
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

pub struct Call {
    // pub channel_id: ChannelId,
    // pub message_id: MessageId,
    // pub region: String,
    // pub ringring: Vec<UserId>,
    // pub unavailable: bool,
    // pub voice_states: Vec<VoiceState>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrivateChannel {
    pub flags: u64,
    pub id: ChannelId,
    pub is_message_request: bool,
    pub is_message_request_timestamp: Option<String>,
    pub is_spam: bool,
    pub last_message_id: Option<MessageId>,
    pub last_pin_timestamp: Option<String>,
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
            .and_then(|x| Some(x.to_value_decoder(MessageId::decode)))
            .transpose()?;
        let last_pin_timestamp = map
            .get("last_pin_timestamp")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let recipient_flags = map.get("recipient_flags").unwrap().to_u64()?;
        let recipient = map
            .get("recipients")
            .unwrap()
            .to_array_decoder(User::decode)?
            .first()
            .unwrap()
            .clone(); // TODO
        let safety_warnings = map
            .get("safety_warnings")
            .unwrap()
            .to_array_value_decoder(|x| x.to_string())?;
        map.check_empty_panic("PrivateChannel");
        Ok(Self {
            flags,
            id,
            is_message_request,
            is_message_request_timestamp,
            is_spam,
            last_message_id,
            last_pin_timestamp,
            recipient_flags,
            recipient,
            safety_warnings,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PublicChannel {
    pub flags: u64,
    pub id: ChannelId,
    pub last_message_id: Option<MessageId>,
    pub name: String,
    pub parent_id: ChannelId,
    // pub permission_overwrites: PermissionOverwrite,
    pub position: u64,
    pub rate_limit_per_user: u64,
    pub topic: String,
    pub version: u64,
}

impl PublicChannel {
    pub fn decode(mut map: WrappedMap) -> Result<Self> {
        let flags = map.get("flags").unwrap().to_u64()?;
        let id = map.get("id").unwrap().to_value_decoder(ChannelId::decode)?;
        let last_message_id = map
            .get("last_message_id")
            .and_then(|x| Some(x.to_value_decoder(MessageId::decode)))
            .transpose()?;
        let name = map.get("name").unwrap().to_string()?;
        let parent_id = map
            .get("parent_id")
            .unwrap()
            .to_value_decoder(ChannelId::decode)?;
        // let permission_overwrites = map
        //     .get("permission_overwrites")
        //     .unwrap()
        //     .to_decoder(PermissionOverwrite::decode)?;
        let position = map.get("position").unwrap().to_u64()?;
        let rate_limit_per_user = map.get("rate_limit_per_user").unwrap().to_u64()?;
        let topic = map.get("topic").unwrap().to_string()?;
        let version = map.get("version").unwrap().to_u64()?;
        map.check_empty_panic("PublicChannel");
        Ok(Self {
            flags,
            id,
            last_message_id,
            name,
            parent_id,
            // permission_overwrites,
            position,
            rate_limit_per_user,
            topic,
            version,
        })
    }

    #[inline(always)]
    pub fn mention(&self) -> Mention {
        self.id.mention()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CurrentUser {
    pub accent_color: Option<u64>,
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
    pub global_name: Option<String>,
    pub id: UserId,
    pub mfa_enabled: bool,
    pub mobile: bool,
    pub nsfw_allowed: bool,
    pub phone: Option<String>,
    pub premium: bool,
    pub premium_type: u64,
    pub primary_guild: Option<String>,
    pub pronouns: String,
    pub public_flags: Option<u64>,
    pub purchased_flags: u64,
    pub username: String,
    pub verified: bool,
}

impl CurrentUser {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let accent_color = map
            .get("accent_color")
            .and_then(|x| Some(x.to_u64()))
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
        let global_name = map
            .get("global_name")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let id = map.get("id").unwrap().to_value_decoder(UserId::decode)?;
        let mfa_enabled = map.get("mfa_enabled").unwrap().to_bool()?;
        let mobile = map.get("mobile").unwrap().to_bool()?;
        let nsfw_allowed = map.get("nsfw_allowed").unwrap().to_bool()?;
        let phone = map
            .get("phone")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let premium = map.get("premium").unwrap().to_bool()?;
        let premium_type = map.get("premium_type").unwrap().to_u64()?;
        let primary_guild = map
            .get("primary_guild")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let pronouns = map.get("pronouns").unwrap().to_string()?;
        let public_flags = map
            .get("public_flags")
            .and_then(|x| Some(x.to_u64()))
            .transpose()?;
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
            public_flags,
            purchased_flags,
            username,
            verified,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Relationship {
    pub id: UserId,
    pub is_spam_request: bool,
    pub nickname: Option<String>,
    pub since: String,
    pub type_relationship: RelationshipType,
    pub user: User,
    pub user_ignored: bool,
}

impl Relationship {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let id = map.get("id").unwrap().to_value_decoder(UserId::decode)?;
        let is_spam_request = map.get("is_spam_request").unwrap().to_bool()?;
        let nickname = map
            .get("nickname")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let since = map.get("since").unwrap().to_string()?;
        let type_relationship = map
            .get("type")
            .unwrap()
            .to_value_decoder(RelationshipType::decode)?;
        let user = map.get("user").unwrap().to_decoder(User::decode)?;
        let user_ignored = map.get("user_ignored").unwrap().to_bool()?;
        map.check_empty_panic("Relationship");
        Ok(Self {
            id,
            is_spam_request,
            nickname,
            since,
            type_relationship,
            user,
            user_ignored,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelationshipType {
    Ignored,
    Friends,
    Blocked,
    IncomingRequest,
    OutgoingRequest,
}

impl RelationshipType {
    fn decode(value: WrappedValue) -> Result<Self> {
        Ok(match value.to_u64()? {
            0 => RelationshipType::Ignored,
            1 => RelationshipType::Friends,
            2 => RelationshipType::Blocked,
            3 => RelationshipType::IncomingRequest,
            4 => RelationshipType::OutgoingRequest,
            other => panic!("unknown type, {:?}", other),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Presence {
    pub activities: Vec<PresenceActivity>,
    pub client_status: PresenceClientStatus,
    pub last_modified: u64,
    pub restricted_application_id: Option<String>,
    pub status: Status,
    pub user: User,
}

impl Presence {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let activities = map
            .get("activities")
            .unwrap()
            .to_array_decoder(PresenceActivity::decode)?;
        let client_status = map
            .get("client_status")
            .unwrap()
            .to_decoder(PresenceClientStatus::decode)?;
        let last_modified = map.get("last_modified").unwrap().to_u64()?;
        let restricted_application_id = map
            .get("restricted_application_id")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
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
            restricted_application_id,
            status,
            user,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Online,
    Idle,
    Dnd,
    Offline,
}

impl Status {
    fn decode(value: WrappedValue) -> Result<Self> {
        Ok(match value.to_string()?.as_str() {
            "online" => Status::Online,
            "idle" => Status::Idle,
            "dnd" => Status::Dnd,
            "offline" => Status::Offline,
            other => panic!("unknown status: {:?}", other),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct PresenceActivity {
    pub application_id: Option<ApplicationId>,
    pub assets: Option<PresenceActivityAsset>,
    pub created_at: u64,
    pub details: Option<String>,
    pub emoji: Option<Emoji>,
    pub flags: Option<u64>,
    pub id: String,
    pub name: String,
    pub party: Option<PresenceActivityParty>,
    pub session_id: Option<String>,
    pub state: Option<String>,
    pub sync_id: Option<String>,
    pub timestamp: Option<Timestamp>,
    pub type_activity: u64, // TODO separate into types
}

impl PresenceActivity {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let application_id = map
            .get("application_id")
            .and_then(|x| Some(x.to_value_decoder(ApplicationId::decode)))
            .transpose()?;
        let assets = map
            .get("assets")
            .and_then(|x| Some(x.to_decoder(PresenceActivityAsset::decode)))
            .transpose()?;
        let created_at = map.get("created_at").unwrap().to_u64()?;
        let details = map
            .get("details")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let emoji = map
            .get("emoji")
            .and_then(|x| Some(x.to_decoder(Emoji::decode)))
            .transpose()?;
        let flags = map
            .get("flags")
            .and_then(|x| Some(x.to_u64()))
            .transpose()?;
        let id = map.get("id").unwrap().to_string()?;
        let name = map.get("name").unwrap().to_string()?;
        let party = map
            .get("party")
            .and_then(|x| Some(x.to_decoder(PresenceActivityParty::decode)))
            .transpose()?;
        let session_id = map
            .get("session_id")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let state = map
            .get("state")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let sync_id = map
            .get("sync_id")
            .and_then(|x| Some(x.to_string()))
            .transpose()?;
        let type_activity = map.get("type").unwrap().to_u64()?;
        let timestamp = map
            .get("timestamps")
            .and_then(|x| Some(x.to_decoder(Timestamp::decode)))
            .transpose()?;
        map.check_empty_panic("PresenceActivity");
        Ok(Self {
            application_id,
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
            timestamp,
            type_activity,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Timestamp {
    pub end: Option<u64>,
    pub start: Option<u64>,
}

impl Timestamp {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let end = map.get("end").and_then(|x| Some(x.to_u64())).transpose()?;
        let start = map
            .get("start")
            .and_then(|x| Some(x.to_u64()))
            .transpose()?;
        map.check_empty_panic("Timestamp");
        Ok(Self { end, start })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PresenceActivityParty {
    pub id: String,
}

impl PresenceActivityParty {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let id = map.get("id").unwrap().to_string()?;
        map.check_empty_panic("PresenceActivityParty");
        Ok(Self { id })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PresenceActivityAsset {
    pub large_image: String,
    pub large_text: String,
}

impl PresenceActivityAsset {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let large_image = map.get("large_image").unwrap().to_string()?;
        let large_text = map.get("large_text").unwrap().to_string()?;
        map.check_empty_panic("PresenceActivityAsset");
        Ok(Self {
            large_image,
            large_text,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ReadyEvent {
    pub presences: Vec<Presence>,
    pub private_channels: Vec<Channel>,
    pub relationships: Vec<Relationship>,
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
        let servers = map
            .get("guilds")
            .unwrap()
            .to_array_decoder(Server::decode)?;
        map.get("notes");
        map.get("notification_settings");
        let presences = map
            .get("presences")
            .unwrap()
            .to_array_decoder(Presence::decode)?;
        let private_channels = map
            .get("private_channels")
            .unwrap()
            .to_array_decoder(Channel::decode)?;
        map.get("read_state");
        let relationships = map
            .get("relationships")
            .unwrap()
            .to_array_decoder(Relationship::decode)?;
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
            relationships,
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
                        .to_u64()? as usize,
                    Event::decode(
                        &map.get("t")
                            .expect("t not found in websocket message")
                            .to_string()?,
                        map.get("d").expect("d not found in websocket message"),
                    )?,
                ),
                Ok(1) => GatewayEvent::Heartbeat(
                    map.get("s")
                        .expect("s not found in websocket message")
                        .to_u64()? as usize,
                ),
                Ok(7) => GatewayEvent::Reconnect,
                Ok(9) => GatewayEvent::InvalidateSession,
                Ok(10) => GatewayEvent::Hello(
                    map.get("d")
                        .expect("d not found in websocket message")
                        .to_map()?
                        .get("heartbeat_interval")
                        .expect("heartbeat_interval not found in websocket message")
                        .to_u64()? as usize,
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
        other => {
            println!("websocket message not text or binary: {:?}", other);
            Err(Error::msg("websocket message not text or binary"))
        }
    }
}
