#![cfg(feature = "structural-diff")]

use std::path::{Path, PathBuf};

use gitoxide_core::structural_diff::{Language, Options, diff};
use gix_testtools::TestResult;

fn render(before: &str, after: &str, options: Options) -> TestResult<String> {
    let fixture = gix_testtools::scripted_fixture_read_only("structural-diff.sh")?;
    let mut out = Vec::new();
    diff(&fixture.join(before), &fixture.join(after), options, &mut out)?;
    Ok(String::from_utf8(out)?)
}

#[test]
fn body_edits_mark_the_function_and_keep_its_source_indentation() -> TestResult {
    let out = render("body-before.rs", "body-after.rs", Options::default())?;
    assert!(
        out.contains("impl Widget {"),
        "the owning implementation provides context"
    );
    assert!(
        out.lines()
            .any(|line| line.starts_with('~') && line.contains("\tpub fn calculate(")),
        "a hidden body edit marks its function as modified without reindenting the signature: {out}"
    );
    assert!(
        out.contains("\t\tinput: i32,"),
        "multiline signatures retain their original tabs"
    );
    assert!(
        !out.contains("secret_"),
        "function statements never appear in the outline"
    );
    assert!(!out.contains('\u{1b}'), "plain output has no terminal color escapes");
    Ok(())
}

#[test]
fn additions_and_removals_show_declarations_without_their_bodies() -> TestResult {
    let out = render("items-before.rs", "items-after.rs", Options::default())?;
    assert!(
        out.lines()
            .any(|line| line.starts_with('-') && line.contains("fn removed()")),
        "deleted declarations receive a removal marker: {out}"
    );
    assert!(
        out.lines()
            .any(|line| line.starts_with('+') && line.contains("fn added()")),
        "new declarations receive an addition marker: {out}"
    );
    assert!(!out.contains("secret_"), "added and removed bodies remain hidden");
    Ok(())
}

#[test]
fn executable_expressions_and_opaque_macros_remain_hidden() -> TestResult {
    let out = render("hidden-before.rs", "hidden-after.rs", Options::default())?;
    for declaration in [
        "CALLBACK",
        "VALUE",
        "fn signature",
        "struct Config",
        "enum Kind",
        "opaque",
    ] {
        assert!(out.contains(declaration), "the outline retains {declaration}: {out}");
    }
    assert!(
        !out.contains("secret_"),
        "closures, const blocks, discriminants, and macro token trees never expose executable code: {out}"
    );
    assert!(
        !out.contains("fn generated"),
        "macro-generated functions stay inside their opaque invocation"
    );
    Ok(())
}

#[test]
fn toml_value_edits_mark_the_key_under_its_table() -> TestResult {
    let out = render("before.toml", "after.toml", Options::default())?;
    assert!(out.contains("[package]"), "a changed key retains its table context");
    assert!(
        out.lines().any(|line| line.starts_with('~') && line.contains("name")),
        "a hidden value edit marks its key as modified: {out}"
    );
    assert!(
        !out.contains("secret_"),
        "configuration values are omitted from the outline"
    );
    Ok(())
}

#[test]
fn markdown_content_edits_mark_the_heading_and_hide_prose_and_code() -> TestResult {
    for (before, after, heading) in [
        ("before.md", "after.md", "## Details"),
        ("setext-before.md", "setext-after.md", "Details"),
    ] {
        let out = render(before, after, Options::default())?;
        assert!(out.contains("Guide"), "the parent heading remains visible: {out}");
        assert!(
            out.lines().any(|line| line.starts_with('~') && line.contains(heading)),
            "the heading owning changed content is marked as modified: {out}"
        );
        assert!(!out.contains("secret_"), "Markdown prose and fenced code remain hidden");
    }
    let out = render("unheaded-before.md", "unheaded-after.md", Options::default())?;
    assert!(
        out.lines().any(|line| line.starts_with('~')),
        "unheaded changes remain observable: {out}"
    );
    assert!(!out.contains("secret_"), "unheaded prose remains hidden");
    Ok(())
}

#[test]
fn temporary_paths_can_use_the_original_path_or_an_explicit_language() -> TestResult {
    for options in [
        Options {
            path: Some(PathBuf::from("original.rs")),
            ..Options::default()
        },
        Options {
            language: Some(Language::Rust),
            ..Options::default()
        },
    ] {
        let out = render("temporary-before", "temporary-after", options)?;
        assert!(
            out.contains("fn calculate"),
            "temporary names do not prevent structural parsing"
        );
        assert!(
            !out.contains("secret_"),
            "explicit language selection still hides bodies"
        );
    }
    Ok(())
}

#[test]
fn null_file_additions_and_deletions_infer_the_language_from_the_other_side() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("structural-diff.sh")?;
    let source = fixture.join("body-before.rs");
    for (before, after, marker) in [
        (Path::new("/dev/null"), source.as_path(), '+'),
        (source.as_path(), Path::new("/dev/null"), '-'),
    ] {
        let mut out = Vec::new();
        diff(before, after, Options::default(), &mut out)?;
        let out = String::from_utf8(out)?;
        assert!(
            out.lines()
                .any(|line| line.starts_with(marker) && line.contains("fn calculate")),
            "Git's null-file counterpart represents a whole-file addition or deletion: {out}"
        );
        assert!(!out.contains("secret_"), "whole-file changes still hide bodies");
    }
    Ok(())
}

