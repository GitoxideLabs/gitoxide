use std::path::PathBuf;

use gix_pack::multi_index::File;

fn multi_index() -> (File, PathBuf) {
    let path = crate::scripted_fixture_read_only("make_pack_gen_repo_multi_index.sh")
        .expect("test fixture exists")
        .join(".git/objects/pack/multi-pack-index");
    let file = gix_pack::multi_index::File::at(&path, None).expect("multi-index fixture opens");
    (file, path)
}

mod access;
mod fuzzed;

mod verify;

mod write;
