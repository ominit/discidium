use std::{
    net::TcpStream,
    sync::{Arc, Mutex},
};

use anyhow::{Error, Result};
use secrecy::{ExposeSecret, SecretString};
use tungstenite::{stream::MaybeTlsStream, WebSocket};

use crate::api::model::{receive_json, Event, GatewayEvent};

use super::model::{ReadyEvent, UserId};

pub struct Connection {
    keepalive_channel: crossbeam_channel::Sender<Status>,
    websocket: Arc<Mutex<WebSocket<MaybeTlsStream<TcpStream>>>>,
    token: SecretString,
    session_id: Option<String>,
    last_sequence: usize,
    identify: ureq::serde_json::Value,
    user_id: UserId,
    ws_url: String,
    // voice
}

impl Connection {
    pub fn new(url: &str, token: SecretString) -> Result<(Self, ReadyEvent)> {
        let d = ureq::json!({
            "token": token.expose_secret(),
            "properties": {
                "$os": std::env::consts::OS,
                "$browser": "discidium",
                "$device": "discidium",
                "$referring_domain": "",
                "$referrer": "",
            },
            "large_threshold": 50,
            "compress": true,
        });
        let identify = ureq::json!({
            "op": 2,
            "d": d,
        });
        let mut websocket = Arc::new(Mutex::new(tungstenite::connect(url)?.0));

        websocket
            .lock()
            .unwrap()
            .send(tungstenite::Message::text(&identify.to_string()))?;

        // get heartbeat
        let heartbeat_interval;
        match receive_json(&mut websocket, GatewayEvent::decode)? {
            GatewayEvent::Hello(interval) => heartbeat_interval = interval,
            _ => return Err(Error::msg("expected hello during handshake")),
        }

        let (sender, receiver) = crossbeam_channel::unbounded();
        let keepalive_websocket = websocket.clone();
        std::thread::Builder::new()
            .name("Discord Websocket Keepalive".to_string())
            .spawn(move || keepalive(heartbeat_interval, keepalive_websocket, receiver))?;

        let sequence;
        let ready;
        match receive_json(&mut websocket, GatewayEvent::decode)? {
            GatewayEvent::Dispatch(seq, Event::Ready(event)) => {
                sequence = seq;
                ready = event;
            }
            GatewayEvent::InvalidateSession => {
                todo!("invalidate session")
            }
            other => {
                panic!("unknown response: {:?}", other)
            }
        }

        let session_id = ready.session_id.clone();

        Ok((
            Self {
                keepalive_channel: sender,
                websocket,
                token,
                session_id: Some(session_id),
                last_sequence: sequence,
                identify,
                user_id: UserId(0), // ready.user.id,
                ws_url: url.to_string(),
            },
            ready,
        ))
    }
}

fn keepalive(
    interval: usize,
    websocket: Arc<Mutex<WebSocket<MaybeTlsStream<TcpStream>>>>,
    channel: crossbeam_channel::Receiver<Status>,
) {
    let mut tick_len = std::time::Duration::from_millis(interval as u64);
    let mut next_tick_at = std::time::Instant::now() + tick_len;
    let mut last_sequence = 0;

    'outer: loop {
        std::thread::sleep(std::time::Duration::from_millis(100));

        loop {
            match channel.try_recv() {
                Ok(Status::SendMessage(val)) => match websocket
                    .lock()
                    .unwrap()
                    .send(tungstenite::Message::Text(val.to_string()))
                {
                    Ok(_) => {}
                    Err(e) => {
                        println!("{:?}", e)
                    }
                },
                Ok(Status::Sequence(seq)) => last_sequence = seq,
                Ok(Status::ChangeInterval(interval)) => {
                    tick_len = std::time::Duration::from_millis(interval as u64);
                    next_tick_at = std::time::Instant::now() + tick_len;
                }
                Ok(Status::Aborted) => break 'outer,
                Err(crossbeam_channel::TryRecvError::Empty) => break,
                Err(crossbeam_channel::TryRecvError::Disconnected) => break 'outer,
            }
        }

        if std::time::Instant::now() >= next_tick_at {
            next_tick_at = std::time::Instant::now() + tick_len;
            let map = ureq::json!({
                "op": 1,
                "d": last_sequence
            });
            websocket
                .lock()
                .unwrap()
                .send(tungstenite::Message::Text(map.to_string()))
                .expect("unable to send message to websocket");
        }
    }
}

enum Status {
    SendMessage(ureq::serde_json::Value),
    Sequence(usize),
    ChangeInterval(u64),
    Aborted,
}
