use gix_merge::blob::builtin_driver::binary::{Pick, ResolveWith};
use gix_merge::blob::{builtin_driver, Resolution};

#[test]
fn binary() {
    assert_eq!(
        builtin_driver::binary(None),
        (Pick::Ours, Resolution::Conflict),
        "by default it picks ours and marks it as conflict"
    );
    assert_eq!(
        builtin_driver::binary(Some(ResolveWith::Ancestor)),
        (Pick::Ancestor, Resolution::CompleteWithAutoResolvedConflict),
        "Otherwise we can pick anything and it will mark it as complete"
    );
    assert_eq!(
        builtin_driver::binary(Some(ResolveWith::Ours)),
        (Pick::Ours, Resolution::CompleteWithAutoResolvedConflict)
    );
    assert_eq!(
        builtin_driver::binary(Some(ResolveWith::Theirs)),
        (Pick::Theirs, Resolution::CompleteWithAutoResolvedConflict)
    );
}

mod text {
    use bstr::ByteSlice;
    use gix_merge::blob::builtin_driver::text::{Conflict, ConflictStyle};
    use gix_merge::blob::{builtin_driver, Resolution};
    use pretty_assertions::assert_str_eq;

    const DIVERGING: &[&str] = &[
        // Somehow, on in zdiff mode, it's different, and I wasn't able to figure out the rule properly.
        // Now we prefer ancestor/before newlines and somewhat ignore our hunks. It's probably a minor issue in practice.
        // gix: "1\r\n2\n<<<<<<< complex/marker-newline-handling-lf2/ours.blob\n4\r\n||||||| complex/marker-newline-handling-lf2/base.blob\r\n2\r\n3\n=======\n5\n>>>>>>> complex/marker-newline-handling-lf2/theirs.blob\n"
        // git: "1\r\n2\n<<<<<<< complex/marker-newline-handling-lf2/ours.blob\n4  \n||||||| complex/marker-newline-handling-lf2/base.blob  \n2\r\n3\n=======\n5\n>>>>>>> complex/marker-newline-handling-lf2/theirs.blob\n"
        "complex/marker-newline-handling-lf2/zdiff3.merged",
        "complex/marker-newline-handling-lf2/zdiff3-histogram.merged",
        // This is related to Git seemingly extending a hunk to increase overlap (see diff3)
        "zdiff3-interesting/merge.merged",
        "zdiff3-interesting/merge-ours.merged",
        "zdiff3-interesting/diff3.merged",
        "zdiff3-interesting/diff3-histogram.merged",
        "zdiff3-interesting/zdiff3.merged",
        "zdiff3-interesting/zdiff3-histogram.merged",
        "zdiff3-interesting/merge-union.merged",
        // Git can extend hunks, similar to above, but the effect is not as noticeable.
        // Implementing this would be interesting, to figure out when the hunk processing should apply.
        "zdiff3-evil/merge.merged",
        "zdiff3-evil/merge-union.merged",
        // Git seems to merge to hunks if they are close together to get a less noisy diff.
        "zdiff3-middlecommon/merge.merged",
        "zdiff3-middlecommon/merge-union.merged",
        // Git has special character handling, which does magic to prevent conflicts
        "complex/auto-simplification/merge.merged",
        "complex/auto-simplification/merge-union.merged",
        // Git has special newline handling when diffing,
        // which auto-inserts a newline when it was removed, kind of.
        "complex/missing-LF-at-EOF/merge.merged",
        "complex/missing-LF-at-EOF/diff3.merged",
        "complex/missing-LF-at-EOF/diff3-histogram.merged",
        "complex/missing-LF-at-EOF/zdiff3.merged",
        "complex/missing-LF-at-EOF/zdiff3-histogram.merged",
        "complex/missing-LF-at-EOF/merge-ours.merged",
        "complex/missing-LF-at-EOF/merge-theirs.merged",
        "complex/missing-LF-at-EOF/merge-union.merged",
        // Git has different diff-slider-heuristics so diffs can be different.
        // See https://github.com/mhagger/diff-slider-tools.
        "complex/spurious-c-conflicts/merge.merged",
        "complex/spurious-c-conflicts/merge-union.merged",
        "complex/spurious-c-conflicts/diff3-histogram.merged",
        "complex/spurious-c-conflicts/zdiff3-histogram.merged",
    ];

