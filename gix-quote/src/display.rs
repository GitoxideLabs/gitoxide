use std::io::Write;

use bstr::{BStr, ByteSlice};

/// Return arbitrary bytes as text safe for display, quoting the entire value with its debug
/// representation if it contains control characters, quotes, backslashes, or invalid UTF-8.
///
/// Examples (input bytes in Rust syntax → displayed text):
/// - `b"hello world"` → `hello world` (no escaping).
/// - `b"hello\nworld"` → `"hello\nworld"` (newline escaped and the entire value quoted).
///
/// Safe text borrows `input`; quoted text borrows the reusable `scratch` buffer, which is cleared
/// on each call. This quoting is for display, not for passing arguments to a shell.
pub fn for_display<'a>(input: &'a BStr, scratch: &'a mut Vec<u8>) -> &'a BStr {
    scratch.clear();
    write!(scratch, "{input:?}").expect("writing to a byte buffer cannot fail");
    let debug_matches_input = scratch
        .strip_prefix(b"\"")
        .and_then(|debug| debug.strip_suffix(b"\""))
        .is_some_and(|debug| debug == input);
    if debug_matches_input { input } else { scratch.as_bstr() }
}
