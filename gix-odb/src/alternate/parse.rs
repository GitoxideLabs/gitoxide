use gix_error::{ExnMessageResult, Message, ResultExt};
use std::{borrow::Cow, path::PathBuf};

use gix_object::bstr::ByteSlice;

/// Parse the raw contents of an `objects/info/alternates` file from `input` into paths.
///
/// Empty entries and comments are ignored. Entries beginning with `"` use Git's C-style quoting,
/// which permits literal newlines in paths. Invalid quoting falls back to the raw entry.
/// Path conversion failures include [metadata](gix_error::Exn::metadata()) `input` (bytes), the original alternates
/// entry.
pub fn parse(mut input: &[u8]) -> ExnMessageResult<Vec<PathBuf>> {
    let mut out = Vec::new();
    while !input.is_empty() {
        let entry = input.as_bstr();
        let end_of_line = || entry.find_byte(b'\n').unwrap_or(entry.len());
        let (path, consumed) = if entry.starts_with(b"#") {
            (None, end_of_line())
        } else {
            // Like Git, try unquoting before treating a newline as the next separator.
            match entry.starts_with(b"\"").then(|| gix_quote::ansi_c::undo(entry)) {
                Some(Ok((unquoted, consumed))) => (Some(unquoted), consumed),
                _ => {
                    let consumed = end_of_line();
                    (Some(Cow::Borrowed(entry[..consumed].as_bstr())), consumed)
                }
            }
        };
        let original = &entry[..consumed];
        let maybe_nl = usize::from(consumed < input.len());
        input = &input[consumed + maybe_nl..];

        let Some(path) = path.filter(|path| !path.is_empty()) else {
            continue;
        };
        out.push(
            gix_path::try_from_bstr(path)
                .or_raise(|| Message::new("Could not obtain an alternate object directory").with("input", original))?
                .into_owned(),
        );
    }
    Ok(out)
}
