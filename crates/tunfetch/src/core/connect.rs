use super::proxy::{Proxy, ProxyScheme};
use super::proxy_http::HttpTunnel;
use super::proxy_socks5::Socks5Tunnel;

use tokio::io::{AsyncRead, AsyncWrite};

/// Errors that can occur when establishing a proxied connection.
#[derive(Debug)]
pub enum ConnectError {
    Socks5(super::proxy_socks5::Socks5Error),
    Http(super::proxy_http::HttpTunnelError),
}

impl std::fmt::Display for ConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectError::Socks5(e) => write!(f, "{e}"),
            ConnectError::Http(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ConnectError {}

impl From<super::proxy_socks5::Socks5Error> for ConnectError {
    fn from(e: super::proxy_socks5::Socks5Error) -> Self {
        ConnectError::Socks5(e)
    }
}

impl From<super::proxy_http::HttpTunnelError> for ConnectError {
    fn from(e: super::proxy_http::HttpTunnelError) -> Self {
        ConnectError::Http(e)
    }
}

/// Establish a tunnel through the given proxy to the target host and port.
///
/// Returns the stream ready for use with hyper (or any other protocol).
pub async fn connect_through_proxy<S>(
    stream: S,
    proxy: &Proxy,
    target_host: String,
    target_port: u16,
) -> Result<S, ConnectError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    match proxy.scheme {
        ProxyScheme::Socks5 => {
            let mut tunnel =
                Socks5Tunnel::new(stream, proxy.clone(), target_host, target_port);
            tunnel.connect().await?;
            Ok(tunnel.into_inner())
        }
        ProxyScheme::Http | ProxyScheme::Https => {
            let mut tunnel =
                HttpTunnel::new(stream, proxy.clone(), target_host, target_port);
            tunnel.connect().await?;
            Ok(tunnel.into_inner())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_utils::MockServer;

    // -----------------------------------------------------------------------
    // SOCKS5 dispatch
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn socks5_dispatches_to_socks5_tunnel() {
        let proxy = Proxy::from_url("socks5://proxy.example.com:1080").unwrap();
        let (client, mut server) = tokio::io::duplex(1024);

        let mut mock = MockServer::new(vec![
            // SOCKS5 server response: chosen method = 0x00 (no auth)
            vec![0x05, 0x00],
            // SOCKS5 CONNECT success
            vec![0x05, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        ]);

        let server_task = tokio::spawn(async move {
            mock.run(&mut server).await;
            mock
        });

        let stream = connect_through_proxy(
            client,
            &proxy,
            "target.example.com".to_string(),
            443,
        )
        .await
        .unwrap();

        // Verify we got a usable stream back
        drop(stream);

        let mock = server_task.await.unwrap();
        let data = mock.client_data();
        // First bytes should be SOCKS5 greeting (version=0x05)
        assert_eq!(data[0], 0x05);
    }

    // -----------------------------------------------------------------------
    // HTTP dispatch
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn http_dispatches_to_http_tunnel() {
        let proxy = Proxy::from_url("http://proxy.example.com:8080").unwrap();
        let (client, mut server) = tokio::io::duplex(2048);

        let mut mock = MockServer::new(vec![
            // HTTP 200 Connection established
            b"HTTP/1.1 200 Connection established\r\n\r\n".to_vec(),
        ]);

        let server_task = tokio::spawn(async move {
            mock.run(&mut server).await;
            mock
        });

        let stream = connect_through_proxy(
            client,
            &proxy,
            "target.example.com".to_string(),
            443,
        )
        .await
        .unwrap();

        drop(stream);

        let mock = server_task.await.unwrap();
        let data = mock.client_data();
        let request = std::str::from_utf8(data).unwrap();
        // Should be an HTTP CONNECT request
        assert!(request.starts_with("CONNECT target.example.com:443 HTTP/1.1"));
    }

    // -----------------------------------------------------------------------
    // HTTPS dispatch (same path as HTTP)
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn https_dispatches_to_http_tunnel() {
        let proxy = Proxy::from_url("https://proxy.example.com:8443").unwrap();
        let (client, mut server) = tokio::io::duplex(2048);

        let mut mock = MockServer::new(vec![
            b"HTTP/1.1 200 Connection established\r\n\r\n".to_vec(),
        ]);

        let server_task = tokio::spawn(async move {
            mock.run(&mut server).await;
            mock
        });

        let stream = connect_through_proxy(
            client,
            &proxy,
            "target.example.com".to_string(),
            443,
        )
        .await
        .unwrap();

        drop(stream);

        let mock = server_task.await.unwrap();
        let data = mock.client_data();
        let request = std::str::from_utf8(data).unwrap();
        assert!(request.starts_with("CONNECT target.example.com:443 HTTP/1.1"));
    }

    // -----------------------------------------------------------------------
    // SOCKS5 with auth dispatches correctly
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn socks5_with_auth_dispatches_to_socks5_tunnel() {
        let proxy =
            Proxy::from_url("socks5://user:pass@proxy.example.com:1080").unwrap();
        let (client, mut server) = tokio::io::duplex(1024);

        let mut mock = MockServer::new(vec![
            // SOCKS5 server: auth required
            vec![0x05, 0x02],
            // Auth success
            vec![0x01, 0x00],
            // CONNECT success
            vec![0x05, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        ]);

        let server_task = tokio::spawn(async move {
            mock.run(&mut server).await;
            mock
        });

        let stream = connect_through_proxy(
            client,
            &proxy,
            "target.example.com".to_string(),
            443,
        )
        .await
        .unwrap();

        drop(stream);

        let mock = server_task.await.unwrap();
        let data = mock.client_data();
        // Should start with SOCKS5 greeting
        assert_eq!(data[0], 0x05);
        // Should include auth method 0x02
        assert_eq!(data[3], 0x02);
    }

    // -----------------------------------------------------------------------
    // HTTP with auth dispatches correctly
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn http_with_auth_dispatches_to_http_tunnel() {
        let proxy =
            Proxy::from_url("http://user:pass@proxy.example.com:8080").unwrap();
        let (client, mut server) = tokio::io::duplex(2048);

        let mut mock = MockServer::new(vec![
            b"HTTP/1.1 200 Connection established\r\n\r\n".to_vec(),
        ]);

        let server_task = tokio::spawn(async move {
            mock.run(&mut server).await;
            mock
        });

        let stream = connect_through_proxy(
            client,
            &proxy,
            "target.example.com".to_string(),
            443,
        )
        .await
        .unwrap();

        drop(stream);

        let mock = server_task.await.unwrap();
        let data = mock.client_data();
        let request = std::str::from_utf8(data).unwrap();
        // Should include Proxy-Authorization header
        assert!(request.contains("Proxy-Authorization: Basic "));
    }

    // -----------------------------------------------------------------------
    // Error propagation: SOCKS5 failure
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn socks5_error_propagates() {
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

        let err = connect_through_proxy(
            client,
            &proxy,
            "target.example.com".to_string(),
            443,
        )
        .await
        .unwrap_err();

        assert!(matches!(err, ConnectError::Socks5(_)));
    }

    // -----------------------------------------------------------------------
    // Error propagation: HTTP failure
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn http_error_propagates() {
        let proxy = Proxy::from_url("http://proxy.example.com:8080").unwrap();
        let (client, mut server) = tokio::io::duplex(2048);

        let mut mock = MockServer::new(vec![
            b"HTTP/1.1 407 Proxy Authentication Required\r\n\r\n".to_vec(),
        ]);

        let _server_task = tokio::spawn(async move {
            mock.run(&mut server).await;
            mock
        });

        let err = connect_through_proxy(
            client,
            &proxy,
            "target.example.com".to_string(),
            443,
        )
        .await
        .unwrap_err();

        assert!(matches!(err, ConnectError::Http(_)));
    }
}