#[test]
fn unsupported_and_malformed_inputs_fail_before_writing_output() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("structural-diff.sh")?;
    for (before, after) in [
        ("unsupported.txt", "unsupported.txt"),
        ("body-before.rs", "malformed.rs"),
    ] {
        let mut out = Vec::new();
        diff(
            &fixture.join(before),
            &fixture.join(after),
            Options::default(),
            &mut out,
        )
        .expect_err("unsupported or incomplete syntax cannot safely produce an outline");
        assert!(out.is_empty(), "failed parsing never emits source or a partial diff");
    }
    Ok(())
}

#[test]
fn unchanged_inputs_produce_no_diff() -> TestResult {
    for path in ["body-before.rs", "before.toml", "before.md", "empty.rs"] {
        let out = render(path, path, Options::default())?;
        assert!(out.is_empty(), "identical {path} produces no diff headers or context");
    }
    Ok(())
}

#[test]
fn forced_color_styles_markers_without_revealing_hidden_content() -> TestResult {
    let out = render(
        "items-before.rs",
        "items-after.rs",
        Options {
            color: true,
            ..Options::default()
        },
    )?;
    assert!(out.contains("\u{1b}["), "forced terminal color emits ANSI styling");
    assert!(!out.contains("secret_"), "color formatting uses only structural output");
    Ok(())
}

#[test]
fn context_counts_siblings_at_each_depth_and_keeps_ancestors() -> TestResult {
    let out = render(
        "context-before.rs",
        "context-after.rs",
        Options {
            context: vec![0, 1],
            ..Options::default()
        },
    )?;
    for visible in ["impl Widget", "fn previous", "fn changed", "fn next"] {
        assert!(
            out.contains(visible),
            "changed ancestors and one sibling each way stay visible: {out}"
        );
    }
    for hidden in ["Unrelated", "Tail", "fn first", "fn last", "secret_"] {
        assert!(!out.contains(hidden), "context does not expand beyond its level: {out}");
    }
    assert!(out.contains('…'), "omitted siblings are marked");

    let out = render(
        "context-before.rs",
        "context-after.rs",
        Options {
            context: vec![1, 0],
            ..Options::default()
        },
    )?;
    assert!(
        out.contains("struct Unrelated"),
        "root context is independent of method context"
    );
    assert!(
        out.contains("struct Tail"),
        "context includes both neighboring siblings"
    );
    assert!(
        !out.contains("hidden_field"),
        "unchanged context containers stay collapsed"
    );
    assert!(
        !out.contains("fn previous"),
        "zero method context shows changed methods only"
    );
    Ok(())
}

#[test]
fn renames_are_opt_in_and_can_cross_unchanged_siblings() -> TestResult {
    let out = render("rename-before.rs", "rename-after.rs", Options::default())?;
    assert!(!out.contains("renamed:"), "similarity matching is disabled by default");
    let out = render(
        "rename-before.rs",
        "rename-after.rs",
        Options {
            find_renames: true,
            context: vec![0],
            ..Options::default()
        },
    )?;
    assert!(
        out.contains("renamed: original -> renamed"),
        "a unique match survives an edited body and a move: {out}"
    );
    assert!(
        out.contains("-fn original") && out.contains("+fn renamed"),
        "the before and after declarations remain visible"
    );
    assert!(!out.contains("secret_original"), "similarity inputs are never printed");
    assert!(
        !out.contains("fn anchor"),
        "rename detection respects structural context"
    );
    Ok(())
}

#[test]
fn word_diffs_preserve_multiline_signatures_and_only_compare_visible_text() -> TestResult {
    let out = render(
        "words-before.rs",
        "words-after.rs",
        Options {
            word_diff: true,
            ..Options::default()
        },
    )?;
    assert!(
        out.contains("~\t\t[-café-]{+thé+}: [-i32-]{+u32+},"),
        "word edits keep Unicode and original tabs: {out}"
    );
    assert!(!out.contains("secret_"), "word diffing never sees bodies");
    assert!(!out.contains("\\n"), "signature newlines remain newlines");
    let out = render(
        "body-before.rs",
        "body-after.rs",
        Options {
            word_diff: true,
            ..Options::default()
        },
    )?;
    assert!(
        !out.contains("[-") && !out.contains("{+"),
        "body-only edits only mark the declaration"
    );
    let out = render(
        "words-before.rs",
        "words-after.rs",
        Options {
            word_diff: true,
            color: true,
            ..Options::default()
        },
    )?;
    assert!(out.contains("\u{1b}["), "generated word colors remain ANSI escapes");
    assert!(
        !out.contains("\\u{1b}"),
        "generated color must not be escaped as source text"
    );
    Ok(())
}

#[test]
fn source_terminal_controls_are_escaped_even_in_word_diffs() -> TestResult {
    for word_diff in [false, true] {
        let out = render(
            "terminal-before.md",
            "terminal-after.md",
            Options {
                word_diff,
                find_renames: true,
                ..Options::default()
            },
        )?;
        assert!(
            !out.contains('\u{1b}'),
            "input controls cannot issue terminal commands: {out}"
        );
        assert!(out.contains("\\u{1b}"), "escaped control bytes remain readable");
        assert!(!out.contains("secret_"), "prose stays hidden");
    }
    Ok(())
}
