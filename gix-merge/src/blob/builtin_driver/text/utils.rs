use crate::blob::builtin_driver::text::ConflictStyle;
use bstr::{BStr, ByteSlice, ByteVec};
use std::iter::Peekable;
use std::ops::Range;

/// Used only when `diff3` is the conflict style as `zdiff3` automatically reduces hunks into nothing.
/// Here we check if all hunks are the same.
pub fn hunks_differ_in_diff3(
    style: ConflictStyle,
    a: &[Hunk],
    b: &[Hunk],
    input: &imara_diff::intern::InternedInput<&[u8]>,
    current_tokens: &[imara_diff::intern::Token],
) -> bool {
    if style != ConflictStyle::Diff3 {
        return true;
    }

    let tokens_for_hunk =
        |hunk: &Hunk| -> &[imara_diff::intern::Token] { tokens_for_side(hunk.side, input, current_tokens) };

    a.iter()
        .flat_map(tokens_for_hunk)
        .ne(b.iter().flat_map(tokens_for_hunk))
}

pub fn contains_lines(hunks: &[Hunk]) -> bool {
    hunks.iter().any(|h| !h.after.is_empty())
}

/// ## Deviation
///
/// This implementation definitely isn't the same as in Git, primarily because it seemed impossible
/// to understand what's going on there without investing more time than it seemed worth.
pub fn detect_line_ending(
    hunks: &[Hunk],
    input: &mut imara_diff::intern::InternedInput<&[u8]>,
    current_tokens: &[imara_diff::intern::Token],
) -> Option<&'static BStr> {
    fn is_eol_crlf(
        hunks: &[Hunk],
        input: &mut imara_diff::intern::InternedInput<&[u8]>,
        current_tokens: &[imara_diff::intern::Token],
    ) -> Option<bool> {
        let (range, side) = hunks.iter().rev().find_map(|h| {
            (!h.after.is_empty())
                .then_some((&h.after, h.side))
                .or((!h.before.is_empty()).then_some((&h.before, Side::Ancestor)))
        })?;

        let tokens = tokens_for_side(side, input, current_tokens);
        {
            let last_line = tokens
                .get(range.end as usize - 1)
                .map(|token| &input.interner[*token])?;
            if last_line.last() == Some(&b'\n') {
                return last_line.get(last_line.len().checked_sub(2)?).map(|c| *c == b'\r');
            }
        }
        let second_to_last_line = tokens
            .get(range.end.checked_sub(2)? as usize)
            .map(|token| &input.interner[*token])?;
        second_to_last_line
            .get(second_to_last_line.len().checked_sub(2)?)
            .map(|c| *c == b'\r')
    }
    is_eol_crlf(hunks, input, current_tokens).map(|is_crlf| if is_crlf { b"\r\n".into() } else { b"\n".into() })
}

pub fn detect_line_ending_or_nl(
    hunks: &[Hunk],
    input: &mut imara_diff::intern::InternedInput<&[u8]>,
    current_tokens: &[imara_diff::intern::Token],
) -> &'static BStr {
    detect_line_ending(hunks, input, current_tokens).unwrap_or(b"\n".into())
}

fn tokens_for_side<'a>(
    side: Side,
    input: &'a imara_diff::intern::InternedInput<&[u8]>,
    current_tokens: &'a [imara_diff::intern::Token],
) -> &'a [imara_diff::intern::Token] {
    match side {
        Side::Current => current_tokens,
        Side::Other => &input.after,
        Side::Ancestor => &input.before,
    }
}

pub fn assure_ends_with_nl(out: &mut Vec<u8>, nl: &BStr) {
    if !out.is_empty() && !out.ends_with(b"\n") {
        out.push_str(nl);
    }
}

pub fn write_conflict_marker(out: &mut Vec<u8>, marker: u8, label: Option<&BStr>, marker_size: u8, nl: &BStr) {
    assure_ends_with_nl(out, nl);
    out.extend(std::iter::repeat(marker).take(marker_size as usize));
    if let Some(label) = label {
        out.push(b' ');
        out.extend_from_slice(label);
    }
    out.push_str(nl);
}

pub fn write_ancestor(input: &imara_diff::intern::InternedInput<&[u8]>, from: u32, to: usize, out: &mut Vec<u8>) {
    if to < from as usize {
        return;
    }
    if let Some(tokens) = input.before.get(from as usize..to) {
        write_tokens(&input.interner, tokens, out);
    }
}

