use std::fmt;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::proxy::Proxy;

/// Errors that can occur during HTTP CONNECT tunnel operations.
#[derive(Debug)]
pub enum HttpTunnelError {
    /// The CONNECT request returned a non-200 status.
    ConnectFailed(u16, String),
    /// Failed to parse the HTTP response.
    InvalidResponse(String),
    /// An I/O error occurred.
    Io(std::io::Error),
}

impl fmt::Display for HttpTunnelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpTunnelError::ConnectFailed(status, body) => {
                write!(f, "CONNECT failed with status {status}: {body}")
            }
            HttpTunnelError::InvalidResponse(msg) => write!(f, "invalid response: {msg}"),
            HttpTunnelError::Io(e) => write!(f, "IO error: {e}"),
        }
    }
}

impl std::error::Error for HttpTunnelError {}

impl From<std::io::Error> for HttpTunnelError {
    fn from(e: std::io::Error) -> Self {
        HttpTunnelError::Io(e)
    }
}

/// An HTTP CONNECT tunnel that performs the CONNECT handshake over an existing stream.
pub struct HttpTunnel<S> {
    stream: S,
    proxy: Proxy,
    target_host: String,
    target_port: u16,
}

impl<S> HttpTunnel<S> {
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

impl<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin> HttpTunnel<S> {
    /// Perform the HTTP CONNECT handshake.
    pub async fn connect(&mut self) -> Result<(), HttpTunnelError> {
        let mut request = format!(
            "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\n",
            self.target_host, self.target_port, self.target_host, self.target_port,
        );

        // Add Proxy-Authorization header if credentials are present
        if let (Some(user), Some(pass)) = (&self.proxy.username, &self.proxy.password) {
            let credentials = format!("{user}:{pass}");
            let encoded = base64_encode(credentials.as_bytes());
            request.push_str(&format!("Proxy-Authorization: Basic {encoded}\r\n"));
        }

        request.push_str("\r\n");

        self.stream.write_all(request.as_bytes()).await?;

        // Read the response status line
        let status_line = read_line(&mut self.stream).await?;
        let status_str = status_line.trim();

        // Parse "HTTP/1.1 200 Connection established"
        let status_code = status_str
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse::<u16>().ok())
            .ok_or_else(|| HttpTunnelError::InvalidResponse(status_str.to_string()))?;

        if status_code != 200 {
            // Read remaining headers until empty line
            let mut body = String::new();
            loop {
                let line = read_line(&mut self.stream).await?;
                if line.trim().is_empty() {
                    break;
                }
                body.push_str(&line);
            }
            return Err(HttpTunnelError::ConnectFailed(status_code, body));
        }

        // Read remaining headers until empty line
        loop {
            let line = read_line(&mut self.stream).await?;
            if line.trim().is_empty() {
                break;
            }
        }

        Ok(())
    }
}

/// Read a single line from the stream (terminated by \n).
async fn read_line<S: tokio::io::AsyncRead + Unpin>(
    stream: &mut S,
) -> Result<String, std::io::Error> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let n = stream.read(&mut byte).await?;
        if n == 0 {
            break;
        }
        buf.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
    }
    String::from_utf8(buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// Simple base64 encoding (no external dependency needed for this).
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = Vec::new();

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARS[((triple >> 18) & 0x3F) as usize]);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize]);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize]);
        } else {
            result.push(b'=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize]);
        } else {
            result.push(b'=');
        }
    }

    String::from_utf8(result).unwrap()
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

        async fn run<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
            &mut self,
            stream: &mut S,
        ) {
            for response in &self.responses {
                let mut buf = [0u8; 2048];
                let n = stream.read(&mut buf).await.unwrap();
                self.read_buf.extend_from_slice(&buf[..n]);

                stream.write_all(response).await.unwrap();
                stream.flush().await.unwrap();
            }
        }

        fn client_data(&self) -> &[u8] {
            &self.read_buf
        }
    }

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
}
