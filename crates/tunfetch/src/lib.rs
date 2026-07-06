pub mod net;

use net::request::{build_request, RequestOptions};
use net::proxy::Proxy;

use http_body_util::BodyExt;
use hyper::client::conn::http1;
use hyper_util::rt::TokioIo;
use wasm_bindgen::prelude::*;
use worker::Socket;

#[derive(Debug)]
enum TunfetchError {
    Request(net::request::RequestError),
    Proxy(net::proxy::ProxyError),
    Connect(net::connect::ConnectError),
    Hyper(hyper::Error),
    Js(JsValue),
}

impl std::fmt::Display for TunfetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TunfetchError::Request(e) => write!(f, "{e}"),
            TunfetchError::Proxy(e) => write!(f, "{e}"),
            TunfetchError::Connect(e) => write!(f, "{e}"),
            TunfetchError::Hyper(e) => write!(f, "hyper error: {e}"),
            TunfetchError::Js(e) => write!(f, "JS error: {:?}", e),
        }
    }
}

impl From<TunfetchError> for JsValue {
    fn from(e: TunfetchError) -> Self {
        JsValue::from_str(&e.to_string())
    }
}

/// Parse the JavaScript options object into RequestOptions and optional Proxy.
fn parse_opts(opts: &JsValue) -> Result<(RequestOptions, Option<Proxy>), TunfetchError> {
    let mut request_opts = RequestOptions::new();
    let mut proxy = None;

    if let Some(opts_obj) = js_sys::Object::try_from(opts) {
        // Parse method
        if let Ok(method_js) = js_sys::Reflect::get(opts_obj, &"method".into()) {
            if let Some(method_str) = method_js.as_string() {
                request_opts = request_opts
                    .with_method(&method_str)
                    .map_err(TunfetchError::Request)?;
            }
        }

        // Parse headers
        if let Ok(headers_js) = js_sys::Reflect::get(opts_obj, &"headers".into()) {
            if let Some(headers_obj) = js_sys::Object::try_from(&headers_js) {
                let entries = js_sys::Object::entries(&headers_obj);
                for entry in entries.iter() {
                    let entry: js_sys::Array = entry.into();
                    let key = entry.get(0).as_string().unwrap_or_default();
                    let value = entry.get(1).as_string().unwrap_or_default();
                    request_opts = request_opts.with_header(&key, &value);
                }
            }
        }

        // Parse body
        if let Ok(body_js) = js_sys::Reflect::get(opts_obj, &"body".into()) {
            if let Some(body_str) = body_js.as_string() {
                request_opts = request_opts.with_body(body_str.into_bytes());
            }
        }

        // Parse proxy
        if let Ok(proxy_js) = js_sys::Reflect::get(opts_obj, &"proxy".into()) {
            if let Some(proxy_str) = proxy_js.as_string() {
                if !proxy_str.is_empty() {
                    proxy = Some(
                        Proxy::from_url(&proxy_str).map_err(TunfetchError::Proxy)?,
                    );
                }
            }
        }
    }

    Ok((request_opts, proxy))
}

#[wasm_bindgen]
pub async fn tunfetch(url: String, opts: JsValue) -> Result<JsValue, JsValue> {
    // 1. Parse options
    let (request_opts, proxy) = parse_opts(&opts)?;

    // 2. Parse URL
    let uri: hyper::Uri = url.parse().map_err(|e: hyper::http::uri::InvalidUri| {
        JsValue::from_str(&e.to_string())
    })?;

    let target_host = uri
        .host()
        .ok_or_else(|| TunfetchError::Js(JsValue::from_str("URL has no host")))?
        .to_string();
    let target_port = uri.port_u16().unwrap_or(80);

    // 3. Build the request
    let request = build_request(&url, &request_opts).map_err(TunfetchError::Request)?;

    // 4. Determine connection target
    let (connect_host, connect_port) = if let Some(ref p) = proxy {
        (p.host.clone(), p.port)
    } else {
        (target_host.clone(), target_port)
    };

    // 6. Open TCP socket
    let socket = Socket::builder()
        .connect(&connect_host, connect_port)
        .map_err(|e| TunfetchError::Js(JsValue::from_str(&e.to_string())))?;

    // 7. Proxy tunnel if needed
    let socket = if let Some(ref p) = proxy {
        net::connect::connect_through_proxy(socket, p, target_host, target_port)
            .await
            .map_err(TunfetchError::Connect)?
    } else {
        socket
    };

    // 8. Hyper handshake
    let io = TokioIo::new(socket);
    let (mut sender, conn) = http1::handshake(io)
        .await
        .map_err(|e| TunfetchError::Hyper(e))?;

    // 9. Drive connection
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = conn.await {
            web_sys::console::log_1(&format!("connection error: {e:?}").into());
        }
    });

    // 10. Send request
    let response = sender
        .send_request(request)
        .await
        .map_err(|e| TunfetchError::Hyper(e))?;

    // 11. Extract status and headers
    let status = response.status().as_u16();
    let response_headers = response.headers().clone();

    // 12. Read body
    let body_bytes = response
        .into_body()
        .collect()
        .await
        .map_err(|e| TunfetchError::Hyper(e))?
        .to_bytes();

    // 13. Build JS Response
    let response_init = web_sys::ResponseInit::new();
    response_init.set_status(status);

    // Add headers
    let headers = web_sys::Headers::new().map_err(|e| TunfetchError::Js(e))?;
    for (key, value) in response_headers.iter() {
        headers
            .set(key.as_str(), value.to_str().unwrap_or(""))
            .map_err(|e| TunfetchError::Js(e))?;
    }
    response_init.set_headers(&headers);

    // Null body for status codes that prohibit body (1xx, 204, 304)
    let js_response = if status == 204 || status == 304 || (status >= 100 && status < 200) {
        web_sys::Response::new_with_opt_u8_array_and_init(None, &response_init)
    } else {
        let js_body = js_sys::Uint8Array::from(&body_bytes[..]);
        web_sys::Response::new_with_opt_js_u8_array_and_init(Some(&js_body), &response_init)
    }
    .map_err(|e| TunfetchError::Js(e))?;

    Ok(JsValue::from(js_response))
}


