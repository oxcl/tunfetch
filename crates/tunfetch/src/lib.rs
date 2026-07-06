pub mod core;

use wasm_bindgen::prelude::*;
use worker::Socket;
use hyper::client::conn::http1;
use hyper_util::rt::TokioIo;
use http_body_util::Empty;
use hyper::body::Bytes;
use core::proxy::Proxy;

#[wasm_bindgen]
pub async fn tunfetch(url: String, opts: JsValue) -> Result<String, JsValue> {
    let uri: hyper::Uri = url.parse().map_err(|e: hyper::http::uri::InvalidUri| {
        JsValue::from_str(&e.to_string())
    })?;

    let target_host = uri
        .host()
        .ok_or_else(|| JsValue::from_str("URL has no host"))?
        .to_string();
    let target_port = uri.port_u16().unwrap_or(443);
    let path = uri.path();

    // Parse proxy option from opts
    let proxy: Option<Proxy> = if let Some(opts_obj) = js_sys::Object::try_from(&opts) {
        if let Ok(proxy_js) = js_sys::Reflect::get(opts_obj, &"proxy".into()) {
            if proxy_js.is_string() {
                let proxy_str = proxy_js.as_string().unwrap();
                if !proxy_str.is_empty() {
                    Some(Proxy::from_url(&proxy_str).map_err(|e| {
                        JsValue::from_str(&e.to_string())
                    })?)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // Determine connection target: proxy or direct
    let (connect_host, connect_port) = if let Some(ref p) = proxy {
        (p.host.clone(), p.port)
    } else {
        (target_host.clone(), target_port)
    };

    // 1. Open the TCP socket via workers-rs
    let socket = Socket::builder()
        .connect(&connect_host, connect_port)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // 2. If proxy is set, establish the tunnel handshake (uses tokio AsyncRead/Write)
    let socket = if let Some(ref p) = proxy {
        core::connect::connect_through_proxy(
            socket,
            p,
            target_host.clone(),
            target_port,
        )
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?
    } else {
        socket
    };

    // 3. Wrap for hyper (converts tokio traits -> hyper traits)
    let io = TokioIo::new(socket);

    // 4. Hyper handshake
    let (mut sender, conn) = http1::handshake(io)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // 5. Drive the connection on the Worker's event loop
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = conn.await {
            web_sys::console::log_1(&format!("connection error: {e:?}").into());
        }
    });

    // 6. Build and send the request
    let req = hyper::Request::builder()
        .uri(path)
        .header("Host", &target_host)
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