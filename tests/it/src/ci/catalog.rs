use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use gix_error::{OptionExt, Result, bail, message};
use serde_json::{Value, json};

use super::run::{self, Command};

pub const PLATFORMS: &[&str] = &[
    "linux-x86_64",
    "linux-aarch64",
    "linux-x86",
    "linux-arm",
    "macos-aarch64",
    "macos-x86_64",
    "windows-x86_64",
    "windows-aarch64",
];
const CI_PLATFORMS: &[&str] = &[
    "linux-x86_64",
    "linux-aarch64",
    "linux-x86",
    "linux-arm",
    "macos-aarch64",
    "windows-x86_64",
    "windows-aarch64",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub id: &'static str,
    pub group: &'static str,
    pub stages: &'static [&'static str],
    pub blocking: bool,
    pub commands: Vec<Command>,
}

impl Task {
    fn new(id: &'static str, group: &'static str, stages: &'static [&'static str], commands: Vec<Command>) -> Self {
        Self {
            id,
            group,
            stages,
            blocking: true,
            commands,
        }
    }

    fn optional(mut self) -> Self {
        self.blocking = false;
        self
    }

    fn json(&self) -> Value {
        json!({"id": self.id, "group": self.group, "blocking": self.blocking,
            "commands": self.commands.iter().map(|command| json!({
                "program": command.program, "args": command.args, "env": command.env
            })).collect::<Vec<_>>()})
    }
}

pub fn host() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

fn parts(platform: &str) -> Result<(&str, &str)> {
    if !PLATFORMS.contains(&platform) {
        bail!(
            "Unsupported CI platform {platform}; supported platforms: {}",
            PLATFORMS.join(", ")
        );
    }
    platform
        .rsplit_once('-')
        .ok_or_raise(|| message("platform must include OS and architecture"))
}

