use std::{ffi::OsStr, path::Path, process::Stdio};

use gix_testtools::TestResult;

fn fixture() -> gix_testtools::Result<std::path::PathBuf> {
    Ok(gix_testtools::scripted_fixture_read_only("make_hook_repos.sh")?.canonicalize()?)
}

#[test]
fn discovery_uses_shared_hooks_and_configured_worktree_relative_paths() -> TestResult {
    let fixture = fixture()?;
    for (name, expected) in [
        ("repo", "repo/.git/hooks/pre-commit"),
        ("linked", "repo/.git/hooks/pre-commit"),
        ("bare.git", "bare.git/hooks/pre-commit"),
    ] {
        let repo = gix::open_opts(fixture.join(name), crate::restricted())?;
        assert_eq!(
            repo.find_hook("pre-commit")?
                .expect("the fixture installs the hook")
                .canonicalize()?,
            fixture.join(expected).canonicalize()?,
            "default hooks belong to the common repository, including from linked worktrees"
        );
        assert!(repo.find_hook("missing")?.is_none(), "absent hooks are not configured");
    }
    for name in ["repo", "linked"] {
        for hooks_path in [
            "custom-hooks".to_owned(),
            fixture.join(name).join("custom-hooks").to_string_lossy().into_owned(),
        ] {
            let repo = gix::open_opts(
                fixture.join(name),
                crate::restricted().config_overrides([format!("core.hooksPath={hooks_path}")]),
            )?;
            assert_eq!(
                repo.find_hook("pre-commit")?
                    .expect("the configured hook exists")
                    .canonicalize()?,
                fixture.join(name).join("custom-hooks/pre-commit").canonicalize()?,
                "relative core.hooksPath starts at the hook's working directory"
            );
        }
    }
    let repo = gix::open_opts(
        fixture.join("repo"),
        crate::restricted().config_overrides(["core.hooksPath="]),
    )?;
    assert_eq!(
        repo.find_hook("pre-commit")?,
        Some(fixture.join("repo/pre-commit")),
        "an explicit empty path selects the worktree root"
    );
    let repo = gix::open_opts(
        fixture.join("repo"),
        crate::restricted().config_overrides(["core.hooksPath=absent"]),
    )?;
    assert!(
        repo.find_hook("pre-commit")?.is_none(),
        "a configured path does not fall back to common hooks"
    );
    Ok(())
}

#[test]
fn discovery_rejects_path_components_and_ignores_non_executable_hooks() -> TestResult {
    let repo = gix::open_opts(fixture()?.join("repo"), crate::restricted())?;
    for name in [
        "",
        ".",
        "..",
        "../pre-commit",
        "hooks/pre-commit",
        "hooks\\pre-commit",
        "/pre-commit",
        "pre\0commit",
    ] {
        assert!(
            repo.find_hook(name)
                .expect_err("hook names cannot escape the hook directory")
                .is_validation(),
            "invalid names are classified as input errors"
        );
    }
    assert!(repo.find_hook("directory")?.is_none(), "directories cannot be hooks");
    #[cfg(unix)]
    assert!(
        repo.find_hook("post-commit")?.is_none(),
        "hooks without execute permission are ignored"
    );
    Ok(())
}

#[test]
fn hook_commands_preserve_arguments_and_match_git() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = fixture()?;
    for (name, expected) in [
        ("repo", "expected-output"),
        ("linked", "expected-linked-output"),
        ("bare.git", "expected-bare-output"),
    ] {
        let repo = gix::open_opts(fixture.join(name), crate::restricted())?;
        let hook = repo.find_hook("pre-commit")?.expect("the fixture installs the hook");
        let mut command = repo.prepare_hook(&hook)?;
        command.args(["one argument", "two"]).stderr(Stdio::piped());
        assert_eq!(
            command.get_current_dir(),
            Some(
                repo.current_dir()
                    .join(repo.workdir().unwrap_or(repo.git_dir()))
                    .as_path()
            ),
            "hooks run in the checkout or bare repository"
        );
        let output = command.output()?;
        assert!(
            output.status.success(),
            "the prepared hook runs successfully: {output:?}"
        );
        assert_eq!(
            output.stdout,
            std::fs::read(fixture.join(expected))?,
            "arguments are passed literally, as with git hook run"
        );
    }
    Ok(())
}

#[test]
fn preparation_anchors_paths_and_repository_context() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = fixture()?;
    let elsewhere = gix_testtools::tempfile::tempdir()?;
    let _cwd = gix_testtools::set_current_dir(fixture.join("repo"))?;
    let repo = gix::open_opts(".", crate::restricted())?;
    let _moved = gix_testtools::set_current_dir(elsewhere.path())?;
    let command = repo.prepare_hook(Path::new("custom-hooks/pre-commit"))?;
    assert_eq!(
        command.get_current_dir(),
        Some(fixture.join("repo").as_path()),
        "preparation uses the repository's captured current directory"
    );
    let context = repo.command_context()?;
    for (key, expected) in [("GIT_DIR", context.git_dir), ("GIT_WORK_TREE", context.worktree_dir)] {
        assert_eq!(
            command
                .get_envs()
                .find(|(name, _)| *name == OsStr::new(key))
                .and_then(|(_, value)| value),
            expected.as_deref().map(Path::as_os_str),
            "hook environment keeps {key} attached to this repository"
        );
    }
    #[cfg(unix)]
    assert_eq!(
        command.get_program(),
        fixture.join("repo/custom-hooks/pre-commit"),
        "ordinary executables are prepared directly without forcing a shell"
    );
    Ok(())
}

