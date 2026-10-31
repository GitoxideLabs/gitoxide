use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
    process::Command,
};

use gix_error::{OptionExt, Result, ResultExt, bail, message};
use serde_json::{Value, json};

use super::ops::{git, output};

struct Package {
    name: String,
    directory: PathBuf,
    dependencies: BTreeSet<PathBuf>,
    extra_inputs: Vec<PathBuf>,
}

pub fn metadata(root: &Path) -> Result<Value> {
    let output = output(Command::new("cargo").current_dir(root).args([
        "metadata",
        "--no-deps",
        "--format-version",
        "1",
        "--locked",
    ]))?;
    serde_json::from_slice(&output).or_raise(|| message("Could not parse Cargo workspace metadata"))
}

pub fn plan(root: &Path, metadata: &Value, mode: &str, base: &str) -> Result<Value> {
    let packages = packages(metadata)?;
    if mode == "exhaustive" {
        return Ok(exhaustive(&packages, "Exhaustive mode was requested"));
    }
    if mode != "fast" {
        bail!(message!("Unknown CI mode: {mode}"));
    }

    let merge_base = git(root)
        .args(["merge-base", "--", base, "HEAD"])
        .output()
        .or_raise(|| message("Could not find the Git merge base"))?;
    if !merge_base.status.success() {
        return Ok(exhaustive(
            &packages,
            &format!("Cannot compare with {base}; fetch the base ref to enable fast selection"),
        ));
    }
    let merge_base = std::str::from_utf8(&merge_base.stdout)
        .or_raise(|| message("Git returned an invalid merge base"))?
        .trim();
    let mut paths = BTreeSet::new();
    // Separate index and worktree diffs retain staged edits even if the worktree restores HEAD.
    for revisions in [vec![merge_base, "HEAD"], vec!["--cached", "HEAD"], vec![]] {
        let bytes = output(
            git(root)
                .args([
                    "diff",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--name-only",
                    "-z",
                    "--no-renames",
                ])
                .args(revisions)
                .arg("--"),
        )?;
        if !add_paths(&bytes, &mut paths) {
            return Ok(exhaustive(&packages, "A changed path is not valid UTF-8"));
        }
    }
    let untracked = output(git(root).args(["ls-files", "--others", "--exclude-standard", "-z"]))?;
    if !add_paths(&untracked, &mut paths) {
        return Ok(exhaustive(&packages, "An untracked path is not valid UTF-8"));
    }
    let mut plan = select(&packages, &paths);
    plan["base"] = merge_base.into();
    Ok(plan)
}

fn add_paths(bytes: &[u8], paths: &mut BTreeSet<PathBuf>) -> bool {
    for path in bytes.split(|byte| *byte == 0).filter(|path| !path.is_empty()) {
        let Ok(path) = std::str::from_utf8(path) else {
            return false;
        };
        paths.insert(path.into());
    }
    true
}

fn packages(metadata: &Value) -> Result<Vec<Package>> {
    let root = Path::new(
        metadata["workspace_root"]
            .as_str()
            .ok_or_raise(|| message("Cargo metadata is missing workspace_root"))?,
    );
    metadata["packages"]
        .as_array()
        .ok_or_raise(|| message("Cargo metadata is missing packages"))?
        .iter()
        .map(|package| {
            let name = package["name"]
                .as_str()
                .ok_or_raise(|| message("A Cargo package is missing its name"))?
                .to_owned();
            let manifest = Path::new(
                package["manifest_path"]
                    .as_str()
                    .ok_or_raise(|| message!("{name} is missing manifest_path"))?,
            );
            let directory = manifest
                .parent()
                .ok_or_raise(|| message!("{name} has no manifest directory"))?
                .strip_prefix(root)
                .or_raise(|| message!("{name} is outside the workspace root"))?
                .to_owned();
            let dependencies = package["dependencies"]
                .as_array()
                .ok_or_raise(|| message!("{name} is missing dependencies"))?
                .iter()
                .filter_map(|dependency| dependency["path"].as_str())
                .filter_map(|path| Path::new(path).strip_prefix(root).ok())
                .map(Path::to_owned)
                .collect();
            let mut extra_inputs = Vec::new();
            if let Some(inputs) = package["metadata"]["ci"].get("extra-inputs") {
                for input in inputs
                    .as_array()
                    .ok_or_raise(|| message!("{name}: ci.extra-inputs must be an array"))?
                {
                    let input = Path::new(
                        input
                            .as_str()
                            .ok_or_raise(|| message!("{name}: ci.extra-inputs entries must be paths"))?,
                    );
                    if input.as_os_str().is_empty()
                        || input.components().any(|part| !matches!(part, Component::Normal(_)))
                    {
                        bail!(message!(
                            "{name}: ci.extra-inputs must contain paths relative to the repository root"
                        ));
                    }
                    extra_inputs.push(input.to_owned());
                }
            }
            Ok(Package {
                name,
                directory,
                dependencies,
                extra_inputs,
            })
        })
        .collect()
}

