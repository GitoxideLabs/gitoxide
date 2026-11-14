use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use gix_error::{OptionExt, Result, ResultExt, bail, message};

use super::{
    ops::{Snapshot, SourceState, checked, empty_snapshot, git, output, source_snapshot, source_state},
    run,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fixtures {
    Archived,
    Generated,
}

#[derive(Default)]
pub(super) struct NativeSnapshots {
    sources: Vec<(Snapshot, SourceState)>,
    // Only generated fixtures use this cache; extracted archives stay in their source snapshot.
    cache: Option<Snapshot>,
}

impl NativeSnapshots {
    fn add(&mut self, root: &Path, count: usize) -> Result<usize> {
        let first = self.sources.len();
        for _ in 0..count {
            let source = snapshot(root)?;
            if self.sources.first().is_some_and(|(_, before)| *before != source.1) {
                self.retain();
                bail!("Source files changed between native test stages; refusing to reuse generated fixtures");
            }
            self.sources.push(source);
        }
        if self.cache.is_none() {
            self.cache = Some(empty_snapshot()?);
        }
        Ok(first)
    }

    fn check(&mut self) -> Result<()> {
        for (snapshot, before) in &self.sources {
            if let Err(error) = unchanged(&snapshot.path, before) {
                self.retain();
                return Err(error);
            }
        }
        Ok(())
    }

    fn retain(&mut self) {
        for (snapshot, _) in &self.sources {
            eprintln!("Retained native test snapshot at {}", snapshot.path.display());
        }
        if let Some(cache) = &self.cache {
            eprintln!("Retained native fixture cache at {}", cache.path.display());
        }
        std::mem::forget(std::mem::take(self));
    }
}

fn fixture_modes(mode: &str, windows: bool) -> Result<&'static [Fixtures]> {
    match mode {
        "fast" if windows => Ok(&[Fixtures::Archived]),
        "fast" => Ok(&[Fixtures::Generated]),
        "exhaustive" => Ok(&[Fixtures::Archived, Fixtures::Generated]),
        _ => bail!("Native test mode must be fast or exhaustive"),
    }
}

fn cargo(root: &Path) -> Result<Command> {
    run::process(root, &run::normalize(run::Command::new("cargo", &[])))
}

fn reuse_binaries(command: &mut Command, metadata: &Path, binaries: &Path, snapshot: &Path, cache: &Path) {
    command
        .args(["nextest", "run", "--cargo-metadata"])
        .arg(metadata)
        .arg("--binaries-metadata")
        .arg(binaries)
        .arg("--workspace-remap")
        .arg(snapshot)
        .arg("--no-fail-fast")
        .env("GIX_TEST_FIXTURE_CACHE", cache)
        .env("GIX_TEST_FIXTURE_SOURCE_ROOT", snapshot);
}

fn test_command(
    root: &Path,
    metadata: &Path,
    binaries: &Path,
    snapshot: &Path,
    cache: &Path,
    fixtures: Fixtures,
    windows_arm: bool,
) -> Result<Command> {
    let mut command = cargo(root)?;
    reuse_binaries(&mut command, metadata, binaries, snapshot, cache);
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
    // Workspace unification can enable notify, but gix/Cargo.toml runs these tests once in a dedicated variant.
    let mut excluded = String::from(
        "(package(=internal-tools) & binary(=qa)) | (package(=gix) & (test(notify::) | test(=repository::worktree::linked_worktree_removal_notifies_through_shared_metadata)))",
    );
    if windows_arm {
        excluded.push_str(" | test(performance) | test(speed)");
    }
    command.args(["-E", &format!("not ({excluded})")]);
    Ok(command)
}

pub fn execute(
    root: &Path,
    args: &[String],
    snapshots: &mut NativeSnapshots,
    log: Option<&File>,
    coverage: &mut Option<super::coverage::Run>,
) -> Result<()> {
    let (mode, args) = args
        .split_first()
        .ok_or_raise(|| message("native-tests needs a mode and Cargo selection arguments"))?;
    let modes = fixture_modes(mode, cfg!(windows))?;
    let before = source_state(root)?;
    let result = run(root, args, modes, snapshots, log, coverage);
    if before != source_state(root)? {
        bail!("Native tests changed tracked or visible untracked source files; changes were retained for inspection");
    }
    result
}

