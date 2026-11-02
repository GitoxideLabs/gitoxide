use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

use gix_error::{OptionExt, Result, ResultExt, bail, message};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    pub program: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub requires: Vec<String>,
}

impl Command {
    pub fn new(program: &str, args: &[&str]) -> Self {
        Self {
            program: program.into(),
            args: args.iter().map(|arg| (*arg).into()).collect(),
            env: BTreeMap::new(),
            requires: Vec::new(),
        }
    }

    pub fn requiring(mut self, requirements: &[&str]) -> Self {
        self.requires.extend(requirements.iter().map(|name| (*name).into()));
        self
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
    "isolated-tests",
    "feature-checks",
    "docs",
    "journey",
    "api",
    "auditable",
];

pub fn commands(metadata: &Value, selection: &Value, stages: &[String]) -> Result<Vec<Command>> {
    let exhaustive = match selection["mode"].as_str() {
        Some("fast") => false,
        Some("exhaustive") => true,
        _ => bail!("QA plan mode must be 'fast' or 'exhaustive'"),
    };
    let packages = metadata["packages"]
        .as_array()
        .ok_or_raise(|| message("Cargo metadata must contain a packages array"))?;
    let mut by_name = BTreeMap::new();
    for package in packages {
        validate_settings(package)?;
        let name = package["name"]
            .as_str()
            .ok_or_raise(|| message("Cargo package names must be strings"))?;
        by_name.insert(name, package);
    }
    let selected = strings(&selection["packages"], "QA plan packages")?;
    for name in &selected {
        if !by_name.contains_key(name) {
            bail!("QA plan contains unknown package '{name}'");
        }
    }
    let mut expanded = Vec::new();
    for stage in stages {
        if stage == "all" {
            expanded.extend_from_slice(STAGES);
        } else if STAGES.contains(&stage.as_str()) {
            expanded.push(stage.as_str());
        } else {
            bail!("Unknown QA stage '{stage}'");
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
                    out.push(
                        Command::new(
                            "just",
                            &["clippy", "-D", "warnings", "-A", "unknown-lints", "--no-deps"],
                        )
                        .requiring(&["clippy"]),
                    );
                } else {
                    let mut command = cargo(&["clippy", "--locked", "--all-targets"], &selected, false);
                    command
                        .args
                        .extend(["--", "-D", "warnings", "-A", "unknown-lints", "--no-deps"].map(String::from));
                    out.push(command);
                }
            }
            "variants" | "isolated-tests" | "default-doctests" | "doctest-variants" | "feature-checks" => {
                let doctests = matches!(stage, "default-doctests" | "doctest-variants");
                let args: &[&str] = if stage == "feature-checks" {
                    &["check", "--locked"]
                } else if doctests {
                    &["test", "--doc", "--locked", "--no-fail-fast"]
                } else {
                    &["nextest", "run", "--locked", "--no-fail-fast"]
                };
                if stage == "default-doctests" {
                    let packages: Vec<_> = selected
                        .iter()
                        .copied()
                        .filter(|name| has_target(by_name[name], "doctest"))
                        .collect();
                    if !packages.is_empty() {
                        out.push(cargo(args, &packages, exhaustive));
                    }
                    continue;
                }
                let category = if doctests { "doctests" } else { "tests" };
                for (name, package) in &by_name {
                    let isolated = flag(package, "isolated")?;
                    if (stage == "isolated-tests" && !isolated) || (stage == "variants" && isolated) {
                        continue;
                    }
                    let settings = &package["metadata"]["qa"];
                    let cases = if stage == "feature-checks" {
                        super::features::checks(package, settings)?
                    } else if let Some(cases) = settings.get(category) {
                        cases
                            .as_array()
                            .ok_or_raise(|| message!("{name}: qa.{category} must be an array"))?
                            .clone()
                    } else if stage == "isolated-tests" {
                        vec![serde_json::json!({})]
                    } else {
                        Vec::new()
                    };
                    for case in cases {
                        let case_packages = match case.get("packages") {
                            Some(value) => strings(value, "QA case packages")?,
                            None => vec![*name],
                        };
                        if case_packages.is_empty() || case_packages.iter().any(|name| !by_name.contains_key(name)) {
                            bail!("{name}: QA case packages must name existing workspace packages");
                        }
                        let command = case_command(args, &case_packages, &case)?;
                        if case_packages.iter().any(|name| selected.contains(name)) {
                            out.push(command);
                        }
                    }
                }
            }
            "docs" if exhaustive => out.push(Command::new("just", &["doc"])),
            "docs" => {
                let packages: Vec<_> = selected
                    .iter()
                    .copied()
                    .filter(|name| has_target(by_name[name], "doc"))
                    .collect();
                if packages.is_empty() {
                    continue;
                }
                let mut command = cargo(&["doc", "--locked", "--no-deps"], &packages, false);
                command.env.insert("RUSTDOCFLAGS".into(), "-D warnings".into());
                out.push(command);
            }
            "journey"
                if exhaustive
                    && selected
                        .iter()
                        .any(|name| by_name[name]["metadata"]["qa"]["journey"] == true) =>
            {
                out.push(Command::new("just", &["qa-journey-tests"]));
            }
            "journey"
                if selected
                    .iter()
                    .any(|name| by_name[name]["metadata"]["qa"]["journey"] == true) =>
            {
                out.push(Command::new("just", &["journey-tests-small"]));
            }
            "api"
                if exhaustive
                    || selected
                        .iter()
                        .any(|name| by_name[name]["metadata"]["qa"]["api-checker"] == true) =>
            {
                out.push(Command::new("just", &["check-api"]).requiring(&["cargo-semver-checks"]));
            }
            "auditable"
                if exhaustive
                    && selected
                        .iter()
                        .any(|name| by_name[name]["metadata"]["qa"]["auditable"] == true) =>
            {
                out.push(Command::new("just", &["check-auditable"]).requiring(&["cargo-auditable"]));
            }
            _ => {}
        }
    }
    Ok(out)
}

