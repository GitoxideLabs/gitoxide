use gix_object::bstr::ByteSlice;
use gix_testtools::bstr::{BString, ByteVec};
use pretty_assertions::StrComparison;

#[test]
fn baseline_v1() -> gix_testtools::Result {
    use gix_diff::blob::{unified_diff::ContextSize, UnifiedDiff};

    let worktree_path = gix_testtools::scripted_fixture_read_only_standalone("make_diff_for_sliders_repo.sh")?;
    let asset_dir = worktree_path.join("assets");

    let dir = std::fs::read_dir(&worktree_path)?;

    let mut diffs = Vec::new();

    for entry in dir {
        let entry = entry?;
        let Some(baseline::DirEntry {
            file_name,
            algorithm,
            old_data,
            new_data,
        }) = baseline::parse_dir_entry(&asset_dir, &entry.file_name())?
        else {
            continue;
        };

        let interner = gix_diff::blob::intern::InternedInput::new(
            tokens_for_diffing(old_data.as_slice()),
            tokens_for_diffing(new_data.as_slice()),
        );

        let actual = gix_diff::blob::diff(
            algorithm,
            &interner,
            UnifiedDiff::new(
                &interner,
                baseline::DiffHunkRecorder::new(),
                ContextSize::symmetrical(3),
            ),
        )?;

        let baseline_path = worktree_path.join(&file_name);
        let baseline = std::fs::read(baseline_path)?;
        let baseline = baseline::Baseline::new(&baseline);

        let actual = actual
            .iter()
            .fold(BString::default(), |mut acc, diff_hunk| {
                acc.push_str(diff_hunk.header.to_string().as_str());
                acc.push(b'\n');

                acc.extend_from_slice(&diff_hunk.lines);

                acc
            })
            .to_string();

        let baseline = baseline.fold_to_unidiff().to_string();
        let actual_matches_baseline = actual == baseline;
        diffs.push((actual, baseline, actual_matches_baseline, file_name));
    }

    if diffs.is_empty() {
        eprintln!("Slider baseline isn't setup - look at ./gix-diff/tests/README.md for instructions");
    }

    assert_diffs(&diffs);

    Ok(())
}

fn tokens_for_diffing(data: &[u8]) -> impl gix_diff::blob::intern::TokenSource<Token = &[u8]> {
    gix_diff::blob::sources::byte_lines(data)
}

#[test]
fn baseline_v2() -> gix_testtools::Result {
    use gix_diff::blob::v2::{Algorithm, BasicLineDiffPrinter, Diff, InternedInput, UnifiedDiffConfig};

    let worktree_path = gix_testtools::scripted_fixture_read_only_standalone("make_diff_for_sliders_repo.sh")?;
    let asset_dir = worktree_path.join("assets");

    let dir = std::fs::read_dir(&worktree_path)?;

    let mut diffs = Vec::new();

    for entry in dir {
        let entry = entry?;
        let Some(baseline::DirEntry {
            file_name,
            algorithm,
            old_data,
            new_data,
        }) = baseline::parse_dir_entry(&asset_dir, &entry.file_name())?
        else {
            continue;
        };

        let input = InternedInput::new(
            old_data.to_str().expect("BUG: we don't have non-ascii here"),
            new_data.to_str().expect("BUG: we don't have non-ascii here"),
        );
        let algorithm = match algorithm {
            gix_diff::blob::Algorithm::Myers => Algorithm::Myers,
            gix_diff::blob::Algorithm::Histogram => Algorithm::Histogram,
            gix_diff::blob::Algorithm::MyersMinimal => Algorithm::MyersMinimal,
        };

        let mut diff = Diff::compute(algorithm, &input);
        diff.postprocess_lines(&input);

        let actual = diff
            .unified_diff(
                &BasicLineDiffPrinter(&input.interner),
                UnifiedDiffConfig::default(),
                &input,
            )
            .to_string();

        let baseline_path = worktree_path.join(&file_name);
        let baseline = std::fs::read(baseline_path)?;
        let baseline = baseline::Baseline::new(&baseline);

        let baseline = baseline.fold_to_unidiff().to_string();

        let actual_matches_baseline = actual == baseline;
        diffs.push((actual, baseline, actual_matches_baseline, file_name));
    }

    if diffs.is_empty() {
        eprintln!("Slider baseline isn't setup - look at ./gix-diff/tests/README.md for instructions");
    }

    assert_diffs(&diffs);
    Ok(())
}

