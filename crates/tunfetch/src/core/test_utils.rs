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
            let mut buf = [0u8; 2048];
            let n = stream.read(&mut buf).await.unwrap();
            self.read_buf.extend_from_slice(&buf[..n]);

            stream.write_all(response).await.unwrap();
            stream.flush().await.unwrap();
        }
    }

    pub fn client_data(&self) -> &[u8] {
        &self.read_buf
    }
}
