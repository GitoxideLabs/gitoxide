use super::{Change, Language, Options, PreparedDiff};
use gix_testtools::TestResult;

fn text(prepared: &PreparedDiff, options: &Options) -> String {
    prepared
        .render(options)
        .into_iter()
        .map(|line| {
            format!(
                "{}{}",
                line.change.marker(),
                line.fragments
                    .into_iter()
                    .map(|fragment| fragment.text)
                    .collect::<String>()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn bodies_mark_their_declarations_without_exposing_implementation() -> TestResult {
    let before = "impl Widget {
\tpub fn calculate(
\t\t&self,
\t\tinput: i32,
\t) -> i32 {
\t\tlet secret_before = input + 1;
\t\tsecret_before
\t}
}";
    let after = before.replace("secret_before", "secret_after").replace("+ 1", "+ 2");
    let prepared = PreparedDiff::new(before, &after, Language::Rust)?;
    let output = text(&prepared, &Options::default());
    assert!(
        output.contains("impl Widget {"),
        "owning declarations provide context: {output}"
    );
    assert!(
        output.contains("~\tpub fn calculate("),
        "a body change marks its function: {output}"
    );
    assert!(
        output.contains("\t\tinput: i32,"),
        "multiline source indentation is retained"
    );
    assert!(
        !output.contains("secret_"),
        "implementation text participates only in comparison"
    );
    Ok(())
}

#[test]
fn modules_traits_fields_and_added_removed_functions_remain_structural() -> TestResult {
    let before = "mod api {
    pub struct Config {
        pub before: i32,
    }
    pub trait Service {
        fn old(&self);
    }
    fn removed() { secret_removed(); }
}";
    let after = "mod api {
    pub struct Config {
        pub after: u32,
    }
    pub trait Service {
        fn new(&self);
    }
    fn added() { secret_added(); }
}";
    let prepared = PreparedDiff::new(before, after, Language::Rust)?;
    let output = text(&prepared, &Options::default());
    for declaration in [
        "mod api",
        "struct Config",
        "trait Service",
        "pub before: i32",
        "pub after: u32",
        "fn old",
        "fn new",
    ] {
        assert!(output.contains(declaration), "{declaration} remains visible: {output}");
    }
    assert!(
        output
            .lines()
            .any(|line| line.starts_with('-') && line.contains("fn removed")),
        "deleted functions are marked"
    );
    assert!(
        output
            .lines()
            .any(|line| line.starts_with('+') && line.contains("fn added")),
        "added functions are marked"
    );
    assert!(
        !output.contains("secret_"),
        "whole declarations never reveal executable bodies"
    );
    Ok(())
}

#[test]
fn renames_and_word_changes_can_be_redrawn_from_the_same_inputs() -> TestResult {
    let renamed = PreparedDiff::new(
        "fn old(value: u32) -> u32 { old(value) + 1 }",
        "fn new(value: u32) -> u32 { new(value) + 2 }",
        Language::Rust,
    )?;
    let mut options = Options {
        find_renames: true,
        word_diff: true,
        ..Options::default()
    };
    let output = text(&renamed, &options);
    assert!(
        output.contains("[renamed: old -> new]"),
        "unique similar declarations are paired: {output}"
    );
    assert!(
        output.contains("[-old-]{+new+}"),
        "paired names receive word changes: {output}"
    );
    options.find_renames = false;
    let output = text(&renamed, &options);
    assert!(
        !output.contains("[renamed:"),
        "disabling matching removes rename annotations"
    );
    assert!(
        output.contains("-fn old") && output.contains("+fn new"),
        "unpaired declarations remain additions and removals: {output}"
    );

    let signature = PreparedDiff::new("fn f(arg: i32) {}", "fn f(arg: u32) {}", Language::Rust)?;
    let lines = signature.render(&options);
    assert!(
        lines
            .iter()
            .flat_map(|line| &line.fragments)
            .any(|fragment| fragment.change == Change::Removed && fragment.text == "[-i32-]"),
        "only removed words carry removal styling"
    );
    assert!(
        lines
            .iter()
            .flat_map(|line| &line.fragments)
            .any(|fragment| fragment.change == Change::Added && fragment.text == "{+u32+}"),
        "only inserted words carry addition styling"
    );
    options.word_diff = false;
    let output = text(&signature, &options);
    assert!(
        output.contains("-fn f(arg: i32)") && output.contains("+fn f(arg: u32)"),
        "word mode can return to separate declaration lines: {output}"
    );
    Ok(())
}

#[test]
fn toml_values_and_markdown_content_mark_keys_and_headings() -> TestResult {
    for (before, after, language, context, changed) in [
        (
            "[package]
name = \"secret_before\"
",
            "[package]
name = \"secret_after\"
",
            Language::Toml,
            "[package]",
            "name = …",
        ),
        (
            "# Guide

## Details
secret_before
",
            "# Guide

## Details
secret_after
",
            Language::Markdown,
            "# Guide",
            "## Details",
        ),
    ] {
        let prepared = PreparedDiff::new(before, after, language)?;
        let output = text(&prepared, &Options::default());
        assert!(output.contains(context), "owning containers remain visible: {output}");
        assert!(
            output
                .lines()
                .any(|line| line.starts_with('~') && line.contains(changed)),
            "hidden content changes mark its owner: {output}"
        );
        assert!(!output.contains("secret_"), "values and prose remain hidden");
    }
    Ok(())
}

#[test]
fn multiline_word_changes_keep_unicode_and_source_indentation() -> TestResult {
    let before = "fn café(
\targ: i32,
) -> i32 { arg }";
    let after = before.replace("arg: i32", "arg: u32");
    let prepared = PreparedDiff::new(before, &after, Language::Rust)?;
    let output = text(
        &prepared,
        &Options {
            word_diff: true,
            ..Options::default()
        },
    );
    assert!(
        output.contains("~fn café("),
        "Unicode identifiers remain intact: {output}"
    );
    assert!(
        output.contains("~\targ: [-i32-]{+u32+},"),
        "changed words retain surrounding multiline layout: {output}"
    );
    Ok(())
}

