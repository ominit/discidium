use std::{
    cell::RefCell,
    net::TcpStream,
    rc::Rc,
    sync::{mpsc, Arc, Mutex},
};

use anyhow::{Error, Result};
use ewebsock::{connect, Options, WsEvent, WsMessage, WsReceiver, WsSender};
use futures::{
    channel::mpsc::{Receiver, Sender, UnboundedReceiver, UnboundedSender},
    StreamExt,
};
use secrecy::{ExposeSecret, SecretString};
use serde_json::Value;
use time::Time;
use web_sys::console::time;
use yew::platform::{spawn_local, time::sleep};

use crate::api::model::{receive_json, Event, GatewayEvent};

use super::model::{ReadyEvent, UserId};

pub struct Connection {
    ws_sender: UnboundedSender<Status>,
    ws_receiver: UnboundedReceiver<GatewayEvent>,
    token: SecretString,
    session_id: Option<String>,
    last_sequence: usize,
    identify: Value,
    user_id: UserId,
    ws_url: String,
    // voice
}

impl Connection {
    pub async fn new(url: &str, token: SecretString) -> Result<(Self, ReadyEvent)> {
        let d = serde_json::json!({
            "token": token.expose_secret(),
            "properties": {
                "$os": std::env::consts::OS,
                "$browser": "discidium",
                "$device": "discidium",
            },
            "large_threshold": 250,
            "compress": true,
        });
        let identify = serde_json::json!({
            "op": 2, // IDENTIFY
            "d": d,
        });
        // let d = serde_json::json!({
        //     "seq": "",
        //     "session_id": "",
        //     "token": token.expose_secret(),
        // });
        // let identify = serde_json::json!({
        //     "op": 6, // RESUME
        //     "d": d,
        // });

        let (ws_sender, to_ws_receiver) = futures::channel::mpsc::unbounded();
        let (to_ws_sender, mut ws_receiver) = futures::channel::mpsc::unbounded();
        spawn_local(keepalive(
            url.to_string(),
            identify.clone(),
            to_ws_sender,
            to_ws_receiver,
        ));

        let sequence;
        let ready;
        match ws_receiver.next().await.unwrap() {
            GatewayEvent::Dispatch(seq, Event::Ready(event)) => {
                sequence = seq;
                ready = event;
            }
            GatewayEvent::InvalidateSession => {
                web_sys::console::log_1(&format!("invalidate session").into());
                todo!("invalidate session")
            }
            other => {
                web_sys::console::log_1(&format!("unknown response: {:?}", other).into());
                panic!("unknown response: {:?}", other)
            }
        }
        web_sys::console::log_1(&format!("a").into());

        let session_id = ready.session_id.clone();
        web_sys::console::log_1(&format!("{:?}", ready.clone()).into());
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
    sender: UnboundedSender<GatewayEvent>,
    mut receiver: UnboundedReceiver<Status>,
) {
    let (ws_sender, mut ws_receiver) = connect(url, Options::default()).unwrap();
    let ws_sender = Rc::new(RefCell::new(ws_sender));
    {
        let mut a = ws_receiver.try_recv();
        while a.is_none() {
            a = ws_receiver.try_recv();
            sleep(std::time::Duration::from_millis(100)).await;
        }
        match a.as_ref().unwrap() {
            WsEvent::Opened => {}
            other => {
                web_sys::console::log_1(&format!("{:?}", other).into());
                eprintln!("{:?}", other);
            }
        }
    }

    ws_sender
        .clone()
        .borrow_mut()
        .send(WsMessage::Text(identify.to_string()));

    // get heartbeat
    let mut interval;
    match receive_json(&mut ws_receiver, GatewayEvent::decode)
        .await
        .unwrap()
    {
        GatewayEvent::Hello(heartbeat_interval) => {
            interval = heartbeat_interval;
        }
        _ => panic!("expected hello during handshake"),
    }

    let mut next_tick_at = time::OffsetDateTime::now_utc().time().millisecond() + interval as u16;
    let mut last_sequence = 0;

    spawn_local(async move {
        loop {
            match receive_json(&mut ws_receiver, GatewayEvent::decode).await {
                Ok(event) => {
                    sender.unbounded_send(event).unwrap();
                }
                Err(_) => {}
            }
            sleep(std::time::Duration::from_millis(100)).await;
        }
    });

    'outer: loop {
        sleep(std::time::Duration::from_millis(100)).await;

        loop {
            match receiver.try_next() {
                Ok(Some(Status::SendMessage(val))) => ws_sender
                    .clone()
                    .borrow_mut()
                    .send(WsMessage::Text(val.to_string())),
                Ok(Some(Status::Sequence(seq))) => last_sequence = seq,
                Ok(Some(Status::ChangeInterval(new_interval))) => {
                    interval = new_interval;
                    next_tick_at =
                        time::OffsetDateTime::now_utc().time().millisecond() + interval as u16;
                }
                Ok(Some(Status::Aborted)) => break 'outer,
                Ok(None) => break 'outer,
                Err(_) => break,
            }
        }

        if time::OffsetDateTime::now_utc().time().millisecond() >= next_tick_at {
            next_tick_at = time::OffsetDateTime::now_utc().time().millisecond() + interval as u16;
            let map = serde_json::json!({
                "op": 1,
                "d": last_sequence
            });
            println!("heartbeat");
            ws_sender
                .clone()
                .borrow_mut()
                .send(WsMessage::Text(map.to_string()));
        }
    }
}

enum Status {
    SendMessage(Value),
    Sequence(usize),
    ChangeInterval(usize),
    Aborted,
}
