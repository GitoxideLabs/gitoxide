use std::{path::PathBuf, sync::Arc};

use gix_pack::data::output;
use gix_testtools::size_ok;

#[test]
fn size_of_entry() {
    let actual = std::mem::size_of::<output::Entry>();
    let sha1 = 80;
    let sha256_extra = 32;
    let expected = sha1 + sha256_extra;
    assert!(
        size_ok(actual, expected),
        "The size of the structure shouldn't change unexpectedly: {actual} <~ {expected}"
    );
}

#[test]
fn size_of_count() {
    let actual = std::mem::size_of::<output::Count>();
    let sha1 = 56;
    let sha256_extra = 16;
    let expected = sha1 + sha256_extra;
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
