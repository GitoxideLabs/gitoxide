//! Verifies that the published `gix-imara-diff` package has explicit provenance metadata for every
//! packaged file, and that files tracked as upstream or modified still match the
//! `UPSTREAM-PROVENANCE.tsv` manifest.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
struct Provenance {
    upstream_commit: String,
    entries: BTreeMap<String, Entry>,
}

#[derive(Debug)]
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
/// - upstream-derived files record the expected upstream commit and retrieval command
/// - upstream files marked `unchanged` still match the recorded upstream blob
/// - upstream files marked `modified` no longer match upstream and carry an in-file notice that
///   they were changed, along with the upstream retrieval command
/// - generated and local-only files do not claim an upstream blob or retrieval command
#[test]
#[cfg(unix)]
fn packaged_files_have_matching_provenance_and_modified_files_have_notices() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("workspace root is the parent of the crate manifest directory");
    let provenance_path = manifest_dir.join("UPSTREAM-PROVENANCE.tsv");
    let provenance = parse_provenance(&provenance_path);
    let packaged_files = cargo_package_list(workspace_root);

    let packaged_set: BTreeSet<_> = packaged_files.into_iter().collect();
    let manifest_set: BTreeSet<_> = provenance.entries.keys().cloned().collect();
    assert_eq!(
        manifest_set, packaged_set,
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

fn cargo_package_list(workspace_root: &Path) -> Vec<String> {
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

fn is_modified_from_upstream(path: &Path, upstream_blob: &str) -> bool {
    let current = fs::read(path).expect("file content to be readable");
    git_blob(&current) != upstream_blob
}

fn git_blob(input: &[u8]) -> String {
    gix_object::compute_hash(gix_hash::Kind::Sha1, gix_object::Kind::Blob, input)
        .expect("blob hash to be computable")
        .to_string()
}

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
