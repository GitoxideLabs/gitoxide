use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use gix_error::{OptionExt, Result, ResultExt, bail, message};

use super::{
    ops::{Snapshot, SourceState, checked, git, output, source_snapshot, source_state},
    run,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fixtures {
    Archived,
    Generated,
}

fn fixture_modes(mode: &str) -> Result<&'static [Fixtures]> {
    match mode {
        "fast" => Ok(&[Fixtures::Generated]),
        "exhaustive" => Ok(&[Fixtures::Archived, Fixtures::Generated]),
        _ => bail!("Native test mode must be fast or exhaustive"),
    }
}

fn cargo(root: &Path) -> Result<Command> {
    run::process(root, &run::normalize(run::Command::new("cargo", &[])))
}

fn reuse_binaries(command: &mut Command, metadata: &Path, binaries: &Path, snapshot: &Path) {
    command
        .args(["nextest", "run", "--cargo-metadata"])
        .arg(metadata)
        .arg("--binaries-metadata")
        .arg(binaries)
        .arg("--workspace-remap")
        .arg(snapshot)
        .arg("--no-fail-fast");
}

fn test_command(
    root: &Path,
    metadata: &Path,
    binaries: &Path,
    snapshot: &Path,
    fixtures: Fixtures,
    windows_arm: bool,
) -> Result<Command> {
    let mut command = cargo(root)?;
    reuse_binaries(&mut command, metadata, binaries, snapshot);
    command
        .args(["--no-tests", "pass"])
        .env_remove("GIX_TEST_IGNORE_ARCHIVES")
        .env_remove("GIX_TEST_CREATE_ARCHIVES_EVEN_ON_CI");
    command.env(
        match fixtures {
            Fixtures::Archived => "GIX_TEST_CREATE_ARCHIVES_EVEN_ON_CI",
            Fixtures::Generated => "GIX_TEST_IGNORE_ARCHIVES",
        },
        "1",
    );
    command.args([
        "-E",
        if windows_arm {
            "not ((package(=internal-tools) & binary(=ci)) | test(performance) | test(speed))"
        } else {
            "not (package(=internal-tools) & binary(=ci))"
        },
    ]);
    Ok(command)
}

pub fn execute(root: &Path, args: &[String]) -> Result<()> {
    let (mode, args) = args
        .split_first()
        .ok_or_raise(|| message("native-tests needs a mode and Cargo selection arguments"))?;
    let modes = fixture_modes(mode)?;
    let before = source_state(root)?;
    let result = run(root, args, modes);
    if before != source_state(root)? {
        bail!("Native tests changed tracked or visible untracked source files; changes were retained for inspection");
    }
    result
}

fn run(root: &Path, args: &[String], modes: &[Fixtures]) -> Result<()> {
    let binaries = output(
        cargo(root)?
            .args([
                "nextest",
                "list",
                "--list-type",
                "binaries-only",
                "--message-format",
                "json",
                "--locked",
            ])
            .args(args)
            .stderr(Stdio::inherit()),
    )?;
    let mut snapshots = Vec::new();
    for _ in modes {
        snapshots.push(snapshot(root)?);
    }
    // Keep metadata beside the first snapshot's index, outside fixture status checks.
    let state = snapshots[0].0.path.join(".git");
    let metadata_path = save_metadata(root, &snapshots[0].0.path)?;
    let binaries_path = state.join("binaries-metadata.json");
    fs::write(&binaries_path, binaries).or_raise(|| message("Save nextest binaries for native tests"))?;
    let mut result = Ok(());
    for (index, fixtures) in modes.iter().enumerate() {
        let (snapshot, state) = &snapshots[index];
        eprintln!("Native tests with {fixtures:?} fixtures in {}", snapshot.path.display());
        let test_result = checked(&mut test_command(
            root,
            &metadata_path,
            &binaries_path,
            &snapshot.path,
            *fixtures,
            cfg!(all(windows, target_arch = "aarch64")),
        )?);
        if let Err(error) = unchanged(&snapshot.path, state) {
            eprintln!("Retained native test snapshot at {}", snapshot.path.display());
            std::mem::forget(snapshots.remove(index));
            return Err(error);
        }
        if test_result.is_err() {
            result = test_result;
        }
    }
    result
}

fn snapshot(root: &Path) -> Result<(Snapshot, SourceState)> {
    let snapshot = source_snapshot(root)?;
    checked(git(&snapshot.path).args(["init", "--quiet"]))?;
    checked(git(&snapshot.path).args(["add", "--force", "--all"]))?;
    let state = source_state(&snapshot.path)?;
    Ok((snapshot, state))
}