fn assert_diffs(diffs: &[(String, String, bool, String)]) {
    let total_diffs = diffs.len();
    let matching_diffs = diffs
        .iter()
        .filter(|(_, _, actual_matches_baseline, _)| *actual_matches_baseline)
        .count();

    assert_eq!(
        matching_diffs,
        total_diffs,
        "matching diffs {} == total diffs {} [{:.2} %]\n\n{}",
        matching_diffs,
        total_diffs,
        ((matching_diffs as f32) / (total_diffs as f32) * 100.0),
        {
            let first_non_matching_diff = diffs
                .iter()
                .find(|(_, _, actual_matches_baseline, _)| !actual_matches_baseline)
                .expect("at least one non-matching diff to be there");

            format!(
                "affected baseline: `{}`\n\n{}",
                first_non_matching_diff.3,
                StrComparison::new(&first_non_matching_diff.0, &first_non_matching_diff.1)
            )
        }
    );
}

mod baseline {
    use gix_diff::blob::unified_diff::{ConsumeHunk, HunkHeader};
    use gix_diff::blob::Algorithm;
    use gix_object::bstr::{self, BString};
    use gix_object::bstr::{ByteSlice, ByteVec};
    use std::ffi::OsStr;
    use std::iter::Peekable;
    use std::path::Path;

    static START_OF_HEADER: &[u8; 4] = b"@@ -";

    #[derive(Debug, PartialEq)]
    pub struct DiffHunk {
        pub header: HunkHeader,
        pub lines: BString,
    }

    pub struct DiffHunkRecorder {
        inner: Vec<DiffHunk>,
    }

    impl DiffHunkRecorder {
        pub fn new() -> Self {
            Self { inner: Vec::new() }
        }
    }

    impl ConsumeHunk for DiffHunkRecorder {
        type Out = Vec<DiffHunk>;

        fn consume_hunk(
            &mut self,
            header: HunkHeader,
            lines: &[(gix_diff::blob::unified_diff::DiffLineKind, &[u8])],
        ) -> std::io::Result<()> {
            let mut buf = Vec::new();

            for &(kind, line) in lines {
                buf.push(kind.to_prefix() as u8);
                buf.extend_from_slice(line);
                buf.push(b'\n');
            }

            let diff_hunk = DiffHunk {
                header,
                lines: buf.into(),
            };

            self.inner.push(diff_hunk);

            Ok(())
        }

        fn finish(self) -> Self::Out {
            self.inner
        }
    }

    type Lines<'a> = Peekable<bstr::Lines<'a>>;

