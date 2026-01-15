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
#[allow(missing_docs)]
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

        // Find path start (first '/' after scheme)
        let path_start = after_scheme.find('/').unwrap_or(after_scheme.len());
        let authority = &after_scheme[..path_start];
        let path = if path_start < after_scheme.len() {
            percent_decode(&after_scheme[path_start..])?
        } else {
            // No path specified - leave empty (caller can default to / if needed)
            String::new()
        };

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

            let (h, p) = Self::parse_host_port(host_port)?;
            // If we have user info, we must have a host
            if h.is_none() {
                return Err(UrlParseError::InvalidDomainCharacter);
            }
            (user, pass, h, p)
        } else {
            // No user info
            let (h, p) = Self::parse_host_port(authority)?;
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

    fn parse_host_port(host_port: &str) -> Result<(Option<String>, Option<u16>), UrlParseError> {
        if host_port.is_empty() {
            return Ok((None, None));
        }

        // Handle IPv6 addresses: [::1] or [::1]:port
        if host_port.starts_with('[') {
            if let Some(bracket_end) = host_port.find(']') {
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

        // Handle regular host:port
        // Use rfind to find the last colon
        if let Some((before_last_colon, after_last_colon)) = host_port.rsplit_once(':') {
            // Check if this looks like a port (all digits after colon)
            // But avoid treating IPv6 addresses as host:port
            // IPv6 addresses have colons in the part before the last colon (e.g., "::1" has "::" before the last ":")
            let has_colon_before_last = before_last_colon.contains(':');
            let is_all_digits_after =
                !after_last_colon.is_empty() && after_last_colon.chars().all(|c| c.is_ascii_digit());

            // Treat as port separator only if:
            // 1. There's no colon before the last colon (normal host:port)
            // 2. OR it's explicitly empty (host: with trailing colon)
            if !has_colon_before_last {
                if after_last_colon.is_empty() {
                    // Empty port like "host:" - store host with trailing colon
                    // This is needed for Git compatibility where "host:" != "host"
                    return Ok((Some(Self::normalize_hostname(host_port)?), None));
                } else if is_all_digits_after {
                    let host = Self::normalize_hostname(before_last_colon)?;
                    let port = after_last_colon
                        .parse::<u16>()
                        .map_err(|_| UrlParseError::InvalidPort)?;
                    // Validate port is in valid range (1-65535, port 0 is invalid)
                    if port == 0 {
                        return Err(UrlParseError::InvalidPort);
                    }
                    return Ok((Some(host), Some(port)));
                }
            }
        }

        // No port, just host (including bare IPv6 addresses)
        Ok((Some(Self::normalize_hostname(host_port)?), None))
    }

    /// Check if a string looks like a valid DNS hostname (for normalization purposes)
    /// Valid DNS names contain only alphanumeric, hyphens, dots, underscores, and wildcards
    fn is_normalizable_hostname(host: &str) -> bool {
        // Allow alphanumeric, -, ., _, and * (for patterns)
        host.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '*'))
    }

    /// Validate and possibly normalize a hostname
    /// Valid DNS hostnames are normalized to lowercase
    /// Hostnames containing ? or whitespace characters are rejected with an error
    fn normalize_hostname(host: &str) -> Result<String, UrlParseError> {
        // Reject invalid characters: ?, space, tab, newline, etc.
        // These characters are forbidden in URLs per RFC 3986
        if host.chars().any(|c| c == '?' || c.is_whitespace()) {
            return Err(UrlParseError::InvalidDomainCharacter);
        }

        // Only normalize if it looks like a valid DNS hostname
        // Preserve case for security checks if it contains special characters
        if Self::is_normalizable_hostname(host) {
            Ok(host.to_ascii_lowercase())
        } else {
            Ok(host.to_owned())
        }
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
