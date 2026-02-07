use gix_testtools::Creation;
pub use gix_testtools::Result;

mod index_as_worktree;
mod index_as_worktree_with_renames;

mod stack;

pub fn fixture_path(name: &str) -> std::path::PathBuf {
    let dir = gix_testtools::scripted_fixture_read_only_standalone(std::path::Path::new(name).with_extension("sh"))
        .expect("script works");
    dir
}

pub fn fixture_path_rw_slow(name: &str) -> gix_testtools::tempfile::TempDir {
    let tmp = gix_testtools::scripted_fixture_writable_with_args_standalone_single_archive(
        std::path::Path::new(name).with_extension("sh"),
        None::<String>,
        Creation::Execute,
    )
    .expect("script works");
    tmp
}

fn hex_to_id(hex: &str) -> gix_hash::ObjectId {
    gix_hash::ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex")
}
