use bstr::ByteSlice;

pub(crate) mod pipeline;
mod platform;
mod slider;
mod textconv;
mod unified_diff;

pub(crate) fn new_attributes_stack(worktree_root: impl Into<std::path::PathBuf>) -> gix_worktree::Stack {
    gix_worktree::Stack::new(
        worktree_root,
        gix_worktree::stack::State::AttributesStack(gix_worktree::stack::state::Attributes::new(
            Default::default(),
            None,
            gix_worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
            Default::default(),
        )),
        gix_worktree::glob::pattern::Case::Sensitive,
        Vec::new(),
        Vec::new(),
    )
}

pub(crate) fn skip_header_and_fold_to_unidiff(content: &[u8]) -> String {
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
        let line = line.to_str().expect("baseline diff is valid utf-8");
        if let Some((ranges, _section)) = line.strip_prefix("@@ ").and_then(|line| line.split_once(" @@")) {
            out.push_str("@@ ");
            out.push_str(ranges);
            out.push_str(" @@");
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

#[test]
fn git_hunk_sections_are_omitted_from_unified_diff_baselines() {
    let git = br#"diff --git a/before b/after
index 0000000..1111111 100644
--- a/before
+++ b/after
@@ -1,3 +1,3 @@ fn example() {
 fn example() {
-    before();
+    after();
 }
"#;
    let expected = r#"@@ -1,3 +1,3 @@
 fn example() {
-    before();
+    after();
 }
"#;

    assert_eq!(
        skip_header_and_fold_to_unidiff(git),
        expected,
        "Git's optional hunk section isn't part of gix-diff's unified diff output"
    );
}