#[test]
fn visible_terminal_controls_are_escaped() -> TestResult {
    let before = "# Guide \u{1b}[31m

secret_before";
    let after = before.replace("secret_before", "secret_after");
    let prepared = PreparedDiff::new(before, &after, Language::Markdown)?;
    let output = text(&prepared, &Options::default());
    assert!(
        !output.contains('\u{1b}'),
        "visible source cannot inject terminal styling: {output}"
    );
    assert!(output.contains("\\u{1b}"), "escaped controls remain observable");
    assert!(!output.contains("secret_"), "section content remains hidden");
    Ok(())
}

#[test]
fn executable_expressions_and_macro_token_trees_are_hidden() -> TestResult {
    let before = "const CALLBACK: fn() -> i32 = || secret_before();
const VALUE: i32 = { secret_before(); 1 };
fn signature(_: [u8; { secret_before(); 3 }]) { secret_before(); }
struct Config { buffer: [u8; { secret_before(); 3 }], }
enum Kind { Value = { secret_before(); 1 }, }
macro_rules! opaque { () => { secret_before() }; }
opaque! { fn generated() { secret_before(); } }";
    let after = before.replace("secret_before", "secret_after");
    let prepared = PreparedDiff::new(before, &after, Language::Rust)?;
    let output = text(&prepared, &Options::default());
    assert!(
        output.contains("CALLBACK") && output.contains("fn signature"),
        "visible declaration shapes remain: {output}"
    );
    assert!(
        !output.contains("secret_"),
        "closures, const expressions, and token trees remain hidden: {output}"
    );
    assert!(
        !output.contains("fn generated"),
        "opaque macro invocations do not expose generated code"
    );
    Ok(())
}

#[test]
fn whole_file_additions_and_deletions_show_only_declarations() -> TestResult {
    let source = "struct Config { field: u32 }
fn calculate() { secret_body(); }";
    for (before, after, expected) in [("", source, Change::Added), (source, "", Change::Removed)] {
        let prepared = PreparedDiff::new(before, after, Language::Rust)?;
        let options = Options {
            find_renames: true,
            word_diff: true,
            ..Options::default()
        };
        let lines = prepared.render(&options);
        assert!(!lines.is_empty(), "whole-file changes retain their declarations");
        assert!(
            lines.iter().all(|line| line.change == expected),
            "every declaration belongs to the whole-file change"
        );
        assert!(
            !text(&prepared, &options).contains("secret_body"),
            "whole-file diffs keep bodies hidden"
        );
    }
    Ok(())
}

#[test]
fn malformed_inputs_fail_and_identical_inputs_are_empty() -> TestResult {
    for (before, after) in [("fn ok() {}", "fn broken("), ("fn broken(", "fn ok() {}")] {
        assert!(
            PreparedDiff::new(before, after, Language::Rust).is_err(),
            "malformed syntax cannot fall back to an implementation diff"
        );
    }
    let prepared = PreparedDiff::new("fn same() {}", "fn same() {}", Language::Rust)?;
    assert!(
        prepared.render(&Options::default()).is_empty(),
        "identical structural inputs emit no rows"
    );
    for (path, expected) in [
        ("src/main.rs", Some(Language::Rust)),
        ("Cargo.toml", Some(Language::Toml)),
        ("README.MD", Some(Language::Markdown)),
        ("binary.dat", None),
    ] {
        assert_eq!(
            Language::from_path(std::path::Path::new(path)),
            expected,
            "the original filename selects a supported grammar"
        );
    }
    Ok(())
}
