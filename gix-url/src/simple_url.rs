use percent_encoding::percent_decode_str;

/// A minimal URL parser that extracts only what we need for git URLs.
/// This is a replacement for the `url` crate dependency.
#[derive(Debug)]
pub(crate) struct ParsedUrl {
    pub scheme: String,           // Owned to allow normalization to lowercase
    pub username: String,         // Owned to allow percent-decoding
    pub password: Option<String>, // Owned to allow percent-decoding
    pub host: Option<String>,     // Owned to allow normalization to lowercase
    pub port: Option<u16>,
    pub path: String, // Owned to allow percent-decoding
}

/// Minimal parse error type to replace url::ParseError
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UrlParseError {
    #[error("relative URL without a base")]
    RelativeUrlWithoutBase,
    #[error("invalid port number - must be between 1-65535")]
    InvalidPort,
    #[error("invalid domain character")]
    InvalidDomainCharacter,
    #[error("Scheme requires host")]
    SchemeRequiresHost,
}

/// Check if a character is valid in a URL scheme.
/// Valid scheme characters: alphanumeric, +, -, or .
fn is_valid_scheme_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.'
}

/// Decode a percent-encoded string, returning an error if the result is not valid UTF-8.
/// Returns the original string if it contains no percent-encoding.
fn percent_decode(s: &str) -> Result<String, UrlParseError> {
    percent_decode_str(s)
        .decode_utf8()
        .map(std::borrow::Cow::into_owned)
        .map_err(|_| UrlParseError::InvalidDomainCharacter)
}

impl ParsedUrl {
    /// Parse a URL string into its components.
    /// Expected format: scheme://[user[:password]@]host[:port]/path
    pub(crate) fn parse(input: &str) -> Result<Self, UrlParseError> {
        // Validate that the entire URL doesn't contain any whitespace (per RFC 3986)
        if input.chars().any(char::is_whitespace) {
            return Err(UrlParseError::InvalidDomainCharacter);
        }

        // Find scheme by looking for first ':'
        let first_colon = input.find(':').ok_or(UrlParseError::RelativeUrlWithoutBase)?;
        let scheme_str = &input[..first_colon];
        // Normalize scheme to lowercase for case-insensitive matching (matches url crate behavior)
        let scheme = scheme_str.to_ascii_lowercase();
        let Some(after_scheme) = input[first_colon..].strip_prefix("://") else {
            return Err(UrlParseError::RelativeUrlWithoutBase);
        };

        // Check for relative URL (scheme without proper authority)
        if scheme_str.is_empty() {
            return Err(UrlParseError::RelativeUrlWithoutBase);
        }

        // Validate scheme characters (check original before lowercase conversion)
        if !scheme_str.chars().all(is_valid_scheme_char) {
            return Err(UrlParseError::RelativeUrlWithoutBase);
        }

        // Find the end of the authority.
        let path_start = after_scheme.find(['/', '?', '#']).unwrap_or(after_scheme.len());
        let authority = &after_scheme[..path_start];
        if authority.contains('\\') {
            return Err(UrlParseError::InvalidDomainCharacter);
        }
        let path = if path_start < after_scheme.len() {
            percent_decode(&after_scheme[path_start..])?
        } else {
            // No path specified - leave empty (caller can default to / if needed)
            String::new()
        };

        let allow_unbracketed_ipv6 = matches!(scheme.as_str(), "git" | "ssh" | "git+ssh" | "ssh+git");

        // Parse authority: [user[:password]@]host[:port]
        let (username, password, host, port) = if let Some((user_info, host_port)) = authority.rsplit_once('@') {
            // Has user info
            let (user, pass) = if let Some((user_str, pass_str)) = user_info.split_once(':') {
                // Treat empty password as None
                let pass = if pass_str.is_empty() {
                    None
                } else {
                    Some(percent_decode(pass_str)?)
                };
                (percent_decode(user_str)?, pass)
            } else {
                // No password, just username
                (percent_decode(user_info)?, None)
            };

            let (h, p) = Self::parse_host_port(host_port, allow_unbracketed_ipv6)?;
            // If we have user info, we must have a host
            if h.is_none() {
                return Err(UrlParseError::InvalidDomainCharacter);
            }
            (user, pass, h, p)
        } else {
            // No user info
            let (h, p) = Self::parse_host_port(authority, allow_unbracketed_ipv6)?;
            (String::new(), None, h, p)
        };

        // Standard schemes (http, https, git, ssh) require a host
        // Scheme is already lowercase at this point
        let requires_host = matches!(scheme.as_str(), "http" | "https" | "git" | "ssh" | "ftp" | "ftps");
        if requires_host && host.is_none() {
            return Err(UrlParseError::SchemeRequiresHost);
        }

        Ok(ParsedUrl {
            scheme,
            username,
            password,
            host,
            port,
            path,
        })
    }

