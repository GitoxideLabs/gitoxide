//! We can consider to move some of these tests to the actual imara-diff test-suite as well.
use gix_diff::blob::{diff_with_slider_heuristics, v2};
use gix_object::bstr::BStr;

/// Test that the UnifiedDiffPrinter can be used with the v0.2 API
#[test]
fn unified_diff_printer_usage() -> crate::Result {
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

    let input = v2::InternedInput::new(before, after);
    let diff = diff_with_slider_heuristics(v2::Algorithm::Histogram, &input);

    let printer = v2::BasicLineDiffPrinter(&input.interner);
    insta::assert_snapshot!(util::unidiff(&diff, &input, &printer), @r#"
    @@ -2,1 +2,1 @@
    -    let x = 1;
    +    let x = 2;
    @@ -4,0 +4,1 @@
    +    println!("done");
    "#);
    Ok(())
}

/// Test that the `UnifiedDiffPrinter` can be used with `BStr` and the v0.2 API
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

    let input = v2::InternedInput::new(before, after);
    let diff = diff_with_slider_heuristics(v2::Algorithm::Histogram, &input);

    let printer = v2::BasicLineDiffPrinter(&input.interner);

    insta::assert_snapshot!(util::unidiff(&diff, &input, &printer), @r#"
    @@ -2,1 +2,1 @@
    -    let x = 1;
    +    let x = 2;
    @@ -4,0 +4,1 @@
    +    println!("done");
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

    let input = v2::InternedInput::new(before, after);
    let diff = diff_with_slider_heuristics(v2::Algorithm::Histogram, &input);

    let printer = v2::BasicLineDiffPrinter(&input.interner);
    insta::assert_snapshot!(util::unidiff(&diff, &input, &printer), @r#"
    @@ -4,0 +4,1 @@
    +        println!("world");
    "#);

    Ok(())
}

/// Test that Myers algorithm also works with slider heuristics
#[test]
fn myers_with_slider_heuristics() -> crate::Result {
    let before = "a\nb\nc\n";
    let after = "a\nx\nc\n";

    let input = v2::InternedInput::new(before, after);
    let diff = diff_with_slider_heuristics(v2::Algorithm::Myers, &input);

    let printer = v2::BasicLineDiffPrinter(&input.interner);
    insta::assert_snapshot!(util::unidiff(&diff, &input, &printer), @r"
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

    let input = v2::InternedInput::new(before, after);
    let diff = diff_with_slider_heuristics(v2::Algorithm::Histogram, &input);

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

    let input = v2::InternedInput::new(before, after);
    let diff = diff_with_slider_heuristics(v2::Algorithm::Histogram, &input);

    let printer = v2::BasicLineDiffPrinter(&input.interner);
    insta::assert_snapshot!(util::unidiff(&diff, &input, &printer), @r"
    @@ -3,0 +3,1 @@
    +    y: i32,
    @@ -7,1 +8,1 @@
    -        Foo { x: 0 }
    +        Foo { x: 0, y: 0 }
    ");

    Ok(())
}

/// Test custom context size in UnifiedDiffConfig
#[test]
fn custom_context_size() -> crate::Result {
    let before = "line1\nline2\nline3\nline4\nline5\nline6\nline7\n";
    let after = "line1\nline2\nline3\nMODIFIED\nline5\nline6\nline7\n";

    let input = v2::InternedInput::new(before, after);
    let diff = diff_with_slider_heuristics(v2::Algorithm::Histogram, &input);

    let printer = v2::BasicLineDiffPrinter(&input.interner);

    // Test with context size of 1
    let mut config = v2::UnifiedDiffConfig::default();
    config.context_len(1);
    let unified = diff.unified_diff(&printer, config, &input);
    insta::assert_snapshot!(unified, @r"
    @@ -3,3 +3,3 @@
     line3
    -line4
    +MODIFIED
     line5
    ");

    // Test with context size of 3 (default)
    let config_default = v2::UnifiedDiffConfig::default();
    let unified_default = diff.unified_diff(&printer, config_default, &input);

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

    let input = v2::InternedInput::new(before, after);
    let diff = diff_with_slider_heuristics(v2::Algorithm::Histogram, &input);

    let hunks: Vec<_> = diff.hunks().collect();

    let printer = v2::BasicLineDiffPrinter(&input.interner);
    insta::assert_snapshot!(util::unidiff(&diff, &input, &printer), @r"
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

    let input = v2::InternedInput::new(before, after);

    // Create diff but postprocess without heuristic
    let mut diff = v2::Diff::compute(v2::Algorithm::Histogram, &input);
    diff.postprocess_no_heuristic(&input);

    let printer = v2::BasicLineDiffPrinter(&input.interner);
    insta::assert_snapshot!(util::unidiff(&diff, &input, &printer), @r"
    @@ -2,1 +2,1 @@
    -b
    +X
    ");

    Ok(())
}

/// Test that the v0.2 API exposes the IndentHeuristic
#[test]
fn indent_heuristic_available() -> crate::Result {
    let before = "fn foo() {\n    x\n}\n";
    let after = "fn foo() {\n    y\n}\n";

    let input = v2::InternedInput::new(before, after);

    let mut diff = v2::Diff::compute(v2::Algorithm::Histogram, &input);

    let heuristic = v2::IndentHeuristic::new(|token| {
        let line: &str = input.interner[token];
        v2::IndentLevel::for_ascii_line(line.as_bytes().iter().copied(), 4)
    });

    diff.postprocess_with_heuristic(&input, heuristic);

    let printer = v2::BasicLineDiffPrinter(&input.interner);
    insta::assert_snapshot!(util::unidiff(&diff, &input, &printer), @r"
    @@ -2,1 +2,1 @@
    -    x
    +    y
    ");

    Ok(())
}

mod util {
    use std::fmt::Display;
    use std::hash::Hash;

    use gix_diff::blob::v2;

    pub fn unidiff<'a, T: AsRef<[u8]> + ?Sized + Hash + Eq + Display>(
        diff: &'a v2::Diff,
        input: &'a v2::InternedInput<&T>,
        printer: &'a v2::BasicLineDiffPrinter<'_, T>,
    ) -> v2::UnifiedDiff<'a, v2::BasicLineDiffPrinter<'a, T>> {
        let mut config = v2::UnifiedDiffConfig::default();
        config.context_len(0);
        diff.unified_diff(printer, config, input)
    }
}
