use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use gix_error::{OptionExt, Result, bail, message};
use serde_json::{Value, json};

use super::{
    requirements::Requirements,
    run::{self, Command},
};

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
    pub requirements: Requirements,
}

impl Task {
    fn new(id: &'static str, group: &'static str, stages: &'static [&'static str], commands: Vec<Command>) -> Self {
        Self {
            id,
            group,
            stages,
            blocking: true,
            requirements: Requirements::commands(&commands),
            commands,
        }
    }

    fn optional(mut self) -> Self {
        self.blocking = false;
        self
    }

    fn json(&self, metadata: &Value) -> Result<Value> {
        Ok(json!({"id": self.id, "group": self.group, "blocking": self.blocking,
            "commands": self.commands.iter().map(|command| json!({
                "program": command.program, "args": command.args, "env": command.env, "requires": command.requires
            })).collect::<Vec<_>>(), "requirements": self.requirements.json(metadata)?}))
    }
}

pub fn host() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

fn parts(platform: &str) -> Result<(&str, &str)> {
    if !PLATFORMS.contains(&platform) {
        bail!(
            "Unsupported QA platform {platform}; supported platforms: {}",
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
                    env!("CARGO_PKG_NAME"),
                    "--no-default-features",
                    "--bin",
                    env!("CARGO_BIN_NAME"),
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
                Command::new("@qa", &["workflows"]),
                Command::new("@qa", &["badge"]),
                Command::new("@qa", &["mode"]),
            ],
        ),
    ];
    if !has_work {
        for task in &out {
            task.json(metadata)?;
        }
        return Ok(out);
    }
    let packages = metadata["packages"]
        .as_array()
        .ok_or_raise(|| message("missing packages"))?;
    let selected_packages: Vec<_> = packages
        .iter()
        .filter(|p| selected.contains(&p["name"].as_str().expect("validated package name")))
        .collect();

    // Clippy performs the default compilation check too; do not schedule cargo check again.
    out.push(Task::new("lint", "lint", &["lint", "check"], core("lint")?));
    let checks = core("feature-checks")?;
    if !checks.is_empty() {
        out.push(Task::new("feature-builds", "features", &["check"], checks));
    }
    if exhaustive {
        out.push(Task::new(
            "feature-contracts",
            "features",
            &["check"],
            vec![Command::new("bash", &["tests/it/qa/check-features.sh"])],
        ));
    }

    if os == "windows" {
        // Nextest compiles examples but can omit binaries declared test = false.
        let binaries: Vec<_> = selected_packages
            .iter()
            .filter(|p| run::has_target(p, "bins"))
            .collect();
        if !binaries.is_empty() {
            let mut command = Command::new("cargo", &["check", "--locked", "--bins"]);
            if exhaustive {
                // Keep workspace feature unification, as for exhaustive nextest runs.
                command.args.push("--workspace".into());
            } else {
                for package in binaries {
                    command
                        .args
                        .extend(["-p".into(), package["name"].as_str().expect("validated name").into()]);
                }
            }
            out.push(Task::new(
                "native-bins",
                "tests",
                &["check", "tests", "default-tests"],
                vec![command],
            ));
        }
    }

    let isolated: Vec<_> = packages
        .iter()
        .filter_map(|package| match run::flag(package, "isolated") {
            Ok(true) => Some(Ok(package["name"].as_str().expect("validated name"))),
            Ok(false) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<Result<_>>()?;
    let tested: Vec<_> = selected_packages
        .iter()
        .filter(|p| run::has_target(p, "test") && !isolated.contains(&p["name"].as_str().expect("validated name")))
        .map(|p| p["name"].as_str().expect("validated name"))
        .collect();
    if !tested.is_empty() {
        let mut command = Command::new("@qa", &["native-tests", if exhaustive { "exhaustive" } else { "fast" }])
            .requiring(&["cargo-nextest"]);
        if exhaustive {
            command.args.push("--workspace".into());
            for name in isolated {
                command.args.extend(["--exclude".into(), name.into()]);
            }
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
    let isolated_tests = core("isolated-tests")?;
    if !isolated_tests.is_empty() {
        out.push(Task::new(
            "isolated-tests",
            "tests",
            &["tests", "default-tests"],
            isolated_tests,
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
    let docs = core("docs")?;
    if !docs.is_empty() {
        out.push(Task::new("docs", "docs", &["docs"], docs));
    }
    if os != "windows" {
        let journey = core("journey")?;
        if !journey.is_empty() {
            out.push(Task::new("journey", "journey", &["journey"], journey));
        }
    }
    let mut api = core("api")?;
    if !api.is_empty() {
        let checker = packages
            .iter()
            .find(|p| p["metadata"]["qa"]["api-checker"] == true)
            .ok_or_raise(|| message("Declare a QA API checker"))?;
        let toolchain = checker["metadata"]["qa"]["api-toolchain"]
            .as_str()
            .ok_or_raise(|| message("QA API checker needs api-toolchain"))?;
        for command in &mut api {
            command.env.insert("RUSTUP_TOOLCHAIN".into(), toolchain.into());
        }
        out.push(Task::new("api", "api", &["api"], api));
    }
    if exhaustive {
        let toolchains = super::ops::msrv_toolchains(metadata)?;
        if !toolchains.is_empty() {
            let command = Command::new("@qa", &["msrv"]).requiring(&["toolchain:nightly"]);
            let mut task = Task::new("msrv", "msrv", &["msrv"], vec![command]);
            task.requirements
                .merge(Requirements::commands(&super::ops::msrv_commands(metadata)?));
            out.push(task);
        }
        out.push(Task::new("auditable", "auditable", &["auditable"], core("auditable")?));
        out.push(
            Task::new(
                "size",
                "size",
                &["size"],
                vec![Command::new("bash", &["etc/scripts/check-package-size.sh"]).requiring(&["cargo-diet"])],
            )
            .optional(),
        );
        if os == "linux" && arch == "x86_64" {
            out.push(Task::new(
                "pure-rust",
                "pure-rust",
                &["pure-rust"],
                vec![Command::new("@qa", &["pure-rust"]).requiring(&["docker"])],
            ));
        }
        if os == "windows" {
            let without_git: Vec<_> = selected_packages
                .iter()
                .filter(|p| p["metadata"]["qa"]["without-git"] == true)
                .collect();
            if !without_git.is_empty() {
                let mut command = Command::new("@qa", &["no-git", "--no-fail-fast"]).requiring(&["cargo-nextest"]);
                for package in without_git {
                    command
                        .args
                        .extend(["-p".into(), package["name"].as_str().expect("validated name").into()]);
                }
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
    let commands = super::ops::wasm_commands(metadata, &selected)?;
    if !commands.is_empty() {
        out.push(Task::new("wasm", "wasm", &["wasm"], commands).optional());
    }
    for task in &out {
        task.json(metadata)?;
    }

    Ok(out)
}

pub const STAGES: [&str; 19] = [
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

pub fn select(tasks: Vec<Task>, stages: &[String]) -> Result<Vec<Task>> {
    for stage in stages {
        if !STAGES.contains(&stage.as_str()) {
            bail!("Unknown QA stage {stage}");
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
        "native-tests" | "native-bins" | "isolated-tests" | "without-git" | "size32" => {
            exhaustive || matches!(platform, "linux-x86_64" | "macos-aarch64" | "windows-x86_64")
        }
        "msrv" => matches!(platform, "linux-x86_64" | "windows-x86_64"),
        "default-doctests" => platform == "linux-x86_64" || (exhaustive && platform == "windows-x86_64"),
        _ => platform == "linux-x86_64",
    }
}

fn job(metadata: &Value, platform: &str, group: &str, tasks: &[Task]) -> Result<Value> {
    let (runner, container, host, architecture) = match platform {
        "linux-x86_64" => ("ubuntu-latest", "", "", ""),
        "linux-aarch64" => ("ubuntu-24.04-arm", "", "", ""),
        // Keep Ubuntu 24.04 until actions/runner-images#14790 is fixed.
        "linux-x86" => (
            "ubuntu-24.04",
            "i386/debian:stable-slim@sha256:c60c0e2d9f3cf661bcfdb673b7da1c550562efefc802cd9f90c153f8f4a0b491",
            "i686-unknown-linux-gnu",
            "amd64",
        ),
        "linux-arm" => (
            "ubuntu-24.04-arm",
            "arm32v7/debian:stable-slim@sha256:f388bccfe138b76e6f6b8b235bcd00ea9935c8310b40a34592541b4f7a176b39",
            "armv7-unknown-linux-gnueabihf",
            "arm64",
        ),
        "macos-aarch64" => ("macos-latest", "", "", ""),
        "macos-x86_64" => ("macos-15-intel", "", "", ""),
        "windows-x86_64" => ("windows-latest", "", "", ""),
        "windows-aarch64" => ("windows-11-arm", "", "", ""),
        _ => unreachable!("validated platform"),
    };
    let mut requirements = Requirements::default();
    for task in tasks {
        requirements.merge(task.requirements.clone());
    }
    let requirements = requirements.json(metadata)?;
    let mut row = json!({"id": format!("{group}@{platform}"), "platform": platform, "runner": runner, "container": container,
        "host": host, "runner_architecture": architecture, "cache": format!("ci-{group}-{platform}"),
        "blocking": tasks.iter().any(|task| task.blocking), "tasks": tasks.iter().map(|task| task.id).collect::<Vec<_>>()});
    row.as_object_mut()
        .expect("job row is an object")
        .extend(requirements.as_object().expect("requirements are an object").clone());
    Ok(row)
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
                described.push(json!({"platform": platform, "task": task.json(metadata)?}));
                grouped.entry(task.group).or_default().push(task);
            } else if task.id == "self-test" && *platform == "linux-x86_64" {
                described.push(json!({"platform": platform, "task": task.json(metadata)?}));
            }
        }
        for (group, tasks) in grouped {
            jobs.push(job(metadata, platform, group, &tasks)?);
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
            "\nQA task: {}{}",
            task.id,
            if task.blocking { "" } else { " (nonblocking)" }
        );
        let result = if matches!(task.id, "feature-tests" | "isolated-tests") {
            super::native_tests::variants(root, task.commands, dry_run)
        } else if task.id == "feature-builds" {
            run::each_case(task.commands, |command| run::execute(root, vec![command], dry_run))
        } else {
            run::execute(root, task.commands, dry_run)
        };
        if let Err(err) = result {
            eprintln!("{err:?}");
            // CI's continue-on-error preserves a visible failure for optional jobs.
            if task.blocking || ci_job {
                failed.push(task.id);
            } else {
                eprintln!("Nonblocking QA task failed: {}", task.id);
            }
        }
    }
    if !failed.is_empty() {
        bail!("Failed QA tasks: {}", failed.join(", "));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata() -> Value {
        json!({"packages": [
            {"name":"gix-path", "targets":[{"kind":["lib"], "test":true,"doc":true,"doctest":true}], "metadata":{"qa":{"without-git":true}}},
            {"name":"internal-tools", "targets":[{"kind":["bin"],"test":true}], "metadata":{"qa":{"api-checker":true,"api-toolchain":"1.99.0"}}},
            {"name":"gitoxide", "targets":[{"kind":["bin"],"test":false}], "metadata":{"qa":{"journey":true,"auditable":true}}}
        ],"metadata":{"qa":{"tool-versions":{
            "cargo-nextest":"0.9.122","cargo-deny":"0.20.0","cargo-machete":"0.9.1",
            "cargo-semver-checks":"0.51.0","cargo-auditable":"0.7.6","cargo-diet":"1.2.7","just":"1.58.0"
        }}}})
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
    fn specialties_follow_manifest_roles_for_new_package_names() -> gix_error::TestResult {
        let mut metadata = metadata();
        metadata["packages"].as_array_mut().expect("test packages").push(json!({
            "name":"new-isolated-library", "targets":[{"kind":["lib"],"test":true,"doc":false,"doctest":false}],
            "metadata":{"qa":{"isolated":true}}
        }));
        let selection = json!({"mode":"fast", "packages":["new-isolated-library"]});
        let tasks = tasks(&metadata, &selection, "linux-x86_64")?;
        assert!(
            !tasks
                .iter()
                .any(|task| matches!(task.id, "native-tests" | "docs" | "default-doctests")),
            "target flags and isolation omit inapplicable baseline work"
        );
        let isolated = tasks
            .iter()
            .find(|task| task.id == "isolated-tests")
            .expect("isolated tests");
        assert!(
            isolated.commands[0]
                .args
                .ends_with(&["-p".into(), "new-isolated-library".into()]),
            "new crates need no central name registration"
        );
        Ok(())
    }

    #[test]
    fn msrv_jobs_preserve_nested_command_requirements() -> gix_error::TestResult {
        let mut metadata = metadata();
        metadata["packages"][0]["rust_version"] = json!("1.98");
        metadata["packages"][0]["metadata"]["qa"]["msrv"] = json!({"checks":[{
            "args":["--target", "wasm32-wasip1"], "requires":["rustfmt"]
        }]});
        let plan = plan(
            &metadata,
            &json!({"mode":"exhaustive","packages":[]}),
            "linux-x86_64",
            true,
        )?;
        let job = plan["jobs"]
            .as_array()
            .expect("jobs")
            .iter()
            .find(|job| job["id"] == "msrv@linux-x86_64")
            .expect("MSRV job");
        assert_eq!(job["rust_toolchains"], json!(["1.98.0", "nightly"]));
        assert_eq!(
            job["rust_targets"],
            json!([{"toolchain":"1.98.0", "target":"wasm32-wasip1"}]),
            "the snapshot wrapper preserves the inner command's target toolchain"
        );
        assert_eq!(
            job["rust_components"],
            json!([{"toolchain":"1.98.0", "component":"rustfmt"}])
        );
        Ok(())
    }

    #[test]
    fn windows_binary_checks_preserve_exhaustive_feature_unification() -> gix_error::TestResult {
        for mode in ["fast", "exhaustive"] {
            let selection = json!({"mode":mode,"packages":["gitoxide"]});
            let tasks = tasks(&metadata(), &selection, "windows-x86_64")?;
            let command = &tasks
                .iter()
                .find(|task| task.id == "native-bins")
                .expect("binary check")
                .commands[0];
            assert_eq!(
                command.args.contains(&"--workspace".into()),
                mode == "exhaustive",
                "exhaustive compilation retains Cargo's workspace feature unification"
            );
            assert_eq!(
                command.args.contains(&"-p".into()),
                mode == "fast",
                "fast compilation selects only packages with untested binaries"
            );
        }
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
                        "every QA task is runnable through the local catalogue"
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