fn exhaustive(packages: &[Package], reason: &str) -> Value {
    json!({
        "mode": "exhaustive",
        "packages": packages.iter().map(|package| package.name.as_str()).collect::<BTreeSet<_>>(),
        "reasons": [reason],
    })
}

fn select(packages: &[Package], paths: &BTreeSet<PathBuf>) -> Value {
    let mut selected = BTreeSet::new();
    let mut reasons = Vec::new();
    for path in paths {
        if global_input(path) {
            return exhaustive(
                packages,
                &format!("Global build or CI input changed: {}", path.display()),
            );
        }
        let mut known = false;
        for package in packages {
            if package.extra_inputs.iter().any(|input| path.starts_with(input)) {
                selected.insert(package.directory.clone());
                known = true;
            }
        }
        let owner = packages
            .iter()
            .filter(|package| {
                if package.directory.as_os_str().is_empty() {
                    path.starts_with("src") || path == Path::new("build.rs")
                } else {
                    path.starts_with(&package.directory)
                }
            })
            .max_by_key(|package| package.directory.components().count());
        if let Some(package) = owner {
            selected.insert(package.directory.clone());
            known = true;
        }
        if known {
            reasons.push(format!("Changed input: {}", path.display()));
        } else if !prose_only(path) {
            return exhaustive(packages, &format!("Unclassified input changed: {}", path.display()));
        }
    }
    loop {
        let previous_count = selected.len();
        for package in packages {
            if !selected.is_disjoint(&package.dependencies) {
                selected.insert(package.directory.clone());
            }
        }
        if selected.len() == previous_count {
            break;
        }
    }
    if selected.is_empty() {
        reasons.push("No build or test inputs changed".into());
    } else {
        reasons.push("Includes reverse normal, build, dev, optional, and target-specific dependencies".into());
    }
    json!({
        "mode": "fast",
        "packages": packages.iter().filter(|package| selected.contains(&package.directory))
            .map(|package| package.name.as_str()).collect::<BTreeSet<_>>(),
        "reasons": reasons,
    })
}

fn global_input(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("Cargo.toml" | "Cargo.lock")
    ) || path.components().any(|part| part.as_os_str() == "fuzz")
        || [
            ".github",
            ".config",
            ".cargo",
            "etc/scripts",
            "tests/it/src/ci",
            "tests/it/ci",
        ]
        .iter()
        .any(|prefix| path.starts_with(prefix))
        || matches!(path.to_str(), Some("justfile" | "tests/it/src/ci.rs"))
}

