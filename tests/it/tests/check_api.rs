use std::{
    fs,
    io::{self, Write},
    path::Path,
    process::{Command, Output, Stdio},
};

use gix_testtools::TestResult;
use serde_json::{Value, json};

fn check_api(directory: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_it"));
    gix_testtools::configure_git_environment(&mut command, directory);
    command
        .current_dir(directory)
        .arg("check-api")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

fn assert_exit(output: &Output, expected: i32, reason: &str) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{reason}; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn public_function_document() -> Value {
    json!({
        "format_version": 61,
        "includes_private": false,
        "root": 0,
        "external_crates": {"1": {"name": "public_dep"}},
        "paths": {"10": {"crate_id": 1, "path": ["public_dep", "Item"]}},
        "index": {
            "0": {"name": "api", "crate_id": 0, "visibility": "public",
                "inner": {"module": {"items": [1]}}},
            "1": {"name": "read", "crate_id": 0, "visibility": "public",
                "inner": {"function": {
                    "sig": {"inputs": [], "output": {"resolved_path": {"id": 10, "path": "Item"}}},
                    "generics": {"params": [], "where_predicates": []}
                }}}
        }
    })
}

#[test]
fn audit_reports_public_dependencies_with_or_without_an_allowlist() -> TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    fs::write(
        directory.path().join("api.json"),
        serde_json::to_vec(&public_function_document())?,
    )?;

    for allowlist in [None, Some("unused_dep,public_dep")] {
        let mut command = check_api(directory.path());
        command.args(["audit", "api.json"]);
        if let Some(allowlist) = allowlist {
            command.arg(allowlist);
        }
        let output = command.output()?;
        assert_exit(
            &output,
            0,
            "audits pass when no allowlist is given or all public dependencies are allowed",
        );
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stdout)?,
            json!({"public_dep": ["public_dep::Item"]}),
            "audit stdout identifies the external type exposed by the public function"
        );
    }
    Ok(())
}

#[test]
fn audit_allowlist_violation_is_a_completed_check() -> TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    fs::write(
        directory.path().join("api.json"),
        serde_json::to_vec(&public_function_document())?,
    )?;

    let output = check_api(directory.path())
        .args(["audit", "api.json", "other_dep"])
        .output()?;
    assert_exit(
        &output,
        10,
        "an exposed dependency outside the allowlist is a violation, not a tool failure",
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout)?,
        json!({"public_dep": ["public_dep::Item"]}),
        "a failed allowlist still reports the public dependency that caused the violation"
    );
    Ok(())
}

#[test]
fn audit_rejects_malformed_json() -> TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    fs::write(directory.path().join("api.json"), b"not JSON")?;

    let output = check_api(directory.path()).args(["audit", "api.json"]).output()?;
    assert_exit(&output, 2, "malformed rustdoc JSON cannot produce an audit verdict");
    assert!(
        !output.stderr.is_empty(),
        "malformed JSON has an actionable error on stderr"
    );
    assert!(
        output.stdout.is_empty(),
        "malformed JSON must not emit a successful dependency report"
    );
    Ok(())
}

#[test]
fn audit_fails_closed_for_unsupported_rustdoc_documents() -> TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    for (case, diagnostic) in [("format", "61"), ("private", "private"), ("item kind", "future_kind")] {
        let mut document = public_function_document();
        match case {
            "format" => document["format_version"] = json!(62),
            "private" => document["includes_private"] = json!(true),
            _ => document["index"]["1"]["inner"] = json!({"future_kind": {}}),
        }
        fs::write(directory.path().join("api.json"), serde_json::to_vec(&document)?)?;

        let output = check_api(directory.path()).args(["audit", "api.json"]).output()?;
        assert_exit(
            &output,
            2,
            &format!("unsupported rustdoc {case} must fail rather than silently omit dependencies"),
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(diagnostic),
            "the diagnostic identifies the unsupported rustdoc {case}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            output.stdout.is_empty(),
            "unsupported rustdoc {case} must not emit a dependency report"
        );
    }
    Ok(())
}

fn with_stdin(command: &mut Command, input: &[u8]) -> io::Result<Output> {
    let mut child = command.stdin(Stdio::piped()).spawn()?;
    child
        .stdin
        .take()
        .expect("the release request input was piped")
        .write_all(input)?;
    child.wait_with_output()
}

