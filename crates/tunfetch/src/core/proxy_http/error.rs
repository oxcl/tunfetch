use std::fmt;

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
