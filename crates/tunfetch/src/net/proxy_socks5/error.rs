/// Errors that can occur during SOCKS5 tunnel operations.
#[derive(Debug, thiserror::Error)]
pub enum Socks5Error {
    /// Server returned an unsupported or invalid SOCKS version.
    #[error("invalid SOCKS version: {0}")]
    InvalidVersion(u8),

    /// No acceptable authentication methods were offered by the server.
    #[error("no acceptable auth methods")]
    NoAcceptableMethods,

    /// Authentication with the server failed.
    #[error("SOCKS5 authentication failed")]
    AuthFailed,

    /// The CONNECT request was rejected by the server.
    #[error("SOCKS5 connect failed: code {0}")]
    ConnectFailed(u8),

    /// An I/O error occurred.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
