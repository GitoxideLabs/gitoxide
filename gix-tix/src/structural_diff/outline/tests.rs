use super::{Change, Language, Options, PreparedDiff, RenderOptions};
use gix_testtools::TestResult;

fn text(prepared: &PreparedDiff, options: &Options) -> String {
    display(prepared.render(options))
}

fn display(lines: Vec<super::Line>) -> String {
    lines
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
    let compact = display(prepared.render_with(
        &Options::default(),
        RenderOptions {
            omit_unchanged_parameters: true,
            ..RenderOptions::default()
        },
    ));
    assert!(
        compact.contains("~\tpub fn calculate(…) -> i32 { … }"),
        "body-only edits keep the return type in a compact signature: {compact}"
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
    let raw = signature.render_with(
        &options,
        RenderOptions {
            omit_word_delimiters: true,
            ..RenderOptions::default()
        },
    );
    for (change, word) in [(Change::Removed, "i32"), (Change::Added, "u32")] {
        assert!(
            raw.iter()
                .flat_map(|line| &line.fragments)
                .any(|fragment| fragment.change == change && fragment.text == word),
            "marker-free fragments keep their change kind and source text"
        );
    }
    options.word_diff = false;
    let output = text(&signature, &options);
    assert!(
        output.contains("-fn f(arg: i32)") && output.contains("+fn f(arg: u32)"),
        "word mode can return to separate declaration lines: {output}"
    );
    Ok(())
}

#[test]
fn compact_signatures_keep_changed_arguments_in_order_and_preserve_the_return_type() -> TestResult {
    let before = "impl Widget {
    #[inline]
    fn convert<T>(
        &self,
        coordinates: (u8, u8),
        keep: bool,
        old: u16,
    ) -> T where T: Copy { hidden_before() }
}
trait Shape {
    fn order(&self, left: u8, right: u8) -> usize;
}
fn deleted(flag: bool) -> bool { hidden_before() }
";
    let after = "impl Widget {
    #[inline]
    fn convert<T>(
        &self,
        coordinates: (u16, u8),
        keep: bool,
        #[cfg(test)] fresh: u32,
    ) -> T where T: Copy { hidden_after() }
}
trait Shape {
    fn order(&self, right: u8, left: u8) -> usize;
}
fn added(flag: bool) -> bool { hidden_after() }
";
    let prepared = PreparedDiff::new(before, after, Language::Rust)?;
    let options = Options {
        context: vec![0],
        ..Options::default()
    };
    let compact = display(prepared.render_with(
        &options,
        RenderOptions {
            omit_unchanged_parameters: true,
            ..RenderOptions::default()
        },
    ));
    for line in [
        "-    fn convert<T>(…, coordinates: (u8, u8), …, old: u16) -> T where T: Copy { … }",
        "+    fn convert<T>(…, coordinates: (u16, u8), …, #[cfg(…)] fresh: u32) -> T where T: Copy { … }",
        "-fn deleted(flag: bool) -> bool { … }",
        "+fn added(flag: bool) -> bool { … }",
    ] {
        assert!(
            compact.lines().any(|candidate| candidate == line),
            "compact signatures retain {line:?}: {compact}"
        );
    }
    let reordered: Vec<_> = compact.lines().filter(|line| line.contains("fn order(")).collect();
    assert_eq!(reordered.len(), 2, "moving a parameter remains visible: {compact}");
    assert!(
        reordered
            .iter()
            .all(|line| line.contains('…') && line.ends_with("-> usize;")),
        "trait method signatures retain their return type and semicolon: {compact}"
    );
    assert!(
        !compact.contains("keep: bool") && !compact.contains("hidden_") && !compact.contains("…, …"),
        "unchanged arguments and hidden bodies are collapsed without repeated placeholders: {compact}"
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

#[test]
fn production_only_removes_nested_tests_and_benchmarks_from_whole_file_changes() -> TestResult {
    let source = "mod api {
    pub fn production() { secret_production(); }
    #[cfg(test)]
    mod tests {
        fn smoke() { secret_test(); }
    }
    #[tokio::test]
    async fn async_test() { secret_test(); }
    #[bench]
    fn benchmark() { secret_benchmark(); }
    #[cfg(all(test, unix))]
    const TEST_ONLY: u32 = 1;
    #[cfg(not(test))]
    fn production_config() {}
    #[cfg(any(test, feature = \"runtime\"))]
    fn maybe_production() {}
}";
    for (before, after, marker) in [("", source, '+'), (source, "", '-')] {
        let prepared = PreparedDiff::new(before, after, Language::Rust)?;
        let options = Options {
            production_only: true,
            ..Options::default()
        };
        assert!(
            prepared.has_code(&options),
            "whole-file changes contain production declarations"
        );
        assert!(
            prepared.has_changes(&options),
            "production additions and removals are changes"
        );
        let output = text(&prepared, &options);
        for declaration in [
            "mod api",
            "fn production",
            "fn production_config",
            "fn maybe_production",
        ] {
            assert!(
                output.contains(declaration),
                "production declarations remain visible: {output}"
            );
        }
        for declaration in ["mod tests", "smoke", "async_test", "benchmark", "TEST_ONLY"] {
            assert!(
                !output.contains(declaration),
                "non-production declaration {declaration} is hidden: {output}"
            );
        }
        assert!(
            output.lines().all(|line| line.starts_with(marker)),
            "whole-file production changes retain their addition or removal markers"
        );
    }
    Ok(())
}

