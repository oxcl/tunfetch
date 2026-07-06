mod error;

pub use error::Socks5Error;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::connect::ConnectError;
use super::proxy::Proxy;
use super::tunnel::Tunnel;

/// A SOCKS5 tunnel that performs the SOCKS5 handshake over an existing stream.
pub struct Socks5Tunnel<'a, S> {
    stream: S,
    proxy: &'a Proxy,
    target_host: String,
    target_port: u16,
}

impl<'a, S> Socks5Tunnel<'a, S> {
    pub fn new(stream: S, proxy: &'a Proxy, target_host: String, target_port: u16) -> Self {
        Self {
            stream,
            proxy,
            target_host,
            target_port,
        }
    }
}

impl<'a, S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin> Tunnel for Socks5Tunnel<'a, S> {
    type Stream = S;

    async fn connect(&mut self) -> Result<(), ConnectError> {
        self.do_connect().await?;
        Ok(())
    }

    fn into_inner(self) -> S {
        self.stream
    }
}

impl<'a, S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin> Socks5Tunnel<'a, S> {
    /// Perform the SOCKS5 handshake and CONNECT to the target.
    async fn do_connect(&mut self) -> Result<(), Socks5Error> {
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

        self.stream.write_all(&req).await?;

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
mod tests;
