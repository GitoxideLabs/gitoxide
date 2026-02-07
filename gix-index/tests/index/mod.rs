mod access;
mod entry;
mod file;
mod fs;
mod init;

use std::path::{Path, PathBuf};

use gix_hash::ObjectId;
use gix_testtools::size_ok;

pub fn hex_to_id(hex: &str) -> ObjectId {
    ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex")
}

pub fn fixture_index_path(name: &str) -> PathBuf {
    let dir =
        gix_testtools::scripted_fixture_read_only_standalone(Path::new("make_index").join(name).with_extension("sh"))
            .expect("script works");
    dir.join(".git").join("index")
}

pub fn loose_file_path(name: &str) -> PathBuf {
    gix_testtools::fixture_path_standalone(Path::new("loose_index").join(name).with_extension("git-index"))
}

#[test]
fn size_of_entry() {
    let actual = std::mem::size_of::<gix_index::Entry>();
    let sha1 = 80;
    let sha256_extra = 16;
    let expected = sha1 + sha256_extra;
    assert!(
        size_ok(actual, expected),
        "the size of this structure should not change unexpectedly: {actual} <~ {expected}"
    );
}

#[test]
fn size_of_entry_time() {
    // The reason we have our own time is that it is half the size.
    let ent_actual = std::mem::size_of::<gix_index::entry::stat::Time>();
    let ent_expected = 8;
    assert!(
        size_ok(ent_actual, ent_expected),
        "the size of this structure should not change unexpectedly: {ent_actual} <~ {ent_expected}"
    );
    let ft_actual = std::mem::size_of::<filetime::FileTime>();
    let ft_expected = 16;
    assert!(
        size_ok(ft_actual, ft_expected),
        "we will want to know if the size of this structure changes: {ft_actual} <~ {ft_expected}"
    );
}

enum Fixture {
    Generated(&'static str),
    Loose(&'static str),
}

impl Fixture {
    pub fn to_path(&self) -> PathBuf {
        match self {
            Fixture::Generated(name) => fixture_index_path(name),
            Fixture::Loose(name) => loose_file_path(name),
        }
    }
    pub fn to_name(&self) -> &'static str {
        match self {
            Fixture::Generated(name) | Fixture::Loose(name) => name,
        }
    }

    pub fn open(&self) -> gix_index::File {
        gix_index::File::at(self.to_path(), gix_hash::Kind::Sha1, false, Default::default())
            .expect("fixtures are always readable")
    }
}