#[test]
fn test_only_edits_do_not_mark_production_containers_as_modified() -> TestResult {
    let before = "mod api {
    impl Widget {
        fn production() { secret_production(); }
        #[cfg(test)]
        fn test_helper() { secret_before(); }
    }
    #[cfg(test)]
    mod tests {
        #[test]
        fn smoke() { secret_before(); }
    }
}";
    let after = before.replace("secret_before", "secret_after");
    let prepared = PreparedDiff::new(before, &after, Language::Rust)?;
    let mut options = Options::default();
    assert!(!prepared.render(&options).is_empty(), "tests are visible by default");
    options.production_only = true;
    assert!(
        prepared.render(&options).is_empty(),
        "test changes do not leave modified modules or implementations"
    );
    options.production_only = false;
    assert!(
        !prepared.render(&options).is_empty(),
        "the same outlines can restore test changes"
    );
    options.production_only = true;
    let mixed = PreparedDiff::new(
        before,
        &after.replace("secret_production", "changed_production"),
        Language::Rust,
    )?;
    let output = text(&mixed, &options);
    assert!(
        output.contains("~        fn production"),
        "production body changes still mark their function: {output}"
    );
    for declaration in ["test_helper", "mod tests", "smoke"] {
        assert!(
            !output.contains(declaration),
            "test declarations are absent even from context: {output}"
        );
    }

    let production = "mod api {
    fn production() {}
}";
    let with_tests = "mod api {
    fn production() {}
    #[test]
    fn added_test() { secret_test(); }
}";
    options.production_only = true;
    for (before, after) in [(production, with_tests), (with_tests, production)] {
        assert!(
            PreparedDiff::new(before, after, Language::Rust)?
                .render(&options)
                .is_empty(),
            "adding or removing tests alone leaves no production diff"
        );
    }
    Ok(())
}

#[test]
fn cfg_predicates_preserve_code_that_can_be_enabled_without_tests() -> TestResult {
    for (condition, visible) in [
        ("test", false),
        ("all(test, unix)", false),
        ("all(/* test gate */ test, unix)", false),
        ("all(feature = \"runtime\", test)", false),
        ("any(test, feature = \"runtime\")", true),
        ("not(test)", true),
        ("not(all(test, unix))", true),
        ("not(any(not(test), unix))", false),
        ("any(all(test, unix), all(test, feature = \"runtime\"))", false),
        ("not(not(test))", false),
        ("feature = \"test\"", true),
        ("test_feature", true),
        ("all(not(test), unix)", true),
        ("all()", true),
        ("any()", false),
    ] {
        let source = format!(
            "#[cfg({condition})]
fn gated() {{ secret_body(); }}"
        );
        let prepared = PreparedDiff::new("", &source, Language::Rust)?;
        let output = text(
            &prepared,
            &Options {
                production_only: true,
                ..Options::default()
            },
        );
        assert_eq!(
            output.contains("fn gated"),
            visible,
            "unknown configuration predicates must not hide potentially production code: {condition}"
        );
    }
    Ok(())
}