pub fn tasks(metadata: &Value, selection: &Value, platform: &str) -> Result<Vec<Task>> {
    let (os, arch) = parts(platform)?;
    // Validate even empty selections before adding mandatory policy checks.
    run::commands(metadata, selection, &[])?;
    let exhaustive = selection["mode"] == "exhaustive";
    let selected: Vec<&str> = if exhaustive {
        metadata["packages"]
            .as_array()
            .ok_or_raise(|| message("missing packages"))?
            .iter()
            .filter_map(|package| package["name"].as_str())
            .collect()
    } else {
        selection["packages"]
            .as_array()
            .ok_or_raise(|| message("missing selection"))?
            .iter()
            .filter_map(Value::as_str)
            .collect()
    };
    let has_work = !selected.is_empty();
    let core = |stage: &str| run::commands(metadata, selection, &[stage.into()]);
    let mut out = vec![
        Task::new(
            "self-test",
            "self-test",
            &["self-test"],
            vec![Command::new(
                "cargo",
                &[
                    "test",
                    "--locked",
                    "-p",
                    "internal-tools",
                    "--no-default-features",
                    "--bin",
                    "ci",
                ],
            )],
        ),
        Task::new(
            "policy",
            "policy",
            &["policy", "audit"],
            vec![Command::new(
                "cargo",
                &[
                    "deny",
                    "--workspace",
                    "--all-features",
                    "--locked",
                    "check",
                    "bans",
                    "licenses",
                    "sources",
                ],
            )],
        ),
        Task::new(
            "advisories",
            "advisories",
            &["advisories", "audit"],
            vec![Command::new(
                "cargo",
                &[
                    "deny",
                    "--workspace",
                    "--all-features",
                    "--locked",
                    "check",
                    "advisories",
                ],
            )],
        )
        .optional(),
        Task::new(
            "housekeeping",
            "housekeeping",
            &["housekeeping"],
            vec![
                Command::new("@ci", &["workflows"]),
                Command::new("@ci", &["badge"]),
                Command::new("@ci", &["mode"]),
            ],
        ),
    ];
    if !has_work {
        return Ok(out);
    }

    // Clippy performs the default compilation check too; do not schedule cargo check again.
    out.push(Task::new("lint", "lint", &["lint", "check"], core("lint")?));
    if exhaustive {
        out.push(Task::new(
            "feature-builds",
            "features",
            &["check"],
            vec![Command::new("bash", &["tests/it/ci/check-features.sh"])],
        ));
    }

    if os == "windows" {
        // Nextest compiles examples but can omit binaries declared test = false.
        let mut command = Command::new("cargo", &["check", "--locked", "--bins"]);
        if exhaustive {
            command.args.push("--workspace".into());
        } else {
            for name in &selected {
                command.args.extend(["-p".into(), (*name).into()]);
            }
        }
        out.push(Task::new(
            "native-bins",
            "tests",
            &["check", "tests", "default-tests"],
            vec![command],
        ));
    }

    let tested: Vec<_> = selected.iter().copied().filter(|name| *name != "gix-error").collect();
    if !tested.is_empty() {
        let mut command = Command::new("@ci", &["native-tests", if exhaustive { "exhaustive" } else { "fast" }]);
        if exhaustive {
            command
                .args
                .extend(["--workspace", "--exclude", "gix-error"].map(String::from));
        } else {
            for name in tested {
                command.args.extend(["-p".into(), name.into()]);
            }
        }
        out.push(Task::new(
            "native-tests",
            "tests",
            &["tests", "default-tests"],
            vec![command],
        ));
    }
    let error_tests = core("error-tests")?;
    if !error_tests.is_empty() {
        out.push(Task::new(
            "error-tests",
            "tests",
            &["tests", "default-tests"],
            error_tests,
        ));
    }
    let variants = core("variants")?;
    if !variants.is_empty() {
        out.push(Task::new("feature-tests", "features", &["tests"], variants));
    }
    for (id, stage) in [
        ("default-doctests", "default-doctests"),
        ("feature-doctests", "doctest-variants"),
    ] {
        let commands = core(stage)?;
        if !commands.is_empty() {
            out.push(Task::new(id, "docs", &["doctests"], commands));
        }
    }
    out.push(Task::new("docs", "docs", &["docs"], core("docs")?));
    if os != "windows" {
        let journey = core("journey")?;
        if !journey.is_empty() {
            out.push(Task::new("journey", "journey", &["journey"], journey));
        }
    }
    let mut api = core("api")?;
    if !api.is_empty() {
        for command in &mut api {
            command.env.insert("RUSTUP_TOOLCHAIN".into(), "1.99.0".into());
        }
        out.push(Task::new("api", "api", &["api"], api));
    }
    if exhaustive {
        out.push(Task::new(
            "msrv",
            "msrv",
            &["msrv"],
            vec![Command::new("@ci", &["msrv"])],
        ));
        out.push(Task::new("auditable", "auditable", &["auditable"], core("auditable")?));
        out.push(
            Task::new(
                "size",
                "size",
                &["size"],
                vec![Command::new("bash", &["etc/scripts/check-package-size.sh"])],
            )
            .optional(),
        );
        if os == "linux" && arch == "x86_64" {
            out.push(Task::new(
                "pure-rust",
                "pure-rust",
                &["pure-rust"],
                vec![Command::new("@ci", &["pure-rust"])],
            ));
        }
        if os == "windows" {
            if selected.contains(&"gix-path") {
                let mut command = Command::new("@ci", &["no-git", "-p", "gix-path", "--no-fail-fast"]);
                if arch == "aarch64" {
                    command
                        .args
                        .extend(["--", "--skip", "performance", "--skip", "speed"].map(String::from));
                }
                out.push(Task::new(
                    "without-git",
                    "tests",
                    &["tests", "default-tests"],
                    vec![command],
                ));
            }
            if arch == "x86_64" {
                out.push(Task::new(
                    "size32",
                    "size32",
                    &["tests", "default-tests"],
                    vec![Command::new(
                        "cargo",
                        &[
                            "nextest",
                            "run",
                            "--locked",
                            "--target",
                            "i686-pc-windows-msvc",
                            "--workspace",
                            "--no-fail-fast",
                            "size",
                        ],
                    )],
                ));
            }
        }
    }
    // The operation filters its supported package/configuration catalogue before building.
    if selected.iter().any(|name| super::ops::supports_wasm(name)) {
        let commands = ["wasm32-unknown-unknown", "wasm32-wasip1", "wasm32-wasip2"].map(|target| {
            let mut command = Command::new("@ci", &["wasm", target]);
            command.args.extend(selected.iter().map(|name| (*name).into()));
            command
        });
        out.push(Task::new("wasm", "wasm", &["wasm"], commands.into()).optional());
    }
    Ok(out)
}

pub fn select(tasks: Vec<Task>, stages: &[String]) -> Result<Vec<Task>> {
    let known = [
        "all",
        "self-test",
        "policy",
        "advisories",
        "audit",
        "housekeeping",
        "lint",
        "check",
        "tests",
        "default-tests",
        "docs",
        "doctests",
        "journey",
        "api",
        "msrv",
        "auditable",
        "size",
        "pure-rust",
        "wasm",
    ];
    for stage in stages {
        if !known.contains(&stage.as_str()) {
            bail!("Unknown CI stage {stage}");
        }
    }
    Ok(tasks
        .into_iter()
        .filter(|task| {
            stages
                .iter()
                .any(|stage| stage == "all" || task.stages.contains(&stage.as_str()))
        })
        .collect())
}

