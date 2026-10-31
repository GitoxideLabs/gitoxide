use std::{collections::BTreeMap, path::Path, process};

use gix_error::{OptionExt, Result, ResultExt, bail, message};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    pub program: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}

impl Command {
    pub fn new(program: &str, args: &[&str]) -> Self {
        Self {
            program: program.into(),
            args: args.iter().map(|arg| (*arg).into()).collect(),
            env: BTreeMap::new(),
        }
    }

    fn packages(&mut self, packages: &[&str]) {
        for package in packages {
            self.args.extend(["-p".into(), (*package).into()]);
        }
    }
}

const STAGES: &[&str] = &[
    "lint",
    "default-doctests",
    "doctest-variants",
    "variants",
    "error-tests",
    "docs",
    "journey",
    "api",
    "auditable",
];

pub fn commands(metadata: &Value, selection: &Value, stages: &[String]) -> Result<Vec<Command>> {
    let exhaustive = match selection["mode"].as_str() {
        Some("fast") => false,
        Some("exhaustive") => true,
        _ => bail!("CI plan mode must be 'fast' or 'exhaustive'"),
    };
    let packages = metadata["packages"]
        .as_array()
        .ok_or_raise(|| message("Cargo metadata must contain a packages array"))?;
    let mut by_name = BTreeMap::new();
    for package in packages {
        let name = package["name"]
            .as_str()
            .ok_or_raise(|| message("Cargo package names must be strings"))?;
        by_name.insert(name, package);
    }
    let selected = strings(&selection["packages"], "CI plan packages")?;
    for name in &selected {
        if !by_name.contains_key(name) {
            bail!("CI plan contains unknown package '{name}'");
        }
    }
    let mut expanded = Vec::new();
    for stage in stages {
        if stage == "all" {
            expanded.extend_from_slice(STAGES);
        } else if STAGES.contains(&stage.as_str()) {
            expanded.push(stage.as_str());
        } else {
            bail!("Unknown CI stage '{stage}'");
        }
    }
    if !exhaustive && selected.is_empty() {
        return Ok(Vec::new());
    }
    let selected = if exhaustive {
        by_name.keys().copied().collect::<Vec<_>>()
    } else {
        selected
    };
    let mut out = Vec::new();
    let mut seen = Vec::new();
    for stage in expanded {
        if seen.contains(&stage) {
            continue;
        }
        seen.push(stage);
        match stage {
            "lint" => {
                out.push(Command::new("cargo", &["fmt", "--all", "--", "--check"]));
                if exhaustive {
                    out.push(Command::new("cargo", &["machete"]));
                    out.push(Command::new(
                        "just",
                        &["clippy", "-D", "warnings", "-A", "unknown-lints", "--no-deps"],
                    ));
                } else {
                    let mut command = cargo(&["clippy", "--locked", "--all-targets"], &selected, false);
                    command
                        .args
                        .extend(["--", "-D", "warnings", "-A", "unknown-lints", "--no-deps"].map(String::from));
                    out.push(command);
                }
            }
            "variants" | "error-tests" | "default-doctests" | "doctest-variants" => {
                let doctests = matches!(stage, "default-doctests" | "doctest-variants");
                let test_packages: Vec<_> = selected
                    .iter()
                    .copied()
                    .filter(|name| {
                        if doctests {
                            by_name[name]["targets"].as_array().is_some_and(|targets| {
                                targets.iter().any(|target| {
                                    target["kind"].as_array().is_some_and(|kinds| {
                                        kinds.iter().any(|kind| {
                                            matches!(
                                                kind.as_str(),
                                                Some("lib" | "proc-macro" | "rlib" | "dylib" | "cdylib" | "staticlib")
                                            )
                                        })
                                    })
                                })
                            })
                        } else {
                            *name != "gix-error"
                        }
                    })
                    .collect();
                let args: &[&str] = if doctests {
                    &["test", "--doc", "--locked", "--no-fail-fast"]
                } else {
                    &["nextest", "run", "--locked", "--no-fail-fast"]
                };
                if !test_packages.is_empty() && stage == "default-doctests" {
                    out.push(cargo(args, &test_packages, exhaustive));
                }
                if stage == "default-doctests" {
                    continue;
                }
                let category = if doctests { "doctests" } else { "tests" };
                for (name, package) in &by_name {
                    if (stage == "error-tests" && *name != "gix-error") || (stage == "variants" && *name == "gix-error")
                    {
                        continue;
                    }
                    let Some(cases) = package["metadata"]["ci"].get(category) else {
                        continue;
                    };
                    for case in cases
                        .as_array()
                        .ok_or_raise(|| message!("{name}: ci.{category} must be an array"))?
                    {
                        let case_packages = match case.get("packages") {
                            Some(value) => strings(value, "CI case packages")?,
                            None => vec![*name],
                        };
                        if case_packages.is_empty() || case_packages.iter().any(|name| !by_name.contains_key(name)) {
                            bail!("{name}: CI case packages must name existing workspace packages");
                        }
                        let extra_args = strings(&case["args"], "CI case args")?;
                        let mut command = cargo(args, &case_packages, false);
                        command.args.extend(extra_args.into_iter().map(String::from));
                        if let Some(env) = case.get("env") {
                            for (key, value) in env
                                .as_object()
                                .ok_or_raise(|| message!("{name}: CI case env must be an object"))?
                            {
                                let value = value
                                    .as_str()
                                    .ok_or_raise(|| message!("{name}: CI case environment values must be strings"))?;
                                if key.is_empty() || key.contains(['=', '\0']) || value.contains('\0') {
                                    bail!("{name}: invalid CI case environment variable");
                                }
                                command.env.insert(key.clone(), value.into());
                            }
                        }
                        if case_packages.iter().any(|name| selected.contains(name)) {
                            out.push(command);
                        }
                    }
                }
            }
            "docs" if exhaustive => out.push(Command::new("just", &["doc"])),
            "docs" => {
                let mut command = cargo(&["doc", "--locked", "--no-deps"], &selected, false);
                command.env.insert("RUSTDOCFLAGS".into(), "-D warnings".into());
                out.push(command);
            }
            "journey" if exhaustive => out.push(Command::new("just", &["ci-journey-tests"])),
            "journey" if selected.contains(&"gitoxide") => out.push(Command::new("just", &["journey-tests-small"])),
            "api"
                if exhaustive
                    || selected.contains(&"internal-tools")
                    || selected
                        .iter()
                        .any(|name| by_name[name]["metadata"].get("api-check").is_some()) =>
            {
                out.push(Command::new("just", &["check-api"]));
            }
            "auditable" if exhaustive => out.push(Command::new("just", &["check-auditable"])),
            _ => {}
        }
    }
    Ok(out)
}

