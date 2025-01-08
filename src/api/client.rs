use anyhow::Result;
use reqwest::{Method, Response};
use secrecy::{ExposeSecret, SecretString};
use serde_json::Value;

use crate::api::USER_AGENT;

use super::{connection::Connection, model::ReadyEvent, ratelimit::RateLimits, ENDPOINT_URL};

#[derive(Debug)]
pub struct Client {
    ratelimits: RateLimits,
    client: reqwest::Client,
    token: SecretString,
}

impl Client {
    pub fn from_user_token(token: SecretString) -> Self {
        Self {
            ratelimits: Default::default(),
            client: reqwest::Client::new(),
            token,
        }
    }

    pub async fn connect(&self) -> Result<(Connection, ReadyEvent)> {
        let url = self.get_gateway_url().await?;
        Connection::new(&url, self.token.clone())
    }

    /// pass None for json_body to make a request without a body
    async fn request(
        &self,
        method: Method,
        url: &str,
        json_body: Option<Value>,
    ) -> Result<Response> {
        let url = format!("{}{}", ENDPOINT_URL, url);
        let url = &url;
        self.ratelimits.pre_check(url);
        let response;
        if json_body.is_some() {
            response = self
                .client
                .request(method.clone(), url)
                .header("Content-Type", "application/json")
                .header("User-Agent", USER_AGENT)
                .header("Authorization", self.token.expose_secret())
                .body(json_body.as_ref().unwrap().to_string())
                .send()
                .await?;
        } else {
            response = self
                .client
                .request(method.clone(), url)
                .header("Content-Type", "application/json")
                .header("User-Agent", USER_AGENT)
                .header("Authorization", self.token.expose_secret())
                .send()
                .await?;
        }
        if self.ratelimits.check_for_ratelimit(url, &response) {
            let response;
            if json_body.is_some() {
                response = self
                    .client
                    .request(method, url)
                    .header("Content-Type", "application/json")
                    .header("User-Agent", USER_AGENT)
                    .header("Authorization", self.token.expose_secret())
                    .body(json_body.unwrap().to_string())
                    .send()
                    .await?;
            } else {
                response = self
                    .client
                    .request(method, url)
                    .header("Content-Type", "application/json")
                    .header("User-Agent", USER_AGENT)
                    .header("Authorization", self.token.expose_secret())
                    .send()
                    .await?;
            }
            self.ratelimits.check_for_ratelimit(url, &response);
            return Ok(response);
        }
        Ok(response)
    }

    async fn get_gateway_url(&self) -> Result<String> {
        let response = self.request(Method::GET, "/gateway", None).await?;
        Ok(response
            .json::<Value>()
            .await?
            .get("url")
            .expect("no url in response to get_gateway_url")
            .as_str()
            .expect("could not parse str")
            .replace("\"", ""))
    }
}
