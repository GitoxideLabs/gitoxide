use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

use gix_testtools::TestResult;

const SCRIPT: &str = r#"#!/usr/bin/env bash
set -eu -o pipefail
printf 'run\n' >> "$FIXTURE_CACHE_RUNS"
printf 'generated\n' > origin
pwd -W 2>/dev/null > generation-path || pwd > generation-path
printf '%s\n' "$@" > arguments
if test -n "${FIXTURE_CACHE_FAIL:-}"; then exit 1; fi
"#;

#[cfg(windows)]
#[test]
fn short_cache_paths_match_paths_recorded_by_git_and_the_shell() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let mut fixture = Cache::new()?;
    fixture.cache = fixture.directory.path().join("long-cache-directory");
    std::fs::create_dir(&fixture.cache)?;
    let root = fixture.snapshot("source", "crate", SCRIPT)?;
    let output = Command::new(gix_testtools::bash_program())
        .args(["-c", "cygpath -d \"$1\"", "short-cache-path"])
        .arg(&fixture.cache)
        .output()?;
    assert!(output.status.success(), "Git Bash can resolve the cache's short path");
    let short_path = PathBuf::from(std::str::from_utf8(&output.stdout)?.trim());
    assert_ne!(short_path, fixture.cache, "the regression exercises a short path alias");
    let mut request = fixture.invocation(&root, "crate", true)?;
    request.command.env("GIX_TEST_FIXTURE_CACHE", short_path);
    let generated = request.run()?;
    assert_eq!(
        generated,
        fixture.invocation(&root, "crate", true)?.run()?,
        "short and long cache paths share the same fixture"
    );
    let mut request = fixture.invocation(&root, "crate", true)?;
    request
        .command
        .env("GIX_TEST_FIXTURE_CACHE", fixture.cache.canonicalize()?);
    assert_eq!(
        request.run()?,
        generated,
        "verbatim cache paths reuse an ordinary fixture path"
    );
    assert_eq!(fixture.runs()?, 1, "path aliases do not regenerate fixtures");
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlinked_cache_roots_return_physical_fixture_paths() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = Cache::new()?;
    let root = fixture.snapshot("source", "crate", SCRIPT)?;
    let alias = fixture.directory.path().join("cache-alias");
    std::os::unix::fs::symlink(&fixture.cache, &alias)?;
    let mut request = fixture.invocation(&root, "crate", true)?;
    request.command.env("GIX_TEST_FIXTURE_CACHE", &alias);
    let path = request.run()?;
    assert_eq!(
        path,
        path.canonicalize()?,
        "fixture paths match the physical paths recorded by Git and the shell"
    );
    assert_eq!(fixture.invocation(&root, "crate", true)?.run()?, path);
    assert_eq!(fixture.runs()?, 1, "cache aliases reuse the same generated fixture");
    Ok(())
}

#[test]
fn missing_archive_generation_is_reused_across_snapshots_and_modes() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = Cache::new()?;
    let first = fixture.snapshot("archived", "crate", SCRIPT)?;
    let second = fixture.snapshot("generated", "crate", SCRIPT)?;
    let archived = fixture.invocation(&first, "crate", false)?.run()?;
    let generated = fixture.invocation(&second, "crate", true)?.run()?;
    assert_eq!(
        archived, generated,
        "a missing archive generates one stable fixture shared by later generated-mode snapshots"
    );
    assert_eq!(
        generated
            .parent()
            .and_then(Path::parent)
            .map(Path::canonicalize)
            .transpose()?,
        Some(fixture.cache.canonicalize()?),
        "hash-keyed fixture entries live directly under the job cache root"
    );
    assert_eq!(fixture.runs()?, 1, "equivalent snapshots execute the script only once");
    assert_eq!(std::fs::read(generated.join("origin"))?, b"generated\n");
    let generation_path = PathBuf::from(std::fs::read_to_string(generated.join("generation-path"))?.trim());
    assert_eq!(
        generation_path.canonicalize()?,
        generated.canonicalize()?,
        "absolute paths recorded by the script remain valid after publication"
    );
    let mut entries = std::fs::read_dir(&generated)?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();
    assert_eq!(
        entries,
        ["arguments", "generation-path", "origin"],
        "cache metadata stays outside the fixture payload"
    );
    Ok(())
}

