use crate::{Algorithm, BasicLineDiffPrinter, Diff, InternedInput, UnifiedDiffConfig};
use expect_test::expect;

#[test]
fn myers_is_even() {
    let before = "a\nb\nx\nx\ny\n";
    let after = "b\na\nx\ny\nx\n";

    cov_mark::check!(EVEN_SPLIT);
    // if the check for is_odd incorrectly always true then we take a fastpath
    // when we shouldn't, which always leads to infinite iterations/recursion
    // still we check the number of iterations here in case the search
    // is buggy in more subtle ways
    cov_mark::check_count!(SPLIT_SEARCH_ITER, 15);
    let input = InternedInput::new(before, after);
    let diff = Diff::compute(Algorithm::Myers, &input);
    expect![[r#"
        @@ -1,5 +1,5 @@
        -a
         b
        -x
        +a
         x
         y
        +x
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

#[test]
fn myers_is_odd() {
    let before = "a\nb\nx\ny\nx\n";
    let after = "b\na\nx\ny\n";

    cov_mark::check!(ODD_SPLIT);
    // if the check for odd doesn't work then
    // we still find the correct result but the number of search
    // iterations increases
    cov_mark::check_count!(SPLIT_SEARCH_ITER, 9);
    let input = InternedInput::new(before, after);
    let diff = Diff::compute(Algorithm::Myers, &input);
    expect![[r#"
        @@ -1,5 +1,4 @@
        -a
         b
        +a
         x
         y
        -x
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

/// Check for parity with Git at frequent-line detection. Even though the line being checked is
/// frequent, it is not discarded because it is bounded by unmatched runs that are short, which
/// is offset by Git counting the line twice. Counting it only once would cause it to be discarded.
#[test]
fn a_frequent_line_between_short_unmatched_runs_is_kept() {
    fn file(tag: &str) -> String {
        // The first and last lines differ so that stripping the common prefix and postfix leaves
        // the blank lines in the region, which is what makes an empty line frequent here. The
        // unmatched runs are bounded by non-blank shared lines, so nothing else inside them is.
        let mut out = format!("first line, {tag}\n");
        for i in 0..8 {
            out.push_str(&format!("shared line {i}\n\n"));
        }
        out.push_str("anchor above\n");
        for i in 0..3 {
            out.push_str(&format!("only in {tag} {i}\n"));
        }
        out.push('\n');
        for i in 3..6 {
            out.push_str(&format!("only in {tag} {i}\n"));
        }
        out.push_str("anchor below\n");
        for i in 8..16 {
            out.push_str(&format!("shared line {i}\n\n"));
        }
        out.push_str(&format!("last line, {tag}\n"));
        out
    }

    let (before, after) = (file("old"), file("new"));
    let input = InternedInput::new(before.as_str(), after.as_str());
    let diff = Diff::compute(Algorithm::Myers, &input);
    let changed = diff.hunks().fold((0, 0), |(removed, inserted), hunk| {
        (removed + hunk.before.len(), inserted + hunk.after.len())
    });
    // `git diff --no-index --numstat` reports 8 and 8 for these two files.
    assert_eq!(
        changed,
        (8, 8),
        "the blank line between the two unmatched runs should still be matched"
    );
}