#[test]
fn release_mode_rejects_invalid_stdin_with_or_without_a_baseline_root() -> TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    fs::create_dir(directory.path().join("baseline"))?;

    for args in [&[][..], &["--baseline-root", "baseline"][..]] {
        for input in [b"not JSON".as_slice(), b"{}".as_slice()] {
            let output = with_stdin(check_api(directory.path()).args(args), input)?;
            assert_exit(
                &output,
                2,
                "malformed JSON or missing release request fields cannot complete a release check",
            );
            assert!(
                !output.stderr.is_empty(),
                "invalid release requests explain the failure on stderr"
            );
            assert!(
                output.stdout.is_empty(),
                "invalid release requests must not emit a compatibility verdict"
            );
        }
    }
    Ok(())
}

#[test]
fn release_progress_is_visible_on_stderr() -> TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    for name in ["current", "baseline"] {
        let root = directory.path().join(name);
        fs::create_dir_all(root.join("src"))?;
        fs::write(
            root.join("Cargo.toml"),
            r#"
[package]
name = "fixture"
version = "1.0.0"
edition = "2024"
[workspace]
[package.metadata.api-check]
profiles = [{ default-features = false, features = [] }]
"#,
        )?;
        fs::write(
            root.join("src/lib.rs"),
            "compile_error!(\"stop after emitting check progress\");\n",
        )?;
    }
    let request = json!({
        "package": "fixture", "manifest_path": "Cargo.toml",
        "baseline_revision": "unused", "unstable_features": [], "dependencies": []
    });
    let output = with_stdin(
        check_api(&directory.path().join("current")).args(["--baseline-root", "../baseline"]),
        &serde_json::to_vec(&request)?,
    )?;
    assert_exit(&output, 2, "the fixture deliberately prevents rustdoc from completing");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Checking fixture, features:"),
        "the shared checker's tracing progress reaches stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "progress must not pollute machine-readable stdout"
    );
    Ok(())
}

#[test]
fn audit_rejects_release_only_arguments() -> TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    let output = check_api(directory.path())
        .args(["--baseline-root", "baseline", "audit", "api.json"])
        .output()?;
    assert_exit(&output, 2, "the baseline override belongs only to release checks");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("cannot be used"),
        "the conflict is rejected before either mode accesses the filesystem"
    );
    Ok(())
}

#[test]
fn help_describes_release_and_audit_modes_without_reading_stdin() -> TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    for args in [&["--help"][..], &["audit", "--help"][..]] {
        let output = check_api(directory.path()).args(args).output()?;
        assert_exit(
            &output,
            0,
            "release and audit help succeed without a request or rustdoc document",
        );
        let help = String::from_utf8(output.stdout)?;
        assert!(
            help.contains("Usage:"),
            "help provides command usage rather than an audit report"
        );
        assert!(help.contains("audit"), "help identifies the audit subcommand");
        if args == ["--help"] {
            assert!(
                help.contains("--baseline-root"),
                "release help documents the baseline checkout override"
            );
        }
    }
    Ok(())
}

