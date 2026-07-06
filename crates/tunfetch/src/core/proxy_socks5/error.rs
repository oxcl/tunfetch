use std::fmt;

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

impl fmt::Display for Socks5Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