fn validate_settings(package: &Value) -> Result<()> {
    let settings = &package["metadata"]["qa"];
    if settings.is_null() {
        return Ok(());
    }
    let settings = settings
        .as_object()
        .ok_or_raise(|| message!("{}: metadata.qa must be a table", package["name"]))?;
    for key in settings.keys() {
        if ![
            "tests",
            "doctests",
            "checks",
            "each-feature",
            "extra-inputs",
            "isolated",
            "without-git",
            "journey",
            "auditable",
            "msrv",
            "wasm",
            "api-checker",
            "api-toolchain",
            "downstream",
        ]
        .contains(&key.as_str())
        {
            bail!("{}: unknown QA setting {key}", package["name"]);
        }
    }
    for key in ["isolated", "without-git", "journey", "auditable", "api-checker"] {
        flag(package, key)?;
    }
    if flag(package, "isolated")?
        && settings
            .get("tests")
            .is_some_and(|tests| tests.as_array().is_some_and(Vec::is_empty))
    {
        bail!("{}: isolated QA tests cannot be an empty array", package["name"]);
    }
    Ok(())
}

pub(super) fn flag(package: &Value, name: &str) -> Result<bool> {
    match package["metadata"]["qa"].get(name) {
        None => Ok(false),
        Some(value) => value
            .as_bool()
            .ok_or_raise(|| message!("{}: qa.{name} must be a boolean", package["name"])),
    }
}

pub(super) fn has_target(package: &Value, role: &str) -> bool {
    package["targets"].as_array().is_some_and(|targets| {
        targets.iter().any(|target| {
            let kinds = target["kind"].as_array();
            let kind = |name: &str| kinds.is_some_and(|kinds| kinds.iter().any(|kind| kind == name));
            match role {
                "test" => target["test"] == true || kind("example"),
                "bins" => kind("bin") && target["test"] == false,
                "doc" => target["doc"] == true,
                "doctest" => target["doctest"] == true,
                _ => false,
            }
        })
    })
}