#[test]
fn boundary_uses_manifest_exceptions_and_resolved_dependency_features() -> TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    let root = directory.path();
    for crate_name in ["consumer", "plumbing"] {
        fs::create_dir_all(root.join(crate_name).join("src"))?;
    }
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"consumer\", \"plumbing\"]\nresolver = \"2\"\n",
    )?;
    fs::write(
        root.join("plumbing/Cargo.toml"),
        r#"
[package]
name = "plumbing"
version = "0.1.0"
edition = "2024"
[features]
unstable = []
leak = []
[dev-dependencies]
plumbing = { path = ".", features = ["leak"] }
"#,
    )?;
    fs::write(
        root.join("plumbing/src/lib.rs"),
        r#"
pub struct Basic;
pub struct Experimental;
impl Basic {
    #[cfg(feature = "unstable")]
    #[doc = "\n\n**Unstable.**"]
    pub fn experimental(&self) -> Experimental { Experimental }
    #[cfg(feature = "leak")]
    pub fn supported(&self) -> Experimental { Experimental }
}

"#,
    )?;
    fs::write(
        root.join("consumer/src/lib.rs"),
        "pub fn read() -> plumbing::Basic { plumbing::Basic }\n",
    )?;
    let manifest = |recursive: bool, features: &str| {
        format!(
            r#"
[package]
name = "consumer"
version = "1.0.0"
edition = "2024"
[dependencies]
plumbing = {{ path = "../plumbing", features = ["unstable"{features}] }}
[package.metadata.api-check]
profiles = [{{ default-features = false, features = [] }}]
allowed-unstable-types = ["plumbing::Basic"]
recursive = {recursive}
"#
        )
    };
    for (recursive, features, expected, reason) in [
        (
            true,
            "",
            0,
            "unified experimental methods remain outside the supported API",
        ),
        (
            true,
            ", \"leak\"",
            10,
            "resolved dependency features expose an unlisted type through a supported method",
        ),
        (
            false,
            ", \"leak\"",
            0,
            "direct checks do not recursively promise dependency members",
        ),
    ] {
        fs::write(root.join("consumer/Cargo.toml"), manifest(recursive, features))?;
        let output = check_api(root)
            .env("CARGO_TARGET_DIR", root.join("target"))
            .args(["boundary", "consumer/Cargo.toml"])
            .output()?;
        assert_exit(&output, expected, reason);
    }
    fs::write(
        root.join("consumer/Cargo.toml"),
        format!(
            r#"{}
[package.metadata.smart-release]
unstable-features = "unstable"
"#,
            manifest(false, "")
        ),
    )?;
    let output = check_api(root).args(["boundary", "consumer/Cargo.toml"]).output()?;
    assert_exit(
        &output,
        2,
        "malformed exclusions cannot silently weaken a boundary audit",
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("expected an array of strings"),
        "the shared checker's metadata diagnostic reaches the CLI"
    );

    fs::write(root.join("consumer/Cargo.toml"), manifest(false, ""))?;
    let lockfile = root.join("Cargo.lock");
    let original_lock = fs::read(&lockfile)?;
    let plumbing_manifest = root.join("plumbing/Cargo.toml");
    fs::write(
        &plumbing_manifest,
        fs::read_to_string(&plumbing_manifest)?.replace("0.1.0", "0.2.0"),
    )?;
    let output = check_api(root).args(["boundary", "consumer/Cargo.toml"]).output()?;
    assert_exit(
        &output,
        2,
        "checks cannot rewrite an existing lockfile to resolve changed versions",
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("--locked"),
        "the CLI explains that lockfile resolution needs a separate update"
    );
    assert_eq!(
        fs::read(lockfile)?,
        original_lock,
        "failed checks preserve the existing lockfile"
    );
    Ok(())
}

#[test]
#[ignore = "requires cargo-semver-checks 0.51+; just check-api runs this test"]
fn release_compares_the_recursively_supported_dependency_api() -> TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    let baseline = directory.path().join("baseline");
    let current = directory.path().join("current");
    let consumer_manifest = r#"
[package]
name = "consumer"
version = "1.0.0"
edition = "2024"
[dependencies]
plumbing = { path = "../plumbing", features = ["unstable"] }
[features]
extra = []
[package.metadata.api-check]
profiles = [{ default-features = false, features = [] }]
allowed-unstable-types = ["plumbing::Basic", "plumbing::Choice", "transitive::Payload"]
recursive = true
"#;
    let plumbing_manifest = r#"
[package]
name = "plumbing"
version = "0.1.0"
edition = "2024"
[dependencies]
transitive = { path = "../transitive" }
[features]
unstable = []
"#;
    let plumbing = r#"
pub struct Basic { pub value: transitive::Payload }
impl Basic {
    pub fn supported(&self) -> bool { true }
    pub fn pair(&self) -> (bool, bool) { (true, false) }
    #[cfg(feature = "unstable")]
    #[doc = "\n\n**Unstable.**"]
    pub fn experimental(&self) -> bool { true }
}
pub enum Choice { One }
pub struct Unrelated;
pub mod unrelated { pub struct Removed; }
pub enum Unselected { One }
"#;
    let transitive_manifest = r#"
