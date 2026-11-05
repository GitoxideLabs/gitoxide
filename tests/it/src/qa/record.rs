use std::{fmt::Write, io, path::Path, process::Command, time::Duration};

use gix_error::{Result, ResultExt, bail, message};
use serde_json::Value;

use super::{Action, catalog::TaskTiming, coverage, ops};

pub fn eligible(action: &Action) -> bool {
    matches!(action, Action::Run { plan: None, from_env: false, job: None, stage, dry_run: false, .. }
        if stage.iter().any(|stage| stage == "all"))
}

pub struct Checkout {
    commit_id: String,
    clean: bool,
}

pub struct Timings {
    pub elapsed: Duration,
    pub planning: Duration,
    pub tasks: Vec<TaskTiming>,
}

impl Checkout {
    pub fn capture(root: &Path) -> Result<Self> {
        let commit_id = ops::output(ops::git(root)?.args(["rev-parse", "--verify", "HEAD"]))?;
        let clean =
            ops::output(ops::git(root)?.args(["status", "--porcelain=v1", "-z", "--untracked-files=all"]))?.is_empty();
        Ok(Self {
            commit_id: String::from_utf8(commit_id)
                .or_raise(|| message("Read QA commit ID"))?
                .trim()
                .to_owned(),
            clean,
        })
    }

    fn unchanged(&self, after: &Self) -> bool {
        self.clean && after.clean && self.commit_id == after.commit_id
    }
}

pub fn finish(
    root: &Path,
    before: Checkout,
    selection: &Value,
    requested_base: &str,
    timings: &Timings,
    coverage: Option<&coverage::Summary>,
) -> Result<()> {
    if timings.tasks.iter().any(|task| !task.passed) {
        eprintln!("QA checks-pass was not recorded: nonblocking tasks failed.");
        return Ok(());
    }
    if !before.unchanged(&Checkout::capture(root)?) {
        eprintln!("QA checks-pass was not recorded: the checkout was dirty or HEAD changed during the run.");
        return Ok(());
    }
    let description = report(selection, requested_base, &super::catalog::host(), timings, coverage);
    let result = command(root, &before.commit_id, &description).status();
    let status = match result {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            eprintln!("QA passed; checks-pass was not recorded because tix is not installed.");
            return Ok(());
        }
        result => result.or_raise(|| message("QA passed, but Tix could not record checks-pass"))?,
    };
    if !status.success() {
        bail!("QA passed, but Tix failed to record checks-pass with {status}");
    }
    Ok(())
}

fn command(root: &Path, commit_id: &str, message: &str) -> Command {
    let mut command = Command::new("tix");
    command
        .current_dir(root)
        .args(["enrich", "tree", "checks-pass", message, "--revision", commit_id]);
    for name in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    ] {
        command.env_remove(name);
    }
    command
}