pub(super) fn case_command(args: &[&str], packages: &[&str], case: &Value) -> Result<Command> {
    let fields = case.as_object().ok_or_raise(|| message("QA cases must be objects"))?;
    for key in fields.keys() {
        if !["args", "env", "requires", "packages"].contains(&key.as_str()) {
            bail!("Unknown QA case field: {key}");
        }
    }
    let mut command = cargo(args, packages, false);
    if let Some(args) = case.get("args") {
        command
            .args
            .extend(strings(args, "QA case args")?.into_iter().map(String::from));
    }
    if let Some(requires) = case.get("requires") {
        command
            .requires
            .extend(strings(requires, "QA case requires")?.into_iter().map(String::from));
    }
    if let Some(env) = case.get("env") {
        for (key, value) in env
            .as_object()
            .ok_or_raise(|| message("QA case env must be an object"))?
        {
            let value = value
                .as_str()
                .ok_or_raise(|| message("QA case environment values must be strings"))?;
            if key.is_empty() || key.contains(['=', '\0']) || value.contains('\0') {
                bail!("Invalid QA case environment variable");
            }
            command.env.insert(key.clone(), value.into());
        }
    }
    Ok(command)
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
    let mut child = if command.program == "@qa" {
        let mut child = process::Command::new(std::env::current_exe().or_raise(|| message("locate QA runner"))?);
        child.arg("exec");
        child
    } else {
        process::Command::new(&command.program)
    };
    // `cargo run` sets this for QA; cargo-machete uses its absence to recognize Cargo invocations.
    child.env_remove("CARGO_PKG_NAME");
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

pub(super) fn each_case(
    commands: Vec<Command>,
    log: Option<&File>,
    mut execute: impl FnMut(Command) -> Result<()>,
) -> Result<()> {
    let mut failed = Vec::new();
    let count = commands.len();
    for (index, command) in commands.into_iter().enumerate() {
        if let Some(mut log) = log {
            eprintln!("  QA case {}/{count}", index + 1);
            writeln!(
                log,
                "QA case {}/{count}: {} {:?}",
                index + 1,
                command.program,
                command.args
            )
            .or_raise(|| message("Write QA case to log"))?;
        } else {
            eprintln!("QA case {}/{count}: {} {:?}", index + 1, command.program, command.args);
        }
        if let Err(error) = execute(command) {
            if let Some(mut log) = log {
                writeln!(log, "{error:?}").or_raise(|| message("Write QA case failure to log"))?;
            } else {
                eprintln!("{error:?}");
            }
            failed.push((index + 1).to_string());
        }
    }
    if !failed.is_empty() {
        bail!("Failed QA cases: {}", failed.join(", "));
    }
    Ok(())
}

pub(super) fn log_directory() -> Result<PathBuf> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .or_raise(|| message("Read time for QA logs"))?
        .as_nanos();
    let path = std::env::temp_dir().join(format!("qa-logs-{}-{stamp}", std::process::id()));
    #[cfg(unix)]
    let directory = {
        use std::os::unix::fs::DirBuilderExt;
        let mut directory = fs::DirBuilder::new();
        directory.mode(0o700);
        directory
    };
    #[cfg(not(unix))]
    let directory = fs::DirBuilder::new();
    directory
        .create(&path)
        .or_raise(|| message!("Create QA log directory {}", path.display()))?;
    Ok(path)
}

pub(super) fn redirect(command: &mut process::Command, log: Option<&File>, stdout: bool) -> Result<()> {
    if let Some(log) = log {
        command.stderr(log.try_clone().or_raise(|| message("Clone QA stderr log"))?);
        if stdout {
            command.stdout(log.try_clone().or_raise(|| message("Clone QA stdout log"))?);
        }
    }
    Ok(())
}

