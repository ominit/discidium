/// A web-socket message.
#[derive(Clone, Debug)]
pub enum WsMessage {
    /// Binary message.
    Binary(Vec<u8>),

    /// Text message.
    Text(String),

    /// Incoming message of unknown type.
    /// You cannot send these.
    Unknown(String),

    /// Only for native.
    Ping(Vec<u8>),

    /// Only for native.
    Pong(Vec<u8>),
}

/// Something happening with the connection.
#[derive(Clone, Debug)]
pub enum WsEvent {
    /// The connection has been established, and you can start sending messages.
    Opened,

    /// A message has been received.
    Message(WsMessage),

    /// An error occurred.
    Error(String),

    /// The connection has been closed.
    Closed,
}

/// Receiver for incoming [`WsEvent`]s.
pub struct WsReceiver {
    rx: std::sync::mpsc::Receiver<WsEvent>,
}

impl WsReceiver {
    /// Returns a receiver and an event-handler that can be passed to `crate::ws_connect`.
    pub fn new() -> (Self, EventHandler) {
        Self::new_with_callback(|| {})
    }

    /// The given callback will be called on each new message.
    ///
    /// This can be used to wake up the UI thread.
    pub fn new_with_callback(wake_up: impl Fn() + Send + Sync + 'static) -> (Self, EventHandler) {
        let (tx, rx) = std::sync::mpsc::channel();
        let on_event = Box::new(move |event| {
            wake_up(); // wake up UI thread
            if tx.send(event).is_ok() {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            }
        });
        let ws_receiver = Self { rx };
        (ws_receiver, on_event)
    }

    /// Try receiving a new event without blocking.
    pub fn try_recv(&self) -> Option<WsEvent> {
        self.rx.try_recv().ok()
    }
}

/// An error.
pub type Error = String;

/// Short for `Result<T, ewebsock::Error>`.
pub type Result<T> = std::result::Result<T, Error>;

pub(crate) type EventHandler = Box<dyn Send + Fn(WsEvent) -> ControlFlow<()>>;

/// Options for a connection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    /// The maximum size of a single incoming message frame, in bytes.
    ///
    /// The primary reason for setting this to something other than [`usize::MAX`] is
    /// to prevent a malicious server from eating up all your RAM.
    ///
    /// Ignored on Web.
    pub max_incoming_frame_size: usize,

    /// Additional Request headers.
    ///
    /// Currently only supported on native.
    pub additional_headers: Vec<(String, String)>,

    /// Additional subprotocols.
    ///
    /// <https://www.iana.org/assignments/websocket/websocket.xml>
    /// <https://developer.mozilla.org/en-US/docs/Web/API/WebSockets_API/Writing_WebSocket_servers#miscellaneous>
    ///
    /// Currently only supported on native.
    pub subprotocols: Vec<String>,

    /// Socket read timeout.
    ///
    /// Reads will block forever if this is set to `None` or `Some(Duration::ZERO)`.
    ///
    /// Defaults to 10ms.
    pub read_timeout: Option<std::time::Duration>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            max_incoming_frame_size: 64 * 1024 * 1024,
            additional_headers: vec![],
            subprotocols: vec![],
            // let the OS schedule something else, otherwise busy-loop
            // TODO: use polling on native instead
            read_timeout: Some(std::time::Duration::from_millis(10)),
        }
    }
}

/// Connect to the given URL, and return a sender and receiver.
///
/// If `on_event` returns [`ControlFlow::Break`], the connection will be closed
/// without calling `on_event` again.
///
/// This is a wrapper around [`ws_connect`].
///
/// # Errors
/// * On native: failure to spawn a thread.
/// * On web: failure to use `WebSocket` API.
///
/// See also the [`connect_with_wakeup`] function,
/// and the more advanced [`ws_connect`].
pub fn connect(url: impl Into<String>, options: Options) -> Result<(WsSender, WsReceiver)> {
    let (ws_receiver, on_event) = WsReceiver::new();
    let ws_sender = ws_connect(url.into(), options, on_event)?;
    Ok((ws_sender, ws_receiver))
}

