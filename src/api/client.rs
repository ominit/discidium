use anyhow::Result;
use secrecy::{ExposeSecret, SecretString};
use ureq::Response;

use crate::api::USER_AGENT;

use super::{connection::Connection, model::ReadyEvent, ratelimit::RateLimits, ENDPOINT_URL};

pub struct Client {
    ratelimits: RateLimits,
    client: ureq::Agent,
    token: SecretString,
}

impl Client {
    pub fn from_user_token(token: SecretString) -> Self {
        Self {
            ratelimits: Default::default(),
            client: ureq::agent(),
            token,
        }
    }

    pub fn connect(&self) -> Result<(Connection, ReadyEvent)> {
        let url = self.get_gateway_url()?;
        Connection::new(&url, self.token.clone())
    }

    /// pass None for json_body to make a request without a body
    fn request(
        &self,
        method: &str,
        url: &str,
        json_body: Option<ureq::serde_json::Value>,
    ) -> Result<Response> {
        let url = format!("{}{}", ENDPOINT_URL, url);
        let url = &url;
        self.ratelimits.pre_check(url);
        let response;
        if json_body.is_some() {
            response = self
                .client
                .request(method.into(), url)
                .set("Content-Type", "application/json")
                .set("User-Agent", USER_AGENT)
                .set("Authorization", &self.token.expose_secret())
                .send_json(json_body.clone().unwrap())?;
        } else {
            response = self
                .client
                .request(method.into(), url)
                .set("Content-Type", "application/json")
                .set("User-Agent", USER_AGENT)
                .set("Authorization", &self.token.expose_secret())
                .call()?;
        }
        if self.ratelimits.check_for_ratelimit(url, &response) {
            let response;
            if json_body.is_some() {
                response = self
                    .client
                    .request(method.into(), url)
                    .set("Content-Type", "application/json")
                    .set("User-Agent", USER_AGENT)
                    .set("Authorization", &self.token.expose_secret())
                    .send_json(json_body.unwrap())?;
            } else {
                response = self
                    .client
                    .request(method.into(), url)
                    .set("Content-Type", "application/json")
                    .set("User-Agent", USER_AGENT)
                    .set("Authorization", &self.token.expose_secret())
                    .call()?;
            }
            self.ratelimits.check_for_ratelimit(url, &response);
            return Ok(response);
        }
        Ok(response)
    }

    fn get_gateway_url(&self) -> Result<String> {
        let response = self.request("GET", "/gateway", None)?;
        Ok(response
            .into_json::<ureq::serde_json::Value>()?
            .get("url")
            .expect("no url in response to get_gateway_url")
            .as_str()
            .expect("could not parse str")
            .replace("\"", ""))
    }
}