fn save_metadata(root: &Path, snapshot: &Path) -> Result<PathBuf> {
    let metadata = output(cargo(root)?.args(["metadata", "--no-deps", "--format-version", "1", "--locked"]))?;
    let path = snapshot.join(".git/cargo-metadata.json");
    fs::write(&path, metadata).or_raise(|| message("Save Cargo metadata for native tests"))?;
    Ok(path)
}

fn variant_arguments(mut command: run::Command) -> Result<(run::Command, Vec<String>)> {
    if command.program != "cargo" || !command.args.starts_with(&["nextest".into(), "run".into()]) {
        bail!("Native variants require cargo nextest run commands");
    }
    let mut build = [
        "nextest",
        "list",
        "--list-type",
        "binaries-only",
        "--message-format",
        "json",
    ]
    .map(String::from)
    .to_vec();
    let mut filters = Vec::new();
    let mut args = command.args.drain(2..);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--no-fail-fast" => {}
            "-E" | "--filter-expr" => {
                filters.push(arg);
                filters.push(
                    args.next()
                        .ok_or_raise(|| message("Missing nextest filter expression"))?,
                );
            }
            _ if arg.starts_with("--filter-expr=") => filters.push(arg),
            _ => build.push(arg),
        }
    }
    drop(args);
    command.args = build;
    Ok((run::normalize(command), filters))
}

fn each_variant(commands: Vec<run::Command>, mut execute: impl FnMut(run::Command) -> Result<()>) -> Result<()> {
    let mut failed = Vec::new();
    let count = commands.len();
    for (index, command) in commands.into_iter().enumerate() {
        eprintln!(
            "CI variant {}/{count}: {} {:?}",
            index + 1,
            command.program,
            command.args
        );
        if let Err(error) = execute(command) {
            eprintln!("{error:?}");
            failed.push((index + 1).to_string());
        }
    }
    if !failed.is_empty() {
        bail!("Failed CI variants: {}", failed.join(", "));
    }
    Ok(())
}

pub fn variants(root: &Path, commands: Vec<run::Command>, dry_run: bool) -> Result<()> {
    if commands.is_empty() {
        return Ok(());
    }
    if dry_run {
        eprintln!("Compile/list each variant in the checkout, then run it in one fresh shared source snapshot");
        return each_variant(commands, |command| {
            let (build, filters) = variant_arguments(command)?;
            run::execute(root, vec![build.clone()], true)?;
            let mut replay = build;
            replay.args = [
                "nextest",
                "run",
                "--cargo-metadata",
                "<snapshot>/.git/cargo-metadata.json",
                "--binaries-metadata",
                "<snapshot>/.git/binaries-metadata.json",
                "--workspace-remap",
                "<snapshot>",
                "--no-fail-fast",
            ]
            .map(String::from)
            .to_vec();
            replay.args.extend(filters);
            run::execute(root, vec![replay], true)
        });
    }
    let before = source_state(root)?;
    let result = (|| {
        let (snapshot, before) = snapshot(root)?;
        let metadata = save_metadata(root, &snapshot.path)?;
        let binaries = snapshot.path.join(".git/binaries-metadata.json");
        eprintln!("Native variants use generated fixtures in {}", snapshot.path.display());
        let result = each_variant(commands, |command| {
            let (mut build, filters) = variant_arguments(command)?;
            let listed = output(run::process(root, &build)?.stderr(Stdio::inherit()))?;
            fs::write(&binaries, listed).or_raise(|| message("Save nextest variant binaries"))?;
            build.args.clear();
            let mut command = run::process(root, &build)?;
            reuse_binaries(&mut command, &metadata, &binaries, &snapshot.path);
            checked(command.args(filters))
        });
        if let Err(error) = unchanged(&snapshot.path, &before) {
            eprintln!("Retained native variant snapshot at {}", snapshot.path.display());
            std::mem::forget(snapshot);
            return Err(error);
        }
        result
    })();
    if before != source_state(root)? {
        bail!(
            "Native variants changed tracked or visible untracked source files; changes were retained for inspection"
        );
    }
    result
}

