use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use gix_error::{OptionExt, Result, ResultExt, bail, message};
use serde_json::{Value, json};
use toml_edit::{DocumentMut, Item, TableLike, value};

use super::ops::{git, output};

struct Consumer<'a> {
    name: &'a str,
    repository: &'a str,
    package: &'a str,
    features: Vec<&'a str>,
    default_features: bool,
}

fn consumers(metadata: &Value) -> Result<Vec<Consumer<'_>>> {
    let packages = metadata["packages"]
        .as_array()
        .ok_or_raise(|| message("Cargo metadata is missing packages"))?;
    let Some(settings) = packages
        .iter()
        .find(|package| package["name"] == "gix")
        .and_then(|package| package["metadata"]["qa"].get("downstream"))
    else {
        return Ok(Vec::new());
    };
    let mut names = BTreeSet::new();
    settings
        .as_array()
        .ok_or_raise(|| message("qa.downstream must be an array"))?
        .iter()
        .map(|entry| {
            let fields = entry
                .as_object()
                .ok_or_raise(|| message("Downstream declarations must be objects"))?;
            for key in fields.keys() {
                if !["name", "repository", "package", "features", "default-features"].contains(&key.as_str()) {
                    bail!("Unknown downstream field: {key}");
                }
            }
            let text = |key| {
                entry[key]
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .ok_or_raise(|| message!("Downstream {key} must be a nonempty string"))
            };
            let name = text("name")?;
            let package = text("package")?;
            let slug = |value: &str| {
                value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
                    && value != "."
                    && value != ".."
            };
            let repository = text("repository")?;
            if !slug(name)
                || !slug(package)
                || repository.split('/').count() != 2
                || !repository.split('/').all(|part| !part.is_empty() && slug(part))
                || !names.insert(name)
            {
                bail!("Invalid or duplicate downstream name, package, or owner/repository: {entry}");
            }
            let features = match entry.get("features") {
                None => Vec::new(),
                Some(features) => features
                    .as_array()
                    .ok_or_raise(|| message("Downstream features must be an array"))?
                    .iter()
                    .map(|feature| {
                        feature
                            .as_str()
                            .filter(|feature| {
                                !feature.is_empty()
                                    && !feature
                                        .chars()
                                        .any(|character| character.is_whitespace() || matches!(character, ',' | '\0'))
                            })
                            .ok_or_raise(|| message("Downstream features must contain individual feature names"))
                    })
                    .collect::<Result<_>>()?,
            };
            let default_features = match entry.get("default-features") {
                None => true,
                Some(value) => value
                    .as_bool()
                    .ok_or_raise(|| message("Downstream default-features must be a boolean"))?,
            };
            Ok(Consumer {
                name,
                repository,
                package,
                features,
                default_features,
            })
        })
        .collect()
}

pub fn matrix(metadata: &Value, selection: &Value) -> Result<Value> {
    let consumers = consumers(metadata)?;
    let selected = selection["mode"] == "exhaustive"
        || selection["packages"]
            .as_array()
            .is_some_and(|packages| packages.iter().any(|name| name == "gix"));
    Ok(
        json!({"include": consumers.into_iter().filter(|_| selected).map(|consumer| {
        json!({"name": consumer.name, "repository": consumer.repository})
    }).collect::<Vec<_>>()}),
    )
}

fn cargo(checkout: &Path) -> Command {
    let mut command = Command::new("cargo");
    command.current_dir(checkout);
    command
}

fn metadata(command: &mut Command) -> Result<Value> {
    serde_json::from_slice(&output(command)?).or_raise(|| message("Read downstream Cargo metadata"))
}

fn feature_args(command: &mut Command, consumer: &Consumer<'_>) {
    if !consumer.default_features {
        command.arg("--no-default-features");
    }
    if !consumer.features.is_empty() {
        command.args([
            "--features",
            &consumer
                .features
                .iter()
                .map(|feature| format!("{}/{feature}", consumer.package))
                .collect::<Vec<_>>()
                .join(","),
        ]);
    }
}