/// Look at all hunks in `in_out` and fill in the ancestor in the range of `ancestor_range`.
/// This is all based on knowing the ranges are sequences of tokens.
pub fn fill_ancestor(Range { start, end }: &Range<u32>, in_out: &mut Vec<Hunk>) {
    fn is_nonzero(num: &u32) -> bool {
        *num > 0
    }
    if in_out.is_empty() {
        return;
    }
    let first = &in_out[0];
    let mut first_idx = 0;
    if let Some(lines_to_add) = first.before.start.checked_sub(*start).filter(is_nonzero) {
        in_out.insert(0, ancestor_hunk(*start, lines_to_add));
        first_idx += 1;
    }

    let mut added_hunks = false;
    for (idx, next_idx) in (first_idx..in_out.len()).map(|idx| (idx, idx + 1)) {
        let Some(next_hunk) = in_out.get(next_idx) else { break };
        let hunk = &in_out[idx];
        if let Some(lines_to_add) = next_hunk.before.start.checked_sub(hunk.before.end).filter(is_nonzero) {
            in_out.push(ancestor_hunk(hunk.before.end, lines_to_add));
            added_hunks = true;
        }
    }
    let in_out_len = in_out.len();
    if added_hunks {
        in_out[first_idx..in_out_len].sort_by_key(|hunk| hunk.before.start);
    }

    let last = &in_out[in_out_len - 1];
    if let Some(lines_to_add) = end.checked_sub(last.before.end).filter(is_nonzero) {
        in_out.push(ancestor_hunk(last.before.end, lines_to_add));
    }
}

fn ancestor_hunk(start: u32, num_lines: u32) -> Hunk {
    let range = start..start + num_lines;
    Hunk {
        before: range.clone(),
        after: range,
        side: Side::Ancestor,
    }
}

/// Reduce the area of `a_hunks` and the hunks in `b_hunks` so that only those lines that are
/// actually different remain. Note that we have to compare the resolved values, not only the tokens,
/// so `current_tokens` is expected to be known to the `input` (and its `interner`).
/// Hunks from all input arrays maybe removed in the process from the front and back, in case they
/// are entirely equal to what's in `hunk`. Note also that `a_hunks` and `b_hunks` are treated to be consecutive,
/// so [`fill_ancestor()`] must have been called beforehand, and are assumed to covert the same space in the
/// ancestor buffer.
/// Use `mode` to determine how hunks may be handled.
///
/// Return a new vector of all the hunks that were removed from front and back, with partial hunks inserted,
/// along with the amount of hunks that go front, with the remaining going towards the back.
#[must_use]
pub fn zealously_contract_hunks(
    a_hunks: &mut Vec<Hunk>,
    b_hunks: &mut Vec<Hunk>,
    input: &imara_diff::intern::InternedInput<&[u8]>,
    current_tokens: &[imara_diff::intern::Token],
) -> (Vec<Hunk>, usize) {
    let line_content = |token_idx: u32, side: Side| {
        let tokens = match side {
            Side::Current => current_tokens,
            Side::Other => &input.after,
            Side::Ancestor => &input.before,
        };
        &input.interner[tokens[token_idx as usize]]
    };
    let (mut last_a_hunk_idx, mut last_b_hunk_idx) = (0, 0);
    let (mut out, hunks_in_front) = {
        let (mut remove_leading_a_hunks_from, mut remove_leading_b_hunks_from) = (None, None);
        let (mut a_hunk_token_equal_till, mut b_hunk_token_equal_till) = (None, None);
        for ((a_token_idx, a_hunk_idx, a_hunk_side), (b_token_idx, b_hunk_idx, b_hunk_side)) in
            iterate_hunks(a_hunks).zip(iterate_hunks(b_hunks))
        {
            let a_line = line_content(a_token_idx, a_hunk_side).as_bstr();
            let b_line = line_content(b_token_idx, b_hunk_side).as_bstr();

            if last_a_hunk_idx != a_hunk_idx {
                a_hunk_token_equal_till = None;
                last_a_hunk_idx = a_hunk_idx;
            }
            if last_b_hunk_idx != b_hunk_idx {
                b_hunk_token_equal_till = None;
                last_b_hunk_idx = b_hunk_idx;
            }
            if a_line == b_line {
                (remove_leading_a_hunks_from, remove_leading_b_hunks_from) = (Some(a_hunk_idx), Some(b_hunk_idx));
                (a_hunk_token_equal_till, b_hunk_token_equal_till) = (Some(a_token_idx), Some(b_token_idx));
            } else {
                break;
            }
        }

        let mut out = Vec::with_capacity(remove_leading_a_hunks_from.unwrap_or_else(|| {
            if a_hunk_token_equal_till.is_some() {
                1
            } else {
                0
            }
        }));
        truncate_hunks_from_from_front(
            a_hunks,
            remove_leading_a_hunks_from,
            a_hunk_token_equal_till,
            Some(&mut out),
        );
        truncate_hunks_from_from_front(b_hunks, remove_leading_b_hunks_from, b_hunk_token_equal_till, None);
        let hunks_in_front = out.len();
        (out, hunks_in_front)
    };

    (last_a_hunk_idx, last_b_hunk_idx) = (0, 0);
    {
        let (mut remove_trailing_a_hunks_from, mut remove_trailing_b_hunks_from) = (None, None);
        let (mut a_hunk_token_equal_from, mut b_hunk_token_equal_from) = (None, None);
        for ((a_token_idx, a_hunk_idx, a_hunk_side), (b_token_idx, b_hunk_idx, b_hunk_side)) in
            iterate_hunks_rev(a_hunks).zip(iterate_hunks_rev(b_hunks))
        {
            let a_line = line_content(a_token_idx, a_hunk_side).as_bstr();
            let b_line = line_content(b_token_idx, b_hunk_side).as_bstr();

            if last_a_hunk_idx != a_hunk_idx {
                a_hunk_token_equal_from = None;
                last_a_hunk_idx = a_hunk_idx;
            }
            if last_b_hunk_idx != b_hunk_idx {
                b_hunk_token_equal_from = None;
                last_b_hunk_idx = b_hunk_idx;
            }

            if a_line == b_line {
                (remove_trailing_a_hunks_from, remove_trailing_b_hunks_from) = (Some(a_hunk_idx), Some(b_hunk_idx));
                (a_hunk_token_equal_from, b_hunk_token_equal_from) = (Some(a_token_idx), Some(b_token_idx));
            } else {
                break;
            }
        }

        truncate_hunks_from_from_back(
            a_hunks,
            remove_trailing_a_hunks_from,
            a_hunk_token_equal_from,
            Some(&mut out),
        );
        truncate_hunks_from_from_back(b_hunks, remove_trailing_b_hunks_from, b_hunk_token_equal_from, None);
    }

    (out, hunks_in_front)
}