    /// Should be a copy of `DIVERGING` once the reverse operation truly works like before
    const DIVERGING_REVERSED: &[&str] = &[
        // expected cases
        "zdiff3-middlecommon/merge.merged-reversed",
        "zdiff3-middlecommon/merge-union.merged-reversed",
        "zdiff3-interesting/merge.merged-reversed",
        "zdiff3-interesting/merge-theirs.merged-reversed",
        "zdiff3-interesting/diff3.merged-reversed",
        "zdiff3-interesting/diff3-histogram.merged-reversed",
        "zdiff3-interesting/zdiff3.merged-reversed",
        "zdiff3-interesting/zdiff3-histogram.merged-reversed",
        "zdiff3-interesting/merge-union.merged-reversed",
        "zdiff3-evil/merge.merged-reversed",
        "zdiff3-evil/merge-union.merged-reversed",
        "complex/missing-LF-at-EOF/merge.merged-reversed",
        "complex/missing-LF-at-EOF/diff3.merged-reversed",
        "complex/missing-LF-at-EOF/diff3-histogram.merged-reversed",
        "complex/missing-LF-at-EOF/zdiff3.merged-reversed",
        "complex/missing-LF-at-EOF/zdiff3-histogram.merged-reversed",
        "complex/missing-LF-at-EOF/merge-ours.merged-reversed",
        "complex/missing-LF-at-EOF/merge-theirs.merged-reversed",
        "complex/missing-LF-at-EOF/merge-union.merged-reversed",
        "complex/auto-simplification/merge.merged-reversed",
        "complex/auto-simplification/merge-union.merged-reversed",
        "complex/marker-newline-handling-lf2/zdiff3.merged-reversed",
        "complex/marker-newline-handling-lf2/zdiff3-histogram.merged-reversed",
        "complex/spurious-c-conflicts/merge.merged-reversed",
        "complex/spurious-c-conflicts/merge-union.merged-reversed",
        "complex/spurious-c-conflicts/diff3-histogram.merged-reversed",
        "complex/spurious-c-conflicts/zdiff3-histogram.merged-reversed",
    ];

    // TODO: fix all of these eventually
    fn is_case_diverging(case: &baseline::Expectation, diverging: &[&str]) -> bool {
        diverging.iter().any(|name| case.name == *name)
    }

    #[test]
    fn fuzzed() {
        for (ours, base, theirs, opts) in [
            (
                &[255, 10, 10, 255][..],
                &[0, 10, 10, 13, 10, 193, 0, 51, 8, 33][..],
                &[10, 255, 10, 10, 10, 0, 10][..],
                builtin_driver::text::Options {
                    conflict: Conflict::ResolveWithUnion,
                    diff_algorithm: imara_diff::Algorithm::Myers,
                },
            ),
            (
                &[],
                &[10, 255, 255, 255],
                &[255, 10, 255, 10, 10, 255, 40],
                builtin_driver::text::Options::default(),
            ),
        ] {
            let mut out = Vec::new();
            let mut input = imara_diff::intern::InternedInput::default();
            gix_merge::blob::builtin_driver::text(&mut out, &mut input, Default::default(), ours, base, theirs, opts);
        }
    }

