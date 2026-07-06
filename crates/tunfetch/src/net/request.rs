use std::collections::HashMap;

use bytes::Bytes;
use http_body_util::Full;

/// Parsed request options from JavaScript.
#[derive(Debug)]
pub struct RequestOptions {
    pub method: http::Method,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

/// Errors that can occur when building a request.
#[derive(Debug, thiserror::Error)]
pub enum RequestError {
    #[error("invalid HTTP method: {0}")]
    InvalidMethod(String),

    #[error("invalid URL: {0}")]
    InvalidUrl(String),
}

impl RequestOptions {
    /// Create a new RequestOptions with default values (GET, no body).
    pub fn new() -> Self {
        Self {
            method: http::Method::GET,
            headers: HashMap::new(),
            body: None,
        }
    }

    /// Set the HTTP method.
    pub fn with_method(mut self, method: &str) -> Result<Self, RequestError> {
        self.method = method
            .parse()
            .map_err(|_| RequestError::InvalidMethod(method.to_string()))?;
        Ok(self)
    }

    /// Add a header.
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers
            .insert(key.to_string(), value.to_string());
        self
    }

    /// Set the body.
    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }
}

/// Build an http::Request from a URL and RequestOptions.
pub fn build_request(
    url: &str,
    opts: &RequestOptions,
) -> Result<http::Request<Full<Bytes>>, RequestError> {
    let uri: http::Uri = url
        .parse()
        .map_err(|_| RequestError::InvalidUrl(url.to_string()))?;

    let body = opts.body.clone().unwrap_or_default();
    let request_body = Full::new(Bytes::from(body));

    let mut builder = http::Request::builder().method(&opts.method).uri(&uri);

    // Add all headers from options
    for (key, value) in &opts.headers {
        builder = builder.header(key.as_str(), value.as_str());
    }

    // Set Host header if not already set
    if !opts.headers.contains_key("host") {
        if let Some(host) = uri.host() {
            // Include port in Host header if it's non-default
            let host_str = match uri.port_u16() {
                Some(port) => {
                    let default_port = match uri.scheme_str() {
                        Some("https") => 443,
                        _ => 80, // HTTP default
                    };
                    if port == default_port {
                        host.to_string()
                    } else {
                        format!("{host}:{port}")
                    }
                }
                None => host.to_string(),
            };
            builder = builder.header("Host", &host_str);
        }
    }

    builder
        .body(request_body)
        .map_err(|_| RequestError::InvalidUrl(url.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    // -----------------------------------------------------------------------
    // Default request
    // -----------------------------------------------------------------------

    #[test]
    fn default_request_is_get() {
        let opts = RequestOptions::new();
        assert_eq!(opts.method, http::Method::GET);
        assert!(opts.headers.is_empty());
        assert!(opts.body.is_none());
    }

    // -----------------------------------------------------------------------
    // Method parsing
    // -----------------------------------------------------------------------

    #[test]
    fn parse_standard_methods() {
        let methods = &[
            ("GET", http::Method::GET),
            ("POST", http::Method::POST),
            ("PUT", http::Method::PUT),
            ("DELETE", http::Method::DELETE),
            ("PATCH", http::Method::PATCH),
        ];

        for (method_str, expected) in methods {
            let opts = RequestOptions::new().with_method(method_str).unwrap();
            assert_eq!(opts.method, *expected, "Failed for method: {method_str}");
        }
    }

    #[test]
    fn accept_custom_method() {
        // http::Method accepts any string (HTTP allows custom methods)
        let opts = RequestOptions::new().with_method("NOTVALID").unwrap();
        assert_eq!(opts.method, "NOTVALID");
    }

    // -----------------------------------------------------------------------
    // Headers
    // -----------------------------------------------------------------------

    #[test]
    fn add_single_header() {
        let opts = RequestOptions::new()
            .with_header("Content-Type", "application/json");
        assert_eq!(
            opts.headers.get("Content-Type").unwrap(),
            "application/json"
        );
    }

    #[test]
    fn add_multiple_headers() {
        let opts = RequestOptions::new()
            .with_header("Content-Type", "application/json")
            .with_header("Authorization", "Bearer token123");
        assert_eq!(opts.headers.len(), 2);
        assert_eq!(
            opts.headers.get("Authorization").unwrap(),
            "Bearer token123"
        );
    }

    // -----------------------------------------------------------------------
    // Body
    // -----------------------------------------------------------------------

    #[test]
    fn with_body() {
        let body = b"hello world".to_vec();
        let opts = RequestOptions::new().with_body(body.clone());
        assert_eq!(opts.body, Some(body));
    }

    // -----------------------------------------------------------------------
    // Build request
    // -----------------------------------------------------------------------

    #[test]
    fn build_simple_get() {
        let opts = RequestOptions::new();
        let req = build_request("http://example.com/path", &opts).unwrap();
        assert_eq!(*req.method(), http::Method::GET);
        assert_eq!(*req.uri(), "http://example.com/path");
        assert_eq!(
            req.headers().get("Host").unwrap().to_str().unwrap(),
            "example.com"
        );
        // Body is an empty Full<Bytes>
        let body = req.into_body();
        let collected = futures::executor::block_on(body.collect()).unwrap();
        assert!(collected.to_bytes().is_empty());
    }

    #[test]
    fn build_post_with_body() {
        let opts = RequestOptions::new()
            .with_method("POST")
            .unwrap()
            .with_header("Content-Type", "application/json")
            .with_body(b"{\"key\":\"value\"}".to_vec());

        let req = build_request("http://example.com/api", &opts).unwrap();
        assert_eq!(*req.method(), http::Method::POST);
        assert_eq!(
            req.headers().get("Content-Type").unwrap().to_str().unwrap(),
            "application/json"
        );
        // Body contains the expected bytes
        let body = req.into_body();
        let collected = futures::executor::block_on(body.collect()).unwrap();
        assert_eq!(&collected.to_bytes()[..], b"{\"key\":\"value\"}");
    }

    #[test]
    fn build_request_with_custom_host() {
        let opts = RequestOptions::new().with_header("Host", "custom.host.com");
        let req = build_request("http://example.com/path", &opts).unwrap();
        assert_eq!(
            req.headers().get("Host").unwrap().to_str().unwrap(),
            "custom.host.com"
        );
    }

    #[test]
    fn build_request_with_port() {
        let opts = RequestOptions::new();
        let req = build_request("http://localhost:8080/path", &opts).unwrap();
        assert_eq!(
            req.headers().get("Host").unwrap().to_str().unwrap(),
            "localhost:8080"
        );
    }

    #[test]
    fn build_request_with_query() {
        let opts = RequestOptions::new();
        let req = build_request("http://example.com/path?foo=bar&baz=qux", &opts).unwrap();
        assert_eq!(*req.uri(), "http://example.com/path?foo=bar&baz=qux");
    }

    #[test]
    fn reject_invalid_url() {
        let opts = RequestOptions::new();
        let err = build_request("not a valid url", &opts).unwrap_err();
        assert!(matches!(err, RequestError::InvalidUrl(_)));
    }
}