fn range_by_side(hunk: &mut Hunk) -> &mut Range<u32> {
    match hunk.side {
        Side::Current | Side::Other => &mut hunk.after,
        Side::Ancestor => &mut hunk.before,
    }
}
fn truncate_hunks_from_from_front(
    hunks: &mut Vec<Hunk>,
    hunks_to_remove_until_idx: Option<usize>,
    hunk_token_equal_till: Option<u32>,
    mut out_hunks: Option<&mut Vec<Hunk>>,
) {
    let Some(hunks_to_remove_until_idx) = hunks_to_remove_until_idx else {
        assert!(hunk_token_equal_till.is_none());
        return;
    };
    let mut last_index_to_remove = Some(hunks_to_remove_until_idx);
    let hunk = &mut hunks[hunks_to_remove_until_idx];
    let range = range_by_side(hunk);
    if let Some(hunk_token_equal_till) = hunk_token_equal_till {
        let orig_start = range.start;
        let new_start = hunk_token_equal_till + 1;
        range.start = new_start;
        if Range::<u32>::is_empty(range) {
            range.start = orig_start;
        } else if let Some(out) = out_hunks.as_deref_mut() {
            last_index_to_remove = hunks_to_remove_until_idx.checked_sub(1);
            let mut removed_hunk = hunk.clone();
            let new_range = range_by_side(&mut removed_hunk);

            new_range.start = orig_start;
            new_range.end = new_start;

            out.push(removed_hunk);
        } else {
            last_index_to_remove = hunks_to_remove_until_idx.checked_sub(1);
        }
    }
    if let Some(last_index_to_remove) = last_index_to_remove {
        let mut current_idx = 0;
        hunks.retain(|hunk| {
            if current_idx > last_index_to_remove {
                true
            } else {
                current_idx += 1;
                if let Some(out) = out_hunks.as_deref_mut() {
                    out.push(hunk.clone());
                }
                false
            }
        });
    }
}

fn truncate_hunks_from_from_back(
    hunks: &mut Vec<Hunk>,
    remove_trailing_hunks_from_idx: Option<usize>,
    hunk_token_equal_from: Option<u32>,
    mut out_hunks: Option<&mut Vec<Hunk>>,
) {
    let Some(mut remove_trailing_hunks_from_idx) = remove_trailing_hunks_from_idx else {
        assert!(hunk_token_equal_from.is_none());
        return;
    };

    let hunk = &mut hunks[remove_trailing_hunks_from_idx];
    let range = range_by_side(hunk);
    if let Some(hunk_token_equal_from) = hunk_token_equal_from {
        let orig_end = range.end;
        let new_end = hunk_token_equal_from;
        range.end = new_end;
        if Range::<u32>::is_empty(range) {
            range.end = orig_end;
        } else if let Some(out) = out_hunks.as_deref_mut() {
            remove_trailing_hunks_from_idx += 1;
            let mut removed_hunk = hunk.clone();
            let new_range = range_by_side(&mut removed_hunk);

            new_range.start = new_end;
            new_range.end = orig_end;

            out.push(removed_hunk);
        } else {
            remove_trailing_hunks_from_idx += 1;
        }
    }
    if let Some(out) = out_hunks {
        out.extend_from_slice(&hunks[remove_trailing_hunks_from_idx..]);
    }
    hunks.truncate(remove_trailing_hunks_from_idx);
}

