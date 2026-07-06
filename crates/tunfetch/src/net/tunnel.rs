#![allow(async_fn_in_trait)]

use tokio::io::{AsyncRead, AsyncWrite};

/// A trait for proxy tunnel implementations.
///
/// This trait defines the common interface for all proxy tunnel types
/// (HTTP CONNECT, SOCKS5). Each implementation performs its specific
/// protocol handshake to establish a tunnel to the target host.
pub trait Tunnel {
    /// The underlying stream type.
    type Stream: AsyncRead + AsyncWrite + Unpin;

    /// Perform the proxy handshake to establish the tunnel.
    async fn connect(&mut self) -> Result<(), super::connect::ConnectError>;

    /// Consume the tunnel and return the underlying stream.
    fn into_inner(self) -> Self::Stream;
}