fn strings<'a>(value: &'a Value, context: &str) -> Result<Vec<&'a str>> {
    value
        .as_array()
        .ok_or_raise(|| message!("{context} must be an array"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|value| !value.contains('\0'))
                .ok_or_raise(|| message!("{context} must contain strings without NUL bytes"))
        })
        .collect()
}

fn cargo(args: &[&str], packages: &[&str], exhaustive: bool) -> Command {
    let mut command = Command::new("cargo", args);
    if exhaustive {
        command.args.push("--workspace".into());
    } else {
        command.packages(packages);
    }
    command
}

pub(super) fn normalize(mut command: Command) -> Command {
    if !command.env.contains_key("GIX_TEST_CREATE_ARCHIVES_EVEN_ON_CI") {
        command.env.insert("GIX_TEST_IGNORE_ARCHIVES".into(), "1".into());
    }
    command
        .env
        .entry("RUSTUP_TOOLCHAIN".into())
        .or_insert_with(|| "stable".into());
    command.env.insert("CI".into(), "true".into());
    command.env.insert("INSTA_UPDATE".into(), "no".into());
    command.env.insert("INSTA_FORCE_PASS".into(), "0".into());
    command.env.insert("CARGO_TERM_COLOR".into(), "always".into());
    command.env.insert("CLICOLOR".into(), "1".into());
    command
}

