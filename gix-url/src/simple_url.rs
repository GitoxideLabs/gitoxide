use percent_encoding::percent_decode_str;

use gix_error::{ErrorExt, Exn, OptionExt, ResultExt, ValidationError};

/// A minimal URL parser that extracts only what we need for git URLs.
/// This is a replacement for the `url` crate dependency.
#[derive(Debug)]
pub(crate) struct ParsedUrl {
    pub scheme: String,
    pub username: String,         // Owned to allow percent-decoding
    pub password: Option<String>, // Owned to allow percent-decoding
    pub host: Option<String>,     // Owned to allow normalization to lowercase
    pub port: Option<u16>,
    pub path: String, // Owned to allow percent-decoding
    /// The original path when it contains percent escapes, allowing lossless serialization.
    pub path_with_percent_escapes: Option<String>,
}

fn relative_url_without_base() -> ValidationError {
    ValidationError::new("relative URL without a base")
}

fn invalid_port() -> ValidationError {
    ValidationError::new("invalid port number - must be between 1-65535")
}

fn invalid_domain_character() -> ValidationError {
    ValidationError::new("invalid domain character")
}

fn scheme_requires_host() -> ValidationError {
    ValidationError::new("Scheme requires host")
}

/// Check if a character is valid in a URL scheme.
/// Valid scheme characters: alphanumeric, +, -, or .
fn is_valid_scheme_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.'
}

fn has_valid_percent_encoding(input: &str) -> bool {
    let mut bytes = input.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'%'
            && !matches!(
                (bytes.next(), bytes.next()),
                (Some(a), Some(b)) if a.is_ascii_hexdigit() && b.is_ascii_hexdigit()
            )
        {
            return false;
        }
    }
    true
}

/// Decode a percent-encoded string, returning an error if the result is not valid UTF-8.
/// Returns the original string if it contains no percent-encoding.
fn percent_decode(s: &str) -> Result<String, Exn<ValidationError>> {
    percent_decode_str(s)
        .decode_utf8()
        .map(std::borrow::Cow::into_owned)
        .or_raise(invalid_domain_character)
}

/// Decode percent-encoded path bytes and retain the original spelling if it contains escapes.
fn percent_decode_path(s: &str) -> Result<(String, Option<String>), Exn<ValidationError>> {
    percent_decode(s).map(|path| (path, s.contains('%').then(|| s.to_owned())))
}

/// Validate and normalize the contents of a bracketed IPv6 host literal.
///
/// The IPv6 address is lowercased. An optional zone identifier must be introduced by the URI-encoded `%25`
/// delimiter and contain only unreserved or percent-encoded characters; its spelling is otherwise preserved.
/// Brackets are not part of the input or output.
///
/// Examples:
/// - `2001:DB8::1` becomes `2001:db8::1`.
/// - `fe80::1%25Eth0` remains `fe80::1%25Eth0`.
/// - `fe80::1%25` and `not-ip` are rejected with `None`.
fn normalize_ipv6_literal(host: &str) -> Option<String> {
    let Some((address, zone)) = host.split_once("%25") else {
        return host
            .parse::<std::net::Ipv6Addr>()
            .is_ok()
            .then(|| host.to_ascii_lowercase());
    };
    if address.parse::<std::net::Ipv6Addr>().is_err() || zone.is_empty() || percent_decode(zone).is_err() {
        return None;
    }
    let mut pos = 0;
    let bytes = zone.as_bytes();
    while pos < bytes.len() {
        if bytes[pos].is_ascii_alphanumeric() || matches!(bytes[pos], b'-' | b'.' | b'_' | b'~') {
            pos += 1;
        } else if bytes[pos] == b'%'
            && bytes.get(pos + 1).is_some_and(u8::is_ascii_hexdigit)
            && bytes.get(pos + 2).is_some_and(u8::is_ascii_hexdigit)
        {
            pos += 3;
        } else {
            return None;
        }
    }
    Some(format!("{}%25{zone}", address.to_ascii_lowercase()))
}

