use worker::*;
use hyper::client::conn::http1;
use hyper_util::rt::TokioIo;
use http_body_util::Empty;
use hyper::body::Bytes;

async fn fetch(
    _req: Request,
    _env: Env,
    _ctx: Context,
) -> Result<Response> {

    // 1. Open the TCP socket via workers-rs
    let socket = Socket::builder()
        .connect("example.com", 80)
        .map_err(|e| Error::from(e.to_string()))?;

    let io = TokioIo::new(socket);

    // 3. Hyper handshake
    let (mut sender, conn) = http1::handshake(io)
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    // 4. Drive the connection on the Worker's event loop
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = conn.await {
            console_log!("connection error: {:?}", e);
        }
    });

    // 5. Build and send the request
    let req = hyper::Request::builder()
        .uri("/")
        .header("Host", "example.com")
        .body(Empty::<Bytes>::new())
        .map_err(|e| Error::from(e.to_string()))?;

    let resp = sender
        .send_request(req)
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    console_log!("status: {}", resp.status());

    Response::ok(format!("Got status: {}", resp.status()))
}