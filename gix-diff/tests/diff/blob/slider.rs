use gix_object::bstr::ByteSlice;
use pretty_assertions::StrComparison;

#[test]
fn baseline() -> gix_testtools::Result {
    use gix_diff::blob::{self, Algorithm, InternedInput, diff_with_slider_heuristics};

    let worktree_path = crate::scripted_fixture_read_only("make_diff_for_sliders_repo.sh")?;
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
        let diff = diff_with_slider_heuristics(
            match algorithm {
                Algorithm::Myers => Algorithm::Myers,
                Algorithm::Histogram => Algorithm::Histogram,
                Algorithm::MyersMinimal => Algorithm::MyersMinimal,
            },
            &input,
        );

        let actual = blob::UnifiedDiff::new(
            &diff,
            &input,
            blob::unified_diff::ConsumeBinaryHunk::new(String::new(), "\n"),
            blob::unified_diff::ContextSize::symmetrical(3),
        )
        .consume()?;

        let baseline_path = worktree_path.join(&file_name);
        let baseline = std::fs::read(baseline_path)?;
        let baseline = baseline::skip_header_and_fold_to_unidiff(&baseline);

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
    use gix_diff::blob::Algorithm;
    use gix_object::bstr::ByteSlice;
    use std::ffi::OsStr;
    use std::path::Path;

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

    pub fn skip_header_and_fold_to_unidiff(content: &[u8]) -> String {
        let mut lines = content.lines();

        assert!(lines.next().expect("diff header").starts_with(b"diff --git "));
        assert!(lines.next().expect("index header").starts_with(b"index "));
        assert!(lines.next().expect("--- header").starts_with(b"--- "));
        assert!(lines.next().expect("+++ header").starts_with(b"+++ "));

        let mut out = String::new();
        for line in lines {
            if line.starts_with(b"\\") {
                continue;
            }
            out.push_str(line.to_str().expect("baseline diff is valid utf-8"));
            out.push('\n');
        }
        out
    }
}

mod heuristics {
    //! We can consider to move some of these tests to the actual imara-diff test-suite as well.
    use gix_diff::blob::{self, diff_with_slider_heuristics};
    use gix_object::bstr::BStr;

    #[test]
    fn basic_usage() -> crate::Result {
        let before = r#"fn foo() {
        let x = 1;
        println!("x = {}", x);
    }
    "#;