#[test]
fn archives_remain_local_and_never_supply_generated_mode() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = Cache::new()?;
    let first = fixture.snapshot("first", "crate", SCRIPT)?;
    let second = fixture.snapshot("second", "crate", SCRIPT)?;
    fixture.archive(&first, 0)?;
    fixture.archive(&second, 0)?;
    let archived = fixture.invocation(&first, "crate", false)?.run()?;
    assert!(
        archived.starts_with(&first),
        "archive extraction stays in its own snapshot"
    );
    assert_eq!(std::fs::read(archived.join("origin"))?, b"archived\n");
    assert_eq!(fixture.runs()?, 0, "a valid archive does not execute its script");

    let generated = fixture.invocation(&first, "crate", true)?.run()?;
    assert_ne!(
        archived, generated,
        "generated mode never reuses archive-derived output"
    );
    assert!(
        generated.canonicalize()?.starts_with(fixture.cache.canonicalize()?),
        "generated fixtures live in the shared cache even when its path uses a short alias"
    );
    assert_eq!(std::fs::read(generated.join("origin"))?, b"generated\n");
    assert_eq!(fixture.invocation(&second, "crate", true)?.run()?, generated);
    let second_archive = fixture.invocation(&second, "crate", false)?.run()?;
    assert!(second_archive.starts_with(&second));
    assert_eq!(std::fs::read(second_archive.join("origin"))?, b"archived\n");
    assert_eq!(fixture.runs()?, 1, "only generated mode populated the shared cache");

    let stale = fixture.snapshot("stale", "crate", SCRIPT)?;
    fixture.archive(&stale, 1)?;
    let fallback = fixture.invocation(&stale, "crate", false)?.run()?;
    assert!(
        fallback.starts_with(&stale),
        "present stale archives keep their existing local fallback"
    );
    assert_eq!(std::fs::read(fallback.join("origin"))?, b"generated\n");
    assert_eq!(fixture.runs()?, 2);
    Ok(())
}

#[test]
fn preferred_and_required_archives_bypass_populated_generated_caches() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = Cache::new()?;
    let seed = fixture.snapshot("seed", "crate", SCRIPT)?;
    fixture.invocation(&seed, "crate", true)?.run()?;
    let archived = fixture.snapshot("archived", "crate", SCRIPT)?;
    fixture.archive(
        &archived,
        crc::Crc::<u32>::new(&crc::CRC_32_CKSUM).checksum(SCRIPT.as_bytes()),
    )?;
    for operation in ["prefer", "require"] {
        let mut request = fixture.invocation(&archived, "crate", true)?;
        request.command.env("GIX_TESTTOOLS_CACHE_OPERATION", operation);
        let path = request.run()?;
        assert!(
            path.starts_with(&archived),
            "archive policies retain snapshot-local extraction"
        );
        assert_eq!(
            std::fs::read(path.join("origin"))?,
            b"archived\n",
            "the policy overrides generated mode"
        );
    }
    let missing = fixture.snapshot("missing", "crate", SCRIPT)?;
    let mut request = fixture.invocation(&missing, "crate", true)?;
    request.command.env("GIX_TESTTOOLS_CACHE_OPERATION", "require");
    assert!(
        request.run()?.as_os_str().is_empty(),
        "a missing required archive returns None despite a generated cache hit"
    );
    assert_eq!(
        fixture.runs()?,
        1,
        "archive policies never consume or regenerate the shared seed"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn archive_creation_stays_local_unless_the_archive_is_excluded() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = Cache::new()?;
    let seed = fixture.snapshot("seed", "crate", SCRIPT)?;
    fixture.invocation(&seed, "crate", true)?.run()?;
    let producer = fixture.snapshot("producer", "crate", SCRIPT)?;
    fixture.archive(&producer, 1)?;
    let extension = if cfg!(feature = "xz") { "tar.xz" } else { "tar" };
    let archive = producer.join(format!("crate/tests/fixtures/generated-archives/make.{extension}"));
    let old_archive = std::fs::read(&archive)?;
    let mut request = fixture.invocation(&producer, "crate", true)?;
    request.command.env("GIX_TEST_CREATE_ARCHIVES_EVEN_ON_CI", "1");
    assert!(
        request.run()?.starts_with(&producer),
        "archive-producing runs generate locally despite a shared cache hit"
    );
    assert_ne!(
        std::fs::read(&archive)?,
        old_archive,
        "local generation recreates the fixture archive"
    );

    let mut shared = Vec::new();
    for (name, generated) in [("excluded-archived", false), ("excluded-generated", true)] {
        let root = fixture.snapshot(name, "crate", &format!("{SCRIPT}# excluded archive\n"))?;
        gix_testtools::git(&root, "init")?;
        let directory = root.join("crate/tests/fixtures/generated-archives");
        std::fs::create_dir(&directory)?;
        std::fs::write(directory.join(".gitignore"), format!("make.{extension}\n"))?;
        let mut request = fixture.invocation(&root, "crate", generated)?;
        request.command.env("GIX_TEST_CREATE_ARCHIVES_EVEN_ON_CI", "1");
        let path = request.run()?;
        assert!(
            path.starts_with(&fixture.cache),
            "excluded archives can share generated fixtures even when archive creation is enabled"
        );
        assert!(
            !directory.join(format!("make.{extension}")).exists(),
            "excluded archives are never created"
        );
        shared.push(path);
    }
    assert_eq!(
        shared[0], shared[1],
        "excluded fixtures share generation across archive modes"
    );
    assert_eq!(
        fixture.runs()?,
        3,
        "the ordinary seed, local archive producer, and excluded seed each generate once"
    );
    Ok(())
}

