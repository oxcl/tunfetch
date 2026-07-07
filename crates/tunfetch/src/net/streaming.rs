use std::cell::RefCell;
use std::rc::Rc;

use bytes::Bytes;
use http_body::Frame;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use wasm_bindgen::prelude::*;

/// Shared state for the ReadableStream controller.
type SharedController = Rc<RefCell<Option<web_sys::ReadableStreamDefaultController>>>;

/// A chunk of data to be enqueued into the stream.
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// A chunk of data.
    Data(Bytes),
    /// Stream is complete.
    Done,
    /// An error occurred.
    Error(String),
}

/// Create a ReadableStream from a hyper Incoming body.
///
/// Returns a (stream, body_reader) tuple where:
/// - `stream` is the JavaScript ReadableStream to return to the caller
/// - `body_reader` is a future that reads from the hyper body and feeds chunks to the stream
pub fn create_streaming_response(
    body: Incoming,
) -> Result<(web_sys::ReadableStream, impl std::future::Future<Output = ()>), JsValue> {
    let controller_state: SharedController = Rc::new(RefCell::new(None));

    // Create the underlying source for ReadableStream
    let source = create_underlying_source(controller_state.clone());

    // Create the ReadableStream
    let stream = web_sys::ReadableStream::new_with_underlying_source(&source)?;

    // Create the body reader future
    let body_reader = feed_body_to_stream(body, controller_state);

    Ok((stream, body_reader))
}

/// Create a JavaScript UnderlyingSource object with a start callback.
fn create_underlying_source(controller_state: SharedController) -> js_sys::Object {
    let source = js_sys::Object::new();

    // Create the start callback using Function constructor
    let start_callback = {
        let controller_state = controller_state.clone();
        Closure::wrap(Box::new(move |controller: web_sys::ReadableStreamDefaultController| {
            *controller_state.borrow_mut() = Some(controller);
        }) as Box<dyn FnMut(web_sys::ReadableStreamDefaultController)>)
    };

    let start_fn = start_callback.as_ref().clone();
    // Leak the closure to keep it alive for the lifetime of the program
    std::mem::forget(start_callback);

    js_sys::Reflect::set(&source, &"start".into(), &start_fn).unwrap();

    source
}

/// Feed data from a hyper body into a ReadableStream.
async fn feed_body_to_stream(body: Incoming, controller_state: SharedController) {
    let mut body = body;

    loop {
        // Wait for the controller to be available
        let controller = loop {
            if let Some(ctrl) = controller_state.borrow().clone() {
                break ctrl;
            }
            // Yield to allow the start callback to run
            wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(&JsValue::NULL))
                .await
                .unwrap();
        };

        // Read the next frame from the body
        match body.frame().await {
            Some(Ok(frame)) => {
                let data = frame.into_data().unwrap_or_else(|_| Bytes::new());
                if !data.is_empty() {
                    let chunk = js_sys::Uint8Array::from(&data[..]);
                    controller
                        .enqueue_with_chunk(&JsValue::from(chunk))
                        .unwrap();
                }
            }
            Some(Err(_)) => {
                controller.error();
                return;
            }
            None => {
                // Body is complete
                controller.close().unwrap();
                return;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Pure Rust helpers (testable without WASM)
// ---------------------------------------------------------------------------

/// Convert a hyper body frame result into a StreamChunk.
///
/// This is a pure function that can be tested without WASM.
pub fn frame_to_chunk(
    frame: Option<Result<Frame<Bytes>, hyper::Error>>,
) -> StreamChunk {
    match frame {
        Some(Ok(frame)) => {
            let data = frame.into_data().unwrap_or_else(|_| Bytes::new());
            if data.is_empty() {
                StreamChunk::Done
            } else {
                StreamChunk::Data(data)
            }
        }
        Some(Err(e)) => StreamChunk::Error(e.to_string()),
        None => StreamChunk::Done,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_to_chunk_with_data() {
        let frame = Some(Ok(Frame::data(Bytes::from("hello"))));
        let chunk = frame_to_chunk(frame);
        match chunk {
            StreamChunk::Data(data) => assert_eq!(&data[..], b"hello"),
            _ => panic!("expected Data variant"),
        }
    }

    #[test]
    fn frame_to_chunk_with_empty_data() {
        let frame = Some(Ok(Frame::data(Bytes::new())));
        let chunk = frame_to_chunk(frame);
        assert!(matches!(chunk, StreamChunk::Done));
    }

    #[test]
    fn frame_to_chunk_with_none() {
        let chunk = frame_to_chunk(None);
        assert!(matches!(chunk, StreamChunk::Done));
    }

    #[test]
    fn stream_chunk_is_cloneable() {
        let chunk = StreamChunk::Data(Bytes::from("test"));
        let cloned = chunk.clone();
        match cloned {
            StreamChunk::Data(data) => assert_eq!(&data[..], b"test"),
            _ => panic!("expected Data variant"),
        }
    }

    #[test]
    fn create_shared_controller_state() {
        let state: SharedController = Rc::new(RefCell::new(None));
        assert!(state.borrow().is_none());
    }

    #[test]
    fn shared_controller_state_is_cloneable() {
        let state: SharedController = Rc::new(RefCell::new(None));
        let state2 = state.clone();

        // Both point to the same underlying data
        assert!(state.borrow().is_none());
        assert!(state2.borrow().is_none());
    }
}
