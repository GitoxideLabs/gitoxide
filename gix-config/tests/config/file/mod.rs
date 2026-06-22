use std::path::PathBuf;

use bstr::BString;
use gix_config::File;
use gix_testtools::fixture_path;

pub fn cow_str(s: &str) -> BString {
    s.into()
}

fn fuzz_artifact_paths(target: &str) -> Vec<PathBuf> {
    let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let old_test_crate_root = crate_root.join("../fuzz/artifacts").join(target);
    let folded_test_root = crate_root.join("fuzz/artifacts").join(target);
    let artifact_root = if old_test_crate_root.is_dir() {
        old_test_crate_root
    } else {
        folded_test_root
    };
    let mut paths = std::fs::read_dir(artifact_root)
        .expect("artifact directory exists")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

#[test]
fn size_in_memory() {
    let actual = std::mem::size_of::<gix_config::File>();
    assert!(
        actual <= 1040,
        "{actual} <= 1040: This shouldn't change without us noticing"
    );
}

mod open {
    use gix_config::File;
    use gix_testtools::fixture_path;

    #[test]
    fn parse_config_with_windows_line_endings_successfully() {
        File::from_path_no_includes(fixture_path("repo-config.crlf"), gix_config::Source::Local).unwrap();
    }
}

#[test]
fn fuzz_file_artifacts_can_be_parsed_without_panicking() {
    for path in fuzz_artifact_paths("fuzz_file") {
        _ = File::from_bytes_no_includes(
            &std::fs::read(path).expect("artifact is readable"),
            gix_config::file::Metadata::default(),
            Default::default(),
        );
    }
}

#[test]
fn fuzzed_stackoverflow() {
    let file = File::from_bytes_no_includes(
        include_bytes!("../../fixtures/fuzzed/stackoverflow-01.config"),
        gix_config::file::Metadata::default(),
        Default::default(),
    )
    .unwrap();
    for section in file.sections() {
        for key in section.value_names() {
            section
                .value_implicit(key.as_ref())
                .expect("The key exists, so should the value.");
        }
    }
}

#[test]
fn fuzzed_long_runtime() -> crate::Result {
    let config = std::fs::read(fixture_path("fuzzed/long-parsetime.config"))?;
    let file = File::from_bytes_no_includes(&config, gix_config::file::Metadata::default(), Default::default())?;
    assert_eq!(file.sections().count(), 52);
    assert!(file.to_bstring().len() < 1200000);
    File::from_bytes_no_includes(
        &file.to_bstring(),
        gix_config::file::Metadata::default(),
        Default::default(),
    )?;

    let mut mutated_file = file.clone();
    mutated_file.append(file);
    assert_eq!(mutated_file.sections().count(), 52 * 2);
    let serialized = mutated_file.to_bstring();
    assert!(serialized.len() < 2400000);
    File::from_bytes_no_includes(&serialized, gix_config::file::Metadata::default(), Default::default())?;
    Ok(())
}

mod access;
mod impls;
mod init;
mod mutable;
mod resolve_includes;
mod write;
