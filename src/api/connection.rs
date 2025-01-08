use std::{
    net::TcpStream,
    sync::{mpsc, Arc, Mutex},
};

use anyhow::{Error, Result};
use ewebsock::Options;
use secrecy::{ExposeSecret, SecretString};
use serde_json::Value;

use crate::api::model::{receive_json, Event, GatewayEvent};

use super::model::{ReadyEvent, UserId};

pub struct Connection {
    keepalive_sender: mpsc::Sender<Status>,
    ws_receiver: ewebsock::WsReceiver,
    token: SecretString,
    session_id: Option<String>,
    last_sequence: usize,
    identify: Value,
    user_id: UserId,
    ws_url: String,
    // voice
}

impl Connection {
    pub fn new(url: &str, token: SecretString) -> Result<(Self, ReadyEvent)> {
        let d = serde_json::json!({
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
        let identify = serde_json::json!({
            "op": 2,
            "d": d,
        });
        let (mut ws_sender, mut ws_receiver) = ewebsock::connect(url, Options::default()).unwrap();
        {
            let mut a = ws_receiver.try_recv();
            while a.is_none() {
                a = ws_receiver.try_recv();
            }
            match a.as_ref().unwrap() {
                ewebsock::WsEvent::Opened => {}
                other => {
                    eprintln!("{:?}", other);
                }
            }
        }

        ws_sender.send(ewebsock::WsMessage::Text(identify.to_string()));

        // get heartbeat
        let heartbeat_interval;
        match receive_json(&mut ws_receiver, GatewayEvent::decode)? {
            GatewayEvent::Hello(interval) => heartbeat_interval = interval,
            _ => return Err(Error::msg("expected hello during handshake")),
        }

        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("Discord Websocket Keepalive".to_string())
            .spawn(move || keepalive(heartbeat_interval, ws_sender, receiver))?;

        let sequence;
        let ready;
        match receive_json(&mut ws_receiver, GatewayEvent::decode)? {
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
        println!("{:?}", ready.clone());

        Ok((
            Self {
                keepalive_sender: sender.clone(),
                ws_receiver,
                token,
                session_id: Some(session_id),
                last_sequence: sequence,
                identify,
                user_id: ready.user.id,
                ws_url: url.to_string(),
            },
            ready,
        ))
    }
}

fn keepalive(interval: usize, mut ws_sender: ewebsock::WsSender, channel: mpsc::Receiver<Status>) {
    let mut tick_len = std::time::Duration::from_millis(interval as u64);
    let mut next_tick_at = std::time::Instant::now() + tick_len;
    let mut last_sequence = 0;

    'outer: loop {
        std::thread::sleep(std::time::Duration::from_millis(100));

        loop {
            match channel.try_recv() {
                Ok(Status::SendMessage(val)) => {
                    ws_sender.send(ewebsock::WsMessage::Text(val.to_string()))
                }
                Ok(Status::Sequence(seq)) => last_sequence = seq,
                Ok(Status::ChangeInterval(interval)) => {
                    tick_len = std::time::Duration::from_millis(interval as u64);
                    next_tick_at = std::time::Instant::now() + tick_len;
                }
                Ok(Status::Aborted) => break 'outer,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    println!("heartbeat disconnected");
                    break 'outer;
                }
            }
        }

        if std::time::Instant::now() >= next_tick_at {
            next_tick_at = std::time::Instant::now() + tick_len;
            let map = serde_json::json!({
                "op": 1,
                "d": last_sequence
            });
            println!("heartbeat");
            ws_sender.send(ewebsock::WsMessage::Text(map.to_string()));
        }
    }
    println!("heartbeat_end");
}

enum Status {
    SendMessage(Value),
    Sequence(usize),
    ChangeInterval(u64),
    Aborted,
}
