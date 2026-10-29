use bstr::ByteSlice;

use super::{MessageId as M, Reporter};

pub(super) fn lines<'a>(data: &'a [u8], out: &mut Reporter<'_>) -> Option<impl Iterator<Item = &'a [u8]> + use<'a>> {
    let end = if let Some(end) = data.find(b"\n\n") {
        end + 1
    } else if data.ends_with(b"\n") {
        data.len()
    } else {
        out.report(M::UnterminatedHeader);
        return None;
    };
    let headers = &data[..end];
    if headers.contains(&0) {
        out.report(M::NulInHeader);
        return None;
    }
    Some(headers.split(|byte| *byte == b'\n').filter(|line| !line.is_empty()))
}

pub(super) fn hash(value: &[u8], hash: gix_hash::Kind) -> bool {
    value.len() == hash.len_in_hex() && value.iter().all(u8::is_ascii_hexdigit)
}

pub(super) fn identity(value: &[u8], out: &mut Reporter<'_>) -> bool {
    let Some(open) = value.iter().position(|byte| matches!(*byte, b'<' | b'>')) else {
        return out.report(M::MissingEmail);
    };
    if value[open] == b'>' {
        return out.report(M::BadName);
    }
    if open == 0 {
        return out.report(M::MissingNameBeforeEmail);
    }
    if value[open - 1] != b' ' {
        return out.report(M::MissingSpaceBeforeEmail);
    }
    let email = &value[open + 1..];
    let Some(close) = email.iter().position(|byte| matches!(*byte, b'<' | b'>')) else {
        return out.report(M::BadEmail);
    };
    if email[close] != b'>' {
        return out.report(M::BadEmail);
    }
    let Some(date) = email[close + 1..].strip_prefix(b" ") else {
        return out.report(M::MissingSpaceBeforeDate);
    };
    let date = date.trim_start_with(|byte| byte == ' ' || byte == '\t');
    let digits = date.iter().take_while(|byte| byte.is_ascii_digit()).count();
    if digits == 0 {
        return out.report(M::BadDate);
    }
    if date[0] == b'0' && date.get(1) != Some(&b' ') {
        return out.report(M::ZeroPaddedDate);
    }
    let timestamp = date[..digits].iter().try_fold(0u64, |value, byte| {
        value.checked_mul(10)?.checked_add(u64::from(byte - b'0'))
    });
    // Git also checks that its unsigned timestamp fits signed system time_t.
    if timestamp.is_none_or(|value| value > i64::MAX as u64) {
        return out.report(M::BadDateOverflow);
    }
    let Some(zone) = date[digits..].strip_prefix(b" ") else {
        return out.report(M::BadDate);
    };
    if zone.len() != 5 || !matches!(zone[0], b'+' | b'-') || !zone[1..].iter().all(u8::is_ascii_digit) {
        return out.report(M::BadTimezone);
    }
    false
}