fn unchanged(snapshot: &Path, before: &SourceState) -> Result<()> {
    if source_state(snapshot)? != *before {
        bail!(
            "Native tests changed source files in {}; inspect this retained snapshot for stale fixtures or baselines",
            snapshot.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, ffi::OsStr};

    use super::*;

    #[test]
    fn variant_builds_preserve_selection_and_replays_preserve_filters() -> gix_error::TestResult {
        let mut command = run::Command::new(
            "cargo",
            &[
                "nextest",
                "run",
                "--locked",
                "--no-fail-fast",
                "-p",
                "gix-transport",
                "--features",
                "async-client",
                "--test",
                "async-transport",
                "-E",
                "test(handshake)",
            ],
        );
        command.env.insert("GIX_TEST_FIXTURE_HASH".into(), "sha256".into());
        let (build, filters) = variant_arguments(command)?;
        assert_eq!(
            build.args,
            [
                "nextest",
                "list",
                "--list-type",
                "binaries-only",
                "--message-format",
                "json",
                "--locked",
                "-p",
                "gix-transport",
                "--features",
                "async-client",
                "--test",
                "async-transport",
            ],
            "list builds exactly the selected Cargo targets and features"
        );
        assert_eq!(
            filters,
            ["-E", "test(handshake)"],
            "test-name filters belong to the replay"
        );
        assert_eq!(
            build.env["GIX_TEST_FIXTURE_HASH"], "sha256",
            "variant fixture hashes survive normalization"
        );
        assert!(
            variant_arguments(run::Command::new("cargo", &["nextest", "run", "-E"])).is_err(),
            "an incomplete filter must not silently run different tests"
        );
        Ok(())
    }

    #[test]
    fn failing_variants_do_not_skip_independent_variants() -> gix_error::TestResult {
        let mut visited = Vec::new();
        let result = each_variant(
            ["first", "second", "third"]
                .map(|name| run::Command::new(name, &[]))
                .to_vec(),
            |command| {
                visited.push(command.program.clone());
                if command.program != "second" {
                    bail!("Test variant failed");
                }
                Ok(())
            },
        );
        assert_eq!(
            visited,
            ["first", "second", "third"],
            "all independent variants execute after failures"
        );
        assert!(
            result
                .expect_err("the failed variants must fail the task")
                .to_string()
                .contains("1, 3"),
            "the final result identifies every failed variant"
        );
        Ok(())
    }

    #[test]
    fn modes_reuse_binaries_with_separate_fixture_environments() -> gix_error::TestResult {
        assert_eq!(fixture_modes("fast")?, [Fixtures::Generated]);
        assert_eq!(fixture_modes("exhaustive")?, [Fixtures::Archived, Fixtures::Generated]);
        assert!(
            fixture_modes("unknown").is_err(),
            "unsupported modes must not silently skip tests"
        );
        for fixtures in fixture_modes("exhaustive")? {
            let command = test_command(
                Path::new("source"),
                Path::new("cargo.json"),
                Path::new("binaries.json"),
                Path::new("snapshot"),
                *fixtures,
                false,
            )?;
            assert_eq!(
                command.get_args().collect::<Vec<_>>(),
                [
                    "nextest",
                    "run",
                    "--cargo-metadata",
                    "cargo.json",
                    "--binaries-metadata",
                    "binaries.json",
                    "--workspace-remap",
                    "snapshot",
                    "--no-fail-fast",
                    "--no-tests",
                    "pass",
                    "-E",
                    "not (package(=internal-tools) & binary(=ci))",
                ]
                .map(OsStr::new),
                "both fixture modes run the exact saved binaries without rebuilding"
            );
            let env: BTreeMap<_, _> = command.get_envs().collect();
            for (name, value) in [
                ("CI", Some("true")),
                ("INSTA_UPDATE", Some("no")),
                ("INSTA_FORCE_PASS", Some("0")),
                ("GIX_TEST_FIXTURE_HASH", None),
                ("GIX_MERGE_UPDATE_CARTESIAN_BASELINE", None),
                ("GIX_MERGE_UPDATE_BASELINE", None),
                (
                    "GIX_TEST_IGNORE_ARCHIVES",
                    (*fixtures == Fixtures::Generated).then_some("1"),
                ),
                (
                    "GIX_TEST_CREATE_ARCHIVES_EVEN_ON_CI",
                    (*fixtures == Fixtures::Archived).then_some("1"),
                ),
            ] {
                assert_eq!(
                    env.get(OsStr::new(name)),
                    Some(&value.map(OsStr::new)),
                    "{name} must be deterministic"
                );
            }
        }
        let command = test_command(
            Path::new("source"),
            Path::new("cargo.json"),
            Path::new("binaries.json"),
            Path::new("snapshot"),
            Fixtures::Generated,
            true,
        )?;
        assert!(
            command.get_args().collect::<Vec<_>>().ends_with(
                &[
                    "-E",
                    "not ((package(=internal-tools) & binary(=ci)) | test(performance) | test(speed))",
                ]
                .map(OsStr::new)
            ),
            "Windows ARM retains its performance-test exclusions"
        );
        Ok(())
    }
}
