#![allow(dead_code, unused)] // TODO :(

use std::io::Read;

use anyhow::{Error, Result};
use chrono::{DateTime, FixedOffset};
use ewebsock::{connect, Options, WsEvent, WsMessage, WsReceiver, WsSender};
use serde_json::{Map, Value};
use yew::platform::time::sleep;

use super::CDN_URL;

#[derive(Debug, Clone)]
pub struct WrappedMap(Map<String, Value>);

impl WrappedMap {
    fn get<T, F: FnOnce(WrappedMap) -> Result<T>>(
        &mut self,
        key: &str,
        decode: F,
    ) -> Option<Result<T>> {
        let value = self.0.remove(key)?;
        if value.is_null() {
            return None;
        }
        Some(WrappedValue(value).to_decoder(decode))
    }

    fn get_value<T, F: FnOnce(WrappedValue) -> Result<T>>(
        &mut self,
        key: &str,
        decode: F,
    ) -> Option<Result<T>> {
        let value = self.0.remove(key)?;
        if value.is_null() {
            return None;
        }
        Some(WrappedValue(value).to_value_decoder(decode))
    }

    fn get_array<T, F: Clone + Fn(WrappedMap) -> Result<T>>(
        &mut self,
        key: &str,
        decode: F,
    ) -> Option<Result<Vec<T>>> {
        let value = self.0.remove(key)?;
        if value.is_null() {
            return None;
        }
        Some(WrappedValue(value).to_array_decoder(decode))
    }

    fn get_array_value<T, F: Clone + Fn(WrappedValue) -> Result<T>>(
        &mut self,
        key: &str,
        decode: F,
    ) -> Option<Result<Vec<T>>> {
        let value = self.0.remove(key)?;
        if value.is_null() {
            return None;
        }
        Some(WrappedValue(value).to_array_value_decoder(decode))
    }

