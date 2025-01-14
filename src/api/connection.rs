use std::{
    net::TcpStream,
    sync::{mpsc, Arc, Mutex},
};

use anyhow::{Error, Result};
use ewebsock::{connect, Options, WsEvent, WsMessage, WsReceiver, WsSender};
use secrecy::{ExposeSecret, SecretString};
use serde_json::Value;
use wasm_bindgen_futures::spawn_local;

use crate::api::model::{receive_json, Event, GatewayEvent};

use super::model::{ReadyEvent, UserId};

pub struct Connection {
    ws_sender: mpsc::Sender<Status>,
    ws_receiver: mpsc::Receiver<GatewayEvent>,
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

        let (ws_sender, to_ws_receiver) = mpsc::channel();
        let (to_ws_sender, ws_receiver) = mpsc::channel();
        spawn_local(keepalive(
            url.to_string(),
            identify.clone(),
            to_ws_sender,
            to_ws_receiver,
        ));

        let sequence;
        let ready;
        match ws_receiver.recv()? {
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
                ws_sender,
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

async fn keepalive(
    url: String,
    identify: Value,
    sender: mpsc::Sender<GatewayEvent>,
    receiver: mpsc::Receiver<Status>,
) {
    let (mut ws_sender, mut ws_receiver) = connect(url, Options::default()).unwrap();
    {
        let mut a = ws_receiver.try_recv();
        while a.is_none() {
            a = ws_receiver.try_recv();
        }
        match a.as_ref().unwrap() {
            WsEvent::Opened => {}
            other => {
                eprintln!("{:?}", other);
            }
        }
    }

    ws_sender.send(WsMessage::Text(identify.to_string()));

    // get heartbeat
    let interval;
    match receive_json(&mut ws_receiver, GatewayEvent::decode).unwrap() {
        GatewayEvent::Hello(heartbeat_interval) => interval = heartbeat_interval,
        _ => panic!("expected hello during handshake"),
    }

    let mut tick_len = std::time::Duration::from_millis(interval as u64);
    let mut next_tick_at = std::time::Instant::now() + tick_len;
    let mut last_sequence = 0;

    'outer: loop {
        std::thread::sleep(std::time::Duration::from_millis(100));

        loop {
            match receiver.try_recv() {
                Ok(Status::SendMessage(val)) => ws_sender.send(WsMessage::Text(val.to_string())),
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

            match receive_json(&mut ws_receiver, GatewayEvent::decode) {
                Ok(event) => sender.send(event).unwrap(),
                Err(_) => {}
            }
        }

        if std::time::Instant::now() >= next_tick_at {
            next_tick_at = std::time::Instant::now() + tick_len;
            let map = serde_json::json!({
                "op": 1,
                "d": last_sequence
            });
            println!("heartbeat");
            ws_sender.send(WsMessage::Text(map.to_string()));
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