fn run(
    root: &Path,
    args: &[String],
    modes: &[Fixtures],
    snapshots: &mut NativeSnapshots,
    log: Option<&File>,
    coverage: &mut Option<super::coverage::Run>,
) -> Result<()> {
    let mut build = cargo(root)?;
    build
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
        .stderr(Stdio::inherit());
    run::redirect(&mut build, log, false)?;
    let (binaries, instrumented) = super::coverage::build(&mut build, coverage.as_ref(), log)?;
    let first = snapshots.add(root, modes.len())?;
    // Keep metadata beside the first snapshot's index, outside fixture status checks.
    let state = snapshots.sources[first].0.path.join(".git");
    let metadata_path = save_metadata(root, &snapshots.sources[first].0.path)?;
    let binaries_path = state.join("binaries-metadata.json");
    fs::write(&binaries_path, binaries).or_raise(|| message("Save nextest binaries for native tests"))?;
    let mut result = Ok(());
    for (index, fixtures) in modes.iter().enumerate() {
        let index = first + index;
        let (snapshot, _) = &snapshots.sources[index];
        eprintln!("Native tests with {fixtures:?} fixtures in {}", snapshot.path.display());
        let mut command = test_command(
            root,
            &metadata_path,
            &binaries_path,
            &snapshot.path,
            &snapshots
                .cache
                .as_ref()
                .expect("adding a snapshot creates the cache")
                .path,
            *fixtures,
            cfg!(all(windows, target_arch = "aarch64")),
        )?;
        let configuration = coverage
            .as_ref()
            .map(|coverage| coverage.configuration(&build, &command));
        let test_result =
            super::coverage::execute(root, configuration, &mut command, instrumented, coverage.as_mut(), log);
        snapshots.check()?;
        if test_result.is_err() {
            result = test_result;
        }
    }
    result
}