#[test]
fn cache_keys_separate_scripts_crates_arguments_postprocessing_and_hashes() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = Cache::new()?;
    let root = fixture.snapshot("source", "crate", SCRIPT)?;
    let mut paths = vec![fixture.invocation(&root, "crate", true)?.run()?];
    fixture.snapshot("source", "other-crate", SCRIPT)?;
    paths.push(fixture.invocation(&root, "other-crate", true)?.run()?);
    let changed = fixture.snapshot("changed", "crate", &format!("{SCRIPT}\nprintf 'changed\\n' > origin\n"))?;
    paths.push(fixture.invocation(&changed, "crate", true)?.run()?);
    std::fs::create_dir(root.join("crate/tests/fixtures/nested"))?;
    std::fs::write(root.join("crate/tests/fixtures/nested/make.sh"), SCRIPT)?;
    for script in ["nested/make.sh", "nested/../make.sh"] {
        let mut nested = fixture.invocation(&root, "crate", true)?;
        nested.command.env("GIX_TESTTOOLS_CACHE_SCRIPT", script);
        paths.push(nested.run()?);
    }

    // Both pairs collide when arguments are concatenated or joined as filesystem paths.
    for args in [["a", "bc"], ["ab", "c"], ["same/path", ""], ["same", "path"]] {
        let mut request = fixture.invocation(&root, "crate", true)?;
        request
            .command
            .env("GIX_TESTTOOLS_CACHE_ARG_COUNT", "2")
            .env("GIX_TESTTOOLS_CACHE_ARG_0", args[0])
            .env("GIX_TESTTOOLS_CACHE_ARG_1", args[1]);
        let path = request.run()?;
        assert_eq!(
            std::fs::read_to_string(path.join("arguments"))?,
            format!("{}\n{}\n", args[0], args[1])
        );
        paths.push(path);
    }
    for version in [0, 1] {
        let mut request = fixture.invocation(&root, "crate", true)?;
        request.command.env("GIX_TESTTOOLS_CACHE_POST", version.to_string());
        let path = request.run()?;
        assert_eq!(std::fs::read_to_string(path.join("post-version"))?, version.to_string());
        paths.push(path);
    }
    let mut sha256 = fixture.invocation(&root, "crate", true)?;
    sha256.command.env("GIX_TEST_FIXTURE_HASH", "sha256");
    paths.push(sha256.run()?);
    let mut unique = paths.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(
        unique.len(),
        paths.len(),
        "every distinct fixture input has its own cache entry"
    );
    assert_eq!(fixture.runs()?, paths.len());
    Ok(())
}

