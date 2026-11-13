mod client;
#[cfg(windows)]
mod client_windows;
#[cfg(feature = "server")]
mod server;

#[cfg(windows)]
fn pipe_name(endpoint: &std::path::Path) -> std::ffi::OsString {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};

    // Keep the mock server and raw client independent of production helpers. Git names its pipe
    // from the canonical endpoint, removing verbatim prefixes and converting drive colons/slashes.
    let mut path: Vec<_> = endpoint.as_os_str().encode_wide().collect();
    let prefix: Vec<_> = "\\\\?\\".encode_utf16().collect();
    let unc_prefix: Vec<_> = "\\\\?\\UNC\\".encode_utf16().collect();
    if path.starts_with(&unc_prefix) {
        path.splice(..unc_prefix.len(), "\\\\".encode_utf16());
    } else if path.starts_with(&prefix) {
        path.drain(..prefix.len());
    }
    let mut pipe: Vec<_> = "\\\\.\\pipe\\".encode_utf16().collect();
    pipe.extend(path.iter().enumerate().map(|(idx, &ch)| match ch {
        ch if ch == u16::from(b':') && idx == 1 => u16::from(b'_'),
        ch if ch == u16::from(b'/') => u16::from(b'\\'),
        ch => ch,
    }));
    std::ffi::OsString::from_wide(&pipe)
}