/// Like [`connect`], but will call the given wake-up function on each incoming event.
///
/// This allows you to wake up the UI thread, for instance.
///
/// If `on_event` returns [`ControlFlow::Break`], the connection will be closed
/// without calling `on_event` again.
///
/// This is a wrapper around [`ws_connect`].
///
/// # Errors
/// * On native: failure to spawn a thread.
/// * On web: failure to use `WebSocket` API.
///
/// Note that you have to wait for [`WsEvent::Opened`] before sending messages.
pub fn connect_with_wakeup(
    url: impl Into<String>,
    options: Options,
    wake_up: impl Fn() + Send + Sync + 'static,
) -> Result<(WsSender, WsReceiver)> {
    let (receiver, on_event) = WsReceiver::new_with_callback(wake_up);
    let sender = ws_connect(url.into(), options, on_event)?;
    Ok((sender, receiver))
}

/// Connect and call the given event handler on each received event.
///
/// If `on_event` returns [`ControlFlow::Break`], the connection will be closed
/// without calling `on_event` again.
///
/// See [`crate::connect`] for a more high-level version.
///
/// # Errors
/// * On native: failure to spawn a thread.
/// * On web: failure to use `WebSocket` API.
pub fn ws_connect(url: String, options: Options, on_event: EventHandler) -> Result<WsSender> {
    ws_connect_impl(url, options, on_event)
}

/// Connect and call the given event handler on each received event.
///
/// This is like [`ws_connect`], but it doesn't return a [`WsSender`],
/// so it can only receive messages, not send them.
///
/// This can be slightly more efficient when you don't need to send messages.
///
/// If `on_event` returns [`ControlFlow::Break`], the connection will be closed
/// without calling `on_event` again.
///
/// # Errors
/// * On native: failure to spawn receiver thread.
/// * On web: failure to use `WebSocket` API.
pub fn ws_receive(url: String, options: Options, on_event: EventHandler) -> Result<()> {
    ws_receive_impl(url, options, on_event)
}

use std::{
    ops::ControlFlow,
    rc::Rc,
    sync::{Arc, Mutex},
};

#[allow(clippy::needless_pass_by_value)]
fn string_from_js_value(s: wasm_bindgen::JsValue) -> String {
    s.as_string().unwrap_or(format!("{s:#?}"))
}

#[allow(clippy::needless_pass_by_value)]
fn string_from_js_string(s: js_sys::JsString) -> String {
    s.as_string().unwrap_or(format!("{s:#?}"))
}

unsafe impl Send for WsSender {}

/// This is how you send messages to the server.
///
/// When this is dropped, the connection is closed.
pub struct WsSender {
    socket: Option<Arc<Mutex<web_sys::WebSocket>>>,
}

impl Drop for WsSender {
    fn drop(&mut self) {
        self.close();
    }
}

impl WsSender {
    /// Send the message to the server.
    pub fn send(&mut self, msg: WsMessage) {
        if let Some(socket) = &mut self.socket {
            let result = match msg {
                WsMessage::Binary(data) => {
                    socket
                        .lock()
                        .unwrap()
                        .set_binary_type(web_sys::BinaryType::Blob);
                    socket.lock().unwrap().send_with_u8_array(&data)
                }
                WsMessage::Text(text) => socket.lock().unwrap().send_with_str(&text),
                unknown => {
                    panic!("Don't know how to send message: {unknown:?}");
                }
            };
            if let Err(err) = result.map_err(string_from_js_value) {
                eprintln!("Failed to send: {err:?}");
            }
        }
    }

    /// Close the connection.
    ///
    /// This is called automatically when the sender is dropped.
    pub fn close(&mut self) {
        if let Some(socket) = self.socket.take() {
            close_socket(&socket);
        }
    }

    /// Forget about this sender without closing the connection.
    pub fn forget(mut self) {
        self.socket = None;
    }
}

pub(crate) fn ws_receive_impl(url: String, options: Options, on_event: EventHandler) -> Result<()> {
    ws_connect_impl(url, options, on_event).map(|sender| sender.forget())
}

