/// A minimal URL parser that extracts only what we need for git URLs.
/// This is a replacement for the `url` crate dependency.
#[derive(Debug)]
pub(crate) struct ParsedUrl<'a> {
    pub scheme: String, // Owned to allow normalization to lowercase
    pub username: &'a str,
    pub password: Option<&'a str>,
    pub host: Option<String>, // Owned to allow normalization to lowercase
    pub port: Option<u16>,
    pub path: &'a str,
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

impl<'a> ParsedUrl<'a> {
    /// Parse a URL string into its components.
    /// Expected format: scheme://[user[:password]@]host[:port]/path
    pub(crate) fn parse(input: &'a str) -> Result<Self, UrlParseError> {
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
            &after_scheme[path_start..]
        } else {
            // No path specified - leave empty (caller can default to / if needed)
            ""
        };

        // Parse authority: [user[:password]@]host[:port]
        let (username, password, host, port) = if let Some(at_pos) = authority.rfind('@') {
            // Has user info
            let user_info = &authority[..at_pos];
            let host_port = &authority[at_pos + 1..];

            let (user, pass) = if let Some(colon_pos) = user_info.find(':') {
                let pass_str = &user_info[colon_pos + 1..];
                // Treat empty password as None
                let pass = if pass_str.is_empty() { None } else { Some(pass_str) };
                (&user_info[..colon_pos], pass)
            } else {
                (user_info, None)
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
            ("", None, h, p)
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
        if let Some(colon_pos) = host_port.rfind(':') {
            let before_last_colon = &host_port[..colon_pos];
            let after_last_colon = &host_port[colon_pos + 1..];

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
    /// Invalid strings (like injection attempts) are preserved as-is but ? is rejected
    fn normalize_hostname(host: &str) -> Result<String, UrlParseError> {
        // Reject ? character which git's url parser rejects
        if host.contains('?') {
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
    fn test_simple_url() {
        let url = ParsedUrl::parse("http://example.com/path").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.host.as_deref(), Some("example.com"));
        assert_eq!(url.path, "/path");
        assert_eq!(url.username, "");
        assert_eq!(url.password, None);
        assert_eq!(url.port, None);
    }

    #[test]
    fn test_url_with_port() {
        let url = ParsedUrl::parse("http://example.com:8080/path").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.host.as_deref(), Some("example.com"));
        assert_eq!(url.port, Some(8080));
        assert_eq!(url.path, "/path");
    }

    #[test]
    fn test_url_with_user() {
        let url = ParsedUrl::parse("http://user@example.com/path").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.username, "user");
        assert_eq!(url.host.as_deref(), Some("example.com"));
        assert_eq!(url.path, "/path");
    }

    #[test]
    fn test_url_with_user_and_password() {
        let url = ParsedUrl::parse("http://user:pass@example.com/path").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.username, "user");
        assert_eq!(url.password, Some("pass"));
        assert_eq!(url.host.as_deref(), Some("example.com"));
        assert_eq!(url.path, "/path");
    }

    #[test]
    fn test_url_with_ipv6() {
        let url = ParsedUrl::parse("http://[::1]/path").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.host.as_deref(), Some("[::1]"));
        assert_eq!(url.path, "/path");
    }

    #[test]
    fn test_url_with_ipv6_and_port() {
        let url = ParsedUrl::parse("http://[::1]:8080/path").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.host.as_deref(), Some("[::1]"));
        assert_eq!(url.port, Some(8080));
        assert_eq!(url.path, "/path");
    }
}
