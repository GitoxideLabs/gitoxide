use std::ops::Range;

use bstr::{BStr, ByteSlice};
use imara_diff::{InternedInput, Token};

use super::ConflictStyle;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    Current,
    Other,
    Conflict,
}

/// A merge region uses coordinates in all three inputs, including unchanged padding.
#[derive(Clone)]
pub struct Region {
    pub ancestor: Range<usize>,
    pub current: Range<usize>,
    pub other: Range<usize>,
    pub pick: Pick,
}

pub struct Hunk {
    before: Range<usize>,
    after: Range<usize>,
    current: bool,
}

pub fn collect_hunks(
    algorithm: imara_diff::Algorithm,
    input: &InternedInput<&[u8]>,
    current: bool,
    hunks: &mut Vec<Hunk>,
) {
    let mut diff = imara_diff::Diff::compute(algorithm, input);
    // Indentation-based placement can move overlapping edits apart and hide conflicts.
    diff.postprocess_no_heuristic(input);
    hunks.extend(diff.hunks().map(|hunk| Hunk {
        before: hunk.before.start as usize..hunk.before.end as usize,
        after: hunk.after.start as usize..hunk.after.end as usize,
        current,
    }));
}

pub fn regions(mut hunks: Vec<Hunk>, current: &[Token], other: &[Token]) -> Vec<Region> {
    hunks.sort_by_key(|hunk| hunk.before.start);
    let mut hunks = hunks.into_iter().peekable();
    let mut out = Vec::new();
    let (mut ancestor_end, mut current_end, mut other_end) = (0, 0, 0);
    while let Some(first) = hunks.next() {
        let start = first.before.start;
        let gap = start - ancestor_end;
        let current_start = current_end + gap;
        let other_start = other_end + gap;
        let mut end = first.before.end;
        let mut group = vec![first];
        // Touching base ranges can conflict too, including insertions at a deletion's boundary.
        while hunks.peek().is_some_and(|hunk| hunk.before.start <= end) {
            let hunk = hunks.next().expect("a touching hunk was peeked");
            end = end.max(hunk.before.end);
            group.push(hunk);
        }
        current_end = current_start + end - start;
        other_end = other_start + end - start;
        let (mut current_changed, mut other_changed) = (false, false);
        for hunk in group {
            let (side_end, changed) = if hunk.current {
                (&mut current_end, &mut current_changed)
            } else {
                (&mut other_end, &mut other_changed)
            };
            *side_end = *side_end + hunk.after.len() - hunk.before.len();
            *changed = true;
        }
        let current_range = current_start..current_end;
        let other_range = other_start..other_end;
        let pick = if !current_changed {
            Pick::Other
        } else if !other_changed || current[current_range.clone()] == other[other_range.clone()] {
            Pick::Current
        } else {
            Pick::Conflict
        };
        out.push(Region {
            ancestor: start..end,
            current: current_range,
            other: other_range,
            pick,
        });
        ancestor_end = end;
    }
    out
}

/// Refine conflict regions before rendering or applying a content favor policy.
pub fn refine(
    regions: &[Region],
    style: ConflictStyle,
    algorithm: imara_diff::Algorithm,
    input: &InternedInput<&[u8]>,
    current: &[Token],
) -> Vec<Region> {
    let mut out: Vec<Region> = Vec::new();
    let mut diff = imara_diff::Diff::default();
    for original in regions {
        let mut region = original.clone();
        if region.pick != Pick::Conflict || style == ConflictStyle::Diff3 {
            out.push(region);
            continue;
        }
        if style == ConflictStyle::ZealousDiff3 {
            while !region.current.is_empty()
                && !region.other.is_empty()
                && current[region.current.start] == input.after[region.other.start]
            {
                region.current.start += 1;
                region.other.start += 1;
            }
            while !region.current.is_empty()
                && !region.other.is_empty()
                && current[region.current.end - 1] == input.after[region.other.end - 1]
            {
                region.current.end -= 1;
                region.other.end -= 1;
            }
            out.push(region);
            continue;
        }
        if region.current.is_empty() || region.other.is_empty() {
            out.push(region);
            continue;
        }
        let ours = &current[region.current.clone()];
        let theirs = &input.after[region.other.clone()];
        diff.compute_with(algorithm, ours, theirs, input.interner.num_tokens());
        diff.postprocess_with(ours, theirs, imara_diff::NoSliderHeuristic);
        out.extend(diff.hunks().map(|hunk| Region {
            current: region.current.start + hunk.before.start as usize..region.current.start + hunk.before.end as usize,
            other: region.other.start + hunk.after.start as usize..region.other.start + hunk.after.end as usize,
            ..region.clone()
        }));
    }
    if style != ConflictStyle::Merge {
        return out;
    }

    // Git combines nearby conflicts, and longer gaps containing only punctuation, to reduce marker noise.
    let mut simplified: Vec<Region> = Vec::with_capacity(out.len());
    for region in out {
        if let Some(previous) = simplified.last_mut() {
            let gap = &current[previous.current.end..region.current.start];
            if previous.pick == Pick::Conflict
                && region.pick == Pick::Conflict
                && (gap.len() <= 3
                    || !gap
                        .iter()
                        .any(|token| input.interner[*token].iter().any(u8::is_ascii_alphanumeric)))
            {
                previous.current.end = region.current.end;
                previous.other.end = region.other.end;
                previous.ancestor.end = region.ancestor.end;
                continue;
            }
        }
        simplified.push(region);
    }
    simplified
}

/// Match Git's marker endings using each post-image's preceding (or first) line, then the base's first line.
pub fn line_ending(region: &Region, input: &InternedInput<&[u8]>, current: &[Token]) -> &'static BStr {
    let probe = |tokens: &[Token], index: usize| -> Option<bool> {
        let line = input.interner[*tokens.get(index)?];
        let line = if line.ends_with(b"\n") {
            line
        } else {
            input.interner[*tokens.get(index.checked_sub(1)?)?]
        };
        Some(line.ends_with(b"\r\n"))
    };
    let mut crlf = probe(current, region.current.start.saturating_sub(1));
    if crlf != Some(false) {
        crlf = probe(&input.after, region.other.start.saturating_sub(1));
    }
    if crlf != Some(false) {
        crlf = probe(&input.before, 0);
    }
    if crlf == Some(true) {
        b"\r\n".as_bstr()
    } else {
        b"\n".as_bstr()
    }
}

pub fn write_tokens(input: &InternedInput<&[u8]>, tokens: &[Token], out: &mut Vec<u8>, newline: Option<&BStr>) {
    for token in tokens {
        out.extend_from_slice(input.interner[*token]);
    }
    if !tokens.is_empty()
        && !out.ends_with(b"\n")
        && let Some(newline) = newline
    {
        out.extend_from_slice(newline);
    }
}

pub fn write_marker(out: &mut Vec<u8>, marker: u8, label: Option<&BStr>, marker_size: u8, newline: &BStr) {
    out.extend(std::iter::repeat_n(marker, marker_size as usize));
    if let Some(label) = label.filter(|label| !label.is_empty()) {
        out.push(b' ');
        out.extend_from_slice(label);
    }
    out.extend_from_slice(newline);
}
