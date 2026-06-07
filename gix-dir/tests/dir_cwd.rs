use std::path::Path;

use gix_dir::{
    entry::{Kind::File, PathspecMatch::Prefix, Status::Untracked},
    walk,
};
use pretty_assertions::assert_eq;

use crate::walk_utils::{collect, entryps, fixture, options};

pub mod walk_utils;

#[test]
fn prefixes_work_as_expected() -> gix_testtools::Result {
    let root = fixture("only-untracked");
    std::env::set_current_dir(root.join("d"))?;
    let troot = Path::new("..").join("d");
    let ((out, _root), entries) = collect(Path::new(".."), Some(&troot), |keep, ctx| {
        walk(Path::new(".."), ctx, options(), keep)
    });
    assert_eq!(
        out,
        walk::Outcome {
            read_dir_calls: 2,
            returned_entries: entries.len(),
            seen_entries: 3,
            ..Default::default()
        }
    );
    assert_eq!(
        &entries,
        &[
            entryps("d/a", Untracked, File, Prefix),
            entryps("d/b", Untracked, File, Prefix),
            entryps("d/d/a", Untracked, File, Prefix),
        ]
    );
    Ok(())
}
