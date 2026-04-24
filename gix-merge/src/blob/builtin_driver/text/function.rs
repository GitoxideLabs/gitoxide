use std::ops::Range;

use crate::blob::{
    builtin_driver::text::{
        utils::{
            assure_ends_with_nl, collect_hunks, contains_lines, detect_line_ending, detect_line_ending_or_nl,
            fill_ancestor, hunks_differ_in_diff3, take_intersecting, tokens, write_ancestor, write_conflict_marker,
            write_hunks, zealously_contract_hunks, Hunk, Side,
        },
        Conflict, ConflictStyle, Labels, Merge, Options,
    },
    Resolution,
};

impl<'input, 'data> Merge<'input, 'data> {
    /// Prepare merge state for `current`, `ancestor`, and `other` using `diff_algorithm`.
    ///
    /// This computes the hunk structure once so it can be rendered multiple times with
    /// [`Merge::run()`], which is useful when experimenting with multiple conflict styles
    /// for the same input triplet.
    ///
    /// The returned [`Merge`] keeps a reference to the provided `input`, which guarantees
    /// that subsequent calls to [`Merge::run()`] use the exact same interner state.
    pub fn new(
        input: &'input mut imara_diff::InternedInput<&'data [u8]>,
        current: &'data [u8],
        ancestor: &'data [u8],
        other: &'data [u8],
        diff_algorithm: imara_diff::Algorithm,
    ) -> Merge<'input, 'data> {
        input.update_before(tokens(ancestor));
        input.update_after(tokens(current));

        let hunks = collect_hunks(diff_algorithm, input, Side::Current, Vec::new());

        let current_tokens = std::mem::take(&mut input.after);
        input.update_after(tokens(other));

        let mut hunks = collect_hunks(diff_algorithm, input, Side::Other, hunks);
        hunks.sort_by_key(|a| a.before.start);

        Merge {
            input,
            current_tokens,
            hunks,
        }
    }