    pub struct Baseline<'a> {
        lines: Lines<'a>,
    }

    impl<'a> Baseline<'a> {
        pub fn new(content: &'a [u8]) -> Baseline<'a> {
            let mut lines = content.lines().peekable();
            skip_header(&mut lines);
            Baseline { lines }
        }
    }

    impl Baseline<'_> {
        /// Converts all baseline [`DiffHunk`]s into a single unified diff format string.
        pub fn fold_to_unidiff(self) -> BString {
            self.fold(BString::default(), |mut acc, diff_hunk| {
                acc.push_str(diff_hunk.header.to_string().as_str());
                acc.push(b'\n');

                acc.extend_from_slice(&diff_hunk.lines);

                acc
            })
        }
    }

    impl Iterator for Baseline<'_> {
        type Item = DiffHunk;

        fn next(&mut self) -> Option<Self::Item> {
            let mut hunk_header = None;
            let mut hunk_lines = Vec::new();

            while let Some(line) = self.lines.next() {
                if line.starts_with(START_OF_HEADER) {
                    assert!(hunk_header.is_none(), "should not overwrite existing hunk_header");
                    hunk_header = parse_hunk_header(line).ok();

                    continue;
                }

                match line[0] {
                    b' ' | b'+' | b'-' => {
                        hunk_lines.extend_from_slice(line);
                        hunk_lines.push(b'\n');
                    }
                    b'\\' => {
                        assert_eq!(line, "\\ No newline at end of file".as_bytes());
                    }
                    _ => unreachable!(
                        "BUG: expecting unified diff format, found line: `{}`",
                        line.to_str_lossy()
                    ),
                }

                match self.lines.peek() {
                    Some(next_line) if next_line.starts_with(START_OF_HEADER) => break,
                    None => break,
                    _ => {}
                }
            }

            hunk_header.map(|hunk_header| DiffHunk {
                header: hunk_header,
                lines: hunk_lines.into(),
            })
        }
    }

    fn skip_header(lines: &mut Lines) {
        // diff --git a/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa b/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
        // index ccccccc..ddddddd 100644
        // --- a/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
        // +++ b/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb

        let line = lines.next().expect("line to be present");
        assert!(line.starts_with(b"diff --git "));

        let line = lines.next().expect("line to be present");
        assert!(line.starts_with(b"index "));

        let line = lines.next().expect("line to be present");
        assert!(line.starts_with(b"--- "));

        let line = lines.next().expect("line to be present");
        assert!(line.starts_with(b"+++ "));
    }

    /// Parse diff hunk headers that conform to the unified diff hunk header format.
    ///
    /// The parser is very primitive and relies on the fact that `+18` is parsed as `18`. This
    /// allows us to split the input on ` ` and `,` only.
    ///
    /// @@ -18,6 +18,7 @@ abc def ghi
    /// @@ -{before_hunk_start},{before_hunk_len} +{after_hunk_start},{after_hunk_len} @@
    fn parse_hunk_header(line: &[u8]) -> gix_testtools::Result<HunkHeader> {
        let line = line.strip_prefix(START_OF_HEADER).unwrap();

        let parts: Vec<_> = line.split(|b| *b == b' ' || *b == b',').collect();
        let [before_hunk_start, before_hunk_len, after_hunk_start, after_hunk_len, ..] = parts[..] else {
            unreachable!()
        };

        Ok(HunkHeader {
            before_hunk_start: parse_number(before_hunk_start),
            before_hunk_len: parse_number(before_hunk_len),
            after_hunk_start: parse_number(after_hunk_start),
            after_hunk_len: parse_number(after_hunk_len),
        })
    }

    fn parse_number(bytes: &[u8]) -> u32 {
        bytes
            .to_str()
            .expect("to be a valid UTF-8 string")
            .parse::<u32>()
            .expect("to be a number")
    }

    pub struct DirEntry {
        pub file_name: String,
        pub algorithm: Algorithm,
        pub old_data: Vec<u8>,
        pub new_data: Vec<u8>,
    }

    /// Returns `None` if the file isn't a baseline entry.
    pub fn parse_dir_entry(asset_dir: &Path, file_name: &OsStr) -> std::io::Result<Option<DirEntry>> {
        let file_name = file_name.to_str().expect("ascii filename").to_owned();

        if !file_name.ends_with(".baseline") {
            return Ok(None);
        }

        let parts: Vec<_> = file_name.split('.').collect();
        let [name, algorithm, ..] = parts[..] else {
            unreachable!("BUG: Need file named '<name>.<algorithm>'")
        };
        let algorithm = match algorithm {
            "myers" => Algorithm::Myers,
            "histogram" => Algorithm::Histogram,
            other => unreachable!("BUG: '{other}' is not a supported algorithm"),
        };

        let parts: Vec<_> = name.split('-').collect();
        let [old_blob_id, new_blob_id] = parts[..] else {
            unreachable!("BUG: name part of filename must be '<old_blob_id>-<new_blob_id>'");
        };

        let old_data = std::fs::read(asset_dir.join(format!("{old_blob_id}.blob")))?;
        let new_data = std::fs::read(asset_dir.join(format!("{new_blob_id}.blob")))?;
        Ok(DirEntry {
            file_name,
            algorithm,
            old_data,
            new_data,
        }
        .into())
    }
}