#[test]
fn writable_copies_are_independent_and_execute_always_runs() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = Cache::new()?;
    let root = fixture.snapshot("source", "crate", SCRIPT)?;
    let seed = fixture.invocation(&root, "crate", true)?.run()?;
    for operation in ["copy", "copy", "execute", "execute"] {
        let mut request = fixture.invocation(&root, "crate", true)?;
        request.command.env("GIX_TESTTOOLS_CACHE_OPERATION", operation);
        let writable = request.run()?;
        assert!(
            !writable.starts_with(&fixture.cache),
            "writers receive private temporary directories"
        );
        assert_ne!(writable, seed);
    }
    assert_eq!(
        std::fs::read(seed.join("origin"))?,
        b"generated\n",
        "writer changes cannot alter the seed"
    );
    assert_eq!(
        fixture.runs()?,
        3,
        "copies reuse the seed, while both explicit executions rerun it"
    );
    Ok(())
}

#[test]
fn failures_remain_unpublished_and_concurrent_requests_share_the_retry() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = Cache::new()?;
    let first = fixture.snapshot("first", "crate", SCRIPT)?;
    let second = fixture.snapshot("second", "crate", SCRIPT)?;
    let mut failed = fixture.invocation(&first, "crate", true)?;
    let output = failed.command.env("FIXTURE_CACHE_FAIL", "1").output()?;
    assert!(
        !output.status.success(),
        "a failing script cannot publish a completed fixture"
    );
    let mut abandoned = Vec::new();
    generation_paths(&fixture.cache, &mut abandoned)?;
    assert_eq!(
        abandoned.len(),
        1,
        "failed generation retains its own diagnostic directory"
    );

    let left = fixture.invocation(&first, "crate", true)?.spawn()?;
    let right = fixture.invocation(&second, "crate", true)?.spawn()?;
    let completed = left.finish()?;
    assert_eq!(
        completed,
        right.finish()?,
        "concurrent processes publish only one completed fixture"
    );
    assert_ne!(
        completed, abandoned[0],
        "retry uses a new directory without recycling the failed path"
    );
    assert!(
        abandoned[0].join("generation-path").is_file(),
        "retry preserves failed diagnostic output"
    );
    assert_eq!(
        fixture.runs()?,
        2,
        "one failed attempt and one successful retry execute the script"
    );
    Ok(())
}

#[test]
fn caching_requires_both_absolute_environment_paths() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = Cache::new()?;
    for (index, (name, value)) in [
        ("GIX_TEST_FIXTURE_CACHE", None),
        ("GIX_TEST_FIXTURE_SOURCE_ROOT", None),
        ("GIX_TEST_FIXTURE_CACHE", Some("relative-cache")),
        ("GIX_TEST_FIXTURE_SOURCE_ROOT", Some("relative-root")),
    ]
    .into_iter()
    .enumerate()
    {
        let root = fixture.snapshot(&format!("source-{index}"), "crate", SCRIPT)?;
        let mut request = fixture.invocation(&root, "crate", true)?;
        if let Some(value) = value {
            request.command.env(name, value);
        } else {
            request.command.env_remove(name);
        }
        let output = request.command.output()?;
        assert!(
            !output.status.success(),
            "incomplete or relative cache settings are rejected"
        );
    }
    assert_eq!(fixture.runs()?, 0, "invalid settings never start fixture generation");
    let root = fixture.snapshot("ordinary", "crate", SCRIPT)?;
    let mut disabled = fixture.invocation(&root, "crate", true)?;
    disabled
        .command
        .env_remove("GIX_TEST_FIXTURE_CACHE")
        .env_remove("GIX_TEST_FIXTURE_SOURCE_ROOT");
    assert!(
        disabled.run()?.starts_with(&root),
        "without opt-in ordinary local behavior is unchanged"
    );
    let outside = fixture.snapshot("outside", "crate", SCRIPT)?;
    let mut request = fixture.invocation(&outside, "crate", true)?;
    request.command.env("GIX_TEST_FIXTURE_SOURCE_ROOT", &root);
    assert!(
        request.run()?.starts_with(outside),
        "sources outside the selected snapshot remain local"
    );
    assert_eq!(fixture.runs()?, 2);
    Ok(())
}