    /// Merge `current` and `other` with `ancestor` as base using to `conflict`
    /// as strategy.
    ///
    /// Use `labels` to annotate conflict sections.
    ///
    /// Place the merged result in `out` (cleared before use) and return the resolution.
    pub fn run(
        &self,
        out: &mut Vec<u8>,
        Labels {
            ancestor: ancestor_label,
            current: current_label,
            other: other_label,
        }: Labels<'_>,
        conflict: Conflict,
    ) -> Resolution {
        out.clear();
        let input = self.input;
        let current_tokens = &self.current_tokens;
        if self.hunks.is_empty() {
            write_ancestor(input, 0, input.before.len(), out);
            return Resolution::Complete;
        }

        let mut hunks = self.hunks.iter().cloned().peekable();
        let mut intersecting = Vec::new();
        let mut ancestor_integrated_until = 0;
        let mut resolution = Resolution::Complete;
        let mut current_hunks = Vec::with_capacity(2);
        while take_intersecting(&mut hunks, &mut current_hunks, &mut intersecting).is_some() {
            if intersecting.is_empty() {
                let hunk = current_hunks.pop().expect("always pushed during intersection check");
                write_ancestor(input, ancestor_integrated_until, hunk.before.start as usize, out);
                ancestor_integrated_until = hunk.before.end;
                write_hunks(std::slice::from_ref(&hunk), input, current_tokens, out);
                continue;
            }

            let filled_hunks_side = current_hunks.first().expect("at least one hunk").side;
            {
                let filled_hunks_range = before_range_from_hunks(&current_hunks);
                let intersecting_range = before_range_from_hunks(&intersecting);
                let extended_range = filled_hunks_range.start..intersecting_range.end.max(filled_hunks_range.end);
                fill_ancestor(&extended_range, &mut current_hunks);
                fill_ancestor(&extended_range, &mut intersecting);
            }
            match conflict {
                Conflict::Keep { style, marker_size } => {
                    let marker_size = marker_size.get();
                    let (hunks_front_and_back, num_hunks_front) = match style {
                        ConflictStyle::Merge | ConflictStyle::ZealousDiff3 => {
                            zealously_contract_hunks(&mut current_hunks, &mut intersecting, input, current_tokens)
                        }
                        ConflictStyle::Diff3 => (Vec::new(), 0),
                    };
                    let (our_hunks, their_hunks) = match filled_hunks_side {
                        Side::Current => (&current_hunks, &intersecting),
                        Side::Other => (&intersecting, &current_hunks),
                        Side::Ancestor => {
                            unreachable!("initial hunks are never ancestors")
                        }
                    };
                    let (front_hunks, back_hunks) = hunks_front_and_back.split_at(num_hunks_front);
                    let first_hunk = first_hunk(front_hunks, our_hunks, their_hunks, back_hunks);
                    let last_hunk = last_hunk(front_hunks, our_hunks, their_hunks, back_hunks);
                    write_ancestor(input, ancestor_integrated_until, first_hunk.before.start as usize, out);
                    write_hunks(front_hunks, input, current_tokens, out);
                    // DEVIATION: this makes tests (mostly) pass, but probably is very different from what Git does.
                    let hunk_storage;
                    let nl = detect_line_ending(
                        if front_hunks.is_empty() {
                            hunk_storage = Hunk {
                                before: ancestor_integrated_until..first_hunk.before.start,
                                after: Default::default(),
                                side: Side::Ancestor,
                            };
                            std::slice::from_ref(&hunk_storage)
                        } else {
                            front_hunks
                        },
                        input,
                        current_tokens,
                    )
                    .or_else(|| detect_line_ending(our_hunks, input, current_tokens))
                    .unwrap_or(b"\n".into());
                    match style {
                        ConflictStyle::Merge => {
                            if contains_lines(our_hunks) || contains_lines(their_hunks) {
                                resolution = Resolution::Conflict;
                                write_conflict_marker(out, b'<', current_label, marker_size, nl);
                                write_hunks(our_hunks, input, current_tokens, out);
                                write_conflict_marker(out, b'=', None, marker_size, nl);
                                write_hunks(their_hunks, input, current_tokens, out);
                                write_conflict_marker(out, b'>', other_label, marker_size, nl);
                            }
                        }
                        ConflictStyle::Diff3 | ConflictStyle::ZealousDiff3 => {
                            if contains_lines(our_hunks) || contains_lines(their_hunks) {
                                if hunks_differ_in_diff3(style, our_hunks, their_hunks, input, current_tokens) {
                                    resolution = Resolution::Conflict;
                                    write_conflict_marker(out, b'<', current_label, marker_size, nl);
                                    write_hunks(our_hunks, input, current_tokens, out);
                                    let ancestor_hunk = Hunk {
                                        before: first_hunk.before.start..last_hunk.before.end,
                                        after: Default::default(),
                                        side: Side::Ancestor,
                                    };
                                    let ancestor_hunk = std::slice::from_ref(&ancestor_hunk);
                                    let ancestor_nl = detect_line_ending_or_nl(ancestor_hunk, input, current_tokens);
                                    write_conflict_marker(out, b'|', ancestor_label, marker_size, ancestor_nl);
                                    write_hunks(ancestor_hunk, input, current_tokens, out);
                                    write_conflict_marker(out, b'=', None, marker_size, nl);
                                    write_hunks(their_hunks, input, current_tokens, out);
                                    write_conflict_marker(out, b'>', other_label, marker_size, nl);
                                } else {
                                    write_hunks(our_hunks, input, current_tokens, out);
                                }
                            }
                        }
                    }

                    write_hunks(back_hunks, input, current_tokens, out);
                    ancestor_integrated_until = last_hunk.before.end;
                }
                Conflict::ResolveWithOurs | Conflict::ResolveWithTheirs => {
                    let (our_hunks, their_hunks) = match filled_hunks_side {
                        Side::Current => (&current_hunks, &intersecting),
                        Side::Other => (&intersecting, &current_hunks),
                        Side::Ancestor => {
                            unreachable!("initial hunks are never ancestors")
                        }
                    };
                    if hunks_differ_in_diff3(ConflictStyle::Diff3, our_hunks, their_hunks, input, current_tokens) {
                        resolution = Resolution::CompleteWithAutoResolvedConflict;
                    }
                    let hunks_to_write = if conflict == Conflict::ResolveWithOurs {
                        our_hunks
                    } else {
                        their_hunks
                    };
                    if let Some(first_hunk) = hunks_to_write.first() {
                        write_ancestor(input, ancestor_integrated_until, first_hunk.before.start as usize, out);
                    }
                    write_hunks(hunks_to_write, input, current_tokens, out);
                    if let Some(last_hunk) = hunks_to_write.last() {
                        ancestor_integrated_until = last_hunk.before.end;
                    }
                }
                Conflict::ResolveWithUnion => {
                    let (hunks_front_and_back, num_hunks_front) =
                        zealously_contract_hunks(&mut current_hunks, &mut intersecting, input, current_tokens);

                    let (our_hunks, their_hunks) = match filled_hunks_side {
                        Side::Current => (&current_hunks, &intersecting),
                        Side::Other => (&intersecting, &current_hunks),
                        Side::Ancestor => {
                            unreachable!("initial hunks are never ancestors")
                        }
                    };
                    if hunks_differ_in_diff3(ConflictStyle::Diff3, our_hunks, their_hunks, input, current_tokens) {
                        resolution = Resolution::CompleteWithAutoResolvedConflict;
                    }
                    let (front_hunks, back_hunks) = hunks_front_and_back.split_at(num_hunks_front);
                    let first_hunk = first_hunk(front_hunks, our_hunks, their_hunks, back_hunks);
                    write_ancestor(input, ancestor_integrated_until, first_hunk.before.start as usize, out);
                    write_hunks(front_hunks, input, current_tokens, out);
                    assure_ends_with_nl(out, detect_line_ending_or_nl(front_hunks, input, current_tokens));
                    write_hunks(our_hunks, input, current_tokens, out);
                    assure_ends_with_nl(out, detect_line_ending_or_nl(our_hunks, input, current_tokens));
                    write_hunks(their_hunks, input, current_tokens, out);
                    if !back_hunks.is_empty() {
                        assure_ends_with_nl(out, detect_line_ending_or_nl(their_hunks, input, current_tokens));
                    }
                    write_hunks(back_hunks, input, current_tokens, out);
                    let last_hunk = last_hunk(front_hunks, our_hunks, their_hunks, back_hunks);
                    ancestor_integrated_until = last_hunk.before.end;
                }
            }
        }
        write_ancestor(input, ancestor_integrated_until, input.before.len(), out);

        resolution
    }
}

