use bstr::ByteSlice;

use crate::{pattern, pattern::Mode};

#[inline]
/// A sloppy parser that performs only the most basic checks, providing additional information
/// using `pattern::Mode` flags.
///
/// If `may_alter` is `false`, we won't parse leading `!` or its escaped form.
///
/// Returns `(pattern, mode, no_wildcard_len)`
pub fn pattern(mut pat: &[u8], may_alter: bool) -> Option<(&[u8], pattern::Mode, Option<usize>)> {
    let mut mode = Mode::empty();
    if pat.is_empty() {
        return None;
    }
    if may_alter {
        if pat.first() == Some(&b'!') {
            mode |= Mode::NEGATIVE;
            pat = &pat[1..];
        } else if pat.first() == Some(&b'\\') {
            let second = pat.get(1);
            if second == Some(&b'!') || second == Some(&b'#') {
                pat = &pat[1..];
            }
        }
    }
    if pat.iter().all(u8::is_ascii_whitespace) {
        return None;
    }
    if pat.first() == Some(&b'/') {
        mode |= Mode::ABSOLUTE;
        pat = &pat[1..];
    }
    if pat.last() == Some(&b'/') {
        mode |= Mode::MUST_BE_DIR;
        pat = &pat[..pat.len() - 1];
    }

    if !pat.contains(&b'/') {
        mode |= Mode::NO_SUB_DIR;
    }
    if pat.first() == Some(&b'*') && first_wildcard_pos(&pat[1..]).is_none() {
        mode |= Mode::ENDS_WITH;
    }

    let pos_of_first_wildcard = first_wildcard_pos(pat);
    Some((pat, mode, pos_of_first_wildcard))
}

fn first_wildcard_pos(pat: &[u8]) -> Option<usize> {
    pat.find_byteset(GLOB_CHARACTERS)
}

pub(crate) const GLOB_CHARACTERS: &[u8] = br"*?[\";
