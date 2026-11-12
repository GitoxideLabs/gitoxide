//! Verifies that the published `gix-imara-diff` package has explicit provenance metadata for every
//! packaged file, and that files tracked as upstream or modified still match the
//! `UPSTREAM-PROVENANCE.tsv` manifest.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Debug)]
struct Provenance {
    #[cfg_attr(not(unix), allow(dead_code))]
    upstream_commit: String,
    entries: BTreeMap<String, Entry>,
}

#[derive(Debug)]
#[cfg_attr(not(unix), allow(dead_code))]
struct Entry {
    origin: String,
    status: String,
    upstream_blob: String,
    cat_file: String,
}

/// Verifies the release-time provenance contract for the `gix-imara-diff` package.
///
/// Specifically, it checks that:
/// - every file that would be included by `cargo package` has exactly one matching entry in
///   `UPSTREAM-PROVENANCE.tsv`
/// - every file with an entry is packaged, except `.cargo_vcs_info.json`, which Cargo may omit
/// - `.cargo_vcs_info.json` has an entry even if Cargo omits it
/// - upstream-derived files record the expected upstream commit and retrieval command
/// - upstream files marked `unchanged` still match the recorded upstream blob
/// - upstream files marked `modified` no longer match upstream and carry an in-file notice that
///   they were changed, along with the upstream retrieval command
/// - generated and local-only files do not claim an upstream blob or retrieval command
#[test]
#[cfg(unix)]
fn packaged_files_have_matching_provenance_and_modified_files_have_notices() {
    let manifest_dir = manifest_dir();
    let workspace_root = manifest_dir
        .parent()
        .expect("workspace root is the parent of the crate manifest directory");
    let provenance = recorded_provenance();
    let packaged_files = cargo_package_list(workspace_root);

    let packaged_set: BTreeSet<_> = packaged_files.into_iter().collect();
    assert_eq!(
        expected_package_surface(&provenance, &packaged_set),
        packaged_set,
        "package surface and provenance manifest diverged"
    );

    for (path, entry) in &provenance.entries {
        match entry.origin.as_str() {
            "upstream" => {
                assert_ne!(
                    entry.upstream_blob, "-",
                    "{path}: upstream files need an upstream blob id"
                );
                let expected_cat_file = format!("git cat-file -p {}:{}", provenance.upstream_commit, path);
                assert_eq!(
                    entry.cat_file, expected_cat_file,
                    "{path}: upstream retrieval command must point at the recorded upstream commit"
                );

                let file_path = manifest_dir.join(path);
                assert!(
                    file_path.is_file(),
                    "{path}: upstream-backed packaged files must exist on disk"
                );

                let is_modified = is_modified_from_upstream(&file_path, &entry.upstream_blob);
                match entry.status.as_str() {
                    "modified" => {
                        assert!(
                            is_modified,
                            "{path}: manifest says modified but current blob matches upstream"
                        );
                        assert_has_notice(&file_path, &expected_cat_file);
                    }
                    "unchanged" => {
                        assert!(
                            !is_modified,
                            "{path}: manifest says unchanged but current blob no longer matches upstream"
                        );
                    }
                    other => panic!("{path}: unexpected upstream status {other}"),
                }
            }
            "generated" => {
                assert_eq!(
                    entry.status, "generated",
                    "{path}: generated files must use generated status"
                );
                assert_eq!(
                    entry.upstream_blob, "-",
                    "{path}: generated files must not have upstream blobs"
                );
                assert_eq!(
                    entry.cat_file, "-",
                    "{path}: generated files must not have upstream cat-file commands"
                );
            }
            "local-only" => {
                assert_eq!(
                    entry.status, "local-only",
                    "{path}: local-only files must use local-only status"
                );
                assert_eq!(
                    entry.upstream_blob, "-",
                    "{path}: local-only files must not have upstream blobs"
                );
                assert_eq!(
                    entry.cat_file, "-",
                    "{path}: local-only files must not have upstream cat-file commands"
                );
            }
            other => panic!("{path}: unknown origin {other}"),
        }
    }
}

/// Cargo's VCS metadata, which it omits when it finds no Git repository it can open, such as outside
/// Git or in a SHA-256 repository (which Cargo 1.99 can't open), or when `HEAD` is unborn.
const VCS_INFO: &str = ".cargo_vcs_info.json";

/// Returns the files `UPSTREAM-PROVENANCE.tsv` expects in a package that contains `packaged`.
///
/// The entry for [`VCS_INFO`] is required even if Cargo omitted the file, so that the manifest
/// stays complete for packages that include it.
fn expected_package_surface(provenance: &Provenance, packaged: &BTreeSet<String>) -> BTreeSet<String> {
    assert!(
        provenance.entries.contains_key(VCS_INFO),
        "{VCS_INFO}: needs a provenance entry even when Cargo omits it"
    );
    let mut expected: BTreeSet<_> = provenance.entries.keys().cloned().collect();
    if !packaged.contains(VCS_INFO) {
        expected.remove(VCS_INFO);
    }
    expected
}

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn recorded_provenance() -> Provenance {
    parse_provenance(&manifest_dir().join("UPSTREAM-PROVENANCE.tsv"))
}