/// Each fixture request gets a fresh process, like a different nextest invocation or QA snapshot.
/// This avoids relying on process-global script identities, object hashes, or exclusion caches.
#[test]
#[ignore = "fixture-cache subprocess entry point"]
fn fixture_process() -> TestResult {
    let Some(report) = std::env::var_os("GIX_TESTTOOLS_CACHE_REPORT") else {
        return Ok(());
    };
    let script = std::env::var("GIX_TESTTOOLS_CACHE_SCRIPT")?;
    let args = (0..std::env::var("GIX_TESTTOOLS_CACHE_ARG_COUNT")?.parse::<usize>()?)
        .map(|index| std::env::var(format!("GIX_TESTTOOLS_CACHE_ARG_{index}")))
        .collect::<Result<Vec<_>, _>>()?;
    let writable;
    let operation = std::env::var("GIX_TESTTOOLS_CACHE_OPERATION")?;
    let fixture = match operation.as_str() {
        "prefer" => gix_testtools::scripted_fixture_read_only_needs_archive(script)?,
        "require" => {
            let Some(path) = gix_testtools::scripted_fixture_read_only_with_git_version(script, |_| false)? else {
                std::fs::write(report, [])?;
                return Ok(());
            };
            path
        }
        "copy" | "execute" => {
            let mode = if std::env::var("GIX_TESTTOOLS_CACHE_OPERATION")? == "copy" {
                gix_testtools::Creation::CopyFromReadOnly
            } else {
                gix_testtools::Creation::Execute
            };
            writable = gix_testtools::scripted_fixture_writable_with_args_single_archive(script, args, mode)?;
            std::fs::write(writable.path().join("origin"), b"writer\n")?;
            writable.path().to_owned()
        }
        "read" => match std::env::var("GIX_TESTTOOLS_CACHE_POST") {
            Ok(version) => {
                gix_testtools::scripted_fixture_read_only_with_args_single_archive_with_post(
                    script,
                    args,
                    version.parse()?,
                    |state| {
                        assert!(
                            state.path().is_absolute(),
                            "post-processing receives absolute paths on every backend"
                        );
                        if let gix_testtools::FixtureState::Uninitialized(path) = state {
                            std::fs::write(path.join("post-version"), &version)?;
                        }
                        Ok(())
                    },
                )?
                .0
            }
            Err(_) => gix_testtools::scripted_fixture_read_only_with_args_single_archive(script, args)?,
        },
        operation => panic!("unknown fixture operation: {operation}"),
    };
    assert!(
        fixture.is_absolute(),
        "generated, cached, and archived fixtures return absolute paths"
    );
    #[cfg(windows)]
    assert!(
        !matches!(fixture.components().next(), Some(std::path::Component::Prefix(prefix)) if prefix.kind().is_verbatim()),
        "ordinary fixture paths do not acquire a verbatim prefix"
    );
    if operation == "read" && fixture.join("generation-path").is_file() {
        let generation_path = std::fs::read_to_string(fixture.join("generation-path"))?;
        assert_eq!(
            gix_path::realpath(&fixture)?,
            Path::new(generation_path.trim()),
            "fixture paths refer to the physical paths recorded by the shell"
        );
    }
    std::fs::write(report, fixture.to_string_lossy().as_bytes())?;
    Ok(())
}

struct Cache {
    directory: gix_testtools::tempfile::TempDir,
    cache: PathBuf,
    runs: PathBuf,
}

impl Cache {
    fn new() -> gix_testtools::Result<Self> {
        let directory = gix_testtools::tempfile::tempdir_in(gix_path::realpath(std::env::temp_dir())?)?;
        let cache = directory.path().join("cache");
        let runs = directory.path().join("runs");
        std::fs::create_dir(&cache)?;
        std::fs::write(&runs, [])?;
        Ok(Self { directory, cache, runs })
    }

