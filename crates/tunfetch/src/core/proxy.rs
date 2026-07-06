use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyScheme {
    Http,
    Https,
    Socks5,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proxy {
    pub scheme: ProxyScheme,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug)]
pub enum ProxyError {
    InvalidScheme(String),
    MissingHost,
    InvalidUrl(String),
    MissingPort,
}

impl fmt::Display for ProxyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProxyError::InvalidScheme(s) => write!(f, "invalid proxy scheme: {s}"),
            ProxyError::MissingHost => write!(f, "proxy URL has no host"),
            ProxyError::InvalidUrl(msg) => write!(f, "invalid proxy URL: {msg}"),
            ProxyError::MissingPort => write!(f, "proxy URL has no port"),
        }
    }
}

impl std::error::Error for ProxyError {}

impl Proxy {
    pub fn from_url(url: &str) -> Result<Self, ProxyError> {
        let url = url.trim();

        // Extract scheme
        let (scheme, rest) = if let Some(s) = url.strip_prefix("socks5://") {
            (ProxyScheme::Socks5, s)
        } else if let Some(s) = url.strip_prefix("https://") {
            (ProxyScheme::Https, s)
        } else if let Some(s) = url.strip_prefix("http://") {
            (ProxyScheme::Http, s)
        } else {
            return Err(ProxyError::InvalidScheme(url.to_string()));
        };

        // Split off userinfo if present
        let (userinfo, host_part) = if let Some(at_idx) = rest.find('@') {
            let userinfo = &rest[..at_idx];
            let host_part = &rest[at_idx + 1..];
            (Some(userinfo), host_part)
        } else {
            (None, rest)
        };

        // Parse username:password from userinfo
        let (username, password) = match userinfo {
            Some(ui) => {
                if let Some(colon_idx) = ui.find(':') {
                    let user = ui[..colon_idx].to_string();
                    let pass = ui[colon_idx + 1..].to_string();
                    (Some(user), Some(pass))
                } else {
                    (Some(ui.to_string()), None)
                }
            }
            None => (None, None),
        };

        // Parse host:port
        let (host, port) = if let Some(colon_idx) = host_part.rfind(':') {
            let host = host_part[..colon_idx].to_string();
            let port_str = &host_part[colon_idx + 1..];
            let port: u16 = port_str
                .parse()
                .map_err(|_| ProxyError::InvalidUrl(format!("invalid port: {port_str}")))?;
            (host, port)
        } else {
            // No port specified, use default
            let default_port = match scheme {
                ProxyScheme::Socks5 => 1080,
                ProxyScheme::Http => 80,
                ProxyScheme::Https => 443,
            };
            (host_part.to_string(), default_port)
        };

        if host.is_empty() {
            return Err(ProxyError::MissingHost);
        }

        Ok(Proxy {
            scheme,
            host,
            port,
            username,
            password,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Scheme parsing
    // -----------------------------------------------------------------------

    #[test]
    fn parse_socks5_scheme() {
        let proxy = Proxy::from_url("socks5://proxy.example.com:1080").unwrap();
        assert_eq!(proxy.scheme, ProxyScheme::Socks5);
    }

    #[test]
    fn parse_http_scheme() {
        let proxy = Proxy::from_url("http://proxy.example.com:8080").unwrap();
        assert_eq!(proxy.scheme, ProxyScheme::Http);
    }

    #[test]
    fn parse_https_scheme() {
        let proxy = Proxy::from_url("https://proxy.example.com:8443").unwrap();
        assert_eq!(proxy.scheme, ProxyScheme::Https);
    }

    #[test]
    fn reject_invalid_scheme() {
        let err = Proxy::from_url("ftp://proxy.example.com:1080").unwrap_err();
        assert!(matches!(err, ProxyError::InvalidScheme(_)));
    }

    #[test]
    fn reject_no_scheme() {
        let err = Proxy::from_url("proxy.example.com:1080").unwrap_err();
        assert!(matches!(err, ProxyError::InvalidScheme(_)));
    }

    // -----------------------------------------------------------------------
    // Host and port parsing
    // -----------------------------------------------------------------------

    #[test]
    fn parse_host_and_port() {
        let proxy = Proxy::from_url("socks5://proxy.example.com:1080").unwrap();
        assert_eq!(proxy.host, "proxy.example.com");
        assert_eq!(proxy.port, 1080);
    }

    #[test]
    fn default_port_socks5() {
        let proxy = Proxy::from_url("socks5://proxy.example.com").unwrap();
        assert_eq!(proxy.port, 1080);
    }

    #[test]
    fn default_port_http() {
        let proxy = Proxy::from_url("http://proxy.example.com").unwrap();
        assert_eq!(proxy.port, 80);
    }

    #[test]
    fn default_port_https() {
        let proxy = Proxy::from_url("https://proxy.example.com").unwrap();
        assert_eq!(proxy.port, 443);
    }

    #[test]
    fn reject_invalid_port() {
        let err = Proxy::from_url("socks5://proxy.example.com:notaport").unwrap_err();
        assert!(matches!(err, ProxyError::InvalidUrl(_)));
    }

    #[test]
    fn reject_empty_host() {
        let err = Proxy::from_url("socks5://:1080").unwrap_err();
        assert!(matches!(err, ProxyError::MissingHost));
    }

    // -----------------------------------------------------------------------
    // Auth parsing
    // -----------------------------------------------------------------------

    #[test]
    fn parse_username_and_password() {
        let proxy = Proxy::from_url("socks5://user:pass@proxy.example.com:1080").unwrap();
        assert_eq!(proxy.username.as_deref(), Some("user"));
        assert_eq!(proxy.password.as_deref(), Some("pass"));
    }

    #[test]
    fn parse_username_only() {
        let proxy = Proxy::from_url("http://user@proxy.example.com:8080").unwrap();
        assert_eq!(proxy.username.as_deref(), Some("user"));
        assert_eq!(proxy.password.as_deref(), None);
    }

    #[test]
    fn no_auth() {
        let proxy = Proxy::from_url("socks5://proxy.example.com:1080").unwrap();
        assert_eq!(proxy.username, None);
        assert_eq!(proxy.password, None);
    }

    #[test]
    fn password_with_colon() {
        let proxy = Proxy::from_url("socks5://user:p:a:s:s@proxy.example.com:1080").unwrap();
        assert_eq!(proxy.username.as_deref(), Some("user"));
        assert_eq!(proxy.password.as_deref(), Some("p:a:s:s"));
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn trim_whitespace() {
        let proxy = Proxy::from_url("  socks5://proxy.example.com:1080  ").unwrap();
        assert_eq!(proxy.scheme, ProxyScheme::Socks5);
        assert_eq!(proxy.host, "proxy.example.com");
    }

    #[test]
    fn localhost() {
        let proxy = Proxy::from_url("http://localhost:3128").unwrap();
        assert_eq!(proxy.host, "localhost");
        assert_eq!(proxy.port, 3128);
    }

    #[test]
    fn ip_address_host() {
        let proxy = Proxy::from_url("socks5://127.0.0.1:1080").unwrap();
        assert_eq!(proxy.host, "127.0.0.1");
        assert_eq!(proxy.port, 1080);
    }

    #[test]
    fn ipv6_host() {
        let proxy = Proxy::from_url("http://[::1]:8080").unwrap();
        assert_eq!(proxy.host, "[::1]");
        assert_eq!(proxy.port, 8080);
    }
}
