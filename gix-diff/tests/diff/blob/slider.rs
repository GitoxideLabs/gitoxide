use std::{collections::BTreeMap, hash::Hash, path::Path};

use gix_diff::blob::{self, Algorithm, InternedInput, diff_with_slider_heuristics};
use gix_object::bstr::ByteSlice;
use pretty_assertions::StrComparison;

#[test]
fn baseline() -> gix_testtools::Result {
    let worktree_path = crate::scripted_fixture_read_only("make_diff_for_sliders_repo.sh")?;
    let asset_dir = worktree_path.join("assets");

    let dir = std::fs::read_dir(&worktree_path)?;
    let should_print_extended_report = std::env::var_os("GIX_DIFF_SLIDER_REPORT").is_some();

    let mut cases = Vec::new();

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

        let gix_no_postprocess = {
            let diff = blob::Diff::compute(algorithm, &input);
            render_unidiff(&diff, &input)?
        };

        let gix_postprocess_no_heuristic = {
            let mut diff = blob::Diff::compute(algorithm, &input);
            diff.postprocess_no_heuristic(&input);
            render_unidiff(&diff, &input)?
        };

        let gix_postprocess_slider_heuristics = {
            let diff = diff_with_slider_heuristics(algorithm, &input);
            render_unidiff(&diff, &input)?
        };

        let baseline_path = worktree_path.join(&file_name);
        let baseline = std::fs::read(baseline_path)?;
        let baseline = crate::blob::skip_header_and_fold_to_unidiff(&baseline);
        let git_no_indent_heuristic = if should_print_extended_report {
            read_no_indent_baseline(&worktree_path, &file_name)?
        } else {
            None
        };

        cases.push(Case {
            file_name,
            algorithm,
            git_postprocess_indent_heuristic: baseline,
            git_no_indent_heuristic,
            gix_no_postprocess,
            gix_postprocess_no_heuristic,
            gix_postprocess_slider_heuristics,
        });
    }

    if cases.is_empty() {
        eprintln!("Slider baseline isn't set up – see ./gix-diff/tests/README.md for instructions");
    }

    if should_print_extended_report {
        print_extended_report(&cases);
    } else {
        assert_diffs(&cases);
    }

    Ok(())
}

