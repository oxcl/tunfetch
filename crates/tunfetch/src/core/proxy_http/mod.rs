mod error;

pub use error::HttpTunnelError;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::proxy::Proxy;
use super::tunnel::TunnelBase;

/// An HTTP CONNECT tunnel that performs the CONNECT handshake over an existing stream.
pub struct HttpTunnel<S> {
    base: TunnelBase<S>,
}

impl<S> HttpTunnel<S> {
    pub fn new(stream: S, proxy: Proxy, target_host: String, target_port: u16) -> Self {
        Self {
            base: TunnelBase::new(stream, proxy, target_host, target_port),
        }
    }

    pub fn into_inner(self) -> S {
        self.base.into_inner()
    }
}

impl<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin> HttpTunnel<S> {
    /// Perform the HTTP CONNECT handshake.
    pub async fn connect(&mut self) -> Result<(), HttpTunnelError> {
        let mut request = format!(
            "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\n",
            self.base.target_host, self.base.target_port, self.base.target_host, self.base.target_port,
        );

        // Add Proxy-Authorization header if credentials are present
        if let (Some(user), Some(pass)) = (&self.base.proxy.username, &self.base.proxy.password) {
            let credentials = format!("{user}:{pass}");
            let encoded = base64_encode(credentials.as_bytes());
            request.push_str(&format!("Proxy-Authorization: Basic {encoded}\r\n"));
        }

        request.push_str("\r\n");

        self.base.stream.write_all(request.as_bytes()).await?;

        // Read the response status line
        let status_line = read_line(&mut self.base.stream).await?;
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
                let line = read_line(&mut self.base.stream).await?;
                if line.trim().is_empty() {
                    break;
                }
                body.push_str(&line);
            }
            return Err(HttpTunnelError::ConnectFailed(status_code, body));
        }

        // Read remaining headers until empty line
        loop {
            let line = read_line(&mut self.base.stream).await?;
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
mod tests;