pub(super) fn process(root: &Path, command: &Command) -> Result<process::Command> {
    let mut child = if command.program == "@ci" {
        let mut child = process::Command::new(std::env::current_exe().or_raise(|| message("locate CI runner"))?);
        child.arg("exec");
        child
    } else {
        process::Command::new(&command.program)
    };
    for name in [
        "GIX_TEST_IGNORE_ARCHIVES",
        "GIX_TEST_CREATE_ARCHIVES_EVEN_ON_CI",
        "GIX_TEST_FIXTURE_HASH",
        "GIX_MERGE_UPDATE_CARTESIAN_BASELINE",
        "GIX_MERGE_UPDATE_BASELINE",
        "INSTA_FORCE_UPDATE",
        "INSTA_FORCE_UPDATE_SNAPSHOTS",
        "TRYBUILD",
        "GIX_DIFF_SLIDER_CASE",
        "GIX_DIFF_SLIDER_STRICT",
        "ON_ERROR",
        "WITH_SNAPSHOT",
        "SNAPSHOT_FILTER",
    ] {
        child.env_remove(name);
    }
    child.args(&command.args).envs(&command.env).current_dir(root);
    Ok(child)
}

pub fn execute(root: &Path, commands: Vec<Command>, dry_run: bool) -> Result<()> {
    for command in commands {
        let command = normalize(command);
        let mut parts = command
            .env
            .iter()
            .map(|(key, value)| format!("{key}={value:?}"))
            .collect::<Vec<_>>();
        parts.push(command.program.clone());
        parts.extend(command.args.iter().map(|arg| format!("{arg:?}")));
        eprintln!("{}", parts.join(" "));
        if dry_run {
            continue;
        }
        let status = process(root, &command)?
            .status()
            .or_raise(|| message!("Failed to start {}", command.program))?;
        if !status.success() {
            bail!("{} failed with {status}", parts.join(" "));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;
    use serde_json::json;

    fn metadata() -> Value {
        json!({"packages": [
            {"name": "a", "targets": [{"kind": ["lib"]}], "metadata": {"ci": {"tests": [
                {"args": ["--features", "extra"], "env": {"GIX_TEST_FIXTURE_HASH": "sha256"}},
                {"args": [], "packages": ["a", "b"]}
            ], "doctests": [{"args": ["--features", "extra"]}]}, "api-check": {}}},
            {"name": "b", "targets": [{"kind": ["lib"]}], "metadata": {}},
            {"name": "gix-error", "targets": [{"kind": ["lib"]}], "metadata": {"ci": {"tests": [{"args": []}]}}},
            {"name": "gitoxide", "targets": [{"kind": ["bin"]}], "metadata": {}},
            {"name": "internal-tools", "targets": [{"kind": ["bin"]}], "metadata": {}}
        ]})
    }

    fn plan(mode: &str, packages: &[&str]) -> Value {
        json!({"mode": mode, "packages": packages})
    }

    #[test]
    fn shared_environment_prevents_local_bypasses_and_preserves_variants() -> gix_error::TestResult {
        let mut command = Command::new("cargo", &["nextest", "run"]);
        command.env.insert("RUSTUP_TOOLCHAIN".into(), "1.99.0".into());
        command.env.insert("GIX_TEST_FIXTURE_HASH".into(), "sha256".into());
        command.env.insert("INSTA_FORCE_PASS".into(), "1".into());
        let command = process(Path::new("workspace"), &normalize(command))?;
        let env: BTreeMap<_, _> = command.get_envs().collect();
        for (name, value) in [
            ("CI", Some("true")),
            ("INSTA_UPDATE", Some("no")),
            ("INSTA_FORCE_PASS", Some("0")),
            ("RUSTUP_TOOLCHAIN", Some("1.99.0")),
            ("GIX_TEST_FIXTURE_HASH", Some("sha256")),
            ("INSTA_FORCE_UPDATE", None),
            ("INSTA_FORCE_UPDATE_SNAPSHOTS", None),
            ("TRYBUILD", None),
            ("GIX_DIFF_SLIDER_CASE", None),
            ("GIX_DIFF_SLIDER_STRICT", None),
            ("ON_ERROR", None),
            ("WITH_SNAPSHOT", None),
            ("SNAPSHOT_FILTER", None),
        ] {
            assert_eq!(
                env.get(OsStr::new(name)),
                Some(&value.map(OsStr::new)),
                "{name} must match CI"
            );
        }
        Ok(())
    }

    #[test]
    fn selects_grouped_variants_and_preserves_environment() -> gix_error::TestResult {
        let commands = commands(&metadata(), &plan("fast", &["b"]), &["variants".into()])?;
        assert_eq!(commands.len(), 1, "a shared variant runs when any member is affected");
        assert_eq!(
            commands[0].args,
            ["nextest", "run", "--locked", "--no-fail-fast", "-p", "a", "-p", "b"],
            "grouped variants keep all member packages"
        );
        let commands = super::commands(&metadata(), &plan("fast", &["a"]), &["variants".into()])?;
        assert_eq!(
            commands[0].env["GIX_TEST_FIXTURE_HASH"], "sha256",
            "fixture hash overrides survive selection"
        );
        assert!(
            commands[0].args.ends_with(&["--features".into(), "extra".into()]),
            "variant features survive selection"
        );
        Ok(())
    }

    #[test]
    fn error_crate_runs_alone_and_empty_selection_does_nothing() -> gix_error::TestResult {
        let commands = commands(&metadata(), &plan("fast", &["gix-error"]), &["error-tests".into()])?;
        assert_eq!(
            commands.len(),
            1,
            "gix-error must not inherit workspace feature unification"
        );
        assert!(
            commands[0].args.ends_with(&["-p".into(), "gix-error".into()]),
            "isolated error tests still run"
        );
        assert!(
            super::commands(&metadata(), &plan("fast", &[]), &["all".into()])?.is_empty(),
            "empty selection must not fall back to the root package"
        );
        let root_only = super::commands(&metadata(), &plan("fast", &["gitoxide"]), &["error-tests".into()])?;
        assert!(
            root_only.is_empty(),
            "unrelated packages must not select isolated error tests"
        );
        assert!(
            super::commands(&metadata(), &plan("fast", &["gitoxide"]), &["default-doctests".into()])?.is_empty(),
            "Cargo rejects binary-only doctest selections"
        );
        Ok(())
    }

    #[test]
    fn exhaustive_and_fast_dispatch_remain_distinct() -> gix_error::TestResult {
        let exhaustive = commands(&metadata(), &plan("exhaustive", &["a"]), &["all".into()])?;
        assert_eq!(
            commands(&metadata(), &plan("exhaustive", &[]), &["all".into()])?,
            exhaustive,
            "an empty saved package list cannot suppress exhaustive coverage"
        );
        assert!(
            exhaustive.contains(&Command::new("just", &["ci-journey-tests"])),
            "exhaustive retains all journey variants"
        );
        assert!(
            exhaustive.contains(&Command::new("just", &["check-auditable"])),
            "exhaustive retains auditable builds"
        );
        let fast = commands(
            &metadata(),
            &plan("fast", &["gitoxide"]),
            &["journey".into(), "auditable".into()],
        )?;
        assert_eq!(
            fast,
            [Command::new("just", &["journey-tests-small"])],
            "fast mode runs one affected journey variant and defers auditable builds"
        );
        Ok(())
    }

    #[test]
    fn api_tooling_changes_run_the_api_checker_and_its_ignored_tests() -> gix_error::TestResult {
        let selected = commands(&metadata(), &plan("fast", &["internal-tools"]), &["api".into()])?;
        assert_eq!(
            selected,
            [Command::new("just", &["check-api"])],
            "the API checker must validate itself even though internal-tools publishes no stable API"
        );
        Ok(())
    }

    #[test]
    fn rejects_invalid_plans_and_metadata() -> gix_error::TestResult {
        assert!(
            commands(&metadata(), &plan("other", &["a"]), &["variants".into()]).is_err(),
            "unknown modes must not silently select coverage"
        );
        assert!(
            commands(&metadata(), &plan("fast", &["unknown"]), &["variants".into()]).is_err(),
            "stale package selections are rejected"
        );
        let mut invalid = metadata();
        invalid["packages"][0]["metadata"]["ci"]["tests"][0]["args"] = json!([42]);
        assert!(
            commands(&invalid, &plan("fast", &["a"]), &["variants".into()]).is_err(),
            "arguments must be strings"
        );
        invalid["packages"][0]["metadata"]["ci"]["tests"][0]["args"] = json!([]);
        invalid["packages"][0]["metadata"]["ci"]["tests"][0]["env"] = json!({"BAD": false});
        assert!(
            commands(&invalid, &plan("fast", &["a"]), &["variants".into()]).is_err(),
            "environment values must be strings"
        );
        Ok(())
    }
}