fn assigned(task: &Task, platform: &str, exhaustive: bool) -> bool {
    match task.id {
        // The gate runs this task before it publishes a matrix.
        "self-test" => false,
        "native-tests" | "native-bins" | "error-tests" | "without-git" | "size32" => {
            exhaustive || matches!(platform, "linux-x86_64" | "macos-aarch64" | "windows-x86_64")
        }
        "msrv" => matches!(platform, "linux-x86_64" | "windows-x86_64"),
        "default-doctests" => platform == "linux-x86_64" || (exhaustive && platform == "windows-x86_64"),
        _ => platform == "linux-x86_64",
    }
}

fn job(platform: &str, group: &str, tasks: &[Task]) -> Value {
    let (runner, container, host, architecture) = match platform {
        "linux-x86_64" => ("ubuntu-latest", "", "", ""),
        "linux-aarch64" => ("ubuntu-24.04-arm", "", "", ""),
        // Keep Ubuntu 24.04 until actions/runner-images#14790 is fixed.
        "linux-x86" => (
            "ubuntu-24.04",
            "i386/debian:stable-slim",
            "i686-unknown-linux-gnu",
            "amd64",
        ),
        "linux-arm" => (
            "ubuntu-24.04-arm",
            "arm32v7/debian:stable-slim",
            "armv7-unknown-linux-gnueabihf",
            "arm64",
        ),
        "macos-aarch64" => ("macos-latest", "", "", ""),
        "macos-x86_64" => ("macos-15-intel", "", "", ""),
        "windows-x86_64" => ("windows-latest", "", "", ""),
        "windows-aarch64" => ("windows-11-arm", "", "", ""),
        _ => unreachable!("validated platform"),
    };
    let has = |id| tasks.iter().any(|task| task.id == id);
    let nextest = has("native-tests") || has("error-tests") || has("feature-tests") || has("size32");
    let mut tools = Vec::new();
    if nextest {
        tools.push("cargo-nextest@0.9.122");
    }
    if has("api") {
        tools.push("cargo-semver-checks@0.51.0");
    }
    if has("policy") || has("advisories") {
        tools.push("cargo-deny@0.20.0");
    }
    if has("lint") {
        tools.push("cargo-machete@0.9.1");
    }
    if has("auditable") {
        tools.push("cargo-auditable@0.7.6");
    }
    json!({"id": format!("{group}@{platform}"), "platform": platform, "runner": runner, "container": container,
        "host": host, "runner_architecture": architecture, "cache": format!("ci-{group}-{platform}"),
        "blocking": tasks.iter().any(|task| task.blocking), "tools": tools,
        "just": has("lint") || has("docs") || has("journey") || has("api") || has("msrv") || has("auditable"),
        "clippy": has("lint"), "semver": has("api"), "msrv": has("msrv"), "wasm": has("wasm"),
        "size32": has("size32"), "xz": has("feature-tests"), "diet": has("size"), "yq": has("housekeeping"),
        "tasks": tasks.iter().map(|task| task.id).collect::<Vec<_>>()})
}

pub fn plan(metadata: &Value, selection: &Value, platform: &str, all_platforms: bool) -> Result<Value> {
    let mut described = Vec::new();
    let mut jobs = Vec::new();
    for platform in if all_platforms {
        CI_PLATFORMS
    } else {
        std::slice::from_ref(&platform)
    } {
        let tasks = tasks(metadata, selection, platform)?;
        let mut grouped: BTreeMap<&str, Vec<Task>> = BTreeMap::new();
        for task in tasks {
            if !all_platforms || assigned(&task, platform, selection["mode"] == "exhaustive") {
                described.push(json!({"platform": platform, "task": task.json()}));
                grouped.entry(task.group).or_default().push(task);
            } else if task.id == "self-test" && *platform == "linux-x86_64" {
                described.push(json!({"platform": platform, "task": task.json()}));
            }
        }
        for (group, tasks) in grouped {
            jobs.push(job(platform, group, &tasks));
        }
    }
    Ok(json!({"selection": selection, "platform": platform, "tasks": described, "jobs": jobs}))
}

