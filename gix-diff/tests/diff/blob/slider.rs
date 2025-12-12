use gix_diff::blob::v2::{Algorithm, BasicLineDiffPrinter, Diff, InternedInput, UnifiedDiffConfig};
use gix_object::bstr::ByteSlice;
use gix_testtools::bstr::{BString, ByteVec};
use pretty_assertions::StrComparison;

#[test]
fn baseline() -> gix_testtools::Result {
    let worktree_path = gix_testtools::scripted_fixture_read_only_standalone("make_diff_for_sliders_repo.sh")?;
    let asset_dir = worktree_path.join("assets");

    let dir = std::fs::read_dir(&worktree_path)?;

    let mut diffs = Vec::new();

    for entry in dir {
        let entry = entry?;
        let file_name = entry.file_name().into_string().expect("to be string");

        if !file_name.ends_with(".baseline") {
            continue;
        }

        let parts: Vec<_> = file_name.split('.').collect();
        let [name, algorithm, ..] = parts[..] else {
            unreachable!()
        };
        let algorithm = match algorithm {
            "myers" => Algorithm::Myers,
            "histogram" => Algorithm::Histogram,
            _ => unreachable!(),
        };

        let parts: Vec<_> = name.split('-').collect();
        let [old_blob_id, new_blob_id] = parts[..] else {
            unreachable!();
        };

        let old_data = std::fs::read(asset_dir.join(format!("{old_blob_id}.blob")))?;
        let new_data = std::fs::read(asset_dir.join(format!("{new_blob_id}.blob")))?;

        let input = InternedInput::new(old_data.to_str().unwrap(), new_data.to_str().unwrap());

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

        let baseline = baseline
            .fold(BString::default(), |mut acc, diff_hunk| {
                acc.push_str(diff_hunk.header.to_string().as_str());
                acc.push(b'\n');

                acc.extend_from_slice(&diff_hunk.lines);

                acc
            })
            .to_string();

        let actual_matches_baseline = actual == baseline;
        diffs.push((actual, baseline, actual_matches_baseline, file_name));
    }

    if diffs.is_empty() {
        eprintln!("Slider baseline isn't setup - look at ./gix-diff/tests/README.md for instructions");
    }

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

    Ok(())
}

mod baseline {
    use gix_object::bstr::ByteSlice;
    use std::iter::Peekable;

    use gix_diff::blob::unified_diff::HunkHeader;
    use gix_object::bstr::{self, BString};

    static START_OF_HEADER: &[u8; 4] = b"@@ -";

    #[derive(Debug, PartialEq)]
    pub struct DiffHunk {
        pub header: HunkHeader,
        pub lines: BString,
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
}