    fn snapshot(&self, name: &str, crate_name: &str, script: &str) -> gix_testtools::Result<PathBuf> {
        let root = self.directory.path().join(name);
        let fixtures = root.join(crate_name).join("tests/fixtures");
        std::fs::create_dir_all(&fixtures)?;
        std::fs::write(fixtures.join("make.sh"), script)?;
        Ok(root)
    }

    fn invocation(&self, root: &Path, crate_name: &str, generated: bool) -> gix_testtools::Result<Invocation> {
        let report = self.directory.path().join(format!("report-{}", fastrand::u64(..)));
        let mut command = Command::new(std::env::current_exe()?);
        gix_testtools::configure_git_environment(&mut command, root)
            .args([
                "--exact",
                "fixture_cache::fixture_process",
                "--include-ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .current_dir(root.join(crate_name))
            .env("GIX_TEST_FIXTURE_CACHE", &self.cache)
            .env("GIX_TEST_FIXTURE_SOURCE_ROOT", root)
            .env("GIX_TEST_FIXTURE_HASH", "sha1")
            .env("FIXTURE_CACHE_RUNS", self.runs.to_string_lossy().replace('\\', "/"))
            .env("GIX_TESTTOOLS_CACHE_REPORT", &report)
            .env("GIX_TESTTOOLS_CACHE_SCRIPT", "make.sh")
            .env("GIX_TESTTOOLS_CACHE_ARG_COUNT", "0")
            .env("GIX_TESTTOOLS_CACHE_OPERATION", "read")
            .env_remove("GIX_TESTTOOLS_CACHE_POST")
            .env_remove("FIXTURE_CACHE_FAIL")
            .env("CI", "true")
            .env_remove("GIX_TEST_CREATE_ARCHIVES_EVEN_ON_CI");
        if generated {
            command.env("GIX_TEST_IGNORE_ARCHIVES", "1");
        } else {
            command.env_remove("GIX_TEST_IGNORE_ARCHIVES");
        }
        Ok(Invocation { command, report })
    }

    fn runs(&self) -> std::io::Result<usize> {
        Ok(std::fs::read_to_string(&self.runs)?.lines().count())
    }

    fn archive(&self, root: &Path, identity: u32) -> gix_testtools::Result<()> {
        let mut archive = tar::Builder::new(Vec::new());
        for (name, bytes) in [
            ("origin", b"archived\n".to_vec()),
            (
                "__gitoxide_meta__/identity",
                format!("{identity}-{}", if cfg!(windows) { "windows" } else { "unix" }).into_bytes(),
            ),
        ] {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            archive.append_data(&mut header, name, bytes.as_slice())?;
        }
        let bytes = archive.into_inner()?;
        let directory = root.join("crate/tests/fixtures/generated-archives");
        std::fs::create_dir_all(&directory)?;
        #[cfg(feature = "xz")]
        {
            use std::io::Write;
            let mut encoder = xz2::write::XzEncoder::new(std::fs::File::create(directory.join("make.tar.xz"))?, 3);
            encoder.write_all(&bytes)?;
            encoder.finish()?;
        }
        #[cfg(not(feature = "xz"))]
        std::fs::write(directory.join("make.tar"), bytes)?;
        Ok(())
    }
}

struct Invocation {
    command: Command,
    report: PathBuf,
}

impl Invocation {
    fn run(self) -> gix_testtools::Result<PathBuf> {
        self.spawn()?.finish()
    }

    fn spawn(mut self) -> std::io::Result<Pending> {
        let child = self.command.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
        Ok(Pending {
            child,
            report: self.report,
        })
    }
}

struct Pending {
    child: Child,
    report: PathBuf,
}

impl Pending {
    fn finish(self) -> gix_testtools::Result<PathBuf> {
        let output = self.child.wait_with_output()?;
        assert!(
            output.status.success(),
            "fixture subprocess failed: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(std::fs::read_to_string(&self.report)?.into())
    }
}

fn generation_paths(directory: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            generation_paths(&entry.path(), out)?;
        } else if entry.file_name() == "generation-path" {
            out.push(directory.to_owned());
        }
    }
    Ok(())
}
