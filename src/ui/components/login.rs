use yew::{function_component, html, use_node_ref, use_state, Callback, Html};

#[function_component]
pub fn Login() -> Html {
    let input = use_node_ref();
    let onclick = {
        let input = input.clone();
        move |_| {
            web_sys::console::log_1(&format!("{:?}", input).into());
        }
    };
    html! {
        <body class="bg-gray-100 flex items-center justify-center min-h-screen">
            <div class="bg-white p-8 rounded-lg shadow-lg w-96">
                <h2 class="text-2xl font-semibold text-center text-gray-800 mb-6">{"Login"}</h2>
                <div class="mb-4">
                    <label for="username" class="block text-gray-700">{"Token"}</label>
                    <input
                        ref={input}
                        type="text"
                        id="username"
                        name="username"
                        class="w-full p-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                        required=true
                    />
                </div>
                <div class="flex justify-center mt-6">
                    <button
                        type="submit"
                        class="bg-blue-500 text-white font-bold py-2 px-4 rounded-lg w-full hover:bg-blue-600 focus:ring-2 focus:ring-blue-500"
                        {onclick}>
                        {"Login"}
                    </button>
                </div>
            </div>
        </body>
    }
}
