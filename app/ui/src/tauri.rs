//! The bridge to the native side: `invoke` a command, `listen` for an event.
//! Arguments and results are the shared types in `lattice_core::api`.

use lattice_core::api::ApiError;
use serde::Serialize;
use serde::de::DeserializeOwned;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen, catch)]
    async fn tauri_listen(event: &str, handler: &Closure<dyn FnMut(JsValue)>) -> Result<JsValue, JsValue>;
}

fn to_js<T: Serialize>(value: &T) -> Result<JsValue, ApiError> {
    // Plain JS objects, not Maps, so Tauri reads them as command arguments.
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| ApiError::Server(format!("Could not send the request: {e}")))
}

fn from_js<T: DeserializeOwned>(value: JsValue) -> Result<T, ApiError> {
    serde_wasm_bindgen::from_value(value).map_err(|e| ApiError::Server(format!("Unexpected answer from the app: {e}")))
}

/// Runs a native command. Its `Err` arrives as `{ kind, message }`.
pub async fn invoke<A: Serialize, R: DeserializeOwned>(cmd: &str, args: &A) -> Result<R, ApiError> {
    match tauri_invoke(cmd, to_js(args)?).await {
        Ok(v) => from_js(v),
        Err(e) => Err(serde_wasm_bindgen::from_value::<ApiError>(e.clone()).unwrap_or_else(|_| {
            ApiError::Server(e.as_string().unwrap_or_else(|| format!("The app could not run {cmd}.")))
        })),
    }
}

/// A command without arguments.
pub async fn call<R: DeserializeOwned>(cmd: &str) -> Result<R, ApiError> {
    invoke(cmd, &serde_json::json!({})).await
}

/// Calls `handler` with each event's payload for the life of the app.
pub fn listen<T: DeserializeOwned + 'static>(event: &'static str, mut handler: impl FnMut(T) + 'static) {
    let closure = Closure::<dyn FnMut(JsValue)>::new(move |e: JsValue| {
        let payload = js_sys::Reflect::get(&e, &JsValue::from_str("payload")).unwrap_or(JsValue::NULL);
        if let Ok(value) = serde_wasm_bindgen::from_value::<T>(payload) {
            handler(value);
        }
    });
    wasm_bindgen_futures::spawn_local(async move {
        let _ = tauri_listen(event, &closure).await;
        // Listeners live as long as the window.
        closure.forget();
    });
}