    #[test]
    fn run_baseline() -> crate::Result {
        let root = gix_testtools::scripted_fixture_read_only("text-baseline.sh")?;
        for (baseline, diverging, expected_percentage) in [
            ("baseline-reversed.cases", DIVERGING_REVERSED, 11),
            ("baseline.cases", DIVERGING, 11),
        ] {
            let cases = std::fs::read_to_string(root.join(baseline))?;
            let mut out = Vec::new();
            let mut num_diverging = 0;
            let mut num_cases = 0;
            for case in baseline::Expectations::new(&root, &cases) {
                num_cases += 1;
                let mut input = imara_diff::intern::InternedInput::default();
                let actual = gix_merge::blob::builtin_driver::text(
                    &mut out,
                    &mut input,
                    case.labels(),
                    &case.ours,
                    &case.base,
                    &case.theirs,
                    case.options,
                );
                if is_case_diverging(&case, diverging) {
                    num_diverging += 1;
                } else {
                    if case.expected.contains_str("<<<<<<<") {
                        assert_eq!(actual, Resolution::Conflict, "{}: resolution mismatch", case.name,);
                    } else {
                        assert!(
                            matches!(
                                actual,
                                Resolution::Complete | Resolution::CompleteWithAutoResolvedConflict
                            ),
                            "{}: resolution mismatch",
                            case.name
                        );
                    }
                    assert_str_eq!(
                        out.as_bstr().to_str_lossy(),
                        case.expected.to_str_lossy(),
                        "{}: output mismatch\n{}",
                        case.name,
                        out.as_bstr()
                    );
                    assert_eq!(out.as_bstr(), case.expected);
                }
            }

            assert_eq!(
                num_diverging,
                diverging.len(),
                "Number of expected diverging cases must match the actual one - probably the implementation improved"
            );
            assert_eq!(
                ((num_diverging as f32 / num_cases as f32) * 100.0) as usize,
                expected_percentage,
                "Just to show the percentage of skipped tests - this should get better"
            );
        }
        Ok(())
    }

    #[test]
    fn both_sides_same_changes_are_conflict_free() {
        for conflict in [
            builtin_driver::text::Conflict::Keep {
                style: ConflictStyle::Merge,
                marker_size: 7.try_into().unwrap(),
            },
            builtin_driver::text::Conflict::Keep {
                style: ConflictStyle::Diff3,
                marker_size: 7.try_into().unwrap(),
            },
            builtin_driver::text::Conflict::Keep {
                style: ConflictStyle::ZealousDiff3,
                marker_size: 7.try_into().unwrap(),
            },
            builtin_driver::text::Conflict::ResolveWithOurs,
            builtin_driver::text::Conflict::ResolveWithTheirs,
            builtin_driver::text::Conflict::ResolveWithUnion,
        ] {
            let options = builtin_driver::text::Options {
                conflict,
                ..Default::default()
            };
            let mut input = imara_diff::intern::InternedInput::default();
            let mut out = Vec::new();
            let actual = builtin_driver::text(
                &mut out,
                &mut input,
                Default::default(),
                b"1\n3\nother",
                b"1\n2\n3",
                b"1\n3\nother",
                options,
            );
            assert_eq!(actual, Resolution::Complete, "{conflict:?}");
        }
    }

    #[test]
    fn both_differ_partially_resolution_is_conflicting() {
        for (conflict, expected) in [
            (
                builtin_driver::text::Conflict::Keep {
                    style: ConflictStyle::Merge,
                    marker_size: 7.try_into().unwrap(),
                },
                Resolution::Conflict,
            ),
            (
                builtin_driver::text::Conflict::Keep {
                    style: ConflictStyle::Diff3,
                    marker_size: 7.try_into().unwrap(),
                },
                Resolution::Conflict,
            ),
            (
                builtin_driver::text::Conflict::Keep {
                    style: ConflictStyle::ZealousDiff3,
                    marker_size: 7.try_into().unwrap(),
                },
                Resolution::Conflict,
            ),
            (
                builtin_driver::text::Conflict::ResolveWithOurs,
                Resolution::CompleteWithAutoResolvedConflict,
            ),
            (
                builtin_driver::text::Conflict::ResolveWithTheirs,
                Resolution::CompleteWithAutoResolvedConflict,
            ),
            (
                builtin_driver::text::Conflict::ResolveWithUnion,
                Resolution::CompleteWithAutoResolvedConflict,
            ),
        ] {
            let options = builtin_driver::text::Options {
                conflict,
                ..Default::default()
            };
            let mut input = imara_diff::intern::InternedInput::default();
            let mut out = Vec::new();
            let actual = builtin_driver::text(
                &mut out,
                &mut input,
                Default::default(),
                b"1\n3\nours",
                b"1\n2\n3",
                b"1\n3\ntheirs",
                options,
            );
            assert_eq!(actual, expected, "{conflict:?}");
        }
    }

