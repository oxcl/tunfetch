pub mod net;

use net::request::{build_request, RequestOptions, RedirectMode};
use net::proxy::Proxy;
use net::redirect::{is_redirect, extract_redirect_url, should_change_method, detect_loop, RedirectError};
use net::streaming::create_streaming_response;

use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::client::conn::http1;
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use wasm_bindgen::prelude::*;
use worker::Socket;

#[derive(Debug)]
enum TunfetchError {
    Request(net::request::RequestError),
    Proxy(net::proxy::ProxyError),
    Connect(net::connect::ConnectError),
    Redirect(RedirectError),
    Hyper(hyper::Error),
    Js(JsValue),
}

impl std::fmt::Display for TunfetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TunfetchError::Request(e) => write!(f, "{e}"),
            TunfetchError::Proxy(e) => write!(f, "{e}"),
            TunfetchError::Connect(e) => write!(f, "{e}"),
            TunfetchError::Redirect(e) => write!(f, "{e}"),
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

        // Parse body (string or Uint8Array)
        if let Ok(body_js) = js_sys::Reflect::get(opts_obj, &"body".into()) {
            if let Some(body_str) = body_js.as_string() {
                request_opts = request_opts.with_body(body_str.into_bytes());
            } else {
                let body_arr = js_sys::Uint8Array::try_from(body_js)
                    .map_err(|_| TunfetchError::Js(JsValue::from_str("body must be a string or Uint8Array")))?;
                request_opts = request_opts.with_body(body_arr.to_vec());
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

        // Parse redirect
        if let Ok(redirect_js) = js_sys::Reflect::get(opts_obj, &"redirect".into()) {
            if let Some(redirect_str) = redirect_js.as_string() {
                match redirect_str.as_str() {
                    "manual" => {
                        request_opts = request_opts.with_redirect(RedirectMode::Manual);
                    }
                    "follow" | _ => {
                        request_opts = request_opts.with_redirect(RedirectMode::Follow);
                    }
                }
            }
        }
    }

    Ok((request_opts, proxy))
}

/// Make a single HTTP request and return the response.
///
/// This is the core request logic without redirect handling.
/// Returns (status, headers, body, url) where body is the raw Incoming stream.
async fn make_request(
    url: &str,
    request_opts: &RequestOptions,
    proxy: &Option<Proxy>,
) -> Result<(
    u16,
    http::HeaderMap,
    Incoming,
    String,
), TunfetchError> {
    // 1. Parse URL
    let uri: hyper::Uri = url.parse().map_err(|e: hyper::http::uri::InvalidUri| {
        TunfetchError::Js(JsValue::from_str(&e.to_string()))
    })?;

    let target_host = uri
        .host()
        .ok_or_else(|| TunfetchError::Js(JsValue::from_str("URL has no host")))?
        .to_string();
    let target_port = uri.port_u16().unwrap_or(80);

    // 2. Build the request
    let request = build_request(url, request_opts).map_err(TunfetchError::Request)?;

    // 3. Determine connection target
    let (connect_host, connect_port) = if let Some(ref p) = proxy {
        (p.host.clone(), p.port)
    } else {
        (target_host.clone(), target_port)
    };

    // 4. Open TCP socket
    let socket = Socket::builder()
        .connect(&connect_host, connect_port)
        .map_err(|e| TunfetchError::Js(JsValue::from_str(&e.to_string())))?;

    // 5. Proxy tunnel if needed
    let socket = if let Some(ref p) = proxy {
        net::connect::connect_through_proxy(socket, p, target_host, target_port)
            .await
            .map_err(TunfetchError::Connect)?
    } else {
        socket
    };

    // 6. Hyper handshake
    let io = TokioIo::new(socket);
    let (mut sender, conn) = http1::handshake(io)
        .await
        .map_err(|e| TunfetchError::Hyper(e))?;

    // 7. Drive connection
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = conn.await {
            web_sys::console::log_1(&format!("connection error: {e:?}").into());
        }
    });

    // 8. Send request
    let response = sender
        .send_request(request)
        .await
        .map_err(|e| TunfetchError::Hyper(e))?;

    // 9. Extract status, headers, and body
    let status = response.status().as_u16();
    let response_headers = response.headers().clone();
    let body = response.into_body();

    Ok((status, response_headers, body, url.to_string()))
}