struct Case {
    file_name: String,
    algorithm: Algorithm,
    git_postprocess_indent_heuristic: String,
    git_no_indent_heuristic: Option<String>,
    gix_no_postprocess: String,
    gix_postprocess_no_heuristic: String,
    gix_postprocess_slider_heuristics: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Classification {
    Exact,
    NoPostprocessMatchesGitDefault,
    PostprocessNoHeuristicMatchesGitDefault,
    PostprocessNoHeuristicMatchesGitNoIndent,
    LikelySliderOnlyMismatch,
    OtherMismatch,
}

impl Case {
    fn classify(&self) -> Classification {
        if self.gix_postprocess_slider_heuristics == self.git_postprocess_indent_heuristic {
            Classification::Exact
        } else if self.gix_no_postprocess == self.git_postprocess_indent_heuristic {
            Classification::NoPostprocessMatchesGitDefault
        } else if self.gix_postprocess_no_heuristic == self.git_postprocess_indent_heuristic {
            Classification::PostprocessNoHeuristicMatchesGitDefault
        } else if self
            .git_no_indent_heuristic
            .as_ref()
            .is_some_and(|git_no_indent_heuristic| &self.gix_postprocess_no_heuristic == git_no_indent_heuristic)
        {
            Classification::PostprocessNoHeuristicMatchesGitNoIndent
        } else if has_same_changed_line_sequence(
            &self.gix_postprocess_slider_heuristics,
            &self.git_postprocess_indent_heuristic,
        ) {
            Classification::LikelySliderOnlyMismatch
        } else {
            Classification::OtherMismatch
        }
    }
}

fn render_unidiff<T: AsRef<[u8]> + Hash + Eq>(diff: &blob::Diff, input: &InternedInput<T>) -> std::io::Result<String> {
    blob::UnifiedDiff::new(
        diff,
        input,
        blob::unified_diff::ConsumeBinaryHunk::new(String::new(), "\n"),
        blob::unified_diff::ContextSize::symmetrical(3),
    )
    .consume()
}

fn read_no_indent_baseline(worktree_path: &Path, primary_file_name: &str) -> std::io::Result<Option<String>> {
    let Some(stem) = primary_file_name.strip_suffix(".baseline") else {
        return Ok(None);
    };

    let path = worktree_path.join(format!("{stem}.no-indent.baseline"));
    let baseline = match std::fs::read(path) {
        Ok(baseline) => baseline,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    };
    Ok(Some(super::skip_header_and_fold_to_unidiff(&baseline)))
}

fn assert_diffs(cases: &[Case]) {
    let total_diffs = cases.len();
    let matching_diffs = cases
        .iter()
        .filter(|case| case.gix_postprocess_slider_heuristics == case.git_postprocess_indent_heuristic)
        .count();

    assert_eq!(
        matching_diffs,
        total_diffs,
        "matching diffs {} == total diffs {} [{:.2} %]\n\n{}",
        matching_diffs,
        total_diffs,
        ((matching_diffs as f32) / (total_diffs as f32) * 100.0),
        {
            let first_non_matching_diff = cases
                .iter()
                .find(|case| case.gix_postprocess_slider_heuristics != case.git_postprocess_indent_heuristic)
                .expect("at least one non-matching diff to be there");

            format!(
                "affected baseline: `{}`\n\n{}",
                first_non_matching_diff.file_name,
                StrComparison::new(
                    &first_non_matching_diff.gix_postprocess_slider_heuristics,
                    &first_non_matching_diff.git_postprocess_indent_heuristic
                )
            )
        }
    );
}

// This function intentionally only is capable to detect a subset of diffs that differ by slider
// placement only. It can only detect sliders that have added/deleted lines in the exact same
// order. I plan on adding more accurate classification in the future, though.
fn has_same_changed_line_sequence(lhs: &str, rhs: &str) -> bool {
    fn changed_lines(diff: &str) -> Vec<&str> {
        diff.lines()
            .filter(|line| line.starts_with('+') || line.starts_with('-'))
            .collect()
    }

    changed_lines(lhs) == changed_lines(rhs)
}

fn slider_detail(gix: &str, git: &str) -> String {
    let gix_hunks = parse_hunks(gix);
    let git_hunks = parse_hunks(git);

    if gix_hunks.is_empty() || git_hunks.is_empty() {
        return format!(
            "diagnostic-error/unparsed-or-empty-hunks/gix-{}/git-{}",
            gix_hunks.len(),
            git_hunks.len()
        );
    }

    if gix_hunks.len() != git_hunks.len() {
        return format!(
            "multi-hunk/different-hunk-count/gix-{}/git-{}",
            gix_hunks.len(),
            git_hunks.len()
        );
    }

    // At this point, we know that `gix_hunks.len() == git_hunks.len()`.
    if gix_hunks.len() > 1 {
        let directions: Vec<_> = gix_hunks
            .iter()
            .zip(&git_hunks)
            .map(|(gix_hunk, git_hunk)| movement_direction(gix_hunk, git_hunk))
            .collect();
        let same_direction = directions
            .first()
            .is_some_and(|first| directions.iter().all(|direction| direction == first));
        let direction = if same_direction {
            "same-direction"
        } else {
            "mixed-direction"
        };

        return format!("multi-hunk/same-count/{direction}");
    }

    let gix_hunk = &gix_hunks[0];
    let git_hunk = &git_hunks[0];

    let gix_is_empty = gix_hunk.removed == 0 && gix_hunk.added == 0;
    let git_is_empty = git_hunk.removed == 0 && git_hunk.added == 0;

    match (gix_is_empty, git_is_empty) {
        (true, true) => return "diagnostic-error/empty-hunks/both".to_owned(),
        (true, false) => return "diagnostic-error/empty-hunks/gix".to_owned(),
        (false, true) => return "diagnostic-error/empty-hunks/git".to_owned(),
        (false, false) => {}
    }

    let kind = match (gix_hunk.removed > 0, gix_hunk.added > 0) {
        (false, true) => "pure-insertion",
        (true, false) => "pure-deletion",
        (true, true) => "modification",
        (false, false) => unreachable!("BUG: we verified that both `added` and `removed` are non-zero"),
    };
    let direction = movement_direction(gix_hunk, git_hunk);
    let distance = movement_distance_bucket(gix_hunk, git_hunk);

    format!("single-hunk/{kind}/{direction}/{distance}")
}

struct ParsedHunk {
    before_start: u32,
    after_start: u32,
    removed: usize,
    added: usize,
}

fn parse_hunks(diff: &str) -> Vec<ParsedHunk> {
    let mut hunks = Vec::<ParsedHunk>::new();

    for line in diff.lines() {
        if line.starts_with("@@ ") {
            if let Some((before_start, after_start)) = parse_hunk_header(line) {
                hunks.push(ParsedHunk {
                    before_start,
                    after_start,
                    removed: 0,
                    added: 0,
                });
            }
        } else if let Some(hunk) = hunks.last_mut() {
            if line.starts_with('-') {
                hunk.removed += 1;
            } else if line.starts_with('+') {
                hunk.added += 1;
            }
        }
    }

    hunks
}

fn parse_hunk_header(line: &str) -> Option<(u32, u32)> {
    // Supported forms include both `@@ -10,3 +10,4 @@` and `@@ -10 +10 @@`.
    let line = line.strip_prefix("@@ ")?;
    let (header, _section) = line.split_once(" @@")?;

    let mut parts = header.split_ascii_whitespace();
    let before = parts.next()?.strip_prefix('-')?;
    let after = parts.next()?.strip_prefix('+')?;

    Some((parse_range_start(before)?, parse_range_start(after)?))
}

fn parse_range_start(range: &str) -> Option<u32> {
    let start = range.split_once(',').map_or(range, |(start, _len)| start);

    start.parse().ok()
}

fn movement_direction(gix_hunk: &ParsedHunk, git_hunk: &ParsedHunk) -> &'static str {
    match gix_hunk.before_start.cmp(&git_hunk.before_start) {
        std::cmp::Ordering::Less => "gix-before-start-earlier",
        std::cmp::Ordering::Greater => "gix-before-start-later",
        std::cmp::Ordering::Equal => match gix_hunk.after_start.cmp(&git_hunk.after_start) {
            std::cmp::Ordering::Less => "same-before-start/gix-after-start-earlier",
            std::cmp::Ordering::Greater => "same-before-start/gix-after-start-later",
            std::cmp::Ordering::Equal => "same-before-start",
        },
    }
}

