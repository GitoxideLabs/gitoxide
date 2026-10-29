use gix_testtools::TestResult;
pub(crate) mod config_snapshot;
#[cfg(feature = "command")]
mod editor;
mod identity;
#[cfg(feature = "worktree-mutation")]
mod path_protection;
mod remote;

#[test]
fn config_path_for_repository_sources_matches_git() -> TestResult {
    use gix::config::Source;

    let fixture = gix_testtools::scripted_fixture_read_only("make_worktree_repo.sh")?;
    for name in ["repo", "wt-a", "natively-bare-repo", "worktree-of-natively-bare-repo"] {
        let repo = gix::open_opts(fixture.join(name), gix::open::Options::isolated())?;
        for (source, filename) in [(Source::Local, "config"), (Source::Worktree, "config.worktree")] {
            let baseline = gix_testtools::git(
                repo.git_dir(),
                &format!("rev-parse --path-format=absolute --git-path {filename}"),
            )?;
            let path = repo.config_path(source)?;
            assert_eq!(
                gix::path::realpath(&path)?,
                gix::path::realpath(baseline.trim())?,
                "{name}: local configuration is shared, while worktree configuration belongs to each Git directory"
            );
            if source == Source::Worktree {
                assert!(
                    !path.exists(),
                    "the worktree path is available without creating a file or enabling extensions.worktreeConfig"
                );
            }
        }
    }
    Ok(())
}

#[cfg(feature = "command")]
#[test]
fn command_context_paths_use_the_captured_current_directory() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?;
    let elsewhere = gix_testtools::tempfile::TempDir::new()?;
    let _cwd = gix_testtools::set_current_dir(&fixture)?;
    let repo = gix::open_opts(".", gix::open::Options::isolated())?;
    let bare = gix::open_opts("bare.git", gix::open::Options::isolated())?;
    assert!(repo.git_dir().is_relative(), "exercise a relative repository path");
    let _moved_cwd = gix_testtools::set_current_dir(elsewhere.path())?;

    for repo in [&repo, &bare] {
        let context = repo.command_context()?;
        assert_eq!(
            context.git_dir,
            Some(repo.current_dir().join(repo.git_dir())),
            "GIT_DIR stays anchored to the repository's captured CWD"
        );
        assert_eq!(
            context.worktree_dir,
            repo.workdir().map(|path| repo.current_dir().join(path)),
            "GIT_WORK_TREE stays anchored to the captured CWD and remains unset for bare repositories"
        );
    }
    Ok(())
}

#[test]
fn big_file_threshold() -> TestResult {
    let repo = repo("with-hasconfig");
    assert_eq!(
        repo.big_file_threshold()?,
        512 * 1024 * 1024,
        "Git really handles huge files, and this is the default"
    );

    let repo = crate::repository::config::repo("big-file-threshold");
    assert_eq!(repo.big_file_threshold()?, 42, "It picks up configured values as well");
    Ok(())
}

#[cfg(feature = "index")]
#[test]
fn invalid_stat_boolean_is_validation_error() -> TestResult {
    let mut diagnostics = Vec::new();
    for key in [
        "core.trustCTime",
        "gitoxide.core.useNsec",
        "gitoxide.core.useStdev",
        "core.checkStat",
    ] {
        let mut repo = repo("with-hasconfig");
        repo.config_snapshot_mut().set_raw_value(key, "invalid")?;
        let err = repo.stat_options().expect_err("invalid value must fail");
        assert!(err.is_validation(), "invalid {key} is classified as a validation error");
        diagnostics.push((key, err));
    }
    insta::assert_debug_snapshot!(diagnostics, "invalid stat options identify their configuration key and rejected value", @r#"
    [
        (
            "core.trustCTime",
            Invalid boolean, key="core.trustCTime"
            
            Caused by:
                0: Booleans need to be 'no', 'off', 'false', '' or 'yes', 'on', 'true' or any number, input="invalid",
        ),
        (
            "gitoxide.core.useNsec",
            Invalid boolean, key="gitoxide.core.useNsec"
            
            Caused by:
                0: Booleans need to be 'no', 'off', 'false', '' or 'yes', 'on', 'true' or any number, input="invalid",
        ),
        (
            "gitoxide.core.useStdev",
            Invalid boolean, key="gitoxide.core.useStdev"
            
            Caused by:
                0: Booleans need to be 'no', 'off', 'false', '' or 'yes', 'on', 'true' or any number, input="invalid",
        ),
        (
            "core.checkStat",
            Invalid configuration value, input="invalid", key="core.checkStat",
        ),
    ]
    "#);
    Ok(())
}

#[cfg(feature = "blocking-network-client")]
mod ssh_options {
    use gix_testtools::TestResult;
    use std::ffi::OsStr;

    use crate::repository::config::repo;

    #[test]
    fn with_command_and_variant() -> TestResult {
        let repo = repo("ssh-all-options");
        let opts = repo.ssh_connect_options()?;
        assert_eq!(opts.command.as_deref(), Some(OsStr::new("ssh -VVV")));
        assert_eq!(
            opts.kind,
            Some(gix::protocol::transport::client::blocking_io::ssh::ProgramKind::Ssh)
        );
        assert!(!opts.disallow_shell, "we can use the shell by default");
        Ok(())
    }

    #[test]
    fn with_command_fallback_which_disallows_shell() -> TestResult {
        let repo = repo("ssh-command-fallback");
        let opts = repo.ssh_connect_options()?;
        assert_eq!(opts.command.as_deref(), Some(OsStr::new("ssh --fallback")));
        assert_eq!(
            opts.kind,
            Some(gix::protocol::transport::client::blocking_io::ssh::ProgramKind::Putty)
        );
        assert!(
            opts.disallow_shell,
            "fallbacks won't allow shells, so must be a program or program name"
        );
        Ok(())
    }
}

#[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
mod transport_options;

pub fn repo(name: &str) -> gix::Repository {
    repo_opts(name, |opts| opts.strict_config(true))
}

pub fn repo_opts(name: &str, modify: impl FnOnce(gix::open::Options) -> gix::open::Options) -> gix::Repository {
    let dir = gix_testtools::scripted_fixture_read_only("make_config_repos.sh").unwrap();
    gix::open_opts(dir.join(name), modify(gix::open::Options::isolated())).unwrap()
}

#[cfg(feature = "command")]
#[test]
fn command_context_preserves_the_unexpanded_namespace() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?;
    let mut repo = gix::open_opts(fixture, gix::open::Options::isolated())?;
    for name in ["team", "team/project", "refs/namespaces"] {
        repo.set_namespace(name)?;
        assert_eq!(repo.command_context()?.ref_namespace, Some(name.into()));
    }
    Ok(())
}