/// Checks [`expected_package_surface`] directly, without running `cargo package`: only the VCS
/// metadata may be missing, and a package missing any other file still mismatches.
#[test]
fn only_vcs_metadata_may_be_missing_from_package() {
    let provenance = recorded_provenance();
    let mut packaged: BTreeSet<_> = provenance.entries.keys().cloned().collect();
    assert_eq!(
        expected_package_surface(&provenance, &packaged),
        packaged,
        "a package with VCS metadata must match every entry"
    );
    packaged.remove(VCS_INFO);
    assert_eq!(
        expected_package_surface(&provenance, &packaged),
        packaged,
        "a package without VCS metadata must match every other entry"
    );

    for path in ["LICENSE", "Cargo.lock"] {
        let mut missing = packaged.clone();
        let removed = missing.remove(path);
        assert!(removed, "regression case must remove a packaged file");
        assert_ne!(
            expected_package_surface(&provenance, &missing),
            missing,
            "{path}: no other file may be missing from the package"
        );
    }
    packaged.insert("src/unrecorded.rs".into());
    assert_ne!(
        expected_package_surface(&provenance, &packaged),
        packaged,
        "packaged files without an entry must be detected"
    );
}

#[test]
#[should_panic(expected = "needs a provenance entry even when Cargo omits it")]
fn vcs_metadata_needs_an_entry_even_when_not_packaged() {
    let mut provenance = recorded_provenance();
    provenance
        .entries
        .remove(VCS_INFO)
        .expect("manifest records VCS metadata");
    let packaged: BTreeSet<_> = provenance.entries.keys().cloned().collect();
    expected_package_surface(&provenance, &packaged);
}

fn parse_provenance(tsv_path: &Path) -> Provenance {
    let mut upstream_commit = None;
    let mut entries = BTreeMap::new();

    for line in fs::read_to_string(tsv_path)
        .expect("provenance manifest to be readable")
        .lines()
    {
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("# upstream-commit: ") {
            upstream_commit = Some(rest.to_owned());
            continue;
        }
        if line.starts_with('#') {
            continue;
        }

        let mut fields = line.split('\t');
        let path = fields.next().expect("path").to_owned();
        let origin = fields.next().expect("origin").to_owned();
        let status = fields.next().expect("status").to_owned();
        let upstream_blob = fields.next().expect("blob").to_owned();
        let cat_file = fields.next().expect("cat-file").to_owned();
        assert!(
            fields.next().is_none(),
            "{path}: provenance entries must have exactly five tab-separated columns"
        );
        let previous = entries.insert(
            path.clone(),
            Entry {
                origin,
                status,
                upstream_blob,
                cat_file,
            },
        );
        assert!(previous.is_none(), "{path}: duplicate provenance entry");
    }

    Provenance {
        upstream_commit: upstream_commit.expect("provenance manifest to record upstream commit"),
        entries,
    }
}

#[cfg(unix)]
fn cargo_package_list(workspace_root: &Path) -> Vec<String> {
    use std::{env, process::Command};

    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .arg("package")
        .arg("-p")
        .arg("gix-imara-diff")
        .arg("--allow-dirty")
        .arg("--list")
        .current_dir(workspace_root)
        .output()
        .expect("cargo package --list to run");
    assert!(
        output.status.success(),
        "cargo package --list failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout)
        .expect("cargo package --list output to be utf8")
        .lines()
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(unix)]
fn is_modified_from_upstream(path: &Path, upstream_blob: &str) -> bool {
    let current = fs::read(path).expect("file content to be readable");
    git_blob(&current) != upstream_blob
}

#[cfg(unix)]
fn git_blob(input: &[u8]) -> String {
    gix_object::compute_hash(gix_hash::Kind::Sha1, gix_object::Kind::Blob, input)
        .expect("blob hash to be computable")
        .to_string()
}

#[cfg(unix)]
fn assert_has_notice(path: &Path, cat_file: &str) {
    let content = fs::read_to_string(path).expect("modified text file to be readable as utf8");
    assert!(
        content.contains("Modified for gitoxide from the upstream imara-diff crate."),
        "\"{}\": modified upstream-derived files must carry a prominent modification notice",
        path.display()
    );
    let expected = format!("Upstream source: {cat_file}");
    assert!(
        content.contains(&expected),
        "\"{}\": modified upstream-derived files must record the upstream retrieval command",
        path.display()
    );
}