pub fn job_tasks(metadata: &Value, plan: &Value, job_id: &str) -> Result<Vec<Task>> {
    let row = plan["jobs"]
        .as_array()
        .ok_or_raise(|| message("plan is missing jobs"))?
        .iter()
        .find(|row| row["id"] == job_id)
        .ok_or_raise(|| message!("Job {job_id} is absent from the plan"))?;
    let platform = row["platform"]
        .as_str()
        .ok_or_raise(|| message("job has no platform"))?;
    if platform != host() {
        bail!("Job {job_id} requires {platform}; this runner is {}", host());
    }
    let ids: BTreeSet<&str> = row["tasks"]
        .as_array()
        .ok_or_raise(|| message("job is missing task IDs"))?
        .iter()
        .map(|value| value.as_str().ok_or_raise(|| message("task IDs must be strings")))
        .collect::<Result<_>>()?;
    let selected: Vec<_> = tasks(metadata, &plan["selection"], platform)?
        .into_iter()
        .filter(|task| ids.contains(task.id))
        .collect();
    if selected.len() != ids.len() {
        bail!("Job {job_id} contains stale task IDs; regenerate the plan");
    }
    Ok(selected)
}

pub fn execute(root: &Path, tasks: Vec<Task>, dry_run: bool, ci_job: bool) -> Result<()> {
    let mut failed = Vec::new();
    for task in tasks {
        eprintln!(
            "\nCI task: {}{}",
            task.id,
            if task.blocking { "" } else { " (nonblocking)" }
        );
        let result = if matches!(task.id, "feature-tests" | "error-tests") {
            super::native_tests::variants(root, task.commands, dry_run)
        } else {
            run::execute(root, task.commands, dry_run)
        };
        if let Err(err) = result {
            eprintln!("{err:?}");
            // CI's continue-on-error preserves a visible failure for optional jobs.
            if task.blocking || ci_job {
                failed.push(task.id);
            } else {
                eprintln!("Nonblocking CI task failed: {}", task.id);
            }
        }
    }
    if !failed.is_empty() {
        bail!("Failed CI tasks: {}", failed.join(", "));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata() -> Value {
        json!({"packages": [
            {"name":"gix-path", "targets":[{"kind":["lib"]}], "metadata":{}},
            {"name":"internal-tools", "targets":[{"kind":["bin"]}], "metadata":{}},
            {"name":"gitoxide", "targets":[{"kind":["bin"]}], "metadata":{}}
        ]})
    }

    #[test]
    fn empty_fast_selection_keeps_required_policy() -> gix_error::TestResult {
        let selection = json!({"mode":"fast", "packages":[]});
        for platform in PLATFORMS {
            let tasks = tasks(&metadata(), &selection, platform)?;
            assert!(
                tasks.iter().any(|task| task.id == "policy" && task.blocking),
                "licenses cannot be selected away"
            );
            assert!(
                !tasks.iter().any(|task| task.id == "native-tests"),
                "empty selections do not run package tests"
            );
        }
        Ok(())
    }

    #[test]
    fn overlapping_stages_select_each_task_once() -> gix_error::TestResult {
        let selection = json!({"mode":"exhaustive", "packages":[]});
        let all = tasks(&metadata(), &selection, "windows-x86_64")?;
        let tests = select(all.clone(), &["tests".into()])?;
        assert_eq!(
            tests,
            select(all, &["tests".into(), "default-tests".into(), "tests".into()])?,
            "stage union cannot repeat native tests"
        );
        Ok(())
    }

    #[test]
    fn ci_jobs_partition_the_same_local_tasks() -> gix_error::TestResult {
        for mode in ["fast", "exhaustive"] {
            let selection = json!({"mode":mode, "packages":["gix-path","gitoxide","internal-tools"]});
            let plan = plan(&metadata(), &selection, "linux-x86_64", true)?;
            let mut seen = BTreeSet::new();
            for job in plan["jobs"].as_array().expect("plan jobs") {
                let platform = job["platform"].as_str().expect("job platform");
                let local = tasks(&metadata(), &selection, platform)?;
                for id in job["tasks"].as_array().expect("job task IDs") {
                    let id = id.as_str().expect("task ID");
                    assert!(
                        seen.insert((platform, id)),
                        "a platform task must have exactly one CI owner"
                    );
                    assert!(
                        local.iter().any(|task| task.id == id),
                        "every CI task is runnable through the local catalogue"
                    );
                }
            }
            for platform in CI_PLATFORMS {
                for task in tasks(&metadata(), &selection, platform)? {
                    assert_eq!(
                        seen.contains(&(*platform, task.id)),
                        assigned(&task, platform, mode == "exhaustive"),
                        "every selected task is assigned once, including nonblocking tasks"
                    );
                }
            }
        }
        Ok(())
    }
}