#[test]
fn inner_test_attributes_hide_entire_files_and_modules() -> TestResult {
    let options = Options {
        production_only: true,
        ..Options::default()
    };
    for source in [
        "#![cfg(test)]
fn test_helper() { secret_test(); }",
        "mod test_support {
    #![cfg(all(test, unix))]
    fn test_helper() { secret_test(); }
}",
    ] {
        let prepared = PreparedDiff::new("", source, Language::Rust)?;
        assert!(
            !prepared.has_code(&options),
            "test-only scopes contain no production declarations"
        );
        assert!(
            !prepared.has_changes(&options),
            "test-only scopes contain no production changes"
        );
        assert!(
            prepared.render(&options).is_empty(),
            "test-only scopes leave no headers or elisions"
        );
    }
    Ok(())
}

#[test]
fn production_only_keeps_hidden_container_signature_and_attribute_changes() -> TestResult {
    let before = "#[cfg(feature = \"before\")]
pub struct Config<const N: usize = { secret_before(); 3 }> {
    #[cfg(test)]
    test_only: bool,
    pub data: [u8; N],
}";
    let options = Options {
        production_only: true,
        ..Options::default()
    };
    for after in [
        before.replace("secret_before", "secret_after"),
        before.replace("feature = \"before\"", "feature = \"after\""),
    ] {
        let prepared = PreparedDiff::new(before, &after, Language::Rust)?;
        assert!(
            prepared.has_changes(&options),
            "hidden production expressions and attributes still participate in comparison"
        );
        let output = text(&prepared, &options);
        assert!(
            output.contains("~pub struct Config"),
            "production signature changes mark their container: {output}"
        );
        assert!(
            !output.contains("secret_") && !output.contains("test_only"),
            "hidden expressions and test fields remain absent"
        );
    }
    Ok(())
}

#[test]
fn production_only_omits_test_fields_from_tuple_structures() -> TestResult {
    let before = "struct Config(
    #[cfg(test)]
    TestBefore,
    pub Production,
);";
    let options = Options {
        production_only: true,
        ..Options::default()
    };
    let after = before.replace("TestBefore", "TestAfter");
    let prepared = PreparedDiff::new(before, &after, Language::Rust)?;
    assert!(
        !prepared.has_changes(&options),
        "test tuple-field changes leave no production diff"
    );
    let all = text(&prepared, &Options::default());
    assert!(
        all.contains("TestBefore") && all.contains("TestAfter"),
        "test tuple fields remain visible by default"
    );
    let production = "struct Config(
    pub Production,
);";
    for (before, after) in [(before, production), (production, before)] {
        assert!(
            !PreparedDiff::new(before, after, Language::Rust)?.has_changes(&options),
            "adding or removing a test tuple field leaves production field positions unchanged"
        );
    }
    for (before, after) in [("", before), (before, "")] {
        let prepared = PreparedDiff::new(before, after, Language::Rust)?;
        let output = text(&prepared, &options);
        assert!(
            output.contains("struct Config") && output.contains("pub Production"),
            "production tuple declarations and fields remain visible"
        );
        assert!(
            !output.contains("TestBefore") && !output.contains("cfg("),
            "test tuple fields and their attributes are omitted"
        );
    }
    let changed = before.replace("Production", "Updated");
    let output = text(&PreparedDiff::new(before, &changed, Language::Rust)?, &options);
    assert!(
        output.contains("Updated"),
        "production tuple-field changes remain visible"
    );
    assert!(
        !output.contains("TestBefore"),
        "test tuple fields are absent even from context"
    );
    Ok(())
}
