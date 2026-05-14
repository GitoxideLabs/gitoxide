use std::collections::HashMap;

use gix_hash::ObjectId;
use gix_testtools::Creation;
pub use gix_testtools::{
    Result, scripted_fixture_read_only, scripted_fixture_writable, scripted_fixture_writable_with_args_single_archive,
};

mod index_as_worktree;
#[cfg(feature = "worktree-rewrites")]
mod index_as_worktree_with_renames;

mod stack;

pub fn fixture_path(name: &str) -> std::path::PathBuf {
    crate::scripted_fixture_read_only(std::path::Path::new(name).with_extension("sh")).expect("script works")
}

pub fn fixture_path_rw_slow(name: &str) -> gix_testtools::tempfile::TempDir {
    crate::scripted_fixture_writable_with_args_single_archive(
        std::path::Path::new(name).with_extension("sh"),
        None::<String>,
        Creation::Execute,
    )
    .expect("script works")
}

fn odb_at(git_dir: &std::path::Path, object_hash: gix_hash::Kind) -> gix_odb::HandleArc {
    gix_odb::at_opts(
        git_dir.join("objects"),
        Vec::new(),
        gix_odb::store::init::Options {
            object_hash,
            ..Default::default()
        },
    )
    .unwrap()
    .into_arc()
    .unwrap()
}

static SHA1_TO_SHA256_HASHES: std::sync::LazyLock<HashMap<&str, &str>> = std::sync::LazyLock::new(|| {
    [
        (
            "3189cd3cb0af8586c39a838aa3e54fd72a872a41",
            "735ec3eb1e74b0815da6d8aeca80ffbffdca25a2b624cc54d5d34caca9bc4dec",
        ),
        (
            "e376f96e6a7f1c9335ca16c3f62e172166146bda",
            "0fd8abe17be34797b2c5f7d31996b753f71ca457f863100da6c72383e4e7ab2d",
        ),
        (
            "ba2906d0666cf726c7eaadd2cd3db615dedfdf3a",
            "e493de051d847062f56a4d6d8535aba26effa5ee920b13f5e2ba87aad8b62ba7",
        ),
        (
            "dde77be9fbfb155ff0473e7fe31781d56d50e5d3",
            "02f895333cb699f3c2093987859e4a8192d9f36577d0324ded4693006660f372",
        ),
        (
            "9daeafb9864cf43055ae93beb0afd6c7d144bfa4",
            "999f24152159e51756a944d32257bf22080ff8608fff87ca9a4a823764e13dbe",
        ),
        (
            "df967b96a579e45a18b8251732d16804b2e56a55",
            "abed979e3cd3667c5a295c2641f8319f950860c65cc168eebd1571c51bb4f6fc",
        ),
        (
            "d244dd0bf67758236f793fd7749a1c814fbfeac4",
            "a9c387002cad1a6d1df14f93a758a8ddead88d8b4490ec2255a26ed361bdd3c9",
        ),
        (
            "e14959721a622239cc8de786a4b8cfcefea8304c",
            "eba7bfbfb4c69d21e48a6c5b424f3d2028565d4652585fdec7ae4a69be204f21",
        ),
        (
            "e019be006cf33489e2d0177a3837a2384eddebc5",
            "8e57afe4b9ab5713ce94fb5f1aa4ee7e2922b2b9ec5ee51a661b6af0bf8312cf",
        ),
        (
            "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
            "473a0f4c3be8a93681a267e3b1e9a7dcda1185436fe141f7749120a303721813",
        ),
        (
            "c7747099cf9e073babc68f52cdfb4d280ba5689f",
            "4be0e5ba4a5a905c1ebfdc2459c2cdda407c4d25dcf169ad5cc401f6aa5abccd",
        ),
        (
            "0835e4f9714005ed591f68d306eea0d6d2ae8fd7",
            "2f46b964cf615d9cdcc0da6c78c0ace2e8839486f8bd72a19dde75063c355634",
        ),
        (
            "b1b716105590454bfc4c0247f193a04088f39c7f",
            "e23d150d7b09ce3cabcc858d8d866a8c3dbd11a8f8eb956a915c3cc76e3297ce",
        ),
        (
            "e45c9c2666d44e0327c1f9c239a74c508336053e",
            "e18941661c834f08aa0a19e626484916937df12c0e08d5f015b3b53d0284aa02",
        ),
        (
            "7d5ae6def200acda76d2ccf7c93170a9d88d6cb1",
            "8dd94999e55b5d13a576adc83d56c9bfb3ea0f98d7dbee30f78937664b6a7422",
        ),
        (
            "aac4af54d6427ef10af2b51a524e7272c4f37c02",
            "5b10c51fc44a8adf5650810cef8c5509f24bb66e627bbef9a35941652af94172",
        ),
    ]
    .into()
});

fn hex_to_id(hex: &str) -> gix_hash::ObjectId {
    match gix_testtools::object_hash_from_env().unwrap_or_default() {
        gix_hash::Kind::Sha1 => ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex"),
        gix_hash::Kind::Sha256 => ObjectId::from_hex(
            SHA1_TO_SHA256_HASHES
                .get(hex)
                .unwrap_or_else(|| panic!("SHA-1 {hex} wasn't mapped to SHA-256 yet"))
                .as_bytes(),
        )
        .expect("64 bytes hex"),
        _ => unimplemented!(),
    }
}