/// Return an iterator over `(token_idx, hunk_idx, hunk_side)` from `hunks`.
fn iterate_hunks(hunks: &[Hunk]) -> impl Iterator<Item = (u32, usize, Side)> + '_ {
    hunks.iter().enumerate().flat_map(|(hunk_idx, hunk)| {
        match hunk.side {
            Side::Current | Side::Other => &hunk.after,
            Side::Ancestor => &hunk.before,
        }
        .clone()
        .map(move |idx| (idx, hunk_idx, hunk.side))
    })
}

/// Return a reverse iterator over `(token_idx, hunk_idx, hunk_side)` from `hunks`.
fn iterate_hunks_rev(hunks: &[Hunk]) -> impl Iterator<Item = (u32, usize, Side)> + '_ {
    hunks.iter().enumerate().rev().flat_map(|(hunk_idx, hunk)| {
        match hunk.side {
            Side::Current | Side::Other => &hunk.after,
            Side::Ancestor => &hunk.before,
        }
        .clone()
        .rev()
        .map(move |idx| (idx, hunk_idx, hunk.side))
    })
}

pub fn write_hunks(
    hunks: &[Hunk],
    input: &imara_diff::intern::InternedInput<&[u8]>,
    current_tokens: &[imara_diff::intern::Token],
    out: &mut Vec<u8>,
) {
    for hunk in hunks {
        let (tokens, range) = match hunk.side {
            Side::Current => (current_tokens, &hunk.after),
            Side::Other => (input.after.as_slice(), &hunk.after),
            Side::Ancestor => (input.before.as_slice(), &hunk.before),
        };
        write_tokens(&input.interner, &tokens[usize_range(range)], out);
    }
}

fn usize_range(range: &Range<u32>) -> Range<usize> {
    range.start as usize..range.end as usize
}

fn write_tokens(
    interner: &imara_diff::intern::Interner<&[u8]>,
    tokens: &[imara_diff::intern::Token],
    out: &mut Vec<u8>,
) {
    for token in tokens {
        out.extend_from_slice(interner[*token]);
    }
}

/// Find all hunks in `iter` which aren't from the same side as `hunk` and intersect with it.
/// Also put `hunk` into `input` so it's the first item, and possibly put more hunks of the side of `hunk` so
/// `iter` doesn't have any overlapping hunks left.
/// Return `true` if `intersecting` is non-empty after the operation, indicating overlapping hunks were found.
pub fn take_intersecting(
    iter: &mut Peekable<impl Iterator<Item = Hunk>>,
    input: &mut Vec<Hunk>,
    intersecting: &mut Vec<Hunk>,
) -> Option<()> {
    input.clear();
    input.push(iter.next()?);
    intersecting.clear();

    fn left_overlaps_right(left: &Hunk, right: &Hunk) -> bool {
        left.side != right.side
            && (right.before.contains(&left.before.start)
                || (right.before.is_empty() && right.before.start == left.before.start))
    }

    loop {
        let hunk = input.last().expect("just pushed");
        while iter.peek().filter(|b_hunk| left_overlaps_right(b_hunk, hunk)).is_some() {
            intersecting.extend(iter.next());
        }
        // The hunks that overlap might themselves overlap with a following hunk of the other side.
        // If so, split it so it doesn't overlap anymore.
        let mut found_more_intersections = false;
        while intersecting
            .last_mut()
            .zip(iter.peek_mut())
            .filter(|(last_intersecting, candidate)| left_overlaps_right(candidate, last_intersecting))
            .is_some()
        {
            input.extend(iter.next());
            found_more_intersections = true;
        }
        if !found_more_intersections {
            break;
        }
    }
    Some(())
}

pub fn tokens(input: &[u8]) -> imara_diff::sources::ByteLines<'_, true> {
    imara_diff::sources::byte_lines_with_terminator(input)
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Side {
    Current,
    Other,
    /// A special marker that is just used to be able to mix-in hunks that only point to the ancestor.
    /// Only `before` matters then.
    Ancestor,
}

#[derive(Debug, Clone)]
pub struct Hunk {
    pub before: Range<u32>,
    pub after: Range<u32>,
    pub side: Side,
}

pub struct CollectHunks {
    pub hunks: Vec<Hunk>,
    pub side: Side,
}

impl imara_diff::Sink for CollectHunks {
    type Out = Vec<Hunk>;

    fn process_change(&mut self, before: Range<u32>, after: Range<u32>) {
        self.hunks.push(Hunk {
            before,
            after,
            side: self.side,
        });
    }

    fn finish(self) -> Self::Out {
        self.hunks
    }
}
