use crate::blob::{
    Resolution,
    builtin_driver::text::{
        Conflict, ConflictStyle, Labels, Merge, Options,
        utils::{Pick, collect_hunks, line_ending, refine, regions, write_marker, write_tokens},
    },
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
        input.update_before(imara_diff::sources::byte_lines(ancestor));
        input.update_after(imara_diff::sources::byte_lines(current));
        let mut hunks = Vec::new();
        collect_hunks(diff_algorithm, input, true, &mut hunks);
        let current_tokens = std::mem::take(&mut input.after);
        input.update_after(imara_diff::sources::byte_lines(other));
        collect_hunks(diff_algorithm, input, false, &mut hunks);
        let regions = regions(hunks, &current_tokens, &input.after);
        Merge {
            input,
            current_tokens,
            regions,
            diff_algorithm,
        }
    }

    /// Merge the prepared inputs according to `conflict`, using `labels` for conflict sections.
    /// Place the merged result in `out` (cleared before use) and return the resolution.
    pub fn run(&self, out: &mut Vec<u8>, labels: Labels<'_>, conflict: Conflict) -> Resolution {
        out.clear();
        let input = self.input;
        let current = &self.current_tokens;
        let style = match conflict {
            Conflict::Keep { style, .. } => style,
            _ => ConflictStyle::Merge,
        };
        let regions = refine(&self.regions, style, self.diff_algorithm, input, current);
        let mut current_end = 0;
        let mut resolution = Resolution::Complete;
        for region in regions {
            write_tokens(input, &current[current_end..region.current.start], out, None);
            let ours = &current[region.current.clone()];
            let theirs = &input.after[region.other.clone()];
            match region.pick {
                Pick::Current => write_tokens(input, ours, out, None),
                Pick::Other => write_tokens(input, theirs, out, None),
                Pick::Conflict => {
                    let newline = line_ending(&region, input, current);
                    match conflict {
                        Conflict::Keep { marker_size, .. } => {
                            resolution = Resolution::Conflict;
                            write_marker(out, b'<', labels.current, marker_size.get(), newline);
                            write_tokens(input, ours, out, Some(newline));
                            if style != ConflictStyle::Merge {
                                write_marker(out, b'|', labels.ancestor, marker_size.get(), newline);
                                write_tokens(input, &input.before[region.ancestor.clone()], out, Some(newline));
                            }
                            write_marker(out, b'=', None, marker_size.get(), newline);
                            write_tokens(input, theirs, out, Some(newline));
                            write_marker(out, b'>', labels.other, marker_size.get(), newline);
                        }
                        Conflict::ResolveWithOurs | Conflict::ResolveWithTheirs | Conflict::ResolveWithUnion => {
                            resolution = Resolution::CompleteWithAutoResolvedConflict;
                            if conflict != Conflict::ResolveWithTheirs {
                                write_tokens(
                                    input,
                                    ours,
                                    out,
                                    (conflict == Conflict::ResolveWithUnion).then_some(newline),
                                );
                            }
                            if conflict != Conflict::ResolveWithOurs {
                                write_tokens(input, theirs, out, None);
                            }
                        }
                    }
                }
            }
            current_end = region.current.end;
        }
        write_tokens(input, &current[current_end..], out, None);
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
pub fn merge<'a>(
    out: &mut Vec<u8>,
    input: &mut imara_diff::InternedInput<&'a [u8]>,
    labels: Labels<'_>,
    current: &'a [u8],
    ancestor: &'a [u8],
    other: &'a [u8],
    Options {
        diff_algorithm,
        conflict,
    }: Options,
) -> Resolution {
    Merge::new(input, current, ancestor, other, diff_algorithm).run(out, labels, conflict)
}