    fn parse_host_port(
        host_port: &str,
        allow_unbracketed_ipv6: bool,
    ) -> Result<(Option<String>, Option<u16>), UrlParseError> {
        if host_port.is_empty() {
            return Ok((None, None));
        }

        // Handle IPv6 addresses: [::1] or [::1]:port
        if host_port.starts_with('[') {
            if let Some(bracket_end) = host_port.find(']') {
                if host_port[1..bracket_end].parse::<std::net::Ipv6Addr>().is_err() {
                    return Err(UrlParseError::InvalidDomainCharacter);
                }
                let remaining = &host_port[bracket_end + 1..];

                if remaining.is_empty() {
                    // IPv6 addresses are case-insensitive, normalize to lowercase
                    let host = Some(host_port[..=bracket_end].to_ascii_lowercase());
                    return Ok((host, None));
                } else if let Some(port_str) = remaining.strip_prefix(':') {
                    if port_str.is_empty() {
                        // Empty port like "[::1]:" - preserve the trailing colon for Git compatibility
                        let host = Some(host_port.to_ascii_lowercase());
                        return Ok((host, None));
                    }
                    let port = port_str.parse::<u16>().map_err(|_| UrlParseError::InvalidPort)?;
                    // Validate port is in valid range (1-65535, port 0 is invalid)
                    if port == 0 {
                        return Err(UrlParseError::InvalidPort);
                    }
                    // IPv6 addresses are case-insensitive, normalize to lowercase
                    let host = Some(host_port[..=bracket_end].to_ascii_lowercase());
                    return Ok((host, Some(port)));
                } else {
                    return Err(UrlParseError::InvalidDomainCharacter);
                }
            } else {
                return Err(UrlParseError::InvalidDomainCharacter);
            }
        }

        if allow_unbracketed_ipv6
            && (host_port.parse::<std::net::Ipv6Addr>().is_ok()
                || host_port
                    .strip_suffix(':')
                    .is_some_and(|host| host.parse::<std::net::Ipv6Addr>().is_ok()))
        {
            return Ok((Some(host_port.to_ascii_lowercase()), None));
        }

        // Handle regular host:port. Unbracketed colons cannot be part of a host.
        if let Some((before_last_colon, after_last_colon)) = host_port.rsplit_once(':') {
            if before_last_colon.is_empty() || before_last_colon.contains(':') {
                return Err(UrlParseError::InvalidDomainCharacter);
            }
            if after_last_colon.is_empty() {
                // Empty port like "host:" - store host with trailing colon for Git compatibility.
                let mut host = Self::normalize_hostname(before_last_colon)?;
                host.push(':');
                return Ok((Some(host), None));
            }
            if !after_last_colon.chars().all(|c| c.is_ascii_digit()) {
                return Err(UrlParseError::InvalidPort);
            }
            let host = Self::normalize_hostname(before_last_colon)?;
            let port = after_last_colon
                .parse::<u16>()
                .map_err(|_| UrlParseError::InvalidPort)?;
            if port == 0 {
                return Err(UrlParseError::InvalidPort);
            }
            return Ok((Some(host), Some(port)));
        }

        // No port, just host.
        Ok((Some(Self::normalize_hostname(host_port)?), None))
    }

