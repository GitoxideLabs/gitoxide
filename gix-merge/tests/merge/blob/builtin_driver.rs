use gix_merge::blob::{
    Resolution, builtin_driver,
    builtin_driver::binary::{Pick, ResolveWith},
};

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
    use arbitrary::Arbitrary;
    use bstr::ByteSlice;
    use gix_merge::blob::{
        Resolution, builtin_driver,
        builtin_driver::text::{self, Conflict, ConflictStyle},
    };
    use gix_testtools::TestResult;
    use pretty_assertions::assert_str_eq;
    use std::num::NonZero;

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
            let mut input = imara_diff::InternedInput::default();
            gix_merge::blob::builtin_driver::text(&mut out, &mut input, Default::default(), ours, base, theirs, opts);
        }
    }

    #[derive(Debug, Arbitrary)]
    struct FuzzCtx<'a> {
        base: &'a [u8],
        ours: &'a [u8],
        theirs: &'a [u8],
        marker_size: NonZero<u8>,
    }

    fn run_fuzz_case(ours: &[u8], base: &[u8], theirs: &[u8], marker_size: NonZero<u8>) {
        let mut out = Vec::new();
        let mut input = imara_diff::InternedInput::default();
        // Keep this in sync with the fuzz target. Histogram remains enabled here because it is the
        // diff algorithm we fuzz through gix-merge itself. Myers-family algorithms have
        // pathological cases that are expensive enough under fuzz instrumentation to turn the
        // target into a timeout reproducer for the diff backend instead of a useful gix-merge
        // fuzz harness.
        for (left, right) in [(ours, theirs), (theirs, ours)] {
            input.clear();
            let merge = text::Merge::new(&mut input, left, base, right, imara_diff::Algorithm::Histogram);
            let resolution = merge.run(&mut out, Default::default(), Conflict::default());
            if resolution == Resolution::Conflict {
                for conflict in [
                    Conflict::ResolveWithOurs,
                    Conflict::ResolveWithTheirs,
                    Conflict::ResolveWithUnion,
                    Conflict::Keep {
                        style: ConflictStyle::Diff3,
                        marker_size,
                    },
                    Conflict::Keep {
                        style: ConflictStyle::ZealousDiff3,
                        marker_size,
                    },
                ] {
                    merge.run(&mut out, Default::default(), conflict);
                }
            }
        }
    }

    #[test]
    fn clusterfuzz_timeout_regression() {
        for (name, data) in [
            (
                "clusterfuzz-testcase-minimized-gix-merge-blob-6377298803884032",
                include_bytes!("../../fixtures/clusterfuzz-testcase-minimized-gix-merge-blob-6377298803884032")
                    .as_slice(),
            ),
            (
                "clusterfuzz-testcase-minimized-gix-merge-blob-5577413097750528",
                include_bytes!("../../fixtures/clusterfuzz-testcase-minimized-gix-merge-blob-5577413097750528")
                    .as_slice(),
            ),
        ] {
            let ctx = FuzzCtx::arbitrary(&mut arbitrary::Unstructured::new(data))
                .unwrap_or_else(|_| panic!("{name}: testcase matches the historical fuzz target input layout"));
            run_fuzz_case(ctx.ours, ctx.base, ctx.theirs, ctx.marker_size);
        }
    }

    #[test]
    fn run_baseline() -> TestResult {
        let root = gix_testtools::scripted_fixture_read_only("text-baseline.sh")?;
        for baseline in ["baseline.cases", "baseline-reversed.cases"] {
            let cases = std::fs::read_to_string(root.join(baseline))?;
            let mut out = Vec::new();
            for case in baseline::Expectations::new(&root, &cases) {
                let mut input = imara_diff::InternedInput::default();
                let actual = gix_merge::blob::builtin_driver::text(
                    &mut out,
                    &mut input,
                    case.labels(),
                    &case.ours,
                    &case.base,
                    &case.theirs,
                    case.options,
                );
                assert_eq!(
                    actual == Resolution::Conflict,
                    case.git_status != 0,
                    "{}: resolution {actual:?} disagrees with Git exit status {}",
                    case.name,
                    case.git_status
                );
                assert_str_eq!(
                    out.as_bstr().to_str_lossy(),
                    case.expected.to_str_lossy(),
                    "{}: output mismatch",
                    case.name
                );
                assert_eq!(out.as_bstr(), case.expected, "{}: output bytes mismatch", case.name);
            }
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
            let mut input = imara_diff::InternedInput::default();
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
            let mut input = imara_diff::InternedInput::default();
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

    #[test]
    fn adjacent_changes_conflict_while_changes_with_a_gap_merge_cleanly() {
        let base = b"one\ntwo\nthree\nfour\nfive\n";
        let ours = b"one\ntwo-ours\nthree\nfour\nfive\n";
        let adjacent_theirs = b"one\ntwo\nthree-theirs\nfour\nfive\n";
        let separated_theirs = b"one\ntwo\nthree\nfour-theirs\nfive\n";
        let labels = text::Labels {
            ancestor: Some("base".into()),
            current: Some("current".into()),
            other: Some("other".into()),
        };
        let options = text::Options {
            conflict: Conflict::Keep {
                style: ConflictStyle::Merge,
                marker_size: NonZero::new(7).expect("seven is non-zero"),
            },
            ..Default::default()
        };

        for (name, current, other, expected) in [
            (
                "ours then theirs",
                ours.as_slice(),
                adjacent_theirs.as_slice(),
                "one\n<<<<<<< current\ntwo-ours\nthree\n=======\ntwo\nthree-theirs\n>>>>>>> other\nfour\nfive\n",
            ),
            (
                "theirs then ours",
                adjacent_theirs.as_slice(),
                ours.as_slice(),
                "one\n<<<<<<< current\ntwo\nthree-theirs\n=======\ntwo-ours\nthree\n>>>>>>> other\nfour\nfive\n",
            ),
        ] {
            let mut out = Vec::new();
            let mut input = imara_diff::InternedInput::default();
            let resolution = builtin_driver::text(&mut out, &mut input, labels, current, base, other, options);
            assert_eq!(
                resolution,
                Resolution::Conflict,
                "{name}: adjacent changes must conflict because no unchanged base line separates them"
            );
            assert_str_eq!(
                out.as_bstr().to_str_lossy(),
                expected,
                "{name}: adjacent changes must be rendered as one conflict"
            );
        }

        for (name, current, other) in [
            ("ours then theirs", ours.as_slice(), separated_theirs.as_slice()),
            ("theirs then ours", separated_theirs.as_slice(), ours.as_slice()),
        ] {
            let mut out = Vec::new();
            let mut input = imara_diff::InternedInput::default();
            let resolution = builtin_driver::text(&mut out, &mut input, labels, current, base, other, options);
            assert_eq!(
                resolution,
                Resolution::Complete,
                "{name}: changes separated by an unchanged base line must merge independently"
            );
            assert_str_eq!(
                out.as_bstr().to_str_lossy(),
                "one\ntwo-ours\nthree\nfour-theirs\nfive\n",
                "{name}: a gap between changes must preserve both edits without conflict markers"
            );
        }
    }

    mod false_conflict {
        use gix_merge::blob::{Resolution, builtin_driver, builtin_driver::text::Conflict};
        use imara_diff::InternedInput;

        /// Minimal reproduction: Myers produces a false conflict where git merge-file resolves cleanly.
        ///
        /// base:   alpha_x / (blank) / bravo_x / charlie_x / (blank)
        /// ours:   (blank) / (blank) / bravo_x / charlie_x
        /// theirs: alpha_x / (blank) / charlie_x / (blank)
        ///
        /// base→ours:  alpha_x deleted (replaced by blank), trailing blank removed
        /// base→theirs: bravo_x deleted
        ///
        /// These are non-overlapping changes that git merges cleanly.
        /// See https://github.com/GitoxideLabs/gitoxide/issues/2475
        #[test]
        fn myers_false_conflict_with_blank_line_ambiguity() {
            let base = b"alpha_x\n\nbravo_x\ncharlie_x\n\n";
            let ours = b"\n\nbravo_x\ncharlie_x\n";
            let theirs = b"alpha_x\n\ncharlie_x\n\n";

            let labels = builtin_driver::text::Labels {
                ancestor: Some("base".into()),
                current: Some("ours".into()),
                other: Some("theirs".into()),
            };

            // Histogram resolves cleanly.
            {
                let options = builtin_driver::text::Options {
                    diff_algorithm: imara_diff::Algorithm::Histogram,
                    conflict: Conflict::Keep {
                        style: builtin_driver::text::ConflictStyle::Merge,
                        marker_size: 7.try_into().unwrap(),
                    },
                };
                let mut out = Vec::new();
                let mut input = InternedInput::default();
                let res = builtin_driver::text(&mut out, &mut input, labels, ours, base, theirs, options);
                assert_eq!(res, Resolution::Complete, "Histogram should resolve cleanly");
            }

            // Myers should also resolve cleanly (it used to produce a false conflict because
            // imara-diff's Myers splits the ours change into two hunks — a deletion at base[0]
            // and an empty insertion at base[2] — and the insertion collided with theirs'
            // deletion at base[2]).
            {
                let options = builtin_driver::text::Options {
                    diff_algorithm: imara_diff::Algorithm::Myers,
                    conflict: Conflict::Keep {
                        style: builtin_driver::text::ConflictStyle::Merge,
                        marker_size: 7.try_into().unwrap(),
                    },
                };
                let mut out = Vec::new();
                let mut input = InternedInput::default();
                let res = builtin_driver::text(&mut out, &mut input, labels, ours, base, theirs, options);
                assert_eq!(
                    res,
                    Resolution::Complete,
                    "Myers should resolve cleanly (git merge-file does). Output:\n{}",
                    String::from_utf8_lossy(&out)
                );
            }
        }
    }

    mod baseline {
        use std::path::Path;

        use bstr::{BString, ByteSlice};
        use gix_merge::blob::builtin_driver::text::{Conflict, ConflictStyle};

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
            pub git_status: u8,
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
                    if arg == "--diff-algorithm=histogram" {
                        options.diff_algorithm = imara_diff::Algorithm::Histogram;
                        continue;
                    }
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

                Some(Expectation {
                    ours: read(ours),
                    ours_marker: ours.into(),
                    theirs: read(theirs),
                    theirs_marker: theirs.into(),
                    base: read(base),
                    base_marker: base.into(),
                    expected: read(output),
                    git_status: read(&format!("{output}.status"))
                        .to_str()
                        .expect("Git's exit status is ASCII")
                        .trim()
                        .parse()
                        .expect("Git wrote a numeric exit status"),
                    name: output.into(),
                    options,
                })
            }
        }

        fn read_blob(root: &Path, rela_path: &str) -> BString {
            std::fs::read(root.join(rela_path))
                .unwrap_or_else(|err| panic!("Failed to read \"{rela_path}\" in \"{}\": {err}", root.display()))
                .into()
        }
    }
}
