pub mod client;
pub mod connection;
pub mod model;
mod ratelimit;
pub mod state;

pub use connection::Connection;

const ENDPOINT_URL: &str = "https://discord.com/api/v9/";
const CDN_URL: &str = "https://cdn.discordapp.com/";
const USER_AGENT: &str = "discidium";
