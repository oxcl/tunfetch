/// Errors that can occur during HTTP CONNECT tunnel operations.
#[derive(Debug, thiserror::Error)]
pub enum HttpTunnelError {
    /// The CONNECT request returned a non-200 status.
    #[error("CONNECT failed with status {0}")]
    ConnectFailed(u16),

    /// Failed to parse the HTTP response.
    #[error("invalid response: {0}")]
    InvalidResponse(String),

    /// An I/O error occurred.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
