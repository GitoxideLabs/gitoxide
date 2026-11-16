use std::fmt::Write;
use std::mem::swap;

use expect_test::expect;
// use git::bstr::BStr;
// use git_repository as git;

use gix_imara_diff::BasicLineDiffPrinter;
use gix_imara_diff::InternedInput;
use gix_imara_diff::sources::words;
use gix_imara_diff::{Algorithm, Diff, UnifiedDiffConfig};

const ALL_ALGORITHMS: [Algorithm; 2] = [Algorithm::Histogram, Algorithm::Myers];

mod fuzzed {
    use std::time::{Duration, Instant};

    use arbitrary::Arbitrary;
    use gix_imara_diff::{Algorithm, Diff, InternedInput};

    #[derive(Debug, Arbitrary)]
    struct ComprehensiveDiffInput<'a> {
        before: &'a [u8],
        before_str: &'a str,
        after: &'a [u8],
        after_str: &'a str,
    }

    fn run_comprehensive_diff_fuzz_case(
        ComprehensiveDiffInput {
            before,
            before_str,
            after,
            after_str,
        }: ComprehensiveDiffInput<'_>,
    ) {
        let mut input = InternedInput::new(before, after);

        for algorithm in [Algorithm::Histogram, Algorithm::Myers, Algorithm::MyersMinimal] {
            // Keep quadratic minimal diffs within the fuzz timeout.
            if algorithm == Algorithm::MyersMinimal {
                input.before.truncate(512);
                input.after.truncate(512);
            }
            let mut diff = Diff::compute(algorithm, &input);

            let _ = diff.count_additions();
            let _ = diff.count_removals();

            for hunk in diff.hunks() {
                let _ = hunk.is_pure_insertion();
                let _ = hunk.is_pure_removal();
                let _ = hunk.invert();
            }

            diff.postprocess_no_heuristic(&input);
            diff.postprocess_lines(&input);
        }

        let input = InternedInput::new(before_str, after_str);
        let mut word_input = InternedInput::default();
        let mut word_diff = Diff::default();

        let diff = Diff::compute(Algorithm::Myers, &input);
        for hunk in diff.hunks() {
            hunk.latin_word_diff(&input, &mut word_input, &mut word_diff);
        }
    }

    #[test]
    fn timeout_regression() {
        let input = ComprehensiveDiffInput::arbitrary(&mut arbitrary::Unstructured::new(include_bytes!(
            "../fixtures/clusterfuzz-testcase-minimized-gix-imara-diff-comprehensive_diff-6497314075377664"
        )))
        .expect("testcase matches the historical fuzz target input layout");

        let start = Instant::now();
        run_comprehensive_diff_fuzz_case(input);
        let elapsed = start.elapsed();

        let expected = Duration::from_secs(3);
        assert!(
            elapsed < expected,
            "clusterfuzz regression took {elapsed:?}, expected less than {expected:?}"
        );
    }
}

#[test]
fn words_tokenizer() {
    let text = "Hello,  imara!\n (foo-bar_baz)";
    let tokens = words(text).collect::<Vec<_>>();
    assert_eq!(
        tokens,
        vec![
            "Hello", ",", "  ", "imara", "!", "\n", " ", "(", "foo", "-", "bar_baz", ")"
        ]
    );
}

#[test]
fn replace() {
    let before = r#"fn foo() -> Bar{
    let mut foo = 2.0;
    foo *= 100 / 2;
    println!("hello world")
}
"#;

    let after = r#"const TEST: i32 = 0;
fn foo() -> Bar{
    let mut foo = 2.0;
    foo *= 100 / 2;
    println!("hello world");
    println!("hello foo {TEST}");
}

"#;
    let input = InternedInput::new(before, after);
    for algorithm in ALL_ALGORITHMS {
        let mut diff = Diff::compute(algorithm, &input);
        diff.postprocess_lines(&input);
        expect![[r#"
            @@ -1,5 +1,8 @@
            +const TEST: i32 = 0;
             fn foo() -> Bar{
                 let mut foo = 2.0;
                 foo *= 100 / 2;
            -    println!("hello world")
            +    println!("hello world");
            +    println!("hello foo {TEST}");
             }
            +
        "#]]
        .assert_eq(
            &diff
                .unified_diff(
                    &BasicLineDiffPrinter(&input.interner),
                    UnifiedDiffConfig::default(),
                    &input,
                )
                .to_string(),
        );
    }
}

#[test]
fn identical_files() {
    let file = r#"fn foo() -> Bar{
    let mut foo = 2.0;
    foo *= 100 / 2;
}"#;

    for algorithm in ALL_ALGORITHMS {
        let input = InternedInput::new(file, file);
        let mut diff = Diff::compute(algorithm, &input);
        diff.postprocess_lines(&input);
        assert_eq!(
            diff.unified_diff(
                &BasicLineDiffPrinter(&input.interner),
                UnifiedDiffConfig::default(),
                &input,
            )
            .to_string(),
            ""
        );
    }
}

