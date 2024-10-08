pub mod client;
mod connection;
pub mod model;
mod ratelimit;
pub mod state;
pub mod user;

const ENDPOINT_URL: &str = "https://discord.com/api/v9/";
const USER_AGENT: &str = "discidium";
