#![cfg(feature = "sbom")]

use std::{collections::BTreeSet, fs, path::Path, process::Command};

use gix_testtools::{Result, tempfile::TempDir};
use serde_json::Value;

fn isolated(command: &mut Command, root: &Path) {
    let root = root.canonicalize().expect("fixture directory already exists");
    gix_testtools::configure_git_environment(command, &root)
        .current_dir(&root)
        .env("CARGO_HOME", root.join("cargo-home"))
        .env("CARGO_TARGET_DIR", root.join("target"))
        .env("CARGO_NET_OFFLINE", "true");
}

fn jtt(root: &Path, args: &[&str]) -> Result<std::process::Output> {
    Ok(jtt_command(root).args(args).output()?)
}

fn jtt_command(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_jtt"));
    isolated(&mut command, root);
    command.arg("sbom");
    command
}

fn fixture() -> Result<TempDir> {
    let root = gix_testtools::tempfile::Builder::new()
        .prefix("sbom with spaces ")
        .tempdir()?;
    // `other` enables OpenSSL on the same transport used by `app`. A package
    // inventory must not inherit this feature from its workspace neighbour.
    // Windows also enables a transport feature; filtering edges alone does not
    // prevent Cargo metadata from leaking its backend into Linux inventories.
    let app = r#"
[features]
default = ["http"]
http = ["transport/rustls"]
native = ["transport/openssl"]
[dependencies]
transport = { path = "../transport", optional = true, default-features = false }
[build-dependencies]
build-tool = { path = "../build-tool" }
[dev-dependencies]
dev-tool = { path = "../dev-tool" }
[target.'cfg(windows)'.dependencies]
windows-only = { path = "../windows-only" }
transport = { path = "../transport", optional = true, default-features = false, features = ["windows"] }
"#;
    let crates = [
        ("app", app),
        ("library", app),
        (
            "other",
            "[dependencies]\ntransport = { path = \"../transport\", features = [\"openssl\"] }\n",
        ),
        (
            "transport",
            r#"
[features]
rustls = ["dep:rustls-backend"]
openssl = ["dep:openssl-backend"]
windows = ["dep:windows-backend"]
[dependencies]
rustls-backend = { path = "../rustls-backend", optional = true }
openssl-backend = { path = "../openssl-backend", optional = true }
windows-backend = { path = "../windows-backend", optional = true }
"#,
        ),
        ("rustls-backend", ""),
        ("openssl-backend", ""),
        ("build-tool", ""),
        ("dev-tool", ""),
        ("windows-only", ""),
        ("windows-backend", ""),
    ];
    let members: Vec<_> = crates.iter().map(|(name, _)| *name).collect();
    fs::write(
        root.path().join("Cargo.toml"),
        format!(
            "[package]\nname = \"workspace-root\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[workspace]\nresolver = \"2\"\nmembers = {}\n",
            serde_json::to_string(&members)?
        ),
    )?;
    fs::create_dir(root.path().join("src"))?;
    fs::write(root.path().join("src/lib.rs"), "")?;
    for (name, extra) in crates {
        let dir = root.path().join(name);
        fs::create_dir_all(dir.join("src"))?;
        let source = if matches!(name, "app" | "other") {
            "main.rs"
        } else {
            "lib.rs"
        };
        fs::write(dir.join("src").join(source), "fn main() {}\n")?;
        fs::write(
            dir.join("Cargo.toml"),
            format!("[package]\nname = {name:?}\nversion = \"0.1.0\"\nedition = \"2024\"\nlicense = \"MIT\"\n{extra}"),
        )?;
    }
    fs::write(root.path().join("app/build.rs"), "fn main() {}\n")?;
    fs::write(root.path().join("library/build.rs"), "fn main() {}\n")?;
    let mut cargo = Command::new(env!("CARGO"));
    isolated(&mut cargo, root.path());
    let output = cargo
        .args(["metadata", "--all-features", "--format-version", "1"])
        .output()?;
    assert!(
        output.status.success(),
        "fixture metadata resolves offline: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(root)
}

#[test]
fn help_and_invalid_selection() -> Result {
    let root = fixture()?;
    let help = jtt(root.path(), &["--help"])?;
    assert!(
        help.status.success(),
        "jtt exposes the SBOM interface: {}",
        String::from_utf8_lossy(&help.stderr)
    );
    assert!(
        String::from_utf8_lossy(&help.stdout).contains("--package"),
        "help explains package selection"
    );
    for (args, diagnostic) in [
        (vec!["--package", "missing"], "unknown workspace package"),
        (vec!["--package", "app", "--features", "missing"], "unknown feature"),
        (vec!["--features", "http"], "--package"),
        (vec!["--package"], "requires a value"),
    ] {
        let output = jtt(root.path(), &args)?;
        assert!(!output.status.success(), "invalid selection must fail: {args:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(diagnostic),
            "diagnostic explains {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn check_documents(output: &Path, scope: &str) -> Result<BTreeSet<String>> {
    let cdx: Value = serde_json::from_slice(&fs::read(output.join(format!("{scope}.cdx.json")))?)?;
    let spdx: Value = serde_json::from_slice(&fs::read(output.join(format!("{scope}.spdx.json")))?)?;
    assert_eq!(cdx["specVersion"], "1.5", "CycloneDX version is explicit");
    assert_eq!(spdx["spdxVersion"], "SPDX-2.3", "SPDX version is explicit");
    assert!(
        cdx["metadata"]["component"]["components"].is_null(),
        "temporary library targets are removed"
    );
    if scope != "workspace" {
        assert_eq!(
            cdx["metadata"]["component"]["name"], scope,
            "the selected crate is the document root"
        );
        assert_eq!(
            cdx["metadata"]["component"]["type"],
            if scope == "app" { "application" } else { "library" },
            "the root retains its crate kind"
        );
    }
    let components = cdx["components"].as_array().expect("CycloneDX component list");
    let components: Vec<_> = components
        .iter()
        .chain(std::iter::once(&cdx["metadata"]["component"]))
        .collect();
    let identities: BTreeSet<_> = components
        .iter()
        .map(|c| (c["name"].to_string(), c["version"].to_string()))
        .collect();
    let packages: BTreeSet<_> = spdx["packages"]
        .as_array()
        .expect("SPDX package list")
        .iter()
        .map(|p| (p["name"].to_string(), p["versionInfo"].to_string()))
        .collect();
    assert_eq!(
        identities, packages,
        "both formats describe the same packages and versions"
    );
    let names: BTreeSet<_> = components
        .iter()
        .map(|c| c["name"].as_str().expect("component name").to_owned())
        .collect();
    assert!(
        !names.contains("gitoxide-sbom-scope"),
        "the temporary Cargo package is absent"
    );
    let refs: BTreeSet<_> = components
        .iter()
        .map(|c| c["bom-ref"].as_str().expect("component reference"))
        .collect();
    assert!(
        refs.iter().all(|id| !id.contains(".sbom-")),
        "references use original package identities"
    );
    for edge in cdx["dependencies"].as_array().expect("dependency graph") {
        assert!(
            refs.contains(edge["ref"].as_str().expect("dependency reference")),
            "dependency roots refer to real components"
        );
        for target in edge["dependsOn"].as_array().into_iter().flatten() {
            assert!(
                refs.contains(target.as_str().expect("target reference")),
                "dependency targets refer to real components"
            );
        }
    }
    let mut refs: BTreeSet<_> = spdx["packages"]
        .as_array()
        .expect("SPDX packages")
        .iter()
        .map(|package| package["SPDXID"].as_str().expect("SPDX package ID"))
        .collect();
    refs.insert(spdx["SPDXID"].as_str().expect("SPDX document ID"));
    for relationship in spdx["relationships"].as_array().expect("SPDX relationships") {
        for key in ["spdxElementId", "relatedSpdxElement"] {
            assert!(
                refs.contains(relationship[key].as_str().expect("SPDX relationship endpoint")),
                "SPDX relationships have valid endpoints"
            );
        }
    }
    Ok(names)
}

#[test]
#[ignore = "requires cargo-cyclonedx and sbom-tools; run just sbom-test"]
fn package_features_and_workspace() -> Result {
    let root = fixture()?;
    let lock = fs::read(root.path().join("Cargo.lock"))?;
    let output_dir = root.path().join("target/sbom");
    let cases: &[(&[&str], &str, &str)] = &[
        (&[], "rustls-backend", "openssl-backend"),
        (&["--no-default-features"], "build-tool", "transport"),
        (
            &["--no-default-features", "--features", "native"],
            "openssl-backend",
            "rustls-backend",
        ),
        (&["--all-features", "--target", "all"], "windows-only", "other"),
        (
            &["--target", "x86_64-unknown-linux-gnu"],
            "rustls-backend",
            "windows-backend",
        ),
        (
            &["--target", "x86_64-pc-windows-msvc"],
            "windows-backend",
            "openssl-backend",
        ),
    ];
    for package in ["app", "library"] {
        for (flags, included, excluded) in cases {
            let args = [&["--package", package][..], flags].concat();
            let output = jtt(root.path(), &args)?;
            assert!(
                output.status.success(),
                "generation succeeds for {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let names = check_documents(&output_dir, package)?;
            assert!(
                names.contains(*included),
                "selected dependency is inventoried for {args:?}"
            );
            assert!(
                !names.contains(*excluded),
                "unselected features stay excluded for {args:?}"
            );
            assert!(names.contains("build-tool"), "build dependencies are inventoried");
            assert!(!names.contains("dev-tool"), "test-only dependencies are excluded");
        }
    }
    let output = jtt(root.path(), &[])?;
    assert!(
        output.status.success(),
        "workspace generation succeeds: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let names = check_documents(&output_dir, "workspace")?;
    for name in [
        "app",
        "library",
        "other",
        "rustls-backend",
        "openssl-backend",
        "build-tool",
        "dev-tool",
        "windows-only",
        "windows-backend",
        "workspace-root",
    ] {
        assert!(names.contains(name), "workspace member {name} is inventoried");
    }
    // A failing converter must not replace either document from an earlier run.
    let cdx = fs::read(output_dir.join("app.cdx.json"))?;
    let spdx = fs::read(output_dir.join("app.spdx.json"))?;
    let tools = root.path().join("broken-tools");
    fs::create_dir(&tools)?;
    // jtt rejects the converter's `convert` subcommand on every supported OS.
    fs::copy(
        env!("CARGO_BIN_EXE_jtt"),
        tools.join(format!("sbom-tools{}", std::env::consts::EXE_SUFFIX)),
    )?;
    let path = std::env::join_paths(
        std::iter::once(tools).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())),
    )?;
    let failure = jtt_command(root.path())
        .args(["--package", "app"])
        .env("PATH", path)
        .output()?;
    assert!(!failure.status.success(), "conversion failure is reported");
    assert_eq!(
        fs::read(output_dir.join("app.cdx.json"))?,
        cdx,
        "failed conversion preserves CycloneDX"
    );
    assert_eq!(
        fs::read(output_dir.join("app.spdx.json"))?,
        spdx,
        "failed conversion preserves SPDX"
    );
    let custom = jtt(root.path(), &["--package", "library", "--output-dir", "custom output"])?;
    assert!(
        custom.status.success(),
        "output directories can contain spaces: {}",
        String::from_utf8_lossy(&custom.stderr)
    );
    check_documents(&root.path().join("custom output"), "library")?;
    assert_eq!(
        fs::read(root.path().join("Cargo.lock"))?,
        lock,
        "generation preserves the source lockfile"
    );
    Ok(())
}