pub fn execute(root: &Path, name: &str, checkout: &Path) -> Result<()> {
    let local = super::plan::metadata(root)?;
    let consumers = consumers(&local)?;
    let consumer = consumers
        .iter()
        .find(|consumer| consumer.name == name)
        .ok_or_raise(|| message!("Unknown downstream consumer: {name}"))?;
    let revision = output(git(checkout)?.args(["rev-parse", "HEAD"]))?;
    eprintln!(
        "Downstream {} at {}",
        consumer.repository,
        String::from_utf8_lossy(&revision).trim()
    );
    let downstream = metadata(cargo(checkout).args(["metadata", "--no-deps", "--format-version", "1"]))?;
    let members = downstream["workspace_members"]
        .as_array()
        .ok_or_raise(|| message("Downstream has no workspace members"))?;
    if !downstream["packages"]
        .as_array()
        .ok_or_raise(|| message("Downstream has no packages"))?
        .iter()
        .any(|package| package["name"] == consumer.package && members.contains(&package["id"]))
    {
        bail!(
            "Consumer package {} is not a downstream workspace member",
            consumer.package
        );
    }
    let libraries = libraries(&local)?;
    rewrite_workspace(&downstream, &libraries)?;
    let local_gix = libraries
        .get("gix")
        .ok_or_raise(|| message("The local workspace has no gix library"))?
        .join("Cargo.toml");
    eprintln!("Checking {} against {}", consumer.name, local_gix.display());
    let mut command = cargo(checkout);
    command.args([
        "check",
        "--package",
        consumer.package,
        "--message-format=json-render-diagnostics",
    ]);
    feature_args(&mut command, consumer);
    check(&mut command, &local_gix)
}

fn libraries(metadata: &Value) -> Result<BTreeMap<String, PathBuf>> {
    metadata["packages"]
        .as_array()
        .ok_or_raise(|| message("Cargo metadata is missing packages"))?
        .iter()
        .filter(|package| {
            package["targets"].as_array().is_some_and(|targets| {
                targets.iter().any(|target| {
                    target["kind"]
                        .as_array()
                        .is_some_and(|kinds| kinds.iter().any(|kind| kind == "lib" || kind == "proc-macro"))
                })
            })
        })
        .map(|package| {
            let name = package["name"]
                .as_str()
                .ok_or_raise(|| message("A local library has no name"))?;
            let manifest = package["manifest_path"]
                .as_str()
                .ok_or_raise(|| message!("{name} has no manifest path"))?;
            let directory = Path::new(manifest)
                .parent()
                .ok_or_raise(|| message!("{name} has no manifest directory"))?;
            Ok((
                name.into(),
                directory.canonicalize().or_raise(|| message!("Locate local {name}"))?,
            ))
        })
        .collect()
}

fn rewrite_workspace(metadata: &Value, libraries: &BTreeMap<String, PathBuf>) -> Result<()> {
    let root = metadata["workspace_root"]
        .as_str()
        .ok_or_raise(|| message("Downstream has no workspace root"))?;
    let members = metadata["workspace_members"]
        .as_array()
        .ok_or_raise(|| message("Downstream has no workspace members"))?;
    let mut manifests = BTreeSet::from([Path::new(root).join("Cargo.toml")]);
    for package in metadata["packages"]
        .as_array()
        .ok_or_raise(|| message("Downstream has no packages"))?
    {
        if members.contains(&package["id"]) {
            manifests.insert(PathBuf::from(
                package["manifest_path"]
                    .as_str()
                    .ok_or_raise(|| message("Downstream package has no manifest"))?,
            ));
        }
    }
    for manifest in manifests {
        let source = fs::read_to_string(&manifest).or_raise(|| message!("Read {}", manifest.display()))?;
        let mut document = source
            .parse::<DocumentMut>()
            .or_raise(|| message!("Parse {}", manifest.display()))?;
        rewrite_document(&mut document, libraries)?;
        let updated = document.to_string();
        if source != updated {
            fs::write(&manifest, updated)
                .or_raise(|| message!("Update downstream dependencies in {}", manifest.display()))?;
        }
    }
    Ok(())
}