fn movement_distance_bucket(gix_hunk: &ParsedHunk, git_hunk: &ParsedHunk) -> &'static str {
    let before_delta = gix_hunk.before_start.abs_diff(git_hunk.before_start);
    let after_delta = gix_hunk.after_start.abs_diff(git_hunk.after_start);

    match before_delta.max(after_delta) {
        0 => "0",
        1 => "1",
        2..=5 => "2-5",
        6..=20 => "6-20",
        _ => ">20",
    }
}

fn print_extended_report(cases: &[Case]) {
    let mut counts = BTreeMap::<Classification, usize>::new();
    let mut likely_slider_only_counts = BTreeMap::<String, usize>::new();

    for case in cases {
        let classification = case.classify();
        *counts.entry(classification).or_default() += 1;

        if classification == Classification::LikelySliderOnlyMismatch {
            *likely_slider_only_counts
                .entry(slider_detail(
                    &case.gix_postprocess_slider_heuristics,
                    &case.git_postprocess_indent_heuristic,
                ))
                .or_default() += 1;
        }
    }

    eprintln!("slider report");
    eprintln!();

    if cases.is_empty() {
        eprintln!("no cases to report");

        return;
    }

    eprintln!("total cases: {}", cases.len());
    eprintln!(
        "git no-indent baselines: {}",
        cases
            .iter()
            .filter(|case| case.git_no_indent_heuristic.is_some())
            .count()
    );
    eprintln!();

    for (classification, count) in counts {
        let percentage = count as f32 / cases.len() as f32 * 100.0;

        eprintln!("{classification:?}: {count} [{percentage:.2}%]");
    }

    eprintln!();
    eprintln!("likely slider-only details");
    eprintln!();

    let likely_slider_only_total: usize = likely_slider_only_counts.values().sum();

    if likely_slider_only_total == 0 {
        eprintln!("no likely slider-only mismatches");
    } else {
        let mut details: Vec<_> = likely_slider_only_counts.into_iter().collect();
        details.sort_by(|(lhs_detail, lhs_count), (rhs_detail, rhs_count)| {
            rhs_count.cmp(lhs_count).then_with(|| lhs_detail.cmp(rhs_detail))
        });

        for (detail, count) in details {
            let slider_only_percentage = count as f32 / likely_slider_only_total as f32 * 100.0;
            let total_percentage = count as f32 / cases.len() as f32 * 100.0;

            eprintln!("{detail}: {count} [{slider_only_percentage:.2}% slider-only, {total_percentage:.2}% total]");
        }
    }

    eprintln!();
    eprintln!("first non-matching cases");
    eprintln!();

    for case in cases
        .iter()
        .filter(|case| case.gix_postprocess_slider_heuristics != case.git_postprocess_indent_heuristic)
        .take(20)
    {
        eprintln!("{} {:?} {:?}", case.file_name, case.algorithm, case.classify());
    }
}

mod baseline {
    use gix_diff::blob::Algorithm;
    use std::ffi::OsStr;
    use std::path::Path;

    pub struct DirEntry {
        pub file_name: String,
        pub algorithm: Algorithm,
        pub old_data: Vec<u8>,
        pub new_data: Vec<u8>,
    }

    /// Returns `None` if the file isn't a primary baseline entry.
    pub fn parse_dir_entry(asset_dir: &Path, file_name: &OsStr) -> std::io::Result<Option<DirEntry>> {
        let file_name = file_name.to_str().expect("ascii filename").to_owned();

        let Some(stem) = file_name.strip_suffix(".baseline") else {
            return Ok(None);
        };

        let parts: Vec<_> = stem.split('.').collect();
        let [name, algorithm] = parts[..] else {
            // Additional baselines like `<name>.<algorithm>.no-indent.baseline` are consumed separately.
            return Ok(None);
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