/// Buffer the entire body and return it as Bytes.
///
/// This is used for redirect responses where we need to read headers.
async fn buffer_body(body: Incoming) -> Result<Bytes, TunfetchError> {
    body.collect()
        .await
        .map_err(TunfetchError::Hyper)
        .map(|collected| collected.to_bytes())
}

/// Build a web_sys::Response with headers and status.
fn build_response_init(status: u16, headers: &http::HeaderMap) -> Result<web_sys::ResponseInit, TunfetchError> {
    let response_init = web_sys::ResponseInit::new();
    response_init.set_status(status);

    let response_headers_js = web_sys::Headers::new()
        .map_err(|e| TunfetchError::Js(e))?;
    for (key, value) in headers.iter() {
        response_headers_js
            .set(key.as_str(), value.to_str().unwrap_or(""))
            .map_err(|e| TunfetchError::Js(e))?;
    }
    response_init.set_headers(&response_headers_js);

    Ok(response_init)
}

#[wasm_bindgen]
pub async fn tunfetch(url: String, opts: JsValue) -> Result<JsValue, JsValue> {
    // 1. Parse options
    let (request_opts, proxy) = parse_opts(&opts)?;

    let mut current_url = url.clone();
    let mut current_opts = request_opts;
    let mut visited_urls: Vec<String> = Vec::new();

    // 2. Follow redirects (unless redirect: "manual")
    loop {
        // Make the request
        let (status, headers, body, final_url) = make_request(
            &current_url,
            &current_opts,
            &proxy,
        ).await?;

        // Check if this is a redirect
        let status_code = http::StatusCode::from_u16(status)
            .map_err(|e| TunfetchError::Js(JsValue::from_str(&e.to_string())))?;

        // If redirect mode is manual, return the redirect response without following
        if is_redirect(status_code) && current_opts.redirect == RedirectMode::Manual {
            // Buffer body to read any content (though we ignore it for redirects)
            let body_bytes = buffer_body(body).await?;

            let response_init = build_response_init(status, &headers)?;
            let js_response = web_sys::Response::new_with_opt_js_u8_array_and_init(
                Some(&js_sys::Uint8Array::from(&body_bytes[..])),
                &response_init,
            )
            .map_err(|e| TunfetchError::Js(e))?;

            return Ok(JsValue::from(js_response));
        }

        if !is_redirect(status_code) {
            // Not a redirect - build and return the streaming response
            let response_init = build_response_init(status, &headers)?;

            // Null body for status codes that prohibit body (1xx, 204, 304)
            let js_response = if status == 204 || status == 304 || (status >= 100 && status < 200) {
                web_sys::Response::new_with_opt_u8_array_and_init(None, &response_init)
            } else {
                // Create streaming response
                let (stream, body_reader) = create_streaming_response(body)
                    .map_err(|e| TunfetchError::Js(e))?;

                // Spawn the body reader task
                wasm_bindgen_futures::spawn_local(body_reader);

                // Create Response with ReadableStream body
                web_sys::Response::new_with_opt_readable_stream_and_init(
                    Some(&stream),
                    &response_init,
                )
            }
            .map_err(|e| TunfetchError::Js(e))?;

            return Ok(JsValue::from(js_response));
        }

        // It's a redirect - buffer body to read headers
        let _body_bytes = buffer_body(body).await?;

        // Get the Location header
        let location = headers
            .get("location")
            .or_else(|| headers.get("Location"))
            .ok_or_else(|| TunfetchError::Redirect(RedirectError::MissingLocation))?
            .to_str()
            .map_err(|e| TunfetchError::Js(JsValue::from_str(&e.to_string())))?;

        // Extract the redirect URL
        let redirect_url = extract_redirect_url(location, &final_url)
            .map_err(TunfetchError::Redirect)?;

        // Check for redirect loops
        if detect_loop(&visited_urls, &redirect_url) {
            return Err(TunfetchError::Redirect(RedirectError::LoopDetected).into());
        }

        // Track this URL
        visited_urls.push(final_url);

        // Check redirect limit
        if visited_urls.len() >= 20 {
            return Err(TunfetchError::Redirect(RedirectError::TooManyRedirects).into());
        }

        // For 303, change method to GET for POST/PUT/PATCH
        if should_change_method(status_code, &current_opts.method) {
            current_opts = net::request::RequestOptions::new();
        }

        current_url = redirect_url;
    }
}