impl ParsedUrl {
    /// Parse a URL string into its components.
    /// Expected format: scheme://[user[:password]@]host[:port]/path
    pub(crate) fn parse(input: &str) -> Result<Self, Exn<ValidationError>> {
        // Validate that the entire URL doesn't contain any whitespace (per RFC 3986)
        if input.chars().any(char::is_whitespace) || !has_valid_percent_encoding(input) {
            return Err(invalid_domain_character().raise());
        }

        // Find scheme by looking for first ':'
        let first_colon = input.find(':').ok_or_raise(relative_url_without_base)?;
        let scheme_str = &input[..first_colon];
        let Some(after_scheme) = input[first_colon..].strip_prefix("://") else {
            return Err(relative_url_without_base().raise());
        };

        // Check for relative URL (scheme without proper authority)
        if scheme_str.is_empty() {
            return Err(relative_url_without_base().raise());
        }

        // Validate scheme characters (check original before lowercase conversion)
        if !scheme_str.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
            || !scheme_str.chars().all(is_valid_scheme_char)
        {
            return Err(relative_url_without_base().raise());
        }

        // Git treats query and fragment delimiters as authority text outside HTTP URLs.
        let path_start = if matches!(scheme_str, "http" | "https") {
            after_scheme.find(['/', '?', '#'])
        } else {
            after_scheme.find('/')
        }
        .unwrap_or(after_scheme.len());
        let authority = &after_scheme[..path_start];
        if authority.contains('\\') {
            return Err(invalid_domain_character().raise());
        }
        let (path, path_with_percent_escapes) = if path_start < after_scheme.len() {
            percent_decode_path(&after_scheme[path_start..])?
        } else {
            // No path specified - leave empty (caller can default to / if needed)
            (String::new(), None)
        };

        let allow_unbracketed_ipv6 = matches!(scheme_str, "git" | "ssh" | "git+ssh" | "ssh+git");
        let strict_authority = matches!(scheme_str, "http" | "https");

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

            let (h, p) = Self::parse_host_port(host_port, allow_unbracketed_ipv6, strict_authority)?;
            // If we have user info, we must have a host
            if h.is_none() {
                return Err(invalid_domain_character().raise());
            }
            (user, pass, h, p)
        } else {
            // No user info
            let (h, p) = Self::parse_host_port(authority, allow_unbracketed_ipv6, strict_authority)?;
            (String::new(), None, h, p)
        };

        // Standard schemes (http, https, git, ssh) require a host
        let requires_host = matches!(scheme_str, "http" | "https" | "git" | "ssh" | "ftp" | "ftps");
        if requires_host && host.is_none() {
            return Err(scheme_requires_host().raise());
        }

