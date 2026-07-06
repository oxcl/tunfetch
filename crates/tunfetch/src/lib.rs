pub mod core;

use wasm_bindgen::prelude::*;
use worker::Socket;
use hyper::client::conn::http1;
use hyper_util::rt::TokioIo;
use http_body_util::Empty;
use hyper::body::Bytes;

#[wasm_bindgen]
pub async fn tunfetch(url: String) -> Result<String, JsValue> {
    let uri: hyper::Uri = url.parse().map_err(|e: hyper::http::uri::InvalidUri| {
        JsValue::from_str(&e.to_string())
    })?;

    let host = uri
        .host()
        .ok_or_else(|| JsValue::from_str("URL has no host"))?
        .to_string();
    let port = uri.port_u16().unwrap_or(80);
    let path = uri.path();

    // 1. Open the TCP socket via workers-rs
    let socket = Socket::builder()
        .connect(&host, port)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let io = TokioIo::new(socket);

    // 2. Hyper handshake
    let (mut sender, conn) = http1::handshake(io)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // 3. Drive the connection on the Worker's event loop
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = conn.await {
            web_sys::console::log_1(&format!("connection error: {e:?}").into());
        }
    });

    // 4. Build and send the request
    let req = hyper::Request::builder()
        .uri(path)
        .header("Host", host)
        .body(Empty::<Bytes>::new())
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let resp = sender
        .send_request(req)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(format!("Got status: {}", resp.status()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}