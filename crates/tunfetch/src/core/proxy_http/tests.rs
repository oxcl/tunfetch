use super::*;
use crate::core::test_utils::MockServer;
use crate::core::proxy::Proxy;

fn http_response(status: u16) -> Vec<u8> {
    let status_line = match status {
        200 => "HTTP/1.1 200 Connection established\r\n",
        407 => "HTTP/1.1 407 Proxy Authentication Required\r\n",
        502 => "HTTP/1.1 502 Bad Gateway\r\n",
        _ => "HTTP/1.1 400 Bad Request\r\n",
    };
    format!("{status_line}\r\n").into_bytes()
}

// -----------------------------------------------------------------------
// Basic CONNECT through HTTP proxy
// -----------------------------------------------------------------------

#[tokio::test]
async fn http_connect_basic() {
    let proxy = Proxy::from_url("http://proxy.example.com:8080").unwrap();
    let (client, mut server) = tokio::io::duplex(2048);

    let mut mock = MockServer::new(vec![http_response(200)]);

    let server_task = tokio::spawn(async move {
        mock.run(&mut server).await;
        mock
    });

    let mut tunnel = HttpTunnel::new(
        client,
        proxy,
        "target.example.com".to_string(),
        443,
    );
    tunnel.connect().await.unwrap();

    let mock = server_task.await.unwrap();
    let data = mock.client_data();
    let request = std::str::from_utf8(data).unwrap();

    // Should send CONNECT method targeting the remote host
    assert!(request.starts_with("CONNECT target.example.com:443 HTTP/1.1\r\n"));
    // Should include Host header
    assert!(request.contains("Host: target.example.com:443\r\n"));
    // Should end with double CRLF
    assert!(request.ends_with("\r\n\r\n"));
}

// -----------------------------------------------------------------------
// CONNECT with proxy auth (Basic)
// -----------------------------------------------------------------------

#[tokio::test]
async fn http_connect_with_basic_auth() {
    let proxy =
        Proxy::from_url("http://user:pass@proxy.example.com:8080").unwrap();
    let (client, mut server) = tokio::io::duplex(2048);

    let mut mock = MockServer::new(vec![http_response(200)]);

    let server_task = tokio::spawn(async move {
        mock.run(&mut server).await;
        mock
    });

    let mut tunnel = HttpTunnel::new(
        client,
        proxy,
        "target.example.com".to_string(),
        443,
    );
    tunnel.connect().await.unwrap();

    let mock = server_task.await.unwrap();
    let data = mock.client_data();
    let request = std::str::from_utf8(data).unwrap();

    // Should include Proxy-Authorization header
    assert!(request.contains("Proxy-Authorization: Basic "));
    // Verify the base64 encoding of "user:pass"
    // user:pass = dXNlcjpwYXNz in base64
    assert!(request.contains("Proxy-Authorization: Basic dXNlcjpwYXNz\r\n"));
}

// -----------------------------------------------------------------------
// CONNECT failure (non-200 status)
// -----------------------------------------------------------------------

#[tokio::test]
async fn http_connect_failure_407() {
    let proxy = Proxy::from_url("http://proxy.example.com:8080").unwrap();
    let (client, mut server) = tokio::io::duplex(2048);

    let mut mock = MockServer::new(vec![http_response(407)]);

    let _server_task = tokio::spawn(async move {
        mock.run(&mut server).await;
        mock
    });

    let mut tunnel = HttpTunnel::new(
        client,
        proxy,
        "target.example.com".to_string(),
        443,
    );
    let err = tunnel.connect().await.unwrap_err();
    assert!(matches!(err, HttpTunnelError::ConnectFailed(407, _)));
}

#[tokio::test]
async fn http_connect_failure_502() {
    let proxy = Proxy::from_url("http://proxy.example.com:8080").unwrap();
    let (client, mut server) = tokio::io::duplex(2048);

    let mut mock = MockServer::new(vec![http_response(502)]);

    let _server_task = tokio::spawn(async move {
        mock.run(&mut server).await;
        mock
    });

    let mut tunnel = HttpTunnel::new(
        client,
        proxy,
        "target.example.com".to_string(),
        443,
    );
    let err = tunnel.connect().await.unwrap_err();
    assert!(matches!(err, HttpTunnelError::ConnectFailed(502, _)));
}

// -----------------------------------------------------------------------
// into_inner returns the stream
// -----------------------------------------------------------------------

#[tokio::test]
async fn into_inner_returns_stream() {
    let proxy = Proxy::from_url("http://proxy.example.com:8080").unwrap();
    let (client, _server) = tokio::io::duplex(1024);
    let tunnel = HttpTunnel::new(
        client,
        proxy,
        "target.example.com".to_string(),
        443,
    );

    let _stream = tunnel.into_inner();
}
