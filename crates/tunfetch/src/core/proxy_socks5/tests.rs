use super::*;
use crate::core::test_utils::MockServer;
use crate::core::proxy::Proxy;

// -----------------------------------------------------------------------
// SOCKS5 handshake with no auth (method 0x00)
// -----------------------------------------------------------------------

#[tokio::test]
async fn socks5_handshake_no_auth() {
    let proxy = Proxy::from_url("socks5://proxy.example.com:1080").unwrap();
    let (client, mut server) = tokio::io::duplex(1024);

    let mut mock = MockServer::new(vec![
        // Server response: chosen method = 0x00 (no auth)
        vec![0x05, 0x00],
        // Server response to CONNECT: success
        vec![0x05, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ]);

    let server_task = tokio::spawn(async move {
        mock.run(&mut server).await;
        mock
    });

    let mut tunnel = Socks5Tunnel::new(
        client,
        proxy,
        "target.example.com".to_string(),
        443,
    );
    tunnel.connect().await.unwrap();

    let mock = server_task.await.unwrap();
    let data = mock.client_data();

    // Greeting: version=0x05, nmethods=1, method=0x00
    assert_eq!(&data[0..3], &[0x05, 0x01, 0x00]);
    // Connect request: version=0x05, cmd=0x01, rsv=0x00, atyp=0x03(domain),
    // host="target.example.com", port=443
    assert_eq!(data[3], 0x05); // version
    assert_eq!(data[4], 0x01); // cmd = connect
    assert_eq!(data[5], 0x00); // reserved
    assert_eq!(data[6], 0x03); // atyp = domain
    let domain_len = data[7] as usize;
    let domain = std::str::from_utf8(&data[8..8 + domain_len]).unwrap();
    assert_eq!(domain, "target.example.com");
    let port = u16::from_be_bytes([data[8 + domain_len], data[9 + domain_len]]);
    assert_eq!(port, 443);
}

// -----------------------------------------------------------------------
// SOCKS5 handshake with username/password auth (method 0x02)
// -----------------------------------------------------------------------

#[tokio::test]
async fn socks5_handshake_with_auth() {
    let proxy =
        Proxy::from_url("socks5://user:pass@proxy.example.com:1080").unwrap();
    let (client, mut server) = tokio::io::duplex(1024);

    let mut mock = MockServer::new(vec![
        // Server response: chosen method = 0x02 (username/password)
        vec![0x05, 0x02],
        // Server response to auth: success
        vec![0x01, 0x00],
        // Server response to CONNECT: success
        vec![0x05, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ]);

    let server_task = tokio::spawn(async move {
        mock.run(&mut server).await;
        mock
    });

    let mut tunnel = Socks5Tunnel::new(
        client,
        proxy,
        "target.example.com".to_string(),
        443,
    );
    tunnel.connect().await.unwrap();

    let mock = server_task.await.unwrap();
    let data = mock.client_data();

    // Greeting: version=0x05, nmethods=2, methods=[0x00, 0x02]
    assert_eq!(&data[0..4], &[0x05, 0x02, 0x00, 0x02]);

    // Auth request: version=0x01, ulen=4, "user", plen=4, "pass"
    let auth_start = 4;
    assert_eq!(data[auth_start], 0x01); // auth version
    let ulen = data[auth_start + 1] as usize;
    let user = std::str::from_utf8(&data[auth_start + 2..auth_start + 2 + ulen]).unwrap();
    assert_eq!(user, "user");
    let plen = data[auth_start + 2 + ulen] as usize;
    let pass = std::str::from_utf8(
        &data[auth_start + 3 + ulen..auth_start + 3 + ulen + plen],
    )
    .unwrap();
    assert_eq!(pass, "pass");
}

// -----------------------------------------------------------------------
// SOCKS5 rejects invalid version
// -----------------------------------------------------------------------

#[tokio::test]
async fn socks5_invalid_version() {
    let proxy = Proxy::from_url("socks5://proxy.example.com:1080").unwrap();
    let (client, mut server) = tokio::io::duplex(1024);

    let mut mock = MockServer::new(vec![
        // Server sends wrong version
        vec![0x04, 0x00],
    ]);

    let _server_task = tokio::spawn(async move {
        mock.run(&mut server).await;
        mock
    });

    let mut tunnel = Socks5Tunnel::new(
        client,
        proxy,
        "target.example.com".to_string(),
        443,
    );
    let err = tunnel.connect().await.unwrap_err();
    assert!(matches!(err, Socks5Error::InvalidVersion(0x04)));
}

// -----------------------------------------------------------------------
// SOCKS5 no acceptable methods
// -----------------------------------------------------------------------

#[tokio::test]
async fn socks5_no_acceptable_methods() {
    let proxy = Proxy::from_url("socks5://proxy.example.com:1080").unwrap();
    let (client, mut server) = tokio::io::duplex(1024);

    let mut mock = MockServer::new(vec![
        // Server rejects all methods
        vec![0x05, 0xFF],
    ]);

    let _server_task = tokio::spawn(async move {
        mock.run(&mut server).await;
        mock
    });

    let mut tunnel = Socks5Tunnel::new(
        client,
        proxy,
        "target.example.com".to_string(),
        443,
    );
    let err = tunnel.connect().await.unwrap_err();
    assert!(matches!(err, Socks5Error::NoAcceptableMethods));
}

// -----------------------------------------------------------------------
// SOCKS5 auth failure
// -----------------------------------------------------------------------

#[tokio::test]
async fn socks5_auth_failure() {
    let proxy =
        Proxy::from_url("socks5://user:pass@proxy.example.com:1080").unwrap();
    let (client, mut server) = tokio::io::duplex(1024);

    let mut mock = MockServer::new(vec![
        // Server requests auth
        vec![0x05, 0x02],
        // Server rejects auth
        vec![0x01, 0x01],
    ]);

    let _server_task = tokio::spawn(async move {
        mock.run(&mut server).await;
        mock
    });

    let mut tunnel = Socks5Tunnel::new(
        client,
        proxy,
        "target.example.com".to_string(),
        443,
    );
    let err = tunnel.connect().await.unwrap_err();
    assert!(matches!(err, Socks5Error::AuthFailed));
}

// -----------------------------------------------------------------------
// SOCKS5 CONNECT failure
// -----------------------------------------------------------------------

#[tokio::test]
async fn socks5_connect_failure() {
    let proxy = Proxy::from_url("socks5://proxy.example.com:1080").unwrap();
    let (client, mut server) = tokio::io::duplex(1024);

    let mut mock = MockServer::new(vec![
        // Server accepts no auth
        vec![0x05, 0x00],
        // Server rejects CONNECT (connection refused)
        vec![0x05, 0x05, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    ]);

    let _server_task = tokio::spawn(async move {
        mock.run(&mut server).await;
        mock
    });

    let mut tunnel = Socks5Tunnel::new(
        client,
        proxy,
        "target.example.com".to_string(),
        443,
    );
    let err = tunnel.connect().await.unwrap_err();
    assert!(matches!(err, Socks5Error::ConnectFailed(0x05)));
}

// -----------------------------------------------------------------------
// into_inner returns the stream
// -----------------------------------------------------------------------

#[tokio::test]
async fn into_inner_returns_stream() {
    let proxy = Proxy::from_url("socks5://proxy.example.com:1080").unwrap();
    let (client, _server) = tokio::io::duplex(1024);
    let tunnel = Socks5Tunnel::new(
        client,
        proxy,
        "target.example.com".to_string(),
        443,
    );

    // We can get the stream back without consuming it
    let _stream = tunnel.into_inner();
}
