mod error;

pub use error::HttpTunnelError;

use base64::Engine;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::connect::ConnectError;
use super::proxy::Proxy;
use super::tunnel::Tunnel;

/// An HTTP CONNECT tunnel that performs the CONNECT handshake over an existing stream.
pub struct HttpTunnel<'a, S> {
    stream: S,
    proxy: &'a Proxy,
    target_host: String,
    target_port: u16,
}

impl<'a, S> HttpTunnel<'a, S> {
    pub fn new(stream: S, proxy: &'a Proxy, target_host: String, target_port: u16) -> Self {
        Self {
            stream,
            proxy,
            target_host,
            target_port,
        }
    }
}

impl<'a, S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin> Tunnel for HttpTunnel<'a, S> {
    type Stream = S;

    async fn connect(&mut self) -> Result<(), ConnectError> {
        self.do_connect().await?;
        Ok(())
    }

    fn into_inner(self) -> S {
        self.stream
    }
}

impl<'a, S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin> HttpTunnel<'a, S> {
    /// Perform the HTTP CONNECT handshake.
    async fn do_connect(&mut self) -> Result<(), HttpTunnelError> {
        let mut request = format!(
            "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\n",
            self.target_host, self.target_port, self.target_host, self.target_port,
        );

        // Add Proxy-Authorization header if credentials are present
        if let (Some(user), Some(pass)) = (&self.proxy.username, &self.proxy.password) {
            let credentials = format!("{user}:{pass}");
            let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());
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
            loop {
                let line = read_line(&mut self.stream).await?;
                if line.trim().is_empty() {
                    break;
                }
            }
            return Err(HttpTunnelError::ConnectFailed(status_code));
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

#[cfg(test)]
mod tests;
