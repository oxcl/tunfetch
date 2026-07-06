use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::proxy::Proxy;

/// Errors that can occur during SOCKS5 tunnel operations.
#[derive(Debug)]
pub enum Socks5Error {
    /// Server returned an unsupported or invalid SOCKS version.
    InvalidVersion(u8),
    /// No acceptable authentication methods were offered by the server.
    NoAcceptableMethods,
    /// Authentication with the server failed.
    AuthFailed,
    /// The CONNECT request was rejected by the server.
    ConnectFailed(u8),
    /// An I/O error occurred.
    Io(std::io::Error),
}

impl std::fmt::Display for Socks5Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Socks5Error::InvalidVersion(v) => write!(f, "invalid SOCKS version: {v}"),
            Socks5Error::NoAcceptableMethods => write!(f, "no acceptable auth methods"),
            Socks5Error::AuthFailed => write!(f, "SOCKS5 authentication failed"),
            Socks5Error::ConnectFailed(code) => write!(f, "SOCKS5 connect failed: code {code}"),
            Socks5Error::Io(e) => write!(f, "IO error: {e}"),
        }
    }
}

impl std::error::Error for Socks5Error {}

impl From<std::io::Error> for Socks5Error {
    fn from(e: std::io::Error) -> Self {
        Socks5Error::Io(e)
    }
}

/// A SOCKS5 tunnel that performs the SOCKS5 handshake over an existing stream.
pub struct Socks5Tunnel<S> {
    stream: S,
    proxy: Proxy,
    target_host: String,
    target_port: u16,
}

impl<S> Socks5Tunnel<S> {
    pub fn new(stream: S, proxy: Proxy, target_host: String, target_port: u16) -> Self {
        Self {
            stream,
            proxy,
            target_host,
            target_port,
        }
    }

    pub fn into_inner(self) -> S {
        self.stream
    }
}

impl<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin> Socks5Tunnel<S> {
    /// Perform the SOCKS5 handshake and CONNECT to the target.
    pub async fn connect(&mut self) -> Result<(), Socks5Error> {
        let method = self.greeting().await?;
        if method == 0x02 {
            self.authenticate().await?;
        }
        self.connect_request().await?;
        Ok(())
    }

    /// Send the SOCKS5 greeting and receive the server's chosen method.
    async fn greeting(&mut self) -> Result<u8, Socks5Error> {
        // Build greeting: [0x05, nmethods, methods...]
        let mut greeting = vec![0x05];
        if self.proxy.username.is_some() {
            greeting.extend_from_slice(&[0x02, 0x00, 0x02]); // no-auth + user/pass
        } else {
            greeting.extend_from_slice(&[0x01, 0x00]); // no-auth only
        }
        self.stream.write_all(&greeting).await?;

        // Read response: [0x05, method]
        let mut resp = [0u8; 2];
        self.stream.read_exact(&mut resp).await?;

        if resp[0] != 0x05 {
            return Err(Socks5Error::InvalidVersion(resp[0]));
        }
        if resp[1] == 0xFF {
            return Err(Socks5Error::NoAcceptableMethods);
        }

        Ok(resp[1])
    }

    /// Perform username/password authentication (RFC 1929).
    async fn authenticate(&mut self) -> Result<(), Socks5Error> {
        let username = self.proxy.username.as_deref().unwrap_or("");
        let password = self.proxy.password.as_deref().unwrap_or("");

        let mut auth = vec![0x01]; // auth version
        auth.push(username.len() as u8);
        auth.extend_from_slice(username.as_bytes());
        auth.push(password.len() as u8);
        auth.extend_from_slice(password.as_bytes());
        self.stream.write_all(&auth).await?;

        // Read response: [0x01, status]
        let mut resp = [0u8; 2];
        self.stream.read_exact(&mut resp).await?;

        if resp[1] != 0x00 {
            return Err(Socks5Error::AuthFailed);
        }

        Ok(())
    }

    /// Send the CONNECT request to the proxy.
    async fn connect_request(&mut self) -> Result<(), Socks5Error> {
        let mut req = vec![0x05, 0x01, 0x00]; // version, cmd=connect, reserved

        // Encode target as domain name (atyp=0x03)
        req.push(0x03);
        req.push(self.target_host.len() as u8);
        req.extend_from_slice(self.target_host.as_bytes());
        req.extend_from_slice(&self.target_port.to_be_bytes());

        self.stream.write_all(&mut req).await?;

        // Read response header: [0x05, rep, rsv, atyp]
        let mut header = [0u8; 4];
        self.stream.read_exact(&mut header).await?;

        if header[1] != 0x00 {
            return Err(Socks5Error::ConnectFailed(header[1]));
        }

        // Read the rest based on address type
        let atyp = header[3];
        match atyp {
            0x01 => {
                // IPv4: 4 bytes + 2 bytes port
                let mut addr = [0u8; 6];
                self.stream.read_exact(&mut addr).await?;
            }
            0x03 => {
                // Domain: 1 byte len + domain + 2 bytes port
                let mut len = [0u8; 1];
                self.stream.read_exact(&mut len).await?;
                let mut addr = vec![0u8; len[0] as usize + 2];
                self.stream.read_exact(&mut addr).await?;
            }
            0x04 => {
                // IPv6: 16 bytes + 2 bytes port
                let mut addr = [0u8; 18];
                self.stream.read_exact(&mut addr).await?;
            }
            _ => {
                return Err(Socks5Error::ConnectFailed(header[1]));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// A mock server that reads bytes from the client and writes predefined responses.
    struct MockServer {
        read_buf: Vec<u8>,
        responses: Vec<Vec<u8>>,
    }

    impl MockServer {
        fn new(responses: Vec<Vec<u8>>) -> Self {
            Self {
                read_buf: Vec::new(),
                responses,
            }
        }

        /// Simulate the server side: read from client stream, write responses.
        async fn run<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
            &mut self,
            stream: &mut S,
        ) {
            for response in &self.responses {
                // Read whatever the client sends (up to 1024 bytes)
                let mut buf = [0u8; 1024];
                let n = stream.read(&mut buf).await.unwrap();
                self.read_buf.extend_from_slice(&buf[..n]);

                // Send the predefined response
                stream.write_all(response).await.unwrap();
                stream.flush().await.unwrap();
            }
        }

        fn client_data(&self) -> &[u8] {
            &self.read_buf
        }
    }

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
}