[package]
name = "transitive"
version = "0.1.0"
edition = "2024"
"#;
    for root in [&baseline, &current] {
        for crate_name in ["consumer", "plumbing", "transitive"] {
            fs::create_dir_all(root.join(crate_name).join("src"))?;
        }
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"consumer\", \"plumbing\", \"transitive\"]\nresolver = \"2\"\n",
        )?;
        fs::write(root.join("consumer/Cargo.toml"), consumer_manifest)?;
        fs::write(
            root.join("consumer/src/lib.rs"),
            "pub fn read(value: plumbing::Basic) -> plumbing::Basic { value }\npub fn choice(value: plumbing::Choice) -> plumbing::Choice { value }\n",
        )?;
        fs::write(root.join("plumbing/Cargo.toml"), plumbing_manifest)?;
        fs::write(root.join("plumbing/src/lib.rs"), plumbing)?;
        fs::write(root.join("transitive/Cargo.toml"), transitive_manifest)?;
        fs::write(
            root.join("transitive/src/lib.rs"),
            "pub struct Payload { pub value: bool }\n",
        )?;
    }
    let request = json!({
        "package": "consumer", "manifest_path": "consumer/Cargo.toml",
        "baseline_revision": "unused", "unstable_features": ["unstable"], "dependencies": []
    });
    for (revision, path, old, new, expected, reason) in [
        (
            &current,
            "plumbing/src/lib.rs",
            "Unrelated",
            "Unrelated",
            0,
            "identical supported graphs pass despite relocated workspace paths",
        ),
        (
            &current,
            "plumbing/src/lib.rs",
            "supported(&self) -> bool { true }",
            "supported(&self) -> u8 { 1 }",
            10,
            "supported dependency method signatures are checked without a version bump",
        ),
        (
            &current,
            "plumbing/src/lib.rs",
            "pub fn supported(&self) -> bool { true }",
            "",
            10,
            "supported dependency method removal is breaking",
        ),
        (
            &current,
            "transitive/src/lib.rs",
            "value: bool",
            "value: u8",
            10,
            "transitively reachable fields are checked",
        ),
        (
            &current,
            "transitive/Cargo.toml",
            "0.1.0",
            "0.2.0",
            10,
            "transitively reached dependency identity changes are breaking",
        ),
        (
            &current,
            "plumbing/src/lib.rs",
            "experimental(&self) -> bool { true }",
            "experimental(&self) -> u8 { 1 }",
            0,
            "experimental members are outside the comparison",
        ),
        (
            &current,
            "plumbing/src/lib.rs",
            "pub struct Unrelated;",
            "pub enum Unrelated { Different }",
            0,
            "unrelated plumbing APIs are outside the comparison",
        ),
        (
            &current,
            "plumbing/src/lib.rs",
            "pub mod unrelated { pub struct Removed; }",
            "",
            0,
            "unrelated modules are outside the supported dependency subset",
        ),
        (
            &current,
            "consumer/Cargo.toml",
            "features = [] }",
            "features = [] }, { default-features = false, features = [\"extra\"] }",
            0,
            "new supported profiles are audited without inventing a baseline promise",
        ),
        (
            &current,
            "plumbing/src/lib.rs",
            "pub struct Unrelated;",
            "impl Basic { pub fn added(&self) {} }",
            0,
            "compatible supported method additions pass",
        ),
        (
            &current,
            "plumbing/src/lib.rs",
            "pub fn supported",
            "#[doc = \"**Unstable.**\"] pub fn supported",
            10,
            "marking a previously supported member unstable cannot erase its promise",
        ),
        (
            &current,
            "consumer/Cargo.toml",
            "recursive = true",
            "recursive = false",
            10,
            "disabling recursion cannot erase baseline dependency promises",
        ),
        (
            &current,
            "consumer/Cargo.toml",
            "transitive::Payload",
            "plumbing::Unrelated",
            10,
            "removing a type exception fails the boundary and compatibility checks",
        ),
        (
            &current,
            "plumbing/src/lib.rs",
            "Choice { One }",
            "Choice { One, Two }",
            10,
            "semver checks protect exhaustive enums even when all old signatures remain",
        ),
        (
            &current,
            "plumbing/src/lib.rs",
            "pub struct Basic",
            "#[non_exhaustive] pub struct Basic",
            10,
            "semver checks protect construction of reachable dependency types",
        ),
        (
            &current,
            "transitive/src/lib.rs",
            "pub struct Payload { pub value: bool }",
            "#[allow(dead_code)] pub struct Payload { pub value: bool, private: std::rc::Rc<()> }",
            10,
            "losing auto traits in transitive private storage is breaking",
        ),
        (
            &baseline,
            "consumer/Cargo.toml",
            "features = [] }",
            "features = [] }, { default-features = false, features = [\"extra\"] }",
            10,
            "removing a baseline feature profile cannot shrink the compatibility contract",
        ),
        (
            &current,
            "plumbing/src/lib.rs",
            "pub struct Basic",
            "not Rust; pub struct Basic",
            2,
            "failed dependency rustdoc generation cannot produce a compatibility verdict",
        ),
    ] {
        for root in [&baseline, &current] {
            // Each scenario deliberately changes package versions; resolve a fresh fixture lock.
            let lockfile = root.join("Cargo.lock");
            if lockfile.exists() {
                fs::remove_file(lockfile)?;
            }
            fs::write(root.join("plumbing/src/lib.rs"), plumbing)?;
            fs::write(
                root.join("transitive/src/lib.rs"),
                "pub struct Payload { pub value: bool }\n",
            )?;
            fs::write(root.join("consumer/Cargo.toml"), consumer_manifest)?;
            fs::write(root.join("transitive/Cargo.toml"), transitive_manifest)?;
        }
        let changed = revision.join(path);
        fs::write(&changed, fs::read_to_string(&changed)?.replace(old, new))?;
        let output = with_stdin(
            check_api(&current)
                .env("CARGO_TARGET_DIR", directory.path().join("target"))
                .args(["--baseline-root", baseline.to_str().expect("temporary path is UTF-8")]),
            &serde_json::to_vec(&request)?,
        )?;
        assert_exit(&output, expected, reason);
    }
    Ok(())
}

