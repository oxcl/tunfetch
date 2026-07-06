use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// A mock server that reads bytes from the client and writes predefined responses.
pub struct MockServer {
    read_buf: Vec<u8>,
    responses: Vec<Vec<u8>>,
}

impl MockServer {
    pub fn new(responses: Vec<Vec<u8>>) -> Self {
        Self {
            read_buf: Vec::new(),
            responses,
        }
    }

    pub async fn run<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
        &mut self,
        stream: &mut S,
    ) {
        for response in &self.responses {
            let mut buf = vec![0u8; 8192];
            let n = stream.read(&mut buf).await.expect("failed to read from stream");
            self.read_buf.extend_from_slice(&buf[..n]);

            stream.write_all(response).await.expect("failed to write to stream");
            stream.flush().await.expect("failed to flush stream");
        }
    }

    pub fn client_data(&self) -> &[u8] {
        &self.read_buf
    }
}
