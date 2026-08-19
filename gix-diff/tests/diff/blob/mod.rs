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
        out.push_str(line.to_str().expect("baseline diff is valid utf-8"));
        out.push('\n');
    }
    out
}