fn rewrite_document(document: &mut DocumentMut, libraries: &BTreeMap<String, PathBuf>) -> Result<()> {
    fn dependencies(table: &mut dyn TableLike, libraries: &BTreeMap<String, PathBuf>) -> Result<()> {
        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            let Some(dependencies) = table.get_mut(section).and_then(Item::as_table_like_mut) else {
                continue;
            };
            for (name, dependency) in dependencies.iter_mut() {
                if dependency.get("workspace").and_then(Item::as_bool) == Some(true) {
                    continue;
                }
                let package = dependency.get("package").and_then(Item::as_str).unwrap_or(name.get());
                let Some(path) = libraries.get(package) else { continue };
                let path = path
                    .to_str()
                    .ok_or_raise(|| message!("Local library path is not UTF-8: {}", path.display()))?;
                if dependency.is_str() {
                    *dependency = value(toml_edit::InlineTable::new());
                }
                let table = dependency
                    .as_table_like_mut()
                    .ok_or_raise(|| message!("Invalid dependency {name}"))?;
                for selector in [
                    "version",
                    "registry",
                    "registry-index",
                    "git",
                    "branch",
                    "tag",
                    "rev",
                    "path",
                ] {
                    table.remove(selector);
                }
                table.insert("path", value(path));
            }
        }
        Ok(())
    }
    dependencies(document.as_table_mut(), libraries)?;
    if let Some(workspace) = document.get_mut("workspace").and_then(Item::as_table_like_mut) {
        dependencies(workspace, libraries)?;
    }
    if let Some(targets) = document.get_mut("target").and_then(Item::as_table_like_mut) {
        for (_, target) in targets.iter_mut() {
            if let Some(target) = target.as_table_like_mut() {
                dependencies(target, libraries)?;
            }
        }
    }
    Ok(())
}