        Ok(ParsedUrl {
            scheme: scheme_str.into(),
            username,
            password,
            host,
            port,
            path,
            path_with_percent_escapes,
        })
    }

    /// `strict_authority` is set for HTTP/HTTPS.
    fn parse_host_port(
        host_port: &str,
        allow_unbracketed_ipv6: bool,
        strict_authority: bool,
    ) -> Result<(Option<String>, Option<u16>), Exn<ValidationError>> {
        if host_port.is_empty() {
            return Ok((None, None));
        }

        // Handle IPv6 addresses: [::1] or [::1]:port
        if host_port.starts_with('[') {
            if let Some(bracket_end) = host_port.find(']') {
                let inner = &host_port[1..bracket_end];
                let host = match normalize_ipv6_literal(inner) {
                    Some(host) if !strict_authority => percent_decode(&host)?,
                    Some(host) => host,
                    None if !strict_authority => percent_decode(inner)?,
                    None => return Err(invalid_domain_character().raise()),
                };
                let remaining = &host_port[bracket_end + 1..];

                if remaining.is_empty() {
                    return Ok((Some(format!("[{host}]")), None));
                } else if let Some(port_str) = remaining.strip_prefix(':') {
                    if port_str.is_empty() {
                        // Empty port like "[::1]:" - preserve the trailing colon for Git compatibility
                        return Ok((Some(format!("[{host}]:")), None));
                    }
                    if !port_str.bytes().all(|b| b.is_ascii_digit()) {
                        return Err(invalid_port().raise());
                    }
                    let port = port_str.parse::<u16>().or_raise(invalid_port)?;
                    if port == 0 && strict_authority {
                        return Err(invalid_port().raise());
                    }
                    return Ok((Some(format!("[{host}]")), Some(port)));
                } else {
                    return Err(invalid_domain_character().raise());
                }
            } else {
                return Err(invalid_domain_character().raise());
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
                return if strict_authority {
                    Err(invalid_domain_character().raise())
                } else {
                    Ok((Some(Self::normalize_git_hostname(host_port)?), None))
                };
            }
            if after_last_colon.is_empty() {
                // Empty port like "host:" - store host with trailing colon for Git compatibility.
                let mut host = if strict_authority {
                    Self::normalize_http_hostname(before_last_colon)?
                } else {
                    Self::normalize_git_hostname(before_last_colon)?
                };
                host.push(':');
                return Ok((Some(host), None));
            }
            if !after_last_colon.chars().all(|c| c.is_ascii_digit()) {
                return Err(invalid_port().raise());
            }
            let host = if strict_authority {
                Self::normalize_http_hostname(before_last_colon)?
            } else {
                Self::normalize_git_hostname(before_last_colon)?
            };
            let port = after_last_colon.parse::<u16>().or_raise(invalid_port)?;
            if port == 0 && strict_authority {
                return Err(invalid_port().raise());
            }
            return Ok((Some(host), Some(port)));
        }

        // No port, just host.
        let host = if strict_authority {
            Self::normalize_http_hostname(host_port)?
        } else {
            Self::normalize_git_hostname(host_port)?
        };
        Ok((Some(host), None))
    }

    fn is_normalizable_hostname(host: &str) -> bool {
        host.bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'.' | b'_' | b'*'))
    }

    /// Validate a hostname and normalize DNS-like ASCII hostnames to lowercase.
    /// Hostnames containing other permitted URL characters retain their original case.
    fn normalize_http_hostname(host: &str) -> Result<String, Exn<ValidationError>> {
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
            return Err(invalid_domain_character().raise());
        }
        Ok(if Self::is_normalizable_hostname(host) {
            host.to_ascii_lowercase()
        } else {
            host.to_owned()
        })
    }

    /// Decode percent escapes before normalizing DNS-like hostnames for Git-compatible, non-HTTP authorities.
    ///
    /// This is separate from [`Self::normalize_http_hostname`] because Git passes the decoded host to transports, whereas
    /// HTTP and HTTPS retain escaped host spelling and apply stricter hostname validation.
    fn normalize_git_hostname(host: &str) -> Result<String, Exn<ValidationError>> {
        let host = percent_decode(host)?;
        Ok(if Self::is_normalizable_hostname(&host) {
            host.to_ascii_lowercase()
        } else {
            host
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
    fn invalid_port_keeps_parse_error() {
        let err = ParsedUrl::parse("http://example.com:65536/path").expect_err("port is out of range");
        assert!(
            err.downcast_any_ref::<std::num::ParseIntError>().is_some(),
            "the port parser cause remains in the error chain"
        );
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
            ("http://[fe80::1%25]/repo", "IPv6 zone identifiers must not be empty"),
            (
                "http://[fe80::1%25eth!0]/repo",
                "IPv6 zone identifiers contain only unreserved or percent-encoded characters",
            ),
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
    fn malformed_schemes_and_percent_escapes_are_rejected() {
        for url in [
            "1http://example.com/",
            "http://example.com/%",
            "http://example.com/%2",
            "http://example.com/%zz",
            "http://user%zz@example.com/",
            "http://example%zz.com/",
        ] {
            assert!(ParsedUrl::parse(url).is_err(), "invalid URL {url:?} must be rejected");
        }
        assert!(
            ParsedUrl::parse("http://example.com/%2f").is_ok(),
            "hex escapes are valid"
        );
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