pub fn execute(root: &Path, commands: Vec<Command>, dry_run: bool, log: Option<&File>) -> Result<()> {
    for command in commands {
        let command = normalize(command);
        let mut parts = command
            .env
            .iter()
            .map(|(key, value)| format!("{key}={value:?}"))
            .collect::<Vec<_>>();
        parts.push(command.program.clone());
        parts.extend(command.args.iter().map(|arg| format!("{arg:?}")));
        if let Some(mut log) = log {
            writeln!(log, "{}\n{}", root.display(), parts.join(" ")).or_raise(|| message("Write QA command to log"))?;
        } else {
            eprintln!("{}", parts.join(" "));
        }
        if dry_run {
            continue;
        }
        let mut child = process(root, &command)?;
        redirect(&mut child, log, true)?;
        let status = child
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

    #[test]
    #[cfg(unix)]
    fn logging_stderr_preserves_machine_readable_stdout() -> gix_error::TestResult {
        use std::os::unix::fs::PermissionsExt;

        let logs = super::super::ops::Snapshot { path: log_directory()? };
        assert_eq!(
            fs::metadata(&logs.path)?.permissions().mode() & 0o777,
            0o700,
            "QA logs are private to the user"
        );
        let path = logs.path.join("metadata.log");
        let log = File::create(&path)?;
        let mut command = process::Command::new("sh");
        command.args(["-c", "printf '{\"binaries\":[]}'; printf build-diagnostic >&2"]);
        redirect(&mut command, Some(&log), false)?;
        let output = command.output()?;
        assert!(output.status.success(), "the metadata command succeeds");
        assert_eq!(
            output.stdout, b"{\"binaries\":[]}",
            "metadata stdout remains available to the caller"
        );
        assert_eq!(
            fs::read_to_string(path)?,
            "build-diagnostic",
            "compiler diagnostics go to the task log"
        );
        Ok(())
    }

    fn metadata() -> Value {
        json!({"packages": [
            {"name": "a", "targets": [{"kind": ["lib"]}], "metadata": {"qa": {"tests": [
                {"args": ["--features", "extra"], "env": {"GIX_TEST_FIXTURE_HASH": "sha256"}},
                {"args": [], "packages": ["a", "b"]}
            ], "doctests": [{"args": ["--features", "extra"]}]}, "api-check": {}}},
            {"name": "b", "targets": [{"kind": ["lib"]}], "metadata": {}},
            {"name": "gix-error", "targets": [{"kind": ["lib"]}], "metadata": {"qa": {"isolated":true,"tests": [{"args": []}]}}},
            {"name": "gitoxide", "targets": [{"kind": ["bin"],"test":true}], "metadata": {"qa":{"journey":true,"auditable":true}}},
            {"name": "internal-tools", "targets": [{"kind": ["bin"],"test":true}], "metadata": {"qa":{"api-checker":true}}}
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
            ("CARGO_PKG_NAME", None),
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
        let commands = commands(&metadata(), &plan("fast", &["gix-error"]), &["isolated-tests".into()])?;
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
        let root_only = super::commands(&metadata(), &plan("fast", &["gitoxide"]), &["isolated-tests".into()])?;
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
            exhaustive.contains(&Command::new("just", &["qa-journey-tests"])),
            "exhaustive retains all journey variants"
        );
        assert!(
            exhaustive.contains(&Command::new("just", &["check-auditable"]).requiring(&["cargo-auditable"])),
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
            [Command::new("just", &["check-api"]).requiring(&["cargo-semver-checks"])],
            "the API checker must validate itself even though internal-tools publishes no stable API"
        );
        Ok(())
    }

    #[test]
    fn target_eligibility_comes_from_cargo_metadata() {
        let package = json!({"targets": [
            {"kind": ["lib"], "test": false, "doctest": false, "doc": false}
        ]});
        for role in ["test", "doctest", "doc", "bins"] {
            assert!(
                !has_target(&package, role),
                "disabled {role} work must not create a baseline task"
            );
        }
        let example = json!({"targets": [{"kind": ["example"], "test": false}]});
        assert!(
            has_target(&example, "test"),
            "cargo test still checks example compilation"
        );
        let binary = json!({"targets": [{"kind": ["bin"], "test": false, "doc": true}]});
        assert!(
            has_target(&binary, "bins"),
            "binaries excluded from nextest need explicit native checks"
        );
        assert!(has_target(&binary, "doc"));
    }

    #[test]
    fn misspelled_policy_cannot_silently_remove_coverage() {
        let package = json!({"name":"sample", "metadata":{"qa":{"isolate":true}}});
        assert!(validate_settings(&package).is_err());
        let package = json!({"name":"sample", "metadata":{"qa":{"isolated":true,"tests":[]}}});
        assert!(
            validate_settings(&package).is_err(),
            "isolated packages need a test configuration"
        );
        assert!(case_command(&["test"], &["sample"], &json!({"require":["xz"]})).is_err());
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
        invalid["packages"][0]["metadata"]["qa"]["tests"][0]["args"] = json!([42]);
        assert!(
            commands(&invalid, &plan("fast", &["a"]), &["variants".into()]).is_err(),
            "arguments must be strings"
        );
        invalid["packages"][0]["metadata"]["qa"]["tests"][0]["args"] = json!([]);
        invalid["packages"][0]["metadata"]["qa"]["tests"][0]["env"] = json!({"BAD": false});
        assert!(
            commands(&invalid, &plan("fast", &["a"]), &["variants".into()]).is_err(),
            "environment values must be strings"
        );
        Ok(())
    }
}