#[test]
fn simple_insert() {
    let before = r#"fn foo() -> Bar{
    let mut foo = 2.0;
    foo *= 100 / 2;
}"#;

    let after = r#"fn foo() -> Bar{
    let mut foo = 2.0;
    foo *= 100 / 2;
    println("hello world")
}"#;

    let mut input = InternedInput::new(before, after);
    for algorithm in ALL_ALGORITHMS {
        let mut diff = Diff::compute(algorithm, &input);
        diff.postprocess_lines(&input);
        expect![[r#"
          @@ -1,4 +1,5 @@
           fn foo() -> Bar{
               let mut foo = 2.0;
               foo *= 100 / 2;
          +    println("hello world")
           }
          "#]]
        .assert_eq(
            &diff
                .unified_diff(
                    &BasicLineDiffPrinter(&input.interner),
                    UnifiedDiffConfig::default(),
                    &input,
                )
                .to_string(),
        );

        swap(&mut input.before, &mut input.after);

        let mut diff = Diff::compute(algorithm, &input);
        diff.postprocess_lines(&input);
        expect![[r#"
            @@ -1,5 +1,4 @@
             fn foo() -> Bar{
                 let mut foo = 2.0;
                 foo *= 100 / 2;
            -    println("hello world")
             }
            "#]]
        .assert_eq(
            &diff
                .unified_diff(
                    &BasicLineDiffPrinter(&input.interner),
                    UnifiedDiffConfig::default(),
                    &input,
                )
                .to_string(),
        );
        swap(&mut input.before, &mut input.after);
    }
}

#[test]
fn unified_diff_context_lines_near_input_start_and_end() {
    let before = r#"a
b
c
d
e
f
g
h
i
"#;

    let after = r#"a
b
c
d
edit
f
g
h
i
"#;

    let input = InternedInput::new(before, after);
    for algorithm in ALL_ALGORITHMS {
        let mut diff = Diff::compute(algorithm, &input);
        diff.postprocess_lines(&input);
        expect![[r#"
          @@ -2,7 +2,7 @@
           b
           c
           d
          -e
          +edit
           f
           g
           h
          "#]]
        .assert_eq(
            &diff
                .unified_diff(
                    &BasicLineDiffPrinter(&input.interner),
                    UnifiedDiffConfig::default(),
                    &input,
                )
                .to_string(),
        );
    }
}

mod latin_word_diff {
    use crate::ALL_ALGORITHMS;

    use gix_imara_diff::sources::words;
    use gix_imara_diff::{Diff, InternedInput, Token};
    use std::mem::swap;
    use std::ops::Range;

    #[test]
    fn pure_insertion_or_removal() {
        let before = r#"fn foo() -> Bar{
    let mut foo = 2.0;
    foo *= 100 / 2;
}"#;
        let after = r#"fn foo() -> Bar{
    let mut foo = 2.0;
    foo *= 100 / 2;
    println("hello world")
}"#;
        let mut input = InternedInput::new(before, after);
        for algorithm in ALL_ALGORITHMS {
            let mut diff_input = InternedInput::default();
            let mut out = Diff::default();

            let mut diff = Diff::compute(algorithm, &input);
            diff.postprocess_lines(&input);

            let mut hunks = diff.hunks();
            let hunk = hunks.next().expect("missing first hunk");
            hunk.latin_word_diff(&input, &mut diff_input, &mut out);
            hunks = out.hunks();

            let first = hunks.next().expect("missing first inner hunk");
            assert!(first.is_pure_insertion());
            assert_eq!(first.before, 0..0);
            assert_eq!(first.after, 0..words("    println(\"hello world\")\n").count() as u32);
            assert_eq!(hunks.next(), None);
            assert_eq!(hunks.next(), None);
            assert_eq!(
                visualise(&diff_input, &first.after, &diff_input.after),
                "    |println|(|\"|hello| |world|\"|)|\n"
            );

            swap(&mut input.before, &mut input.after);

            let mut diff = Diff::compute(algorithm, &input);
            diff.postprocess_lines(&input);

            hunks = diff.hunks();
            let hunk = hunks.next().expect("missing first hunk");
            hunk.latin_word_diff(&input, &mut diff_input, &mut out);
            hunks = out.hunks();

            let first = hunks.next().expect("missing first inner hunk");
            assert!(first.is_pure_removal());
            assert_eq!(first.before, 0..words("    println(\"hello world\")\n").count() as u32);
            assert_eq!(first.after, 0..0);
            assert_eq!(hunks.next(), None);
            assert_eq!(hunks.next(), None);
            assert_eq!(
                visualise(&diff_input, &first.before, &diff_input.before),
                "    |println|(|\"|hello| |world|\"|)|\n"
            );

            swap(&mut input.before, &mut input.after);
        }
    }

    #[test]
    fn modification() {
        let before = r#"fn foo() -> Bar {
    let mut foo = 2.0;
    foo *= 100 / 2;
}"#;
        let after = r#"fn foo() -> Bar {
    let mut foo = 3.0 * 2.0;
    foo += 100 / 2;
}"#;
        let mut input = InternedInput::new(before, after);
        for algorithm in ALL_ALGORITHMS {
            let mut diff_input = InternedInput::default();
            let mut out = Diff::default();

            let mut diff = Diff::compute(algorithm, &input);
            diff.postprocess_lines(&input);

            let mut hunks = diff.hunks();
            let hunk = hunks.next().expect("missing first hunk");
            hunk.latin_word_diff(&input, &mut diff_input, &mut out);
            hunks = out.hunks();

            let first = hunks.next().expect("missing first inner hunk");
            assert!(first.is_pure_insertion());
            let off = words("    let mut foo = ").count() as u32;
            assert_eq!(first.before, off..off);
            let ins = words("3.0 * ").count() as u32;
            assert_eq!(first.after, off..ins + off);
            assert_eq!(visualise(&diff_input, &first.before, &diff_input.before), "");
            assert_eq!(visualise(&diff_input, &first.after, &diff_input.after), "3|.|0| |*| ");

            let second = hunks.next().expect("missing second inner hunk");
            let off = words(
                r#"    let mut foo = 2.0;
    foo "#,
            )
            .count() as u32;
            assert_eq!(second.before, off..1 + off);
            assert_eq!(second.after, ins + off..1 + ins + off);
            assert_eq!(visualise(&diff_input, &second.before, &diff_input.before), "*");
            assert_eq!(visualise(&diff_input, &second.after, &diff_input.after), "+");
            assert_eq!(hunks.next(), None);
            assert_eq!(hunks.next(), None);

            swap(&mut input.before, &mut input.after);

            let mut diff = Diff::compute(algorithm, &input);
            diff.postprocess_lines(&input);

            hunks = diff.hunks();
            let hunk = hunks.next().expect("missing first hunk");

            hunk.latin_word_diff(&input, &mut diff_input, &mut out);
            hunks = out.hunks();

            let first = hunks.next().expect("missing first inner hunk");
            assert!(first.is_pure_removal());
            let off = words("    let mut foo = ").count() as u32;
            let rem = words("3.0 * ").count() as u32;
            assert_eq!(first.before, off..rem + off);
            assert_eq!(first.after, off..off);
            let second = hunks.next().expect("missing second inner hunk");
            let off = words(
                r#"    let mut foo = 2.0;
    foo "#,
            )
            .count() as u32;
            assert_eq!(second.before, rem + off..1 + rem + off);
            assert_eq!(second.after, off..1 + off);
            assert_eq!(hunks.next(), None);
            assert_eq!(hunks.next(), None);

            swap(&mut input.before, &mut input.after);
        }
    }

    fn visualise(diff_input: &InternedInput<&str>, token_ids: &Range<u32>, tokens: &[Token]) -> String {
        token_ids
            .clone()
            .map(|id| {
                let id = id as usize;
                diff_input.interner[tokens[id]]
            })
            .collect::<Vec<_>>()
            .join("|")
    }
}

#[test]
#[cfg(not(miri))]
fn hand_checked_unidiffs() {
    let before = r#"use crate::{
    alpha::Alpha,
    beta::Beta,
    gamma::Gamma,
};

use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

pub struct Engine {
    cache: HashMap<String, usize>,
    steps: Vec<&'static str>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            steps: vec!["parse", "render"],
        }
    }

    pub fn update(&mut self, path: &Path) {
        let _ = path;
        self.steps.push("scan");
    }
}