/// Merge `current` and `other` with `ancestor` as base according to `opts`.
///
/// Use `labels` to annotate conflict sections.
///
/// `input` is for reusing memory for lists of tokens, but note that it grows indefinitely
/// while tokens for `current`, `ancestor` and `other` are added.
/// Place the merged result in `out` (cleared before use) and return the resolution.
///
/// # Important
///
/// *The caller* is responsible for clearing `input`, otherwise tokens will accumulate.
/// This idea is to save time if the input is known to be very similar.
#[allow(clippy::too_many_arguments)]
pub fn merge<'a>(
    out: &mut Vec<u8>,
    input: &mut imara_diff::InternedInput<&'a [u8]>,
    Labels {
        ancestor: ancestor_label,
        current: current_label,
        other: other_label,
    }: Labels<'_>,
    current: &'a [u8],
    ancestor: &'a [u8],
    other: &'a [u8],
    Options {
        diff_algorithm,
        conflict,
    }: Options,
) -> Resolution {
    out.clear();
    let merge = Merge::new(input, current, ancestor, other, diff_algorithm);
    merge.run(
        out,
        Labels {
            ancestor: ancestor_label,
            current: current_label,
            other: other_label,
        },
        conflict,
    )
}

fn first_hunk<'a>(front: &'a [Hunk], ours: &'a [Hunk], theirs: &'a [Hunk], back: &'a [Hunk]) -> &'a Hunk {
    front
        .first()
        .or(ours.first())
        .or(theirs.first())
        .or(back.first())
        .expect("at least one hunk - we aborted if there are none anywhere")
}

/// Note that last-hunk could be [`first_hunk()`], so the hunk must only be used accordingly.
fn last_hunk<'a>(front: &'a [Hunk], ours: &'a [Hunk], theirs: &'a [Hunk], back: &'a [Hunk]) -> &'a Hunk {
    back.last()
        .or(theirs.last())
        .or(ours.last())
        .or(front.last())
        .expect("at least one hunk - we aborted if there are none anywhere")
}

fn before_range_from_hunks(hunks: &[Hunk]) -> Range<u32> {
    hunks
        .first()
        .zip(hunks.last())
        .map(|(f, l)| f.before.start..l.before.end)
        .expect("at least one entry")
}