    fn is_normalizable_hostname(host: &str) -> bool {
        host.bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'.' | b'_' | b'*'))
    }

    /// Validate a hostname and normalize DNS-like ASCII hostnames to lowercase.
    /// Hostnames containing other permitted URL characters retain their original case.
    fn normalize_hostname(host: &str) -> Result<String, UrlParseError> {
        if !host.bytes().all(|c| {
            c.is_ascii_alphanumeric()
                || matches!(
                    c,
                    b'-' | b'.'
                        | b'_'
                        | b'~'
                        | b'!'
                        | b'$'
                        | b'&'
                        | b'\''
                        | b'('
                        | b')'
                        | b'*'
                        | b'+'
                        | b','
                        | b';'
                        | b'='
                        | b'%'
                )
        }) {
            return Err(UrlParseError::InvalidDomainCharacter);
        }
        Ok(if Self::is_normalizable_hostname(host) {
            host.to_ascii_lowercase()
        } else {
            host.to_owned()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_url() {
        let url = ParsedUrl::parse("http://example.com/path").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.host.as_deref(), Some("example.com"));
        assert_eq!(url.path, "/path");
        assert_eq!(url.username, "");
        assert_eq!(url.password, None);
        assert_eq!(url.port, None);
    }

    #[test]
    fn url_with_port() {
        let url = ParsedUrl::parse("http://example.com:8080/path").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.host.as_deref(), Some("example.com"));
        assert_eq!(url.port, Some(8080));
        assert_eq!(url.path, "/path");
    }

    #[test]
    fn url_with_user() {
        let url = ParsedUrl::parse("http://user@example.com/path").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.username, "user");
        assert_eq!(url.host.as_deref(), Some("example.com"));
        assert_eq!(url.path, "/path");
    }

    #[test]
    fn url_with_user_and_password() {
        let url = ParsedUrl::parse("http://user:pass@example.com/path").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.username, "user");
        assert_eq!(url.password.as_deref(), Some("pass"));
        assert_eq!(url.host.as_deref(), Some("example.com"));
        assert_eq!(url.path, "/path");
    }

    #[test]
    fn url_with_ipv6() {
        let url = ParsedUrl::parse("http://[::1]/path").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.host.as_deref(), Some("[::1]"));
        assert_eq!(url.path, "/path");
    }

    #[test]
    fn url_with_ipv6_and_port() {
        let url = ParsedUrl::parse("http://[::1]:8080/path").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.host.as_deref(), Some("[::1]"));
        assert_eq!(url.port, Some(8080));
        assert_eq!(url.path, "/path");
    }

    #[test]
    fn git_schemes_allow_unbracketed_ipv6() {
        for scheme in ["git", "ssh", "git+ssh", "ssh+git"] {
            let url = ParsedUrl::parse(&format!("{scheme}://user@::1/repo"))
                .expect("Git schemes allow unbracketed IPv6 hosts");
            assert_eq!(url.host.as_deref(), Some("::1"), "the IPv6 address is the host");
            assert_eq!(url.path, "/repo", "the path remains separate from the IPv6 host");
        }
    }

    #[test]
    fn malformed_authorities_are_rejected() {
        for (url, message) in [
            (
                r"http://redirected.example\@original.example/repo",
                "backslashes in the authority must be rejected",
            ),
            ("http://example.com:abc/", "non-numeric ports must be rejected"),
            ("http://foo:bar:baz/", "unbracketed colons must be rejected"),
            ("http://[not-ip]/", "bracketed hosts must be valid IPv6 addresses"),
            ("http://bücher.example/", "non-ASCII hostnames must be rejected"),
            ("http://::1/", "unbracketed IPv6 addresses must be rejected for HTTP"),
        ] {
            assert!(ParsedUrl::parse(url).is_err(), "{message}");
        }
    }

    #[test]
    fn utf8_user_information_is_accepted() {
        let url = ParsedUrl::parse("ssh://jörg:passwörd@example.com/repo").expect("valid UTF-8 user information");
        assert_eq!(url.username, "jörg", "the username is preserved");
        assert_eq!(url.password.as_deref(), Some("passwörd"), "the password is preserved");
    }

    #[test]
    fn url_with_space_in_host_is_rejected() {
        assert!(ParsedUrl::parse("http://has a space").is_err());
        assert!(ParsedUrl::parse("http://has a space/path").is_err());
        assert!(ParsedUrl::parse("https://example.com with space/path").is_err());
    }

    #[test]
    fn url_with_tab_in_host_is_rejected() {
        assert!(ParsedUrl::parse("http://has\ta\ttab").is_err());
    }

    #[test]
    fn url_with_newline_in_host_is_rejected() {
        assert!(ParsedUrl::parse("http://has\na\nnewline").is_err());
    }

    #[test]
    fn url_with_percent_encoded_username() {
        let url = ParsedUrl::parse("http://user%20name@example.com/path").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.username, "user name");
        assert_eq!(url.password, None);
        assert_eq!(url.host.as_deref(), Some("example.com"));
        assert_eq!(url.path, "/path");
    }

    #[test]
    fn url_with_percent_encoded_password() {
        let url = ParsedUrl::parse("http://user:pass%20word@example.com/path").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.username, "user");
        assert_eq!(url.password.as_deref(), Some("pass word"));
        assert_eq!(url.host.as_deref(), Some("example.com"));
        assert_eq!(url.path, "/path");
    }

    #[test]
    fn url_with_percent_encoded_username_and_password() {
        let url = ParsedUrl::parse("http://user%20name:pass%20word@example.com/path").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.username, "user name");
        assert_eq!(url.password.as_deref(), Some("pass word"));
        assert_eq!(url.host.as_deref(), Some("example.com"));
        assert_eq!(url.path, "/path");
    }

    #[test]
    fn url_with_special_chars_in_username() {
        let url = ParsedUrl::parse("http://user%40name@example.com/path").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.username, "user@name");
        assert_eq!(url.password, None);
        assert_eq!(url.host.as_deref(), Some("example.com"));
        assert_eq!(url.path, "/path");
    }

    #[test]
    fn url_with_special_chars_in_password() {
        let url = ParsedUrl::parse("http://user:p%40ss%3Aword@example.com/path").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.username, "user");
        assert_eq!(url.password.as_deref(), Some("p@ss:word"));
        assert_eq!(url.host.as_deref(), Some("example.com"));
        assert_eq!(url.path, "/path");
    }

    #[test]
    fn url_with_percent_encoded_path() {
        let url = ParsedUrl::parse("http://example.com/path/with%20spaces/file").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.host.as_deref(), Some("example.com"));
        assert_eq!(url.path, "/path/with spaces/file");
    }
}