#[test]
fn reduced_trust_does_not_offer_repository_hooks_for_execution() -> TestResult {
    let repo = gix::open_opts(
        fixture()?.join("repo"),
        crate::restricted().with(gix::sec::Trust::Reduced),
    )?;
    assert!(
        repo.find_hook("pre-commit")
            .expect_err("repository hooks require full trust")
            .is_permission_denied(),
        "untrusted hook discovery is rejected"
    );
    assert!(
        repo.prepare_hook("pre-commit")
            .expect_err("repository hook commands require full trust")
            .is_permission_denied(),
        "preparation cannot bypass repository trust"
    );
    Ok(())
}

#[test]
fn configured_paths_keep_empty_optional_and_implicit_values_distinct() -> TestResult {
    let fixture = fixture()?;
    let workdir = fixture.join("repo");
    for (value, expected) in [
        (":(optional)missing", ".git/hooks"),
        (":(optional)", ".git/hooks"),
        (":(optional).git/hooks", ".git/hooks"),
    ] {
        let overrides = [
            "core.hooksPath=custom-hooks".to_owned(),
            format!("core.hooksPath={value}"),
        ];
        let repo = gix::open_opts(&workdir, crate::restricted().config_overrides(overrides.clone()))?;
        assert_eq!(
            repo.find_hook("pre-commit")?,
            Some(workdir.join(expected).join("pre-commit")),
            "missing optional paths select the default hook directory"
        );
        if *gix_testtools::GIT_VERSION >= (2, 53, 0) {
            let output = gix_testtools::git_command(&workdir)
                .args([
                    "-c",
                    &overrides[0],
                    "-c",
                    &overrides[1],
                    "rev-parse",
                    "--path-format=absolute",
                    "--git-path",
                    "hooks/pre-commit",
                ])
                .output()?;
            assert!(output.status.success(), "Git accepts optional hook paths");
            assert_eq!(
                Path::new(std::str::from_utf8(&output.stdout)?.trim()).canonicalize()?,
                repo.find_hook("pre-commit")?.expect("hook exists").canonicalize()?,
                "Git and gitoxide select the same hook after optional-path filtering"
            );
        }
    }
    let repo = gix::open_opts(&workdir, crate::restricted().config_overrides(["core.hooksPath"]))?;
    assert!(
        repo.find_hook("pre-commit")
            .expect_err("an implicit path is not an explicit empty path")
            .is_validation(),
        "hooksPath needs a value"
    );
    let output = gix_testtools::git_command(&workdir)
        .args(["-c", "core.hooksPath", "hook", "run", "pre-commit"])
        .output()?;
    assert!(!output.status.success(), "Git rejects an implicit hook path as well");
    Ok(())
}

#[cfg(unix)]
#[test]
fn hook_paths_retain_non_utf8_bytes_and_native_executables() -> TestResult {
    use std::os::unix::ffi::OsStrExt;
    let fixture = fixture()?;
    let repo = gix::open_opts(fixture.join("repo"), crate::restricted())?;
    let path = Path::new(OsStr::from_bytes(b"custom-\xff/pre-commit"));
    assert_eq!(
        repo.prepare_hook(path)?.get_program(),
        fixture.join("repo").join(path),
        "preparation retains non-UTF-8 path bytes even on filesystems that cannot store them"
    );
    let native = std::env::current_exe()?;
    let command = repo.prepare_hook(&native)?;
    assert_eq!(command.get_program(), native, "native binaries are executed directly");
    assert!(
        command.get_args().next().is_none(),
        "preparation does not add shell or interpreter arguments to native binaries"
    );
    Ok(())
}

#[test]
fn inherited_repository_paths_cannot_redirect_prepared_hooks() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = fixture()?;
    let _environment = gix_testtools::Env::new()
        .set(
            "GIT_COMMON_DIR",
            fixture.join("bare.git").to_str().expect("fixture path is UTF-8"),
        )
        .set(
            "GIT_WORK_TREE",
            fixture.join("linked").to_str().expect("fixture path is UTF-8"),
        );
    for name in ["repo", "bare.git"] {
        let repo = gix::open_opts(fixture.join(name), crate::restricted())?;
        let hook = repo
            .find_hook("pre-auto-gc")?
            .expect("the fixture installs the environment hook");
        let output = repo.prepare_hook(hook)?.output()?;
        assert!(output.status.success(), "the environment hook completes");
        let expected = format!(
            "{}\n{}\n",
            repo.current_dir().join(repo.common_dir()).display(),
            repo.workdir().map_or_else(
                || "unset".into(),
                |path| repo.current_dir().join(path).display().to_string()
            )
        );
        assert_eq!(
            output.stdout,
            expected.as_bytes(),
            "the hook belongs to its repository, even if inherited Git paths name another repository"
        );
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn discovery_checks_the_callers_execute_permission() -> TestResult {
    let fixture = fixture()?;
    let repo = gix::open_opts(fixture.join("repo"), crate::restricted())?;
    assert_eq!(
        repo.find_hook("applypatch-msg")?.is_some(),
        fixture.join("mode-hook-executable").exists(),
        "Git and gitoxide honor the caller's permissions, not merely any executable bit"
    );
    Ok(())
}
