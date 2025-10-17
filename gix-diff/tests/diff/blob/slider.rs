use std::{iter::Peekable, path::PathBuf};

use gix_diff::blob::{
    intern::TokenSource,
    unified_diff::{ConsumeHunk, ContextSize, HunkHeader},
    Algorithm, UnifiedDiff,
};
use gix_object::bstr::{self, BString, ByteVec};

#[derive(Debug, PartialEq)]
struct DiffHunk {
    header: HunkHeader,
    lines: BString,
}

struct DiffHunkRecorder {
    inner: Vec<DiffHunk>,
}

impl DiffHunkRecorder {
    fn new() -> Self {
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

struct Baseline<'a> {
    lines: Peekable<bstr::Lines<'a>>,
}

mod baseline {
    use std::path::Path;

    use gix_diff::blob::unified_diff::HunkHeader;
    use gix_object::bstr::ByteSlice;

    use super::{Baseline, DiffHunk};

    static START_OF_HEADER: &[u8; 4] = b"@@ -";

    impl Baseline<'_> {
        pub fn collect(baseline_path: impl AsRef<Path>) -> std::io::Result<Vec<DiffHunk>> {
            let content = std::fs::read(baseline_path)?;

            let mut baseline = Baseline {
                lines: content.lines().peekable(),
            };

            baseline.skip_header();

            Ok(baseline.collect())
        }

        fn skip_header(&mut self) {
            // diff --git a/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa b/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
            // index ccccccc..ddddddd 100644
            // --- a/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
            // +++ b/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb

            let line = self.lines.next().expect("line to be present");
            assert!(line.starts_with(b"diff --git "));

            let line = self.lines.next().expect("line to be present");
            assert!(line.starts_with(b"index "));

            let line = self.lines.next().expect("line to be present");
            assert!(line.starts_with(b"--- "));

            let line = self.lines.next().expect("line to be present");
            assert!(line.starts_with(b"+++ "));
        }

        /// Parse diff hunk headers that conform to the unified diff hunk header format.
        ///
        /// The parser is very primitive and relies on the fact that `+18` is parsed as `18`. This
        /// allows us to split the input on ` ` and `,` only.
        ///
        /// @@ -18,6 +18,7 @@ abc def ghi
        /// @@ -{before_hunk_start},{before_hunk_len} +{after_hunk_start},{after_hunk_len} @@
        fn parse_hunk_header(&self, line: &[u8]) -> gix_testtools::Result<HunkHeader> {
            let Some(line) = line.strip_prefix(START_OF_HEADER) else {
                todo!()
            };

            let parts: Vec<_> = line.split(|b| *b == b' ' || *b == b',').collect();
            let [before_hunk_start, before_hunk_len, after_hunk_start, after_hunk_len, ..] = parts[..] else {
                todo!()
            };

            Ok(HunkHeader {
                before_hunk_start: self.parse_number(before_hunk_start),
                before_hunk_len: self.parse_number(before_hunk_len),
                after_hunk_start: self.parse_number(after_hunk_start),
                after_hunk_len: self.parse_number(after_hunk_len),
            })
        }

        fn parse_number(&self, bytes: &[u8]) -> u32 {
            bytes
                .to_str()
                .expect("to be a valid UTF-8 string")
                .parse::<u32>()
                .expect("to be a number")
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
                    hunk_header = self.parse_hunk_header(line).ok();

                    continue;
                }

                match line[0] {
                    b' ' | b'+' | b'-' => {
                        hunk_lines.extend_from_slice(line);
                        hunk_lines.push(b'\n');
                    }
                    _ => todo!(),
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
}

#[test]
fn sliders() -> gix_testtools::Result {
    let worktree_path = fixture_path()?;
    let asset_dir = worktree_path.join("assets");

    let dir = std::fs::read_dir(&worktree_path)?;

    for entry in dir {
        let entry = entry?;
        let file_name = entry.file_name().into_string().expect("to be string");

        if !file_name.ends_with(".baseline") {
            continue;
        }

        let parts: Vec<_> = file_name.split('.').collect();
        let [name, algorithm, ..] = parts[..] else {
            unimplemented!()
        };
        let algorithm = match algorithm {
            "myers" => Algorithm::Myers,
            "histogram" => Algorithm::Histogram,
            _ => unimplemented!(),
        };

        let parts: Vec<_> = name.split('-').collect();
        let [old_blob_id, new_blob_id] = parts[..] else {
            unimplemented!();
        };

        let old_data = std::fs::read(asset_dir.join(format!("{old_blob_id}.blob")))?;
        let new_data = std::fs::read(asset_dir.join(format!("{new_blob_id}.blob")))?;

        let interner = gix_diff::blob::intern::InternedInput::new(
            tokens_for_diffing(old_data.as_slice()),
            tokens_for_diffing(new_data.as_slice()),
        );

        let actual = gix_diff::blob::diff(
            algorithm,
            &interner,
            UnifiedDiff::new(&interner, DiffHunkRecorder::new(), ContextSize::symmetrical(3)),
        )?;

        let baseline_path = worktree_path.join(file_name);
        let baseline = Baseline::collect(baseline_path).unwrap();

        let actual = actual
            .iter()
            .fold(BString::default(), |mut acc, diff_hunk| {
                acc.push_str(diff_hunk.header.to_string().as_str());
                acc.push(b'\n');

                acc.extend_from_slice(&diff_hunk.lines);

                acc
            })
            .to_string();

        let baseline = baseline
            .iter()
            .fold(BString::default(), |mut acc, diff_hunk| {
                acc.push_str(diff_hunk.header.to_string().as_str());
                acc.push(b'\n');

                acc.extend_from_slice(&diff_hunk.lines);

                acc
            })
            .to_string();

        pretty_assertions::assert_eq!(actual, baseline);
    }

    Ok(())
}

fn tokens_for_diffing(data: &[u8]) -> impl TokenSource<Token = &[u8]> {
    gix_diff::blob::sources::byte_lines(data)
}

fn fixture_path() -> gix_testtools::Result<PathBuf> {
    gix_testtools::scripted_fixture_read_only_standalone("make_diff_for_sliders_repo.sh")
}