    /// type_name is used in the panic message
    fn check_empty_panic(self, type_name: &str) {
        if !self.0.is_empty() {
            // panic!("value not taken out of {}: {:?}", type_name, self.0);
            println!("value not taken out of {}: {:?}", type_name, self.0);
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
        Ok(Self(
            map.get_value("name", WrappedValue::to_string).unwrap()?,
        ))
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
        let flags = map.get_value("flags", WrappedValue::to_u64).unwrap()?;
        let id = map.get_value("id", ChannelId::decode).unwrap()?;
        let name = map.get_value("name", WrappedValue::to_string).unwrap()?;
        let parent_id = map.get_value("parent_id", ChannelId::decode).transpose()?;
        let permission_overwrites = map
            .get_array("permission_overwrites", PermissionOverwrite::decode)
            .unwrap()?;
        let position = map.get_value("position", WrappedValue::to_u64).unwrap()?;
        let version = map.get_value("version", WrappedValue::to_u64).unwrap()?;
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
        Ok(
            match map
                .get_value("type", WrappedValue::to_string)
                .unwrap()?
                .as_str()
            {
                "member" => PermissionOverwrite::Member(PermissionOverwriteMember::decode(map)?),
                "role" => PermissionOverwrite::Role(PermissionOverwriteRole::decode(map)?),
                other => todo!("{:?}", other),
            },
        )
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
        let allow = map.get_value("allow", Permission::decode).unwrap()?;
        let allow_new = map.get_value("allow_new", Permission::decode).unwrap()?;
        let deny = map.get_value("deny", Permission::decode).unwrap()?;
        let deny_new = map.get_value("deny_new", Permission::decode).unwrap()?;
        let id = map.get_value("id", UserId::decode).unwrap()?;
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
        let allow = map.get_value("allow", Permission::decode).unwrap()?;
        let allow_new = map.get_value("allow_new", Permission::decode).unwrap()?;
        let deny = map.get_value("deny", Permission::decode).unwrap()?;
        let deny_new = map.get_value("deny_new", Permission::decode).unwrap()?;
        let id = map.get_value("id", RoleId::decode).unwrap()?;
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
            .get_value("afk_channel_id", ChannelId::decode)
            .transpose()?;
        let afk_timeout = map
            .get_value("afk_timeout", WrappedValue::to_u64)
            .unwrap()?;
        let channels = map.get_array("channels", Channel::decode).unwrap()?;
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
    // pub primary_guild: Option<Clan>,
    pub public_flags: Option<u64>,
    pub system: Option<bool>,
    pub username: String,
}

impl User {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let avatar = map
            .get_value("avatar", WrappedValue::to_string)
            .transpose()?;
        let avatar_decoration_data = map
            .get("avatar_decoration_data", AvatarDecorationData::decode)
            .transpose()?;
        let bot = map.get_value("bot", WrappedValue::to_bool).transpose()?;
        let clan = map.get("clan", Clan::decode).transpose()?;
        let discriminator = map
            .get_value("discriminator", WrappedValue::to_string)
            .unwrap()?
            .parse::<u16>()?;
        let global_name = map
            .get_value("global_name", WrappedValue::to_string)
            .transpose()?;
        let id = map.get_value("id", UserId::decode).unwrap()?;
        // let primary_guild = map
        //     .get("primary_guild")
        //     .and_then(|x| Some(x.to_decoder(Clan::decode)))
        //     .transpose()?;
        let public_flags = map
            .get_value("public_flags", WrappedValue::to_u64)
            .transpose()?;
        let system = map.get_value("system", WrappedValue::to_bool).transpose()?;
        let username = map
            .get_value("username", WrappedValue::to_string)
            .unwrap()?;
        map.check_empty_panic("User");
        Ok(Self {
            avatar,
            avatar_decoration_data,
            bot,
            clan,
            discriminator,
            global_name,
            id,
            // primary_guild,
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
            .get_value("badge", WrappedValue::to_string)
            .transpose()?;
        let identity_enabled = map
            .get_value("identity_enabled", WrappedValue::to_bool)
            .unwrap()?;
        let identity_guild_id = map
            .get_value("identity_guild_id", ServerId::decode)
            .transpose()?;
        let tag = map.get_value("tag", WrappedValue::to_string).transpose()?;
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
        let asset = map.get_value("asset", WrappedValue::to_string).unwrap()?;
        let expires_at = map
            .get_value("expires_at", WrappedValue::to_u64)
            .transpose()?;
        let sku_id = map.get_value("sku_id", WrappedValue::to_string).unwrap()?;
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
    Voice(VoiceChannel),
    Category(ChannelCategory),
    News,
    Store,
}

impl Channel {
    pub fn decode(mut map: WrappedMap) -> Result<Self> {
        Ok(match map.get_value("type", ChannelType::decode).unwrap()? {
            ChannelType::Group => Channel::Group(Group::decode(map)?),
            ChannelType::Private => Channel::Private(PrivateChannel::decode(map)?),
            ChannelType::Public => Channel::Public(PublicChannel::decode(map)?),
            ChannelType::Voice => Channel::Voice(VoiceChannel::decode(map)?),
            ChannelType::Category => Channel::Category(ChannelCategory::decode(map)?),
            other => todo!("{:?}", other),
        })
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
            .get_value("blocked_user_warning_dismissed", WrappedValue::to_bool)
            .unwrap()?;
        let flags = map.get_value("flags", WrappedValue::to_u64).unwrap()?;
        let icon = map.get_value("icon", WrappedValue::to_string).transpose()?;
        let id = map.get_value("id", ChannelId::decode).unwrap()?;
        let last_message_id = map
            .get_value("last_message_id", MessageId::decode)
            .unwrap()?;
        let last_pin_timestamp = map
            .get_value("last_pin_timestamp", WrappedValue::to_string)
            .transpose()?;
        let name = map.get_value("name", WrappedValue::to_string).transpose()?;
        let owner_id = map.get_value("owner_id", UserId::decode).unwrap()?;
        let recipient_flags = map
            .get_value("recipient_flags", WrappedValue::to_u64)
            .unwrap()?;
        let recipients = map.get_array("recipients", User::decode).unwrap()?;
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
        let flags = map.get_value("flags", WrappedValue::to_u64).unwrap()?;
        let id = map.get_value("id", ChannelId::decode).unwrap()?;
        let is_message_request = map
            .get_value("is_message_request", WrappedValue::to_bool)
            .unwrap()?;
        let is_message_request_timestamp = map
            .get_value("is_message_request_timestamp", WrappedValue::to_string)
            .transpose()?;
        let is_spam = map.get_value("is_spam", WrappedValue::to_bool).unwrap()?;
        let last_message_id = map
            .get_value("last_message_id", MessageId::decode)
            .transpose()?;
        let last_pin_timestamp = map
            .get_value("last_pin_timestamp", WrappedValue::to_string)
            .transpose()?;
        let recipient_flags = map
            .get_value("recipient_flags", WrappedValue::to_u64)
            .unwrap()?;
        let recipient = map
            .get_array("recipients", User::decode)
            .unwrap()?
            .first()
            .unwrap()
            .clone(); // TODO
        let safety_warnings = map
            .get_array_value("safety_warnings", WrappedValue::to_string)
            .unwrap()?;
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
    pub parent_id: Option<ChannelId>,
    pub permission_overwrites: Vec<PermissionOverwrite>,
    pub position: u64,
    pub rate_limit_per_user: u64,
    pub topic: Option<String>,
    pub version: u64,
}

impl PublicChannel {
    pub fn decode(mut map: WrappedMap) -> Result<Self> {
        let flags = map.get_value("flags", WrappedValue::to_u64).unwrap()?;
        let id = map.get_value("id", ChannelId::decode).unwrap()?;
        let last_message_id = map
            .get_value("last_message_id", MessageId::decode)
            .transpose()?;
        let name = map.get_value("name", WrappedValue::to_string).unwrap()?;
        let parent_id = map.get_value("parent_id", ChannelId::decode).transpose()?;
        let permission_overwrites = map
            .get_array("permission_overwrites", PermissionOverwrite::decode)
            .unwrap()?;
        let position = map.get_value("position", WrappedValue::to_u64).unwrap()?;
        let rate_limit_per_user = map
            .get_value("rate_limit_per_user", WrappedValue::to_u64)
            .unwrap()?;
        let topic = map
            .get_value("topic", WrappedValue::to_string)
            .transpose()?;
        let version = map.get_value("version", WrappedValue::to_u64).unwrap()?;
        map.check_empty_panic("PublicChannel");
        Ok(Self {
            flags,
            id,
            last_message_id,
            name,
            parent_id,
            permission_overwrites,
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
pub struct VoiceChannel {
    pub bitrate: u64,
    pub flags: u64,
    pub id: ChannelId,
    pub last_message_id: Option<MessageId>,
    pub name: String,
    pub parent_id: Option<ChannelId>,
    pub permission_overwrites: Vec<PermissionOverwrite>,
    pub position: u64,
    pub rate_limit_per_user: u64,
    pub rtc_region: Option<String>,
    pub user_limit: u64,
    pub version: u64,
}

impl VoiceChannel {
    pub fn decode(mut map: WrappedMap) -> Result<Self> {
        let bitrate = map.get_value("bitrate", WrappedValue::to_u64).unwrap()?;
        let flags = map.get_value("flags", WrappedValue::to_u64).unwrap()?;
        let id = map.get_value("id", ChannelId::decode).unwrap()?;
        let last_message_id = map
            .get_value("last_message_id", MessageId::decode)
            .transpose()?;
        let name = map.get_value("name", WrappedValue::to_string).unwrap()?;
        let parent_id = map.get_value("parent_id", ChannelId::decode).transpose()?;
        let permission_overwrites = map
            .get_array("permission_overwrites", PermissionOverwrite::decode)
            .unwrap()?;
        let position = map.get_value("position", WrappedValue::to_u64).unwrap()?;
        let rate_limit_per_user = map
            .get_value("rate_limit_per_user", WrappedValue::to_u64)
            .unwrap()?;
        let rtc_region = map
            .get_value("rtc_region", WrappedValue::to_string)
            .transpose()?;
        let user_limit = map.get_value("user_limit", WrappedValue::to_u64).unwrap()?;
        let version = map.get_value("version", WrappedValue::to_u64).unwrap()?;
        map.check_empty_panic("VoiceChannel");
        Ok(Self {
            bitrate,
            flags,
            id,
            last_message_id,
            name,
            parent_id,
            permission_overwrites,
            position,
            rate_limit_per_user,
            rtc_region,
            user_limit,
            version,
        })
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
    pub clan: Option<Clan>,
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
            .get_value("accent_color", WrappedValue::to_u64)
            .transpose()?;
        let avatar = map.get_value("avatar", WrappedValue::to_string).unwrap()?;
        let avatar_decoration_data = map
            .get_value("avatar_decoration_data", WrappedValue::to_string)
            .transpose()?;
        let banner = map
            .get_value("banner", WrappedValue::to_string)
            .transpose()?;
        let banner_color = map
            .get_value("banner_color", WrappedValue::to_string)
            .transpose()?;
        let bio = map.get_value("bio", WrappedValue::to_string).unwrap()?;
        let clan = map.get("clan", Clan::decode).transpose()?;
        let desktop = map.get_value("desktop", WrappedValue::to_bool).unwrap()?;
        let discriminator = map
            .get_value("discriminator", WrappedValue::to_string)
            .unwrap()?
            .parse::<u16>()?;
        let email = map.get_value("email", WrappedValue::to_string).unwrap()?;
        let flags = map.get_value("flags", WrappedValue::to_u64).unwrap()?;
        let global_name = map
            .get_value("global_name", WrappedValue::to_string)
            .transpose()?;
        let id = map.get_value("id", UserId::decode).unwrap()?;
        let mfa_enabled = map
            .get_value("mfa_enabled", WrappedValue::to_bool)
            .unwrap()?;
        let mobile = map.get_value("mobile", WrappedValue::to_bool).unwrap()?;
        let nsfw_allowed = map
            .get_value("nsfw_allowed", WrappedValue::to_bool)
            .unwrap()?;
        let phone = map
            .get_value("phone", WrappedValue::to_string)
            .transpose()?;
        let premium = map.get_value("premium", WrappedValue::to_bool).unwrap()?;
        let premium_type = map
            .get_value("premium_type", WrappedValue::to_u64)
            .unwrap()?;
        let primary_guild = map
            .get_value("primary_guild", WrappedValue::to_string)
            .transpose()?;
        let pronouns = map
            .get_value("pronouns", WrappedValue::to_string)
            .unwrap()?;
        let public_flags = map
            .get_value("public_flags", WrappedValue::to_u64)
            .transpose()?;
        let purchased_flags = map
            .get_value("purchased_flags", WrappedValue::to_u64)
            .unwrap()?;
        let username = map
            .get_value("username", WrappedValue::to_string)
            .unwrap()?;
        let verified = map.get_value("verified", WrappedValue::to_bool).unwrap()?;
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
        let id = map.get_value("id", UserId::decode).unwrap()?;
        let is_spam_request = map
            .get_value("is_spam_request", WrappedValue::to_bool)
            .unwrap()?;
        let nickname = map
            .get_value("nickname", WrappedValue::to_string)
            .transpose()?;
        let since = map.get_value("since", WrappedValue::to_string).unwrap()?;
        let type_relationship = map.get_value("type", RelationshipType::decode).unwrap()?;
        let user = map.get("user", User::decode).unwrap()?;
        let user_ignored = map
            .get_value("user_ignored", WrappedValue::to_bool)
            .unwrap()?;
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
            .get_array("activities", PresenceActivity::decode)
            .unwrap()?;
        let client_status = map
            .get("client_status", PresenceClientStatus::decode)
            .unwrap()?;
        let last_modified = map
            .get_value("last_modified", WrappedValue::to_u64)
            .unwrap()?;
        let restricted_application_id = map
            .get_value("restricted_application_id", WrappedValue::to_string)
            .transpose()?;
        let status = map.get_value("status", Status::decode).unwrap()?;
        let user = map.get("user", User::decode).unwrap()?;
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
        let desktop = map.get_value("desktop", Status::decode).transpose()?;
        let mobile = map.get_value("mobile", Status::decode).transpose()?;
        let web = map.get_value("web", Status::decode).transpose()?;
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
            .get_value("application_id", ApplicationId::decode)
            .transpose()?;
        let assets = map
            .get("assets", PresenceActivityAsset::decode)
            .transpose()?;
        let created_at = map.get_value("created_at", WrappedValue::to_u64).unwrap()?;
        let details = map
            .get_value("details", WrappedValue::to_string)
            .transpose()?;
        let emoji = map.get("emoji", Emoji::decode).transpose()?;
        let flags = map.get_value("flags", WrappedValue::to_u64).transpose()?;
        let id = map.get_value("id", WrappedValue::to_string).unwrap()?; // TODO
        let name = map.get_value("name", WrappedValue::to_string).unwrap()?;
        let party = map
            .get("party", PresenceActivityParty::decode)
            .transpose()?;
        let session_id = map
            .get_value("session_id", WrappedValue::to_string)
            .transpose()?;
        let state = map
            .get_value("state", WrappedValue::to_string)
            .transpose()?;
        let sync_id = map
            .get_value("sync_id", WrappedValue::to_string)
            .transpose()?;
        let type_activity = map.get_value("type", WrappedValue::to_u64).unwrap()?;
        let timestamp = map.get("timestamps", Timestamp::decode).transpose()?;
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
pub struct Timestamp {
    pub end: Option<u64>,
    pub start: Option<u64>,
}

impl Timestamp {
    fn decode(mut map: WrappedMap) -> Result<Self> {
        let end = map.get_value("end", WrappedValue::to_u64).transpose()?;
        let start = map.get_value("start", WrappedValue::to_u64).transpose()?;
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
        let id = map.get_value("id", WrappedValue::to_string).unwrap()?;
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
        let large_image = map
            .get_value("large_image", WrappedValue::to_string)
            .unwrap()?;
        let large_text = map
            .get_value("large_text", WrappedValue::to_string)
            .unwrap()?;
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
        map.get("_trace", |_| Err::<u32, Error>(Error::msg("")));
        map.get("analytics_token", |_| Err::<u32, Error>(Error::msg("")));
        map.get("api_code_version", |_| Err::<u32, Error>(Error::msg("")));
        map.get("auth", |_| Err::<u32, Error>(Error::msg("")));
        map.get("auth_session_id_hash", |_| {
            Err::<u32, Error>(Error::msg(""))
        });
        map.get("broadcaster_user_ids", |_| {
            Err::<u32, Error>(Error::msg(""))
        });
        map.get("connected_accounts", |_| Err::<u32, Error>(Error::msg("")));
        map.get("consents", |_| Err::<u32, Error>(Error::msg("")));
        map.get("country_code", |_| Err::<u32, Error>(Error::msg("")));
        map.get("experiments", |_| Err::<u32, Error>(Error::msg("")));
        map.get("explicit_content_scan_version", |_| {
            Err::<u32, Error>(Error::msg(""))
        });
        map.get("friend_suggestion_count", |_| {
            Err::<u32, Error>(Error::msg(""))
        });
        map.get("game_relationships", |_| Err::<u32, Error>(Error::msg("")));
        map.get("geo_ordered_rtc_regions", |_| {
            Err::<u32, Error>(Error::msg(""))
        });
        map.get("guild_experiments", |_| Err::<u32, Error>(Error::msg("")));
        map.get("guild_join_requests", |_| Err::<u32, Error>(Error::msg("")));
        let servers = map.get_array("guilds", Server::decode).unwrap()?;
        map.get("notes", |_| Err::<u32, Error>(Error::msg("")));
        map.get("notification_settings", |_| {
            Err::<u32, Error>(Error::msg(""))
        });
        let presences = map.get_array("presences", Presence::decode).unwrap()?;
        let private_channels = map
            .get_array("private_channels", Channel::decode)
            .unwrap()?;
        map.get("read_state", |_| Err::<u32, Error>(Error::msg("")));
        let relationships = map
            .get_array("relationships", Relationship::decode)
            .unwrap()?;
        map.get("resume_gateway_url", |_| Err::<u32, Error>(Error::msg("")));
        let session_id = map
            .get_value("session_id", WrappedValue::to_string)
            .unwrap()?;
        map.get("session_type", |_| Err::<u32, Error>(Error::msg("")));
        map.get("sessions", |_| Err::<u32, Error>(Error::msg("")));
        map.get("static_client_session_id", |_| {
            Err::<u32, Error>(Error::msg(""))
        });
        map.get("tutorial", |_| Err::<u32, Error>(Error::msg("")));
        let user = map.get("user", CurrentUser::decode).unwrap()?;
        map.get("user_guild_settings", |_| Err::<u32, Error>(Error::msg("")));
        map.get("user_settings", |_| Err::<u32, Error>(Error::msg("")));
        map.get("user_settings_proto", |_| Err::<u32, Error>(Error::msg("")));
        let v = map.get_value("v", WrappedValue::to_u64).unwrap()?;
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
            "READY" => Ok(Self::Ready(value.to_decoder(ReadyEvent::decode)?)),
            // "SESSIONS_REPLACE" => Ok(Self::SessionsReplace(value.to_map()?.get(, ))),
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
            match map
                .get_value("op", WrappedValue::to_u64)
                .expect("op is null in GatewayEvent")
            {
                Ok(0) => GatewayEvent::Dispatch(
                    map.get_value("s", WrappedValue::to_u64)
                        .expect("s not found in websocket message")? as usize,
                    Event::decode(
                        &map.get_value("t", WrappedValue::to_string)
                            .expect("t not found in websocket message")?,
                        map.get_value("d", |x| Ok(x))
                            .expect("d not found in websocket message")?,
                    )?,
                ),
                Ok(1) => GatewayEvent::Heartbeat(
                    map.get_value("s", WrappedValue::to_u64)
                        .expect("s not found in websocket message")? as usize,
                ),
                Ok(7) => GatewayEvent::Reconnect,
                Ok(9) => GatewayEvent::InvalidateSession,
                Ok(10) => GatewayEvent::Hello(
                    map.get_value("d", |x| {
                        x.to_map()?
                            .get_value("heartbeat_interval", WrappedValue::to_u64)
                            .expect("heartbeat_interval not found in websocket message")
                    })
                    .expect("d not found in websocket message")? as usize,
                ),
                Ok(11) => Self::HeartbeatAck,
                _ => return Err(Error::msg("unexpected opcode")),
            },
        )
    }
}

pub async fn receive_json<F, T>(ws_receiver: &mut WsReceiver, decode: F) -> Result<T>
where
    F: FnOnce(WrappedMap) -> Result<T>,
{
    let received = {
        let mut a = ws_receiver.try_recv();
        while a.is_none() {
            sleep(std::time::Duration::from_millis(300)).await;
            a = ws_receiver.try_recv();
        }
        a.unwrap()
    };
    match received {
        WsEvent::Opened => Err(Error::msg("websocket should have already been opened")),
        WsEvent::Message(message) => match message {
            WsMessage::Binary(bin) => {
                let mut vec;
                let text = {
                    vec = Vec::new();
                    flate2::read::ZlibDecoder::new(&bin[..]).read_to_end(&mut vec)?;
                    &vec[..]
                };
                serde_json::from_reader(text)
                    .map_err(From::from)
                    .and_then(|x| WrappedValue(x).to_decoder(decode))
                    .map_err(|e| e)
            }
            WsMessage::Text(text) => serde_json::from_str(&text)
                .map_err(From::from)
                .and_then(|x| WrappedValue(x).to_decoder(decode))
                .map_err(|e| e),
            WsMessage::Unknown(_) => todo!(),
            WsMessage::Ping(vec) => todo!(),
            WsMessage::Pong(vec) => todo!(),
        },
        WsEvent::Error(_) => todo!(),
        WsEvent::Closed => panic!("websocket closed"),
        // other => {
        //     println!("websocket message not text or binary: {:?}", other);
        //     Err(Error::msg("websocket message not text or binary"))
        // }
    }
}
