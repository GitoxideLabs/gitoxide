use std::{path::PathBuf, sync::Arc};

use gix_pack::data::output;
use gix_testtools::size_ok;

#[test]
fn size_of_entry() {
    let actual = std::mem::size_of::<output::Entry>();
    let expected = 112;
    assert!(
        size_ok(actual, expected),
        "The size of the structure shouldn't change unexpectedly: {actual} <~ {expected}"
    );
}

#[test]
fn size_of_count() {
    let actual = std::mem::size_of::<output::Count>();
    let expected = 72;
    assert!(
        size_ok(actual, expected),
        "The size of the structure shouldn't change unexpectedly: {actual} <~ {expected}"
    );
}

enum DbKind {
    DeterministicGeneratedContent,
    DeterministicGeneratedContentMultiIndex,
}

fn db(kind: DbKind) -> crate::Result<gix_odb::HandleArc> {
    use DbKind::*;
    let name = match kind {
        DeterministicGeneratedContent => "make_pack_gen_repo.sh",
        DeterministicGeneratedContentMultiIndex => "make_pack_gen_repo_multi_index.sh",
    };
    let path: PathBuf = crate::scripted_fixture_read_only(name)?.join(".git").join("objects");
    gix_odb::Store::at_opts(path, &mut None.into_iter(), gix_odb::store::init::Options::default())
        .map_err(Into::into)
        .map(|store| {
            let mut cache = Arc::new(store).to_cache_arc();
            cache.prevent_pack_unload();
            cache
        })
}

mod count_and_entries;
