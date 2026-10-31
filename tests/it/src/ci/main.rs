use std::{fs::OpenOptions, io::Write, path::PathBuf};

use clap::{Parser, Subcommand};
use gix_error::{OptionExt, Result, ResultExt, message};
use serde_json::{Value, json};

mod catalog;
mod native_tests;
mod ops;
mod plan;
mod run;

#[derive(Parser)]
#[command(about = "Run the same CI tasks locally, or distribute them across CI platforms")]
struct Args {
    #[command(subcommand)]
    command: Action,
}

#[derive(clap::Args)]
struct Selection {
    /// Fast selects affected checks; exhaustive selects every distinct configuration.
    #[arg(long, default_value = "fast", value_parser = ["fast", "exhaustive"])]
    mode: String,
    /// Compare from the merge base with this revision, including local edits.
    #[arg(long, default_value = "origin/main")]
    base: String,
}

#[derive(Subcommand)]
enum Action {
    /// Show complete tasks and their commands for this host, or the CI assignment.
    Plan {
        #[command(flatten)]
        selection: Selection,
        /// Inspect another platform without running its commands.
        #[arg(long)]
        platform: Option<String>,
        #[arg(long)]
        all_platforms: bool,
        #[arg(long)]
        github_output: Option<PathBuf>,
    },
    /// Run all applicable tasks serially; independent tasks continue after a failure.
    Run {
        #[command(flatten)]
        selection: Selection,
        #[arg(long, conflicts_with = "from_env")]
        plan: Option<PathBuf>,
        /// Read CI_PLAN and optionally CI_JOB supplied by the CI scheduler.
        #[arg(long)]
        from_env: bool,
        /// Execute one job from a saved plan.
        #[arg(long)]
        job: Option<String>,
        /// Select a union of stages; overlapping selectors never repeat a task.
        #[arg(long, value_delimiter = ',', default_value = "all")]
        stage: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },
    #[command(hide = true)]
    Exec {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        args: Vec<String>,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();
    let current_dir = std::env::current_dir().or_raise(|| message("read current directory"))?;
    if let Action::Exec { args } = &args.command {
        if args[0] == "native-tests" {
            return native_tests::execute(&current_dir, &args[1..]);
        }
        return ops::execute(&current_dir, args);
    }
    let metadata = plan::metadata(&current_dir)?;
    let root = PathBuf::from(
        metadata["workspace_root"]
            .as_str()
            .ok_or_raise(|| message("missing workspace_root"))?,
    );
    match args.command {
        Action::Plan {
            selection,
            platform,
            all_platforms,
            github_output,
        } => {
            let selected = plan::plan(&root, &metadata, &selection.mode, &selection.base)?;
            let platform = platform.unwrap_or_else(catalog::host);
            let plan = catalog::plan(&metadata, &selected, &platform, all_platforms)?;
            if let Some(path) = github_output {
                let mut output = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .or_raise(|| message!("open GitHub output file {}", path.display()))?;
                writeln!(
                    output,
                    "matrix={}\nplan={}",
                    json!({"include": plan["jobs"]}),
                    execution_plan(&plan)
                )
                .or_raise(|| message("write GitHub job outputs"))?;
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&plan).or_raise(|| message("serialize CI plan"))?
            );
        }
        Action::Run {
            selection,
            plan: saved,
            from_env,
            job,
            stage,
            dry_run,
        } => {
            let saved: Option<Value> = if let Some(path) = saved {
                Some(
                    serde_json::from_slice(
                        &std::fs::read(&path).or_raise(|| message!("read plan {}", path.display()))?,
                    )
                    .or_raise(|| message("parse saved CI plan"))?,
                )
            } else if from_env {
                Some(
                    serde_json::from_str(&std::env::var("CI_PLAN").or_raise(|| message("read CI_PLAN"))?)
                        .or_raise(|| message("parse CI_PLAN"))?,
                )
            } else {
                None
            };
            let job = job.or_else(|| from_env.then(|| std::env::var("CI_JOB").ok()).flatten());
            let ci_job = job.is_some();
            let tasks = if let Some(job) = job {
                let saved = saved.as_ref().ok_or_raise(|| message("--job requires a saved plan"))?;
                catalog::job_tasks(&metadata, saved, &job)?
            } else {
                let selection = match &saved {
                    Some(plan) => plan["selection"].clone(),
                    None => plan::plan(&root, &metadata, &selection.mode, &selection.base)?,
                };
                eprintln!("CI selection: {selection}");
                catalog::select(catalog::tasks(&metadata, &selection, &catalog::host())?, &stage)?
            };
            eprintln!("{} tasks selected for {}", tasks.len(), catalog::host());
            catalog::execute(&root, tasks, dry_run, ci_job)?;
        }
        Action::Exec { .. } => unreachable!("exec was handled before workspace discovery"),
    }
    Ok(())
}

// Keep CI's environment payload below Windows' 32,767-character limit.
fn execution_plan(plan: &Value) -> Value {
    json!({"selection": {"mode": plan["selection"]["mode"], "packages": plan["selection"]["packages"]}, "jobs": plan["jobs"]})
}

#[cfg(test)]
mod tests {
    #[test]
    fn ci_payload_omits_descriptions_and_unbounded_reasons() {
        let plan = serde_json::json!({
            "selection": {"mode": "fast", "packages": ["gix"], "reasons": ["x".repeat(40_000)]},
            "tasks": ["command descriptions"], "jobs": [{"id": "tests@linux-x86_64"}]
        });
        assert_eq!(
            super::execution_plan(&plan),
            serde_json::json!({
                "selection": {"mode": "fast", "packages": ["gix"]}, "jobs": plan["jobs"]
            }),
            "job execution needs only selection and assignment, even for very large diffs"
        );
    }

    #[test]
    fn clap() {
        use clap::CommandFactory;
        super::Args::command().debug_assert();
    }
}