fn report(
    selection: &Value,
    requested_base: &str,
    host: &str,
    timings: &Timings,
    coverage: Option<&coverage::Summary>,
) -> String {
    let title = if selection["mode"] == "exhaustive" {
        "🔬 Thorough QA"
    } else if coverage.is_some_and(|coverage| coverage.omitted > 0) {
        "⚡ Fast QA (coverage selection)"
    } else {
        "⚡ Fast QA"
    };
    let mut message = format!(
        "{title} passed in {}\n\nHost: {host}\nMode: {}\nRequested base: {requested_base}\nComparison base: {}\nPackages: {}\n",
        duration(timings.elapsed),
        selection["mode"].as_str().unwrap_or_default(),
        selection["base"].as_str().unwrap_or("not used (exhaustive selection)"),
        selection["packages"].as_array().map_or(0, Vec::len),
    );
    for reason in selection["reasons"].as_array().into_iter().flatten() {
        if let Some(reason) = reason.as_str() {
            writeln!(message, "Selection: {reason}").expect("writing to a string cannot fail");
        }
    }
    if let Some(coverage) = coverage {
        writeln!(
            message,
            "Coverage: {:?}\nTest executions ({}/{} configurations counted): {} run, {} omitted\nCoverage configurations: {} reused, {} collected, {} fallback",
            coverage.mode,
            coverage.measured_cases,
            coverage.cases,
            coverage.executed,
            coverage.omitted,
            coverage.reused,
            coverage.collected,
            coverage.fallback,
        )
        .expect("writing to a string cannot fail");
    } else {
        message.push_str("Coverage: inactive (no tests omitted by coverage)\n");
    }
    writeln!(
        message,
        "\nPackage/configuration planning: {:.3}s (before tasks)",
        timings.planning.as_secs_f64(),
    )
    .expect("writing to a string cannot fail");
    message.push_str("\nTask timings (including setup):\n");
    for task in &timings.tasks {
        writeln!(
            message,
            "- {}: {} — {}{}",
            task.id,
            duration(task.elapsed),
            if task.passed { "passed" } else { "failed" },
            if task.blocking { "" } else { " (nonblocking)" },
        )
        .expect("writing to a string cannot fail");
    }
    if let Some(coverage) = coverage {
        writeln!(
            message,
            "\nCoverage timing breakdown (included in task timings):\n- Test inventory: {:.3}s\n- Per-test selection: {:.3}s\n- Index analysis: {:.3}s",
            coverage.inventory_time.as_secs_f64(),
            coverage.selection_time.as_secs_f64(),
            coverage.analysis_time.as_secs_f64(),
        )
        .expect("writing to a string cannot fail");
    }
    message
}