#[allow(clippy::needless_pass_by_value)] // For consistency with the native version
pub(crate) fn ws_connect_impl(
    url: String,
    _ignored_options: Options,
    on_event: EventHandler,
) -> Result<WsSender> {
    // Based on https://rustwasm.github.io/wasm-bindgen/examples/websockets.html

    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast as _;

    // Connect to an server
    let socket = web_sys::WebSocket::new(&url).map_err(string_from_js_value)?;
    let socket = Arc::new(Mutex::new(socket));

    // For small binary messages, like CBOR, Arraybuffer is more efficient than Blob handling
    socket
        .lock()
        .unwrap()
        .set_binary_type(web_sys::BinaryType::Arraybuffer);

    // Allow it to be shared by the different callbacks:
    let on_event: Rc<dyn Send + Fn(WsEvent) -> ControlFlow<()>> = on_event.into();

    // onmessage callback
    {
        let on_event = on_event.clone();
        let socket2 = socket.clone();
        let onmessage_callback = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
            // Handle difference Text/Binary,...
            let control = if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let array = js_sys::Uint8Array::new(&abuf);
                on_event(WsEvent::Message(WsMessage::Binary(array.to_vec())))
            } else if let Ok(blob) = e.data().dyn_into::<web_sys::Blob>() {
                // better alternative to juggling with FileReader is to use https://crates.io/crates/gloo-file
                let file_reader = web_sys::FileReader::new().expect("Failed to create FileReader");
                let file_reader_clone = file_reader.clone();
                // create onLoadEnd callback
                let on_event = on_event.clone();
                let socket3 = socket2.clone();
                let onloadend_cb = Closure::wrap(Box::new(move |_e: web_sys::ProgressEvent| {
                    let control = match file_reader_clone.result() {
                        Ok(file_reader) => {
                            let array = js_sys::Uint8Array::new(&file_reader);
                            on_event(WsEvent::Message(WsMessage::Binary(array.to_vec())))
                        }
                        Err(err) => on_event(WsEvent::Error(format!(
                            "Failed to read binary blob: {}",
                            string_from_js_value(err)
                        ))),
                    };
                    if control.is_break() {
                        close_socket(&socket3);
                    }
                })
                    as Box<dyn FnMut(web_sys::ProgressEvent)>);
                file_reader.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
                file_reader
                    .read_as_array_buffer(&blob)
                    .expect("blob not readable");
                onloadend_cb.forget();
                ControlFlow::Continue(())
            } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                on_event(WsEvent::Message(WsMessage::Text(string_from_js_string(
                    txt,
                ))))
            } else {
                println!("Unknown websocket message received: {:?}", e.data());
                on_event(WsEvent::Message(WsMessage::Unknown(string_from_js_value(
                    e.data(),
                ))))
            };
            if control.is_break() {
                close_socket(&socket2);
            }
        }) as Box<dyn FnMut(web_sys::MessageEvent)>);

        // set message event handler on WebSocket
        socket
            .lock()
            .unwrap()
            .set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));

        // forget the callback to keep it alive
        onmessage_callback.forget();
    }

    {
        let on_event = on_event.clone();
        let onerror_callback = Closure::wrap(Box::new(move |error_event: web_sys::ErrorEvent| {
            // using reflect instead of error_event.message() to avoid panic on null
            let message = js_sys::Reflect::get(&error_event, &"message".into()).unwrap_or_default();
            let error = js_sys::Reflect::get(&error_event, &"error".into()).unwrap_or_default();
            eprintln!("error event: {:?}: {:?}", message, error);
            on_event(WsEvent::Error(
                message
                    .as_string()
                    .unwrap_or_else(|| "Unknown error".to_owned()),
            ));
        }) as Box<dyn FnMut(web_sys::ErrorEvent)>);
        socket
            .lock()
            .unwrap()
            .set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();
    }

    {
        let socket2 = socket.clone();
        let on_event = on_event.clone();
        let onopen_callback = Closure::wrap(Box::new(move |_| {
            let control = on_event(WsEvent::Opened);
            if control.is_break() {
                close_socket(&socket2);
            }
        }) as Box<dyn FnMut(wasm_bindgen::JsValue)>);
        socket
            .lock()
            .unwrap()
            .set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();
    }

    {
        let onclose_callback = Closure::wrap(Box::new(move |_| {
            on_event(WsEvent::Closed);
        }) as Box<dyn FnMut(wasm_bindgen::JsValue)>);
        socket
            .lock()
            .unwrap()
            .set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
        onclose_callback.forget();
    }

    Ok(WsSender {
        socket: Some(socket),
    })
}

fn close_socket(socket: &Arc<Mutex<web_sys::WebSocket>>) {
    if let Err(err) = socket.lock().unwrap().close() {
        eprintln!("Failed to close WebSocket: {}", string_from_js_value(err));
    } else {
        println!("Closed WebSocket");
    }
}
