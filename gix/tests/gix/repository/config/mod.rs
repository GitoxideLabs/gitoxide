pub(crate) mod config_snapshot;
#[cfg(feature = "command")]
mod editor;
mod identity;
mod remote;

#[test]
fn config_path_for_repository_sources_matches_git() -> crate::Result {
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

#[test]
fn big_file_threshold() -> crate::Result {
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

#[cfg(feature = "blocking-network-client")]
mod ssh_options {
    use std::ffi::OsStr;

    use crate::repository::config::repo;

    #[test]
    fn with_command_and_variant() -> crate::Result {
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
    fn with_command_fallback_which_disallows_shell() -> crate::Result {
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