fn prose_only(path: &Path) -> bool {
    matches!(
        path.to_str(),
        Some(
            "AGENTS.md"
                | "CHANGELOG.md"
                | "CODE_OF_CONDUCT.md"
                | "COLLABORATING.md"
                | "CONTRIBUTING.md"
                | "DEVELOPMENT.md"
                | "README.md"
                | "SECURITY.md"
                | "SHORTCOMINGS.md"
                | "STABILITY.md"
                | "crate-status.md"
                | "general-tasks.md"
                | "tasks.md"
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use gix_error::TestResult;

    fn workspace() -> Result<Vec<Package>> {
        let root = Path::new("workspace");
        let mut entries = Vec::new();
        for (name, kind, optional, target) in [
            ("leaf", Value::Null, false, Value::Null),
            ("normal", Value::Null, false, Value::Null),
            ("build", json!("build"), false, Value::Null),
            ("dev", json!("dev"), false, Value::Null),
            ("optional", Value::Null, true, Value::Null),
            ("platform", Value::Null, false, json!("cfg(windows)")),
            ("unrelated", Value::Null, false, Value::Null),
        ] {
            entries.push(json!({
                "name": name,
                "manifest_path": root.join(name).join("Cargo.toml"),
                "dependencies": if matches!(name, "leaf" | "unrelated") { vec![] } else {
                    vec![json!({"path": root.join("leaf"), "kind": kind, "optional": optional, "target": target})]
                },
            }));
        }
        packages(&json!({"workspace_root": root, "packages": entries}))
    }

    #[test]
    fn all_dependency_kinds_are_reverse_dependencies() -> TestResult {
        let packages = workspace()?;
        let plan = select(&packages, &BTreeSet::from(["leaf/src/lib.rs".into()]));
        assert_eq!(plan["mode"], "fast", "A crate edit has a bounded dependency selection");
        assert_eq!(
            plan["packages"],
            json!(["build", "dev", "leaf", "normal", "optional", "platform"]),
            "Inactive features, dev dependencies, and other platforms remain relevant"
        );
        Ok(())
    }

    #[test]
    fn deleted_paths_and_both_rename_ends_are_retained() -> TestResult {
        let mut paths = BTreeSet::new();
        assert!(
            add_paths(
                b"leaf/tests/deleted.rs\0unrelated/src/old name.rs\0normal/src/new\nname.rs\0",
                &mut paths
            ),
            "NUL-delimited names preserve spaces and newlines"
        );
        let plan = select(&workspace()?, &paths);
        assert_eq!(
            plan["packages"].as_array().map(Vec::len),
            Some(7),
            "Selection uses Git paths without requiring deleted or renamed files to exist"
        );
        assert!(
            !add_paths(b"leaf/\xff\0", &mut paths),
            "Non-UTF-8 input requires exhaustive mode"
        );
        Ok(())
    }

    #[test]
    fn global_and_unknown_paths_require_exhaustive_checks() -> TestResult {
        let mut packages = workspace()?;
        packages.push(Package {
            name: "internal-tools".into(),
            directory: "tests/it".into(),
            dependencies: BTreeSet::new(),
            extra_inputs: Vec::new(),
        });
        for path in [
            "Cargo.lock",
            "leaf/Cargo.toml",
            "justfile",
            ".config/nextest.toml",
            "leaf/fuzz/fuzz_targets/parse.rs",
            "tests/it/src/ci/plan.rs",
            "tests/it/ci/check-features.sh",
            "tests/helpers.sh",
            "unknown-crate/src/lib.rs",
        ] {
            let plan = select(&packages, &BTreeSet::from([path.into()]));
            assert_eq!(
                plan["mode"], "exhaustive",
                "Unclassified or global path {path} cannot skip checks"
            );
        }
        let plan = select(&packages, &BTreeSet::from(["DEVELOPMENT.md".into()]));
        assert_eq!(plan["packages"], json!([]), "Known prose changes need no Rust builds");
        Ok(())
    }

    #[test]
    fn extra_inputs_select_consumers_and_their_dependents() -> TestResult {
        let mut packages = workspace()?;
        packages[0].extra_inputs.push("unrelated/tests/fixtures".into());
        let plan = select(&packages, &BTreeSet::from(["unrelated/tests/fixtures/input".into()]));
        assert_eq!(
            plan["packages"].as_array().map(Vec::len),
            Some(7),
            "Shared fixtures propagate to consumers"
        );
        let plan = select(
            &packages,
            &BTreeSet::from(["unrelated/tests/fixtures-other/input".into()]),
        );
        assert_eq!(
            plan["packages"],
            json!(["unrelated"]),
            "Input prefixes match whole path components"
        );
        Ok(())
    }

    #[test]
    fn fallback_selects_every_package_and_explains_why() -> TestResult {
        let plan = exhaustive(&workspace()?, "Missing comparison base");
        assert_eq!(
            plan["mode"], "exhaustive",
            "Missing history must not silently skip work"
        );
        assert_eq!(
            plan["packages"].as_array().map(Vec::len),
            Some(7),
            "Fallback includes every crate"
        );
        assert_eq!(
            plan["reasons"],
            json!(["Missing comparison base"]),
            "Fallback is reviewable"
        );
        Ok(())
    }
}