fn snapshot(root: &Path) -> Result<(Snapshot, SourceState)> {
    let snapshot = source_snapshot(root)?;
    checked(git(&snapshot.path)?.args(["init", "--quiet"]))?;
    // Fixtures need tracked paths, while source_state hashes the actual files without storing Git blobs.
    checked(git(&snapshot.path)?.args(["add", "--intent-to-add", "--force", "--all"]))?;
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

pub fn variants(
    root: &Path,
    commands: Vec<run::Command>,
    dry_run: bool,
    snapshots: &mut NativeSnapshots,
    log: Option<&File>,
    coverage: &mut Option<super::coverage::Run>,
) -> Result<()> {
    if commands.is_empty() {
        return Ok(());
    }
    if dry_run {
        eprintln!("Compile/list each variant in the checkout, then run it in one fresh shared source snapshot");
        return run::each_case(commands, log, |command| {
            let (build, filters) = variant_arguments(command)?;
            run::execute(root, vec![build.clone()], true, log)?;
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
            run::execute(root, vec![replay], true, log)
        });
    }
    let before = source_state(root)?;
    let result = (|| {
        let index = snapshots.add(root, 1)?;
        let (snapshot, _) = &snapshots.sources[index];
        let metadata = save_metadata(root, &snapshot.path)?;
        let binaries = snapshot.path.join(".git/binaries-metadata.json");
        eprintln!("Native variants use generated fixtures in {}", snapshot.path.display());
        let result = run::each_case(commands, log, |command| {
            let (mut build, filters) = variant_arguments(command)?;
            let mut compilation = run::process(root, &build)?;
            compilation.stderr(Stdio::inherit());
            run::redirect(&mut compilation, log, false)?;
            let (listed, instrumented) = super::coverage::build(&mut compilation, coverage.as_ref(), log)?;
            fs::write(&binaries, listed).or_raise(|| message("Save nextest variant binaries"))?;
            build.args.clear();
            let mut command = run::process(root, &build)?;
            reuse_binaries(
                &mut command,
                &metadata,
                &binaries,
                &snapshot.path,
                &snapshots
                    .cache
                    .as_ref()
                    .expect("adding a snapshot creates the cache")
                    .path,
            );
            command.args(filters);
            let configuration = coverage
                .as_ref()
                .map(|coverage| coverage.configuration(&compilation, &command));
            super::coverage::execute(root, configuration, &mut command, instrumented, coverage.as_mut(), log)
        });
        snapshots.check()?;
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
    fn native_and_variant_replays_share_the_generated_fixture_cache() -> gix_error::TestResult {
        let mut variant = cargo(Path::new("source"))?;
        variant.env("GIX_TEST_FIXTURE_HASH", "sha256");
        reuse_binaries(
            &mut variant,
            Path::new("cargo.json"),
            Path::new("binaries.json"),
            Path::new("variant-snapshot"),
            Path::new("fixture-cache"),
        );
        let env: BTreeMap<_, _> = variant.get_envs().collect();
        assert_eq!(
            env.get(OsStr::new("GIX_TEST_FIXTURE_CACHE")),
            Some(&Some(OsStr::new("fixture-cache"))),
            "variant replays must use the run's generated fixture cache"
        );
        assert_eq!(
            env.get(OsStr::new("GIX_TEST_FIXTURE_SOURCE_ROOT")),
            Some(&Some(OsStr::new("variant-snapshot"))),
            "each replay identifies its own source snapshot"
        );
        assert_eq!(
            env.get(OsStr::new("GIX_TEST_FIXTURE_HASH")),
            Some(&Some(OsStr::new("sha256"))),
            "sharing preserves the variant's fixture identity"
        );
        let native = test_command(
            Path::new("source"),
            Path::new("cargo.json"),
            Path::new("binaries.json"),
            Path::new("native-snapshot"),
            Path::new("fixture-cache"),
            Fixtures::Generated,
            false,
        )?;
        let native_env: BTreeMap<_, _> = native.get_envs().collect();
        assert_eq!(
            native_env.get(OsStr::new("GIX_TEST_FIXTURE_CACHE")),
            env.get(OsStr::new("GIX_TEST_FIXTURE_CACHE")),
            "native and variant replays share one cache despite different source snapshots"
        );
        Ok(())
    }

    #[test]
    fn native_cache_is_fresh_and_retained_with_changed_snapshots() -> gix_error::TestResult {
        let source = empty_snapshot()?;
        checked(git(&source.path)?.args(["init", "--quiet"]))?;
        fs::write(source.path.join("input"), "original")?;
        checked(git(&source.path)?.args(["add", "input"]))?;
        let mut previous_cache = PathBuf::new();
        for changed in [false, true] {
            let mut snapshots = NativeSnapshots::default();
            assert!(snapshots.cache.is_none(), "tasks without tests create no fixture cache");
            assert_eq!(snapshots.add(&source.path, 1)?, 0);
            let cache = snapshots
                .cache
                .as_ref()
                .expect("even fast mode prepares generated fixtures for later tasks")
                .path
                .clone();
            fs::write(cache.join("generated"), "fixture")?;
            snapshots.check()?;
            assert_eq!(snapshots.add(&source.path, 1)?, 1);
            assert_eq!(
                snapshots
                    .cache
                    .as_ref()
                    .expect("the cache survives between stages")
                    .path,
                cache,
                "later native stages reuse the same cache"
            );
            let sources: Vec<_> = snapshots
                .sources
                .iter()
                .map(|(snapshot, _)| snapshot.path.clone())
                .collect();
            assert!(
                cache.is_absolute() && sources.iter().all(|path| path.is_absolute()),
                "cache and source roots must resolve identically from every test working directory"
            );
            assert_ne!(cache, previous_cache, "each invocation owns a new cache");
            assert!(
                fs::read_dir(&cache)?.count() == 1,
                "only this run can populate the cache"
            );
            assert_ne!(
                sources[0], sources[1],
                "different stages keep separate source snapshots alive"
            );
            assert_eq!(
                fs::read(cache.join("generated"))?,
                b"fixture",
                "the cache survives the first replay"
            );
            if changed {
                fs::write(sources[0].join("input"), "changed through a cached fixture")?;
            }
            let result = snapshots.check();
            if changed {
                assert!(
                    result.is_err(),
                    "later stages cannot change even an earlier source snapshot through a shared fixture"
                );
                assert!(
                    cache.is_dir() && sources.iter().all(|path| path.is_dir()),
                    "inspection retains the cache and every source snapshot it may reference"
                );
                assert_eq!(
                    fs::read(source.path.join("input"))?,
                    b"original",
                    "the checkout stays isolated"
                );
                for path in sources.into_iter().chain([cache.clone()]) {
                    drop(Snapshot { path });
                }
            } else {
                result?;
                drop(snapshots);
                assert!(
                    !cache.exists() && sources.iter().all(|path| !path.exists()),
                    "completion removes both source snapshots and the shared cache"
                );
            }
            previous_cache = cache;
        }
        Ok(())
    }

    #[test]
    fn source_changes_between_stages_cannot_reuse_fixtures() -> gix_error::TestResult {
        let source = empty_snapshot()?;
        checked(git(&source.path)?.args(["init", "--quiet"]))?;
        fs::write(source.path.join("input"), "original")?;
        checked(git(&source.path)?.args(["add", "input"]))?;
        let mut snapshots = NativeSnapshots::default();
        snapshots.add(&source.path, 1)?;
        let copy = snapshots.sources[0].0.path.clone();
        let cache = snapshots
            .cache
            .as_ref()
            .expect("the first stage creates a cache")
            .path
            .clone();
        fs::write(source.path.join("input"), "changed between stages")?;
        assert!(
            snapshots.add(&source.path, 1).is_err(),
            "cache identities cannot account for changes to arbitrary fixture helper inputs"
        );
        assert!(
            copy.is_dir() && cache.is_dir(),
            "prior snapshots and cache survive for inspection"
        );
        assert!(
            snapshots.sources.is_empty() && snapshots.cache.is_none(),
            "retained caches cannot be reused"
        );
        for path in [copy, cache] {
            drop(Snapshot { path });
        }
        Ok(())
    }

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
        let result = run::each_case(
            ["first", "second", "third"]
                .map(|name| run::Command::new(name, &[]))
                .to_vec(),
            None,
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
        assert_eq!(fixture_modes("fast", true)?, [Fixtures::Archived]);
        assert_eq!(fixture_modes("fast", false)?, [Fixtures::Generated]);
        for windows in [false, true] {
            assert_eq!(
                fixture_modes("exhaustive", windows)?,
                [Fixtures::Archived, Fixtures::Generated],
                "exhaustive QA still regenerates fixtures on every platform"
            );
            assert!(
                fixture_modes("unknown", windows).is_err(),
                "unsupported modes must not silently skip tests"
            );
        }
        for fixtures in fixture_modes("exhaustive", true)? {
            let command = test_command(
                Path::new("source"),
                Path::new("cargo.json"),
                Path::new("binaries.json"),
                Path::new("snapshot"),
                Path::new("fixture-cache"),
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
                    "not ((package(=internal-tools) & binary(=qa)) | (package(=gix) & (test(notify::) | test(=repository::worktree::linked_worktree_removal_notifies_through_shared_metadata))))",
                ]
                .map(OsStr::new),
                "both fixture modes reuse saved binaries without repeating self-tests or the dedicated notify variant"
            );
            let env: BTreeMap<_, _> = command.get_envs().collect();
            for (name, value) in [
                ("CI", Some("true")),
                ("INSTA_UPDATE", Some("no")),
                ("INSTA_FORCE_PASS", Some("0")),
                ("GIX_TEST_FIXTURE_HASH", None),
                ("QA_PLAN", None),
                ("QA_JOB", None),
                ("GIX_TEST_FIXTURE_CACHE", Some("fixture-cache")),
                ("GIX_TEST_FIXTURE_SOURCE_ROOT", Some("snapshot")),
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
            Path::new("fixture-cache"),
            Fixtures::Generated,
            true,
        )?;
        assert!(
            command.get_args().collect::<Vec<_>>().ends_with(
                &[
                    "-E",
                    "not ((package(=internal-tools) & binary(=qa)) | (package(=gix) & (test(notify::) | test(=repository::worktree::linked_worktree_removal_notifies_through_shared_metadata))) | test(performance) | test(speed))",
                ]
                .map(OsStr::new)
            ),
            "Windows ARM retains performance-test exclusions without repeating the dedicated notify variant"
        );
        let env: BTreeMap<_, _> = command.get_envs().collect();
        for (name, value) in [
            ("GIX_TEST_FIXTURE_CACHE", "fixture-cache"),
            ("GIX_TEST_FIXTURE_SOURCE_ROOT", "snapshot"),
        ] {
            assert_eq!(
                env.get(OsStr::new(name)),
                Some(&Some(OsStr::new(value))),
                "fast mode uses the run-owned cache and source snapshot"
            );
        }
        Ok(())
    }
}