fn check(command: &mut Command, manifest: &Path) -> Result<()> {
    let manifest = manifest
        .canonicalize()
        .or_raise(|| message("Locate local gix manifest"))?;
    let stdout = output(command.stdout(Stdio::piped()).stderr(Stdio::inherit()))?;
    let mut compiled_local_gix = false;
    for event in serde_json::Deserializer::from_slice(&stdout).into_iter::<Value>() {
        let event = event.or_raise(|| message("Parse cargo check output"))?;
        if event["reason"] == "compiler-artifact" {
            let path = event["manifest_path"]
                .as_str()
                .ok_or_raise(|| message("Cargo artifact has no manifest path"))?;
            compiled_local_gix |= Path::new(path)
                .canonicalize()
                .or_raise(|| message("Locate compiled manifest"))?
                == manifest;
        }
    }
    // A declared optional dependency is not proof that the selected build uses it.
    if !compiled_local_gix {
        bail!("Downstream cargo check did not build local gix; check its selected package, features, and target");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_selects_declared_consumers_and_validates_skipped_work() -> gix_error::TestResult {
        let entry = json!({"name": "consumer", "repository": "owner/repository", "package": "application"});
        let metadata = |entries| json!({"packages": [{"name": "gix", "metadata": {"qa": {"downstream": entries}}}]});
        assert_eq!(
            matrix(&metadata(json!([entry])), &json!({"mode":"exhaustive"}))?,
            json!({"include":[{"name":"consumer", "repository":"owner/repository"}]}),
            "the external matrix exposes only checkout and lookup information"
        );
        assert_eq!(
            matrix(&metadata(json!([entry])), &json!({"mode":"fast", "packages":["gix"]}))?,
            matrix(&metadata(json!([entry])), &json!({"mode":"exhaustive"}))?,
            "changes affecting gix include its downstreams"
        );
        assert_eq!(
            matrix(&metadata(json!([entry])), &json!({"mode":"fast", "packages":[]}))?,
            json!({"include": []}),
            "unaffected downstreams need no jobs"
        );
        for invalid in [
            json!([entry, entry]),
            json!([{"name":"consumer", "repository":"https://github.com/owner/repo", "package":"application"}]),
            json!([{"name":"consumer", "repository":"../repo", "package":"application"}]),
            json!([{"name":"consumer", "repository":"owner/repo", "package":"application", "features":[false]}]),
            json!([{"name":"consumer", "repository":"owner/repo", "package":"application", "default-features":"false"}]),
        ] {
            assert!(
                matrix(&metadata(invalid), &json!({"mode":"fast", "packages":[]})).is_err(),
                "invalid declarations cannot hide behind selection"
            );
        }
        Ok(())
    }

    #[test]
    fn rewriting_preserves_dependency_behavior_across_manifest_forms() -> gix_error::TestResult {
        let mut document = r#"
[dependencies]
gix = "0.87"
unrelated = "1"
inherited = { workspace = true, features = ["extra"] }
[workspace.dependencies]
inherited = { package = "gix", git = "https://example.invalid/gix", branch = "main", version = "=0.87.0", default-features = false, features = ["old-feature"] }
[target.'cfg(windows)'.build-dependencies]
hash = { package = "gix-hash", version = "1", registry = "other", optional = true }
[dev-dependencies.gix]
path = "old"
version = "0.87"
default-features = false
"#.parse::<DocumentMut>()?;
        let libraries = BTreeMap::from([
            ("gix".into(), PathBuf::from("/local source/gix")),
            ("gix-hash".into(), PathBuf::from("/local source/gix-hash")),
        ]);
        rewrite_document(&mut document, &libraries)?;
        assert_eq!(
            document["dependencies"]["gix"]["path"].as_str(),
            Some("/local source/gix")
        );
        assert_eq!(document["dependencies"]["unrelated"].as_str(), Some("1"));
        assert_eq!(document["dependencies"]["inherited"]["workspace"].as_bool(), Some(true));
        let inherited = &document["workspace"]["dependencies"]["inherited"];
        assert_eq!(inherited["package"].as_str(), Some("gix"));
        assert_eq!(inherited["default-features"].as_bool(), Some(false));
        assert_eq!(
            inherited["features"]
                .as_array()
                .and_then(|features| features.get(0))
                .and_then(toml_edit::Value::as_str),
            Some("old-feature"),
            "removed upstream features remain visible incompatibilities"
        );
        assert!(
            inherited.get("git").is_none() && inherited.get("version").is_none() && inherited.get("branch").is_none()
        );
        let hash = &document["target"]["cfg(windows)"]["build-dependencies"]["hash"];
        assert_eq!(hash["path"].as_str(), Some("/local source/gix-hash"));
        assert_eq!(hash["optional"].as_bool(), Some(true));
        assert!(
            hash.get("registry").is_none(),
            "a path override must not retain a registry selector"
        );
        assert_eq!(
            document["dev-dependencies"]["gix"]["path"].as_str(),
            Some("/local source/gix")
        );
        assert_eq!(
            document["dev-dependencies"]["gix"]["default-features"].as_bool(),
            Some(false)
        );
        Ok(())
    }

    #[test]
    fn offline_consumer_uses_local_types_and_requires_actual_feature_activation() -> gix_error::TestResult {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let path = std::env::temp_dir().join(format!("qa-downstream-{}-{stamp}", std::process::id()));
        fs::create_dir(&path)?;
        let fixture = super::super::ops::Snapshot { path };
        let write = |path: &str, content: &str| -> std::io::Result<()> {
            let path = fixture.path.join(path);
            fs::create_dir_all(path.parent().expect("fixture files have a parent"))?;
            fs::write(path, content)
        };
        write(
            "local source/Cargo.toml",
            r#"
[workspace]
members = ["gix", "gix-hash"]
resolver = "2"
"#,
        )?;
        write(
            "local source/gix/Cargo.toml",
            r#"
[package]
name = "gix"
version = "99.0.0"
edition = "2021"
[features]
default = ["must-remain-disabled"]
must-remain-disabled = []
requested = []
inherited = []
[dependencies]
gix-hash = { path = "../gix-hash" }
"#,
        )?;
        write(
            "local source/gix/src/lib.rs",
            r#"
#[cfg(feature = "must-remain-disabled")]
compile_error!("the downstream disabled gix defaults");
#[cfg(all(feature = "requested", feature = "inherited"))]
pub fn local() -> gix_hash::Id { gix_hash::Id }
"#,
        )?;
        write(
            "local source/gix-hash/Cargo.toml",
            r#"
[package]
name = "gix-hash"
version = "99.0.0"
edition = "2021"
"#,
        )?;
        write("local source/gix-hash/src/lib.rs", "pub struct Id;\n")?;
        write(
            "consumer/Cargo.toml",
            r#"
[workspace]
members = ["application", "feature-enabler"]
resolver = "2"
[workspace.dependencies]
alternate = { package = "gix", git = "https://example.invalid/must-not-fetch", rev = "1111111111111111111111111111111111111111", version = "=0.1.0", default-features = false, features = ["inherited"] }
gix-hash = "=0.1.0"
"#,
        )?;
        write(
            "consumer/application/Cargo.toml",
            r#"
[package]
name = "application"
version = "0.0.0"
edition = "2021"
[features]
exercise = ["dep:alternate"]
[dependencies]
alternate = { workspace = true, optional = true, features = ["requested"] }
gix-hash.workspace = true
"#,
        )?;
        write(
            "consumer/application/src/lib.rs",
            r#"
#[cfg(feature = "exercise")]
pub fn use_local_types() -> gix_hash::Id { alternate::local() }
"#,
        )?;
        write(
            "consumer/feature-enabler/Cargo.toml",
            r#"
[package]
name = "feature-enabler"
version = "0.0.0"
edition = "2021"
[dependencies]
application = { path = "../application", features = ["exercise"] }
"#,
        )?;
        write("consumer/feature-enabler/src/lib.rs", "pub fn placeholder() {}\n")?;
        let command = |checkout: &Path| {
            let mut command = cargo(checkout);
            command
                .env("CARGO_HOME", fixture.path.join("cargo-home"))
                .env("CARGO_TARGET_DIR", fixture.path.join("target"))
                .env("CARGO_NET_OFFLINE", "true")
                .env_remove("RUSTFLAGS")
                .env_remove("CARGO_ENCODED_RUSTFLAGS");
            command
        };
        let local = fixture.path.join("local source");
        let checkout = fixture.path.join("consumer");
        let local_metadata = metadata(command(&local).args(["metadata", "--no-deps", "--format-version", "1"]))?;
        let before = metadata(command(&checkout).args(["metadata", "--no-deps", "--format-version", "1"]))?;
        rewrite_workspace(&before, &libraries(&local_metadata)?)?;
        let mut consumer = Consumer {
            name: "test",
            repository: "owner/repo",
            package: "application",
            features: vec!["exercise"],
            default_features: false,
        };
        let check_command = |consumer: &Consumer<'_>| {
            let mut command = command(&checkout);
            command.args([
                "check",
                "--package",
                consumer.package,
                "--message-format=json-render-diagnostics",
            ]);
            feature_args(&mut command, consumer);
            command
        };
        let local_gix = local.join("gix/Cargo.toml");
        check(&mut check_command(&consumer), &local_gix)?;
        check(&mut check_command(&consumer), &local_gix)?;
        consumer.features.clear();
        assert!(
            check(&mut check_command(&consumer), &local_gix)
                .expect_err("an inactive optional dependency is not downstream coverage")
                .to_string()
                .contains("did not build local gix")
        );
        write(
            "consumer/application/Cargo.toml",
            r#"
[package]
name = "application"
version = "0.0.0"
edition = "2021"
[features]
exercise = []
[dev-dependencies]
alternate = { workspace = true, features = ["requested"] }
gix-hash.workspace = true
"#,
        )?;
        assert!(
            check(&mut check_command(&consumer), &local_gix)
                .expect_err("dev dependencies are not compiled by cargo check")
                .to_string()
                .contains("did not build local gix")
        );
        let mut failing = command(&local);
        failing.args(["check", "--package", "gix", "--message-format=json-render-diagnostics"]);
        let failure = failing.output()?;
        assert!(!failure.status.success(), "the fixture rejects enabling gix defaults");
        assert!(
            String::from_utf8_lossy(&failure.stderr).contains("the downstream disabled gix defaults"),
            "json-render-diagnostics keeps Rust errors on stderr, which the runner inherits even on failure"
        );
        Ok(())
    }
}
