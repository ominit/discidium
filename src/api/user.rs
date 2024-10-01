pub struct Client {}

impl Client {
    pub fn login_user(email: impl Into<String>, password: impl Into<String>) -> String {
        let post_body = ureq::json!({
            "email": email.into(),
            "password": password.into(),
        });

        let endpoint = "https://discord.com/api/v9/auth/login";

        let raw_response = ureq::post(&endpoint)
            .set("Content-Type", "application/json")
            .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) discord/1.0.9163 Chrome/124.0.6367.243 Electron/30.2.0 Safari/537.36")
            .send_json(post_body)
            .unwrap();

        let text = raw_response.into_string().unwrap();
        // println!("{:?}", text.clone());

        ureq::serde_json::from_str::<ureq::serde_json::Value>(&text)
            .unwrap()
            .get("token")
            .unwrap()
            .to_string()
    }

    pub fn send_message(
        channel_id: impl Into<String>,
        text: impl Into<String>,
        token: impl Into<String>,
    ) {
        let endpoint = format!(
            "https://discord.com/api/v9/channels/{}/messages",
            channel_id.into()
        );

        let post_body = ureq::json!({
            "content": text.into(),
        });
        println!("{}", post_body);

        let raw_response = ureq::post(&endpoint)
            .set("Content-Type", "application/json")
            .set("Authorization", &token.into())
            .send_json(post_body)
            .unwrap();
        println!("{:?}", raw_response);
        println!("{:?}", raw_response.into_string().unwrap());
    }

    pub fn get_guilds(token: impl Into<String>) -> Vec<Guild> {
        let endpoint = format!("https://discord.com/api/v9/users/@me/guilds");

        let raw_response = ureq::get(&endpoint)
            .set("Content-Type", "application/json")
            .set("Authorization", &token.into())
            .call()
            .unwrap();
        // println!("{:?}", raw_response);
        // println!("{:?}", raw_response.into_string().unwrap());
        // vec![]
        ureq::serde_json::from_str::<Vec<Guild>>(&raw_response.into_string().unwrap()).unwrap()
    }

    pub fn get_dms(token: impl Into<String>) -> Vec<DMChat> {
        let endpoint = format!("https://discord.com/api/v9/users/@me/channels");

        let raw_response = ureq::get(&endpoint)
            .set("Content-Type", "application/json")
            .set("Authorization", &token.into())
            .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) discord/1.0.9163 Chrome/124.0.6367.243 Electron/30.2.0 Safari/537.36")
            .call()
            .unwrap();
        // println!("{:?}", raw_response);
        // println!("{:?}", raw_response.into_string().unwrap().clone());
        // vec![]
        ureq::serde_json::from_str::<Vec<DMChat>>(&raw_response.into_string().unwrap()).unwrap()
    }

    pub fn get_user_profile(token: impl Into<String>, user_id: impl Into<String>) -> Profile {
        let endpoint = format!("https://discord.com/api/v9/users/{}/profile?with_mutual_guilds=true&with_mutual_friends=true&with_mutual_friends_count=false", user_id.into());

        let raw_response = ureq::get(&endpoint)
            .set("Content-Type", "application/json")
            .set("Authorization", &token.into())
            .call()
            .unwrap();
        // println!("{:?}", raw_response);
        // println!("{:?}", raw_response.into_string().unwrap().clone());
        // vec![]
        ureq::serde_json::from_str::<Profile>(&raw_response.into_string().unwrap()).unwrap()
    }

    pub fn get_messages(token: impl Into<String>, channel_id: impl Into<String>) -> Vec<Message> {
        let endpoint = format!(
            "https://discord.com/api/v9/channels/{}/messages?limit=50",
            channel_id.into()
        );

        let raw_response = ureq::get(&endpoint)
            .set("Content-Type", "application/json")
            .set("Authorization", &token.into())
            .call()
            .unwrap();
        // println!("{:?}", raw_response);
        // println!("{:?}", raw_response.into_string().unwrap().clone());
        // vec![]
        let mut messages =
            ureq::serde_json::from_str::<Vec<Message>>(&raw_response.into_string().unwrap())
                .unwrap();

        messages.reverse();
        messages
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct Message {
    #[serde(rename = "type")]
    message_type: usize,
    pub content: String,
    // mentions: Vec<String>,
    // mention_roles: Vec<String>,
    // attachments: Vec<String>,
    // embeds: Vec<String>,
    // timestamp: String,
    // edited_timestamp: Option<String>,
    // flags: usize,
    // components: Vec<String>,
    // id: String,
    // channel_id: String,
    pub author: User,
    pinned: bool,
    mention_everyone: bool,
    tts: bool,
}

#[derive(Debug, serde::Deserialize)]
pub struct Guild {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub banner: Option<String>,
    pub owner: bool,
    pub permissions: String,
    pub features: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct DMChat {
    pub id: String,
    pub recipients: Vec<User>,
    pub owner_id: Option<String>,
    pub icon: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "type")]
    dm_type: usize,
    // pub last_message_id: String,
    pub flags: usize,
}

impl DMChat {
    pub fn get_dm_name(&self) -> String {
        if self.name.is_some() {
            return self.name.as_ref().unwrap().clone();
        }
        self.recipients
            .iter()
            .map(|x| x.username.clone())
            .collect::<Vec<String>>()
            .join(", ")
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct Profile {
    pub user: User,
    pub connected_accounts: Vec<Connection>,
    pub premium_since: Option<String>,
    pub premium_type: Option<String>,
    pub premium_guild_since: Option<String>,
    pub profile_themes_experiment_bucket: usize,
    pub badges: Vec<Badge>,
    // pub guild_badges: Vec<>,
    pub mutual_friends: Vec<User>,
    pub mutual_guilds: Vec<MutualGuild>,
}

#[derive(Debug, serde::Deserialize)]
pub struct MutualGuild {
    pub id: String,
    pub nick: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Badge {
    id: String,
    description: String,
    icon: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct Connection {
    #[serde(rename = "type")]
    connection_type: String,
    id: String,
    name: String,
    verified: bool,
}

#[derive(Debug, serde::Deserialize)]
pub struct User {
    pub id: String,
    // pub avatar: String,
    pub username: String,
    // pub global_name: Option<String>,
    // pub avatar_decoration_data: Option<String>,
    // pub discriminator: String,
    pub public_flags: usize,
    // pub clan: Option<String>,
    pub flags: Option<usize>,
    // pub banner: Option<String>,
    // pub banner_color: Option<String>,
    pub accent_color: Option<usize>,
    // pub bio: Option<String>,
}