    mod baseline {
        use bstr::BString;
        use gix_merge::blob::builtin_driver::text::{Conflict, ConflictStyle};
        use std::path::Path;

        #[derive(Debug)]
        pub struct Expectation {
            pub ours: BString,
            pub ours_marker: String,
            pub theirs: BString,
            pub theirs_marker: String,
            pub base: BString,
            pub base_marker: String,
            pub name: BString,
            pub expected: BString,
            pub options: gix_merge::blob::builtin_driver::text::Options,
        }

        impl Expectation {
            pub fn labels(&self) -> gix_merge::blob::builtin_driver::text::Labels<'_> {
                gix_merge::blob::builtin_driver::text::Labels {
                    ancestor: Some(self.base_marker.as_str().as_ref()),
                    current: Some(self.ours_marker.as_str().as_ref()),
                    other: Some(self.theirs_marker.as_str().as_ref()),
                }
            }
        }

        pub struct Expectations<'a> {
            root: &'a Path,
            lines: std::str::Lines<'a>,
        }

        impl<'a> Expectations<'a> {
            pub fn new(root: &'a Path, cases: &'a str) -> Self {
                Expectations {
                    root,
                    lines: cases.lines(),
                }
            }
        }

        impl Iterator for Expectations<'_> {
            type Item = Expectation;

            fn next(&mut self) -> Option<Self::Item> {
                let line = self.lines.next()?;
                let mut words = line.split(' ');
                let (Some(ours), Some(base), Some(theirs), Some(output)) =
                    (words.next(), words.next(), words.next(), words.next())
                else {
                    panic!("need at least the input and output")
                };

                let read = |rela_path: &str| read_blob(self.root, rela_path);

                let mut options = gix_merge::blob::builtin_driver::text::Options::default();
                let marker_size = 7.try_into().unwrap();
                for arg in words {
                    options.conflict = match arg {
                        "--diff3" => Conflict::Keep {
                            style: ConflictStyle::Diff3,
                            marker_size,
                        },
                        "--zdiff3" => Conflict::Keep {
                            style: ConflictStyle::ZealousDiff3,
                            marker_size,
                        },
                        "--ours" => Conflict::ResolveWithOurs,
                        "--theirs" => Conflict::ResolveWithTheirs,
                        "--union" => Conflict::ResolveWithUnion,
                        _ => panic!("Unknown argument to parse into options: '{arg}'"),
                    }
                }
                if output.contains("histogram") {
                    options.diff_algorithm = imara_diff::Algorithm::Histogram;
                }

                Some(Expectation {
                    ours: read(ours),
                    ours_marker: ours.into(),
                    theirs: read(theirs),
                    theirs_marker: theirs.into(),
                    base: read(base),
                    base_marker: base.into(),
                    expected: read(output),
                    name: output.into(),
                    options,
                })
            }
        }

        fn read_blob(root: &Path, rela_path: &str) -> BString {
            std::fs::read(root.join(rela_path))
                .unwrap_or_else(|_| panic!("Failed to read '{rela_path}' in '{}'", root.display()))
                .into()
        }
    }
}
