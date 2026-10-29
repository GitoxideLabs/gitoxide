use bstr::ByteSlice;
use percent_encoding::percent_decode;

// Only shell-static definitions can be handled without executing their configured shell expressions.
pub(super) fn is_static(input: &[u8]) -> bool {
    let mut quote = None;
    let mut bytes = input.iter().copied();
    while let Some(byte) = bytes.next() {
        match (quote, byte) {
            (Some(b'\''), b'\'') | (Some(b'"'), b'"') => quote = None,
            (Some(b'\''), _) => {}
            (_, b'\\') => {
                bytes.next();
            }
            (Some(b'"'), b'$' | b'`') => return false,
            (Some(b'"'), _) => {}
            (None, b'\'' | b'"') => quote = Some(byte),
            (
                None,
                b'|' | b'&' | b';' | b'<' | b'>' | b'(' | b')' | b'$' | b'`' | b'\n' | b'*' | b'?' | b'[' | b'#' | b'~'
                | b'%',
            ) => return false,
            _ => {}
        }
    }
    quote.is_none()
}

pub(super) struct Url {
    pub protocol: Vec<u8>,
    pub host: Vec<u8>,
    pub username: Option<Vec<u8>>,
    pub password: Option<Vec<u8>>,
    pub path: Option<Vec<u8>>,
}

pub(super) fn parse_url(input: &[u8]) -> Option<Url> {
    // Credential keys are literal URL components: normalization would change their identity.
    let scheme_end = input.find(b"://")?;
    if scheme_end == 0 {
        return None;
    }
    let rest = &input[scheme_end + 3..];
    let host_end = rest.find_byteset(b"/?#").unwrap_or(rest.len());
    let authority = &rest[..host_end];
    let mut parsed = Url {
        protocol: input[..scheme_end].into(),
        host: Vec::new(),
        username: None,
        password: None,
        path: None,
    };
    let host = match authority.find_byte(b'@') {
        None => authority,
        Some(at) => {
            let userinfo = &authority[..at];
            let (user, password) = match userinfo.find_byte(b':') {
                Some(colon) => (&userinfo[..colon], Some(&userinfo[colon + 1..])),
                None => (userinfo, None),
            };
            parsed.username = Some(percent_decode(user).collect());
            parsed.password = password.map(|value| percent_decode(value).collect());
            &authority[at + 1..]
        }
    };
    parsed.host = percent_decode(host).collect();
    let path = rest[host_end..].trim_start_with(|byte| byte == '/');
    if !path.is_empty() {
        let path: Vec<u8> = percent_decode(path).collect();
        let path = path.trim_end_with(|byte| byte == '/');
        parsed.path = Some(if path.is_empty() { b"/".to_vec() } else { path.into() });
    }
    for value in [
        Some(&parsed.protocol),
        Some(&parsed.host),
        parsed.username.as_ref(),
        parsed.password.as_ref(),
        parsed.path.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        if value.find_byteset(b"\0\n").is_some() {
            return None;
        }
    }
    Some(parsed)
}
