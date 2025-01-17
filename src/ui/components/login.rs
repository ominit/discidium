use std::sync::mpsc::Sender;

use futures::FutureExt;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::{
    function_component, html, platform::pinned::mpsc::UnboundedSender, use_state, Callback, Html,
    InputEvent, UseStateHandle,
};
use yew_autoprops::autoprops;

use crate::ui::{DiscidiumData, Message};

#[autoprops]
#[function_component]
pub fn Login(sender_callback: Callback<Message>) -> Html {
    let input = use_state(|| String::new());
    let oninput = {
        let input = input.clone();
        move |input_event: InputEvent| {
            input.set(
                input_event
                    .target()
                    .unwrap()
                    .dyn_into::<HtmlInputElement>()
                    .unwrap()
                    .value(),
            )
        }
    };
    let onclick = Callback::from(move |_| {
        let input = input.clone();
        let sender_callback = sender_callback.clone();
        spawn_local(async {
            async move {
                let data = DiscidiumData::from_token(input.to_string()).await;
                web_sys::console::log_1(&format!("{:?}", data.is_some()).into());
                if data.is_some() {
                    web_sys::console::log_1(&format!("logged in").into());
                    sender_callback.emit(Message::Login(data.unwrap()));
                    input.set(String::new());
                }
                web_sys::console::log_1(&format!("{:?}", input.to_string()).into());
            }
            .await
        })
    });
    html! {
        <body class="bg-gray-100 flex items-center justify-center min-h-screen">
            <div class="bg-white p-8 rounded-lg shadow-lg w-96">
                <h2 class="text-2xl font-semibold text-center text-gray-800 mb-6">
                    {"Login"}
                </h2>
                <div class="mb-4">
                    <label class="block text-gray-700">
                        {"Token"}
                    </label>
                    <input
                        {oninput}
                        type="text"
                        class="w-full p-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                </div>
                <div class="flex justify-center mt-6">
                    <button
                        class="bg-blue-500 text-white font-bold py-2 px-4 rounded-lg w-full hover:bg-blue-600 focus:ring-2 focus:ring-blue-500"
                        {onclick}
                    >
                        {"Login"}
                    </button>
                </div>
            </div>
        </body>
    }
}