        let after = r#"fn foo() {
        let x = 2;
        println!("x = {}", x);
        println!("done");
    }
    "#;

        let input = blob::InternedInput::new(before, after);
        let diff = diff_with_slider_heuristics(blob::Algorithm::Histogram, &input);

        insta::assert_snapshot!(util::unidiff(&diff, &input), @r#"
        @@ -2,1 +2,1 @@
        -        let x = 1;
        +        let x = 2;
        @@ -4,0 +4,1 @@
        +        println!("done");
        "#);
        Ok(())
    }

    #[test]
    fn unified_diff_with_bstr_printer_usage() -> crate::Result {
        let before: &BStr = r#"fn foo() {
        let x = 1;
        println!("x = {}", x);
    }
    "#
        .into();

        let after: &BStr = r#"fn foo() {
        let x = 2;
        println!("x = {}", x);
        println!("done");
    }
    "#
        .into();

        let input = blob::InternedInput::new(before, after);
        let diff = diff_with_slider_heuristics(blob::Algorithm::Histogram, &input);

        insta::assert_snapshot!(util::unidiff(&diff, &input), @r#"
        @@ -2,1 +2,1 @@
        -        let x = 1;
        +        let x = 2;
        @@ -4,0 +4,1 @@
        +        println!("done");
        "#);
        Ok(())
    }

    /// Test slider heuristics with indentation
    #[test]
    fn slider_heuristics_with_indentation() -> crate::Result {
        let before = r#"fn main() {
        if true {
            println!("hello");
        }
    }
    "#;

        let after = r#"fn main() {
        if true {
            println!("hello");
            println!("world");
        }
    }
    "#;

        let input = blob::InternedInput::new(before, after);
        let diff = diff_with_slider_heuristics(blob::Algorithm::Histogram, &input);

        insta::assert_snapshot!(util::unidiff(&diff, &input), @r#"
        @@ -4,0 +4,1 @@
        +            println!("world");
        "#);

        Ok(())
    }

    /// Test that Myers algorithm also works with slider heuristics
    #[test]
    fn myers_with_slider_heuristics() -> crate::Result {
        let before = "a\nb\nc\n";
        let after = "a\nx\nc\n";

        let input = blob::InternedInput::new(before, after);
        let diff = diff_with_slider_heuristics(blob::Algorithm::Myers, &input);

        insta::assert_snapshot!(util::unidiff(&diff, &input), @r"
        @@ -2,1 +2,1 @@
        -b
        +x
        ");

        Ok(())
    }

    /// Test empty diff
    #[test]
    fn empty_diff_with_slider_heuristics() -> crate::Result {
        let before = "unchanged\n";
        let after = "unchanged\n";

        let input = blob::InternedInput::new(before, after);
        let diff = diff_with_slider_heuristics(blob::Algorithm::Histogram, &input);

        assert_eq!(diff.count_removals(), 0);
        assert_eq!(diff.count_additions(), 0);

        Ok(())
    }

    /// Test complex multi-hunk diff with slider heuristics
    #[test]
    fn multi_hunk_diff_with_slider_heuristics() -> crate::Result {
        let before = r#"struct Foo {
        x: i32,
    }
    
    impl Foo {
        fn new() -> Self {
            Foo { x: 0 }
        }
    }
    "#;

        let after = r#"struct Foo {
        x: i32,
        y: i32,
    }
    
    impl Foo {
        fn new() -> Self {
            Foo { x: 0, y: 0 }
        }
    }
    "#;

        let input = blob::InternedInput::new(before, after);
        let diff = diff_with_slider_heuristics(blob::Algorithm::Histogram, &input);

        insta::assert_snapshot!(util::unidiff(&diff, &input), @"
        @@ -3,0 +3,1 @@
        +        y: i32,
        @@ -7,1 +8,1 @@
        -            Foo { x: 0 }
        +            Foo { x: 0, y: 0 }
        ");

        Ok(())
    }

    /// Test custom context size in the local unified diff printer.
    #[test]
    fn custom_context_size() -> crate::Result {
        let before = "line1\nline2\nline3\nline4\nline5\nline6\nline7\n";
        let after = "line1\nline2\nline3\nMODIFIED\nline5\nline6\nline7\n";

        let input = blob::InternedInput::new(before, after);
        let diff = diff_with_slider_heuristics(blob::Algorithm::Histogram, &input);

        // Test with context size of 1
        let unified = util::unidiff_with_context(&diff, &input, 1)?;
        insta::assert_snapshot!(unified, @r"
        @@ -3,3 +3,3 @@
         line3
        -line4
        +MODIFIED
         line5
        ");

        // Test with context size of 3 (default)
        let unified_default = util::unidiff_with_context(&diff, &input, 3)?;

        // Smaller context should have fewer lines
        insta::assert_snapshot!(unified_default, @r"
        @@ -1,7 +1,7 @@
         line1
         line2
         line3
        -line4
        +MODIFIED
         line5
         line6
         line7
        ");

        Ok(())
    }

    /// Test that hunks iterator works correctly
    #[test]
    fn hunks_iterator() -> crate::Result {
        let before = "a\nb\nc\nd\ne\n";
        let after = "a\nX\nc\nY\ne\n";

        let input = blob::InternedInput::new(before, after);
        let diff = diff_with_slider_heuristics(blob::Algorithm::Histogram, &input);

        let hunks: Vec<_> = diff.hunks().collect();

        insta::assert_snapshot!(util::unidiff(&diff, &input), @r"
        @@ -2,1 +2,1 @@
        -b
        +X
        @@ -4,1 +4,1 @@
        -d
        +Y
        ");
        // Should have two separate hunks
        insta::assert_debug_snapshot!(hunks, @r"
        [
            Hunk {
                before: 1..2,
                after: 1..2,
            },
            Hunk {
                before: 3..4,
                after: 3..4,
            },
        ]
        ");
        Ok(())
    }

    /// Test postprocessing without heuristic
    #[test]
    fn postprocess_no_heuristic() -> crate::Result {
        let before = "a\nb\nc\n";
        let after = "a\nX\nc\n";

        let input = blob::InternedInput::new(before, after);

        // Create diff but postprocess without heuristic
        let mut diff = blob::Diff::compute(blob::Algorithm::Histogram, &input);
        diff.postprocess_no_heuristic(&input);

        insta::assert_snapshot!(util::unidiff(&diff, &input), @r"
        @@ -2,1 +2,1 @@
        -b
        +X
        ");

        Ok(())
    }

    #[test]
    fn indent_heuristic_available() -> crate::Result {
        let before = "fn foo() {\n    x\n}\n";
        let after = "fn foo() {\n    y\n}\n";

        let input = blob::InternedInput::new(before, after);

        let mut diff = blob::Diff::compute(blob::Algorithm::Histogram, &input);

        let heuristic = blob::IndentHeuristic::new(|token| {
            let line: &str = input.interner[token];
            blob::IndentLevel::for_ascii_line(line.as_bytes().iter().copied(), 4)
        });

        diff.postprocess_with_heuristic(&input, heuristic);

        insta::assert_snapshot!(util::unidiff(&diff, &input), @r"
        @@ -2,1 +2,1 @@
        -    x
        +    y
        ");

        Ok(())
    }

    mod util {
        use std::hash::Hash;

        use gix_diff::blob;

        pub fn unidiff<T: AsRef<[u8]> + ?Sized + Hash + Eq>(
            diff: &blob::Diff,
            input: &blob::InternedInput<&T>,
        ) -> String {
            unidiff_with_context(diff, input, 0).expect("rendering unified diff succeeds")
        }

        pub fn unidiff_with_context<T: AsRef<[u8]> + ?Sized + Hash + Eq>(
            diff: &blob::Diff,
            input: &blob::InternedInput<&T>,
            context_len: u32,
        ) -> std::io::Result<String> {
            blob::UnifiedDiff::new(
                diff,
                input,
                blob::unified_diff::ConsumeBinaryHunk::new(String::new(), "\n"),
                blob::unified_diff::ContextSize::symmetrical(context_len),
            )
            .consume()
        }
    }
}
