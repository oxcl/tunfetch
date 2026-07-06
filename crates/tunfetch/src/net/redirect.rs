use http::StatusCode;

/// Maximum number of redirects to follow before giving up.
const MAX_REDIRECTS: usize = 20;

/// Errors that can occur during redirect handling.
#[derive(Debug, thiserror::Error)]
pub enum RedirectError {
    /// Too many redirects (exceeded MAX_REDIRECTS).
    #[error("too many redirects")]
    TooManyRedirects,

    /// Missing Location header in redirect response.
    #[error("missing Location header")]
    MissingLocation,

    /// Invalid Location header URL.
    #[error("invalid redirect URL: {0}")]
    InvalidLocation(String),

    /// Redirect loop detected (same URL visited twice).
    #[error("redirect loop detected")]
    LoopDetected,
}

/// Check if a status code indicates a redirect.
pub fn is_redirect(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::MOVED_PERMANENTLY      // 301
            | StatusCode::FOUND             // 302
            | StatusCode::SEE_OTHER         // 303
            | StatusCode::TEMPORARY_REDIRECT // 307
            | StatusCode::PERMANENT_REDIRECT // 308
    )
}

/// Extract the redirect URL from the Location header.
///
/// Handles both absolute and relative URLs.
pub fn extract_redirect_url(
    location: &str,
    base_url: &str,
) -> Result<String, RedirectError> {
    let location = location.trim();

    if location.is_empty() {
        return Err(RedirectError::MissingLocation);
    }

    // If location starts with http:// or https://, it's absolute
    if location.starts_with("http://") || location.starts_with("https://") {
        return Ok(location.to_string());
    }

    // Relative URL - resolve against base
    if let Some(base) = base_url.rfind('/') {
        let base = &base_url[..=base];

        // Handle protocol-relative URLs like "//example.com/path"
        if location.starts_with("//") {
            let scheme = if base_url.starts_with("https://") {
                "https:"
            } else {
                "http:"
            };
            return Ok(format!("{scheme}{location}"));
        }

        // Handle relative paths
        if location.starts_with('/') {
            // Absolute path - use scheme + host from base
            if let Some(scheme_end) = base_url.find("://") {
                let scheme = &base_url[..scheme_end + 3];
                let rest = &base_url[scheme_end + 3..];
                if let Some(authority_end) = rest.find('/') {
                    let authority = &rest[..authority_end];
                    return Ok(format!("{scheme}{authority}{location}"));
                }
                return Ok(format!("{scheme}{rest}{location}"));
            }
        }

        // Relative path - append to base
        return Ok(format!("{base}{location}"));
    }

    Err(RedirectError::InvalidLocation(location.to_string()))
}

/// Determine if we should change the method after a redirect.
///
/// For 303 (See Other), change POST/PUT/PATCH to GET.
/// For 307/308, preserve the method.
pub fn should_change_method(status: StatusCode, method: &http::Method) -> bool {
    if status == StatusCode::SEE_OTHER {
        return matches!(
            method,
            &http::Method::POST | &http::Method::PUT | &http::Method::PATCH
        );
    }
    false
}

/// Check if a redirect URL has been visited (loop detection).
pub fn detect_loop(visited: &[String], current: &str) -> bool {
    visited.iter().any(|v| v == current)
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // is_redirect
    // -----------------------------------------------------------------------

    #[test]
    fn identify_redirect_status_codes() {
        assert!(is_redirect(StatusCode::MOVED_PERMANENTLY));
        assert!(is_redirect(StatusCode::FOUND));
        assert!(is_redirect(StatusCode::SEE_OTHER));
        assert!(is_redirect(StatusCode::TEMPORARY_REDIRECT));
        assert!(is_redirect(StatusCode::PERMANENT_REDIRECT));
    }

    #[test]
    fn non_redirect_status_codes() {
        assert!(!is_redirect(StatusCode::OK));
        assert!(!is_redirect(StatusCode::NOT_FOUND));
        assert!(!is_redirect(StatusCode::INTERNAL_SERVER_ERROR));
    }

    // -----------------------------------------------------------------------
    // extract_redirect_url
    // -----------------------------------------------------------------------

    #[test]
    fn absolute_url() {
        let url = extract_redirect_url("https://example.com/new", "http://old.com/").unwrap();
        assert_eq!(url, "https://example.com/new");
    }

    #[test]
    fn relative_path() {
        let url = extract_redirect_url("/new/path", "http://example.com/old/path").unwrap();
        assert_eq!(url, "http://example.com/new/path");
    }

    #[test]
    fn relative_path_no_trailing_slash() {
        let url = extract_redirect_url("/new", "http://example.com/old").unwrap();
        assert_eq!(url, "http://example.com/new");
    }

    #[test]
    fn protocol_relative_url() {
        let url = extract_redirect_url("//cdn.example.com/path", "http://example.com/").unwrap();
        assert_eq!(url, "http://cdn.example.com/path");
    }

    #[test]
    fn protocol_relative_url_https() {
        let url = extract_redirect_url("//cdn.example.com/path", "https://example.com/").unwrap();
        assert_eq!(url, "https://cdn.example.com/path");
    }

    #[test]
    fn missing_location_returns_error() {
        let err = extract_redirect_url("", "http://example.com/").unwrap_err();
        assert!(matches!(err, RedirectError::MissingLocation));
    }

    #[test]
    fn empty_location_returns_error() {
        let err = extract_redirect_url("  ", "http://example.com/").unwrap_err();
        assert!(matches!(err, RedirectError::MissingLocation));
    }

    // -----------------------------------------------------------------------
    // should_change_method
    // -----------------------------------------------------------------------

    #[test]
    fn change_method_for_303_post() {
        assert!(should_change_method(StatusCode::SEE_OTHER, &http::Method::POST));
    }

    #[test]
    fn change_method_for_303_put() {
        assert!(should_change_method(StatusCode::SEE_OTHER, &http::Method::PUT));
    }

    #[test]
    fn change_method_for_303_patch() {
        assert!(should_change_method(StatusCode::SEE_OTHER, &http::Method::PATCH));
    }

    #[test]
    fn no_change_method_for_303_get() {
        assert!(!should_change_method(StatusCode::SEE_OTHER, &http::Method::GET));
    }

    #[test]
    fn no_change_method_for_301() {
        assert!(!should_change_method(
            StatusCode::MOVED_PERMANENTLY,
            &http::Method::POST
        ));
    }

    #[test]
    fn no_change_method_for_307() {
        assert!(!should_change_method(
            StatusCode::TEMPORARY_REDIRECT,
            &http::Method::POST
        ));
    }

    #[test]
    fn no_change_method_for_308() {
        assert!(!should_change_method(
            StatusCode::PERMANENT_REDIRECT,
            &http::Method::POST
        ));
    }

    // -----------------------------------------------------------------------
    // detect_loop
    // -----------------------------------------------------------------------

    #[test]
    fn detect_loop_when_url_visited() {
        let visited = vec![
            "http://example.com/a".to_string(),
            "http://example.com/b".to_string(),
        ];
        assert!(detect_loop(&visited, "http://example.com/a"));
    }

    #[test]
    fn no_loop_when_url_new() {
        let visited = vec![
            "http://example.com/a".to_string(),
            "http://example.com/b".to_string(),
        ];
        assert!(!detect_loop(&visited, "http://example.com/c"));
    }

    #[test]
    fn no_loop_when_empty_history() {
        let visited = vec![];
        assert!(!detect_loop(&visited, "http://example.com/"));
    }
}
