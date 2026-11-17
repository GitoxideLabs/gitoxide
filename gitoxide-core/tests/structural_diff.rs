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
