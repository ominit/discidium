pub mod client;
pub mod connection;
pub mod model;
mod ratelimit;
pub mod state;
#[cfg(feature = "web")]
mod websocket;

pub use connection::Connection;
pub use model::*;
pub use state::State;

const ENDPOINT_URL: &str = "https://discord.com/api/v9/";
const CDN_URL: &str = "https://cdn.discordapp.com/";
const USER_AGENT: &str = "discidium";