#[cfg(unix)]
#[test]
#[ignore = "requires cargo-semver-checks 0.51+; just check-api runs this test"]
fn local_comparison_selects_supported_release_tags_and_resolves_explicit_revisions() -> TestResult {
    use std::os::unix::fs::PermissionsExt;

    let root = gix_testtools::scripted_fixture_read_only("make_api_releases.sh")?;
    let directory = gix_testtools::tempfile::tempdir()?;
    let script = directory.path().join("compare.sh");
    fs::write(&script, include_str!("../../../etc/scripts/check-api-compare.sh"))?;
    let checker = directory.path().join("check-api.sh");
    fs::write(
        &checker,
        r#"#!/usr/bin/env bash
exec "$IT_BINARY" check-api "$@"
"#,
    )?;
    fs::set_permissions(&checker, fs::Permissions::from_mode(0o755))?;
    let compare = |root: &Path, revision: Option<&str>| -> io::Result<Output> {
        let mut command = Command::new("bash");
        gix_testtools::configure_git_environment(&mut command, root);
        command
            .current_dir(root)
            .env("IT_BINARY", env!("CARGO_BIN_EXE_it"))
            .env("CARGO_TARGET_DIR", directory.path().join("target"))
            .arg(&script);
        if let Some(revision) = revision {
            command.arg(revision);
        }
        command.output()
    };
    let output = compare(&root, None)?;
    assert_exit(&output, 0, "uncommitted body changes preserve the supported API");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Using supported release tag gix-v0.2.0"),
        "TOML comments cannot hide the supported tag; newer ungated and unreachable tags are skipped: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = compare(&root, Some("HEAD"))?;
    assert_exit(&output, 0, "explicit ungated revisions can be compared");
    let output = compare(&root, Some("gix-v99.0.0"))?;
    assert_exit(
        &output,
        10,
        "explicit revisions need not be ancestors and their incompatible signatures fail comparison",
    );
    let output = compare(&root, Some("missing-revision"))?;
    assert_exit(&output, 2, "missing revisions cannot silently skip comparison");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("Could not resolve baseline"),
        "the shared checker's revision diagnostic reaches the local command"
    );

    let before = gix_testtools::scripted_fixture_read_only_with_args("make_api_releases.sh", ["before"])?;
    let output = compare(&before, None)?;
    assert_exit(
        &output,
        0,
        "before the first supported release, the current boundary is audited",
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("No supported release baseline for gix"),
        "the shared checker reports the missing baseline"
    );
    Ok(())
}
