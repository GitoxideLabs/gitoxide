use crate::bstr::{BStr, ByteSlice};

/// Returns title and body, without separator
pub fn message_title_and_body(input: &[u8]) -> (&BStr, Option<&BStr>) {
    let mut pos = 0;
    while pos < input.len() {
        if let Some(first_len) = newline_len(&input[pos..])
            && let Some(second_len) = newline_len(&input[pos + first_len..])
        {
            let body = &input[pos + first_len + second_len..];
            return (input[..pos].as_bstr(), (!body.is_empty()).then(|| body.as_bstr()));
        }
        pos += 1;
    }
    (input.as_bstr(), None)
}

fn newline_len(input: &[u8]) -> Option<usize> {
    if input.starts_with(b"\r\n") {
        Some(2)
    } else if input.starts_with(b"\n") {
        Some(1)
    } else {
        None
    }
}