fn unchanged_one() {
    println!("one");
}

fn unchanged_two() {
    println!("two");
}

pub enum Error {
    InvalidPath,
    Unknown,
}

pub struct Layer {
    pub depth: usize,
}

impl Layer {
    pub fn parse(&self) -> Result<(), Error> {
        Ok(())
    }
}
"#;
    let after = r#"use crate::{
    alpha::Alpha,
    beta::Beta,
    gamma::Gamma,
};

use std::{
    collections::HashMap,
    mem::replace,
    path::Path,
};

pub struct Engine {
    cache: HashMap<String, usize>,
    steps: Vec<&'static str>,
    dirty: bool,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            steps: vec!["parse", "render"],
            dirty: false,
        }
    }

    pub fn update(&mut self, path: &Path) {
        let _previous = replace(&mut self.dirty, true);
        let _ = path;
        self.steps.push("scan");
    }
}

fn unchanged_one() {
    println!("one");
}

fn unchanged_two() {
    println!("two");
}

pub enum Error {
    InvalidPath,
    InvalidState,
    Unknown,
}

pub struct Layer {
    pub depth: u32,
}

impl Layer {
    pub fn parse(&self) -> Result<(), Error> {
        Ok(())
    }
}
"#;

    for algorithm in ALL_ALGORITHMS {
        println!("{algorithm:?}");
        let input = InternedInput::new(before, after);
        let mut diff = Diff::compute(algorithm, &input);
        diff.postprocess_lines(&input);
        expect![[r#"
@@ -5,13 +5,15 @@
 };
 
 use std::{
-    collections::{HashMap, HashSet},
+    collections::HashMap,
+    mem::replace,
     path::Path,
 };
 
 pub struct Engine {
     cache: HashMap<String, usize>,
     steps: Vec<&'static str>,
+    dirty: bool,
 }
 
 impl Engine {
@@ -19,10 +21,12 @@
         Self {
             cache: HashMap::new(),
             steps: vec!["parse", "render"],
+            dirty: false,
         }
     }
 
     pub fn update(&mut self, path: &Path) {
+        let _previous = replace(&mut self.dirty, true);
         let _ = path;
         self.steps.push("scan");
     }
@@ -38,11 +42,12 @@
 
 pub enum Error {
     InvalidPath,
+    InvalidState,
     Unknown,
 }
 
 pub struct Layer {
-    pub depth: usize,
+    pub depth: u32,
 }
 
 impl Layer {
"#]]
        .assert_eq(
            &diff
                .unified_diff(
                    &BasicLineDiffPrinter(&input.interner),
                    UnifiedDiffConfig::default(),
                    &input,
                )
                .to_string(),
        );
    }
}

#[test]
fn postprocess() {
    let before = r#"
       /*
        * Stay on the safe side. if read_directory() has run once on
        * "dir", some sticky flag may have been left. Clear them all.
        */
       clear_sticky(dir);

       /*
        * exclude patterns are treated like positive ones in
        * create_simplify. Usually exclude patterns should be a
        * subset of positive ones, which has no impacts on
        * foo
        * bar
        * test
        */
        foo
    "#;
    let after = r#"
       /*
        * exclude patterns are treated like positive ones in
        * create_simplify. Usually exclude patterns should be a
        * subset of positive ones, which has no impacts on
        * foo
        * bar
        * test
        */
        foo
    "#;

    let input = InternedInput::new(before, after);
    for algorithm in [Algorithm::Histogram, Algorithm::Myers] {
        let mut diff = Diff::compute(algorithm, &input);
        diff.postprocess_lines(&input);
        let diff = diff
            .unified_diff(
                &BasicLineDiffPrinter(&input.interner),
                UnifiedDiffConfig::default(),
                &input,
            )
            .to_string();
        expect![[r#"
            @@ -1,10 +1,4 @@
             
            -       /*
            -        * Stay on the safe side. if read_directory() has run once on
            -        * "dir", some sticky flag may have been left. Clear them all.
            -        */
            -       clear_sticky(dir);
            -
                    /*
                     * exclude patterns are treated like positive ones in
                     * create_simplify. Usually exclude patterns should be a
        "#]]
        .assert_eq(&diff);
    }
}

/// Check for parity with Git at frequent-line detection. Even though the line being checked is
/// frequent, it is not discarded because it is bounded by unmatched runs that are short, which
/// is offset by Git counting the line twice. Counting it only once would cause it to be discarded.
#[test]
fn a_frequent_line_between_short_unmatched_runs_is_kept() -> gix_testtools::TestResult {
    fn file(tag: &str) -> gix_testtools::TestResult<String> {
        // The first and last lines differ so that stripping the common prefix and postfix leaves
        // the blank lines in the region, which is what makes an empty line frequent here. The
        // unmatched runs are bounded by non-blank shared lines, so nothing else inside them is.
        let mut out = format!("first line, {tag}\n");
        for i in 0..8 {
            writeln!(out, "shared line {i}\n")?;
        }
        out.push_str("anchor above\n");
        for i in 0..3 {
            writeln!(out, "only in {tag} {i}")?;
        }
        out.push('\n');
        for i in 3..6 {
            writeln!(out, "only in {tag} {i}")?;
        }
        out.push_str("anchor below\n");
        for i in 8..16 {
            writeln!(out, "shared line {i}\n")?;
        }
        writeln!(out, "last line, {tag}")?;
        Ok(out)
    }

    let (before, after) = (file("old")?, file("new")?);
    let input = InternedInput::new(before.as_str(), after.as_str());
    let diff = Diff::compute(Algorithm::Myers, &input);
    let changed = diff.hunks().fold((0, 0), |(removed, inserted), hunk| {
        (removed + hunk.before.len(), inserted + hunk.after.len())
    });
    assert_eq!(
        changed,
        (8, 8),
        "the blank line between the two unmatched runs should still be matched, just like `git diff --no-index --numstat`"
    );
    Ok(())
}

#[test]
fn a_repeated_line_below_gits_frequency_limit_is_kept() {
    // These 19 lines give Git's `xdl_bogosqrt` a frequency limit of 8; rounding
    // down gives 4 and incorrectly treats the four blank lines as frequent.
    // Eight unmatched lines surround the last blank, so it would then be pruned.
    // Different first and last lines prevent common-edge stripping.
    let before = "old start
a

b

c

anchor above
old 0
old 1
old 2
old 3

old 4
old 5
old 6
old 7
anchor below
old end
";
    let after = before.replace("old", "new");
    let input = InternedInput::new(before, after.as_str());
    let diff = Diff::compute(Algorithm::Myers, &input);
    assert_eq!(
        (diff.count_removals(), diff.count_additions()),
        (10, 10),
        "the blank line between the unmatched runs should still be matched, just like `git diff --no-index --numstat`"
    );
}

/// Git counts how often each line occurs in the *whole* of both files before it trims their common
/// prefix and postfix, and derives its frequency limit from their full lengths, too. Only the lines
/// between the trimmed ends are then judged by those numbers.
#[test]
fn line_frequency_is_counted_across_the_common_prefix_and_postfix() -> gix_testtools::TestResult {
    assert_myers_matches_git("whole-file-frequency", Algorithm::Myers, (9, 9))?;
    assert_myers_matches_git("whole-file-frequency", Algorithm::MyersMinimal, (8, 8))
}

/// Once the cost of a split reaches its limit, Git settles for the furthest reaching path.
/// `xdl_split()` numbers the cost from 1 and gives up as soon as it reaches the limit, so a search
/// that numbered it from 0 ran one round longer and picked a different split.
#[test]
fn an_expensive_split_gives_up_after_as_many_rounds_as_git() -> gix_testtools::TestResult {
    assert_myers_matches_git("expensive-split", Algorithm::Myers, (286, 286))
}

#[test]
fn an_expensive_minimal_split_keeps_searching_until_the_optimum_is_found() -> gix_testtools::TestResult {
    assert_myers_matches_git("expensive-minimal-split", Algorithm::MyersMinimal, (271, 271))
}

#[test]
fn trivial_myers_changes_match_git() -> gix_testtools::TestResult {
    for (case, additions) in [
        ("trivial-empty", 0),
        ("trivial-identical", 0),
        ("trivial-empty-file", 2),
        ("trivial-prepend", 1),
        ("trivial-append", 1),
        ("trivial-middle", 2),
        ("trivial-repeated", 1),
        ("trivial-frequent", 9),
        ("trivial-no-final-newline", 1),
    ] {
        for algorithm in [Algorithm::Myers, Algorithm::MyersMinimal] {
            assert_myers_matches_git(case, algorithm, (0, additions))?;
            assert_myers_matches_git(&format!("{case}-reversed"), algorithm, (additions, 0))?;
        }
    }
    Ok(())
}

#[test]
fn near_trivial_myers_changes_still_need_preprocessing() -> gix_testtools::TestResult {
    for algorithm in [Algorithm::Myers, Algorithm::MyersMinimal] {
        for (case, expected) in [
            ("near-miss-newline", (1, 2)),
            ("near-miss-kept-anchor", (0, 6)),
            (
                "near-miss-pruned-anchor",
                if algorithm == Algorithm::Myers { (1, 8) } else { (0, 7) },
            ),
        ] {
            assert_myers_matches_git(case, algorithm, expected)?;
            assert_myers_matches_git(&format!("{case}-reversed"), algorithm, (expected.1, expected.0))?;
        }
    }
    Ok(())
}

fn assert_myers_matches_git(case: &str, algorithm: Algorithm, expected: (u32, u32)) -> gix_testtools::TestResult {
    // Git 2.50 disabled frequency pruning for `--minimal`.
    let Some(root) = gix_testtools::scripted_fixture_read_only_with_git_version("make_myers_baseline.sh", |version| {
        version >= (2, 50, 0)
    })?
    else {
        return Ok(());
    };
    let root = root.join(case);
    let before = std::fs::read_to_string(root.join("old"))?;
    let after = std::fs::read_to_string(root.join("new"))?;
    let baseline = std::fs::read_to_string(root.join(format!("{algorithm:?}.numstat")))?;
    let git_counts = if baseline.is_empty() {
        (0, 0)
    } else {
        let mut fields = baseline.split('\t');
        let additions: u32 = fields.next().expect("Git numstat contains additions").parse()?;
        let removals: u32 = fields.next().expect("Git numstat contains removals").parse()?;
        (removals, additions)
    };
    assert_eq!(
        git_counts, expected,
        "{case}: Git's {algorithm:?} baseline must retain the regression's expected edit counts"
    );

    let git_hunks = std::fs::read_to_string(root.join(format!("{algorithm:?}.hunks")))?
        .lines()
        .map(|line| {
            let (before, after) = line.split_once(' ').expect("Git hunk header contains both ranges");
            Ok(gix_imara_diff::Hunk {
                before: git_hunk_range(before)?,
                after: git_hunk_range(after)?,
            })
        })
        .collect::<gix_testtools::TestResult<Vec<_>>>()?;

    let input = InternedInput::new(before.as_str(), after.as_str());
    // Seed both change arrays so an early return must clear stale flags, including for identical inputs.
    let mut reused = Diff::compute(algorithm, &InternedInput::new("old\nold\n", "new\nnew\n"));
    reused.compute_with(algorithm, &input.before, &input.after, input.interner.num_tokens());
    for (storage, mut diff) in [("fresh", Diff::compute(algorithm, &input)), ("reused", reused)] {
        assert_eq!(
            (diff.count_removals(), diff.count_additions()),
            git_counts,
            "{case}: {algorithm:?} with {storage} storage must match Git's removal and addition counts"
        );
        // Even --no-indent-heuristic compacts changes; compare equivalent postprocessing.
        diff.postprocess_no_heuristic(&input);
        assert_eq!(
            diff.hunks().collect::<Vec<_>>(),
            git_hunks,
            "{case}: {algorithm:?} with {storage} storage must match Git's exact hunk placement"
        );
    }
    Ok(())
}

fn git_hunk_range(range: &str) -> gix_testtools::TestResult<std::ops::Range<u32>> {
    let range = &range[1..]; // Strip the '-' or '+' sign.
    let (start, len) = range.split_once(',').unwrap_or((range, "1"));
    let start: u32 = start.parse()?;
    let len: u32 = len.parse()?;
    // Empty Git ranges name the preceding line; nonempty ranges use one-based line numbers.
    let start = start - u32::from(len != 0);
    Ok(start..start + len)
}
