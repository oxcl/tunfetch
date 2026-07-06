use super::proxy::Proxy;

/// Common tunnel state shared by all proxy tunnel implementations.
pub struct TunnelBase<S> {
    pub stream: S,
    pub proxy: Proxy,
    pub target_host: String,
    pub target_port: u16,
}

impl<S> TunnelBase<S> {
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