fn duration(elapsed: Duration) -> String {
    let seconds = elapsed.as_secs();
    if seconds >= 3600 {
        format!("{}h {}m {}s", seconds / 3600, seconds / 60 % 60, seconds % 60)
    } else if seconds >= 60 {
        format!("{}m {}s", seconds / 60, seconds % 60)
    } else {
        format!("{:.1}s", elapsed.as_secs_f64())
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use serde_json::json;

    use super::*;

    #[test]
    fn only_complete_local_profiles_are_recorded() -> gix_error::TestResult {
        for (args, expected) in [
            (vec!["qa", "run"], true),
            (vec!["qa", "run", "--mode", "exhaustive"], true),
            (vec!["qa", "run", "--stage", "tests,all"], true),
            (vec!["qa", "run", "--stage", "tests,doctests"], false),
            (vec!["qa", "run", "--dry-run"], false),
            (vec!["qa", "run", "--plan", "plan.json"], false),
            (vec!["qa", "run", "--from-env"], false),
            (vec!["qa", "run", "--job", "tests"], false),
            (vec!["qa", "plan"], false),
        ] {
            assert_eq!(
                eligible(&super::super::Args::try_parse_from(&args)?.command),
                expected,
                "{args:?} must not certify partial or stale coverage"
            );
        }
        Ok(())
    }

    #[test]
    fn recording_requires_the_same_clean_commit_at_both_ends() -> gix_error::TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let root = gix_testtools::scripted_fixture_read_only("make_qa_recording_repositories.sh")?;
        let clean = Checkout::capture(&root.join("clean"))?;
        assert!(clean.unchanged(&Checkout::capture(&root.join("clean"))?));
        for state in ["untracked", "unstaged", "staged", "moved"] {
            let changed = Checkout::capture(&root.join(state))?;
            assert_eq!(changed.clean, state == "moved", "moving HEAD can leave a clean tree");
            assert!(
                !clean.unchanged(&changed) && !changed.unchanged(&clean),
                "{state} at either end prevents recording, even when the other end is clean"
            );
        }
        assert!(
            Checkout::capture(&root.join("unborn")).is_err(),
            "a checkout without a commit has no tree to certify"
        );
        Ok(())
    }

    #[test]
    fn reports_actual_coverage_and_preserves_message_as_one_argument() {
        let timings = Timings {
            elapsed: Duration::from_secs(3672),
            planning: Duration::from_millis(12),
            tasks: vec![TaskTiming {
                id: "advisories",
                elapsed: Duration::from_millis(1250),
                blocking: false,
                passed: true,
            }],
        };
        let selection = json!({
            "mode": "exhaustive", "packages": ["gix"],
            "reasons": ["Cannot compare with HEAD^; fetch the base ref to enable fast selection"]
        });
        let message = report(&selection, "HEAD^", "test-host", &timings, None);
        assert_eq!(
            message,
            "🔬 Thorough QA passed in 1h 1m 12s

Host: test-host
Mode: exhaustive
Requested base: HEAD^
Comparison base: not used (exhaustive selection)
Packages: 1
Selection: Cannot compare with HEAD^; fetch the base ref to enable fast selection
Coverage: inactive (no tests omitted by coverage)

Package/configuration planning: 0.012s (before tasks)

Task timings (including setup):
- advisories: 1.2s — passed (nonblocking)
",
            "the effective mode, coverage and task timing remain reviewable"
        );
        let command = command(Path::new("checkout"), "captured-commit", &message);
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            [
                "enrich",
                "tree",
                "checks-pass",
                &message,
                "--revision",
                "captured-commit"
            ],
            "the full message and original commit are passed without shell interpretation"
        );
        assert!(command.get_envs().all(|(_, value)| value.is_none()));
        let fast = report(
            &json!({"mode": "fast", "base": "resolved-base", "packages": []}),
            "HEAD^",
            "test-host",
            &Timings {
                elapsed: Duration::from_secs(72),
                planning: Duration::ZERO,
                tasks: Vec::new(),
            },
            None,
        );
        assert!(fast.starts_with("⚡ Fast QA passed in 1m 12s\n\n"));
        assert!(fast.contains("Comparison base: resolved-base"));
    }

    #[test]
    fn coverage_results_report_actual_omissions_and_selection_cost() {
        let mut coverage = coverage::Summary::new(coverage::Mode::Auto);
        coverage.cases = 4;
        coverage.measured_cases = 3;
        coverage.executed = 10;
        coverage.omitted = 20;
        coverage.reused = 2;
        coverage.collected = 1;
        coverage.fallback = 1;
        coverage.inventory_time = Duration::from_millis(5);
        coverage.selection_time = Duration::from_millis(3);
        coverage.analysis_time = Duration::from_millis(7);
        let timings = Timings {
            elapsed: Duration::from_secs(72),
            planning: Duration::from_millis(12),
            tasks: Vec::new(),
        };
        let selection = json!({"mode": "fast", "base": "resolved-base", "packages": []});
        let message = report(&selection, "HEAD^", "test-host", &timings, Some(&coverage));
        assert!(
            message.starts_with("⚡ Fast QA (coverage selection) passed in 1m 12s"),
            "actual test omissions identify the coverage selection profile"
        );
        assert!(
            message.contains("10 run, 20 omitted"),
            "counts describe the actual test executions"
        );
        assert!(
            message.contains("3/4 configurations counted"),
            "unknown inventories do not invent test totals"
        );
        assert!(message.contains("2 reused, 1 collected, 1 fallback"));
        assert!(message.contains("Package/configuration planning: 0.012s (before tasks)"));
        assert!(message.contains("Coverage timing breakdown (included in task timings):"));
        assert!(message.contains("Test inventory: 0.005s"));
        assert!(message.contains("Per-test selection: 0.003s"));
        assert!(message.contains("Index analysis: 0.007s"));

        coverage.omitted = 0;
        for mode in [coverage::Mode::Auto, coverage::Mode::Shadow, coverage::Mode::Collect] {
            coverage.mode = mode;
            let message = report(&selection, "HEAD^", "test-host", &timings, Some(&coverage));
            assert!(
                message.starts_with("⚡ Fast QA passed"),
                "the title identifies selection only when tests were omitted"
            );
        }
    }
}
