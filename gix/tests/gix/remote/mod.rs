use std::path::PathBuf;

use gix_testtools::scripted_fixture_read_only;

pub(crate) fn repo_path(name: &str) -> PathBuf {
    let dir = scripted_fixture_read_only("make_remote_repos.sh").unwrap();
    dir.join(name)
}

pub(crate) fn repo(name: &str) -> gix::Repository {
    gix::open_opts(repo_path(name), gix::open::Options::isolated()).unwrap()
}

/// Spawn a git-daemon hosting all directories in or below `base_dir` if we are in async mode - currently only TCP is
/// available in async mode, and it's probably going to stay that way as we don't want to chose a particular runtime
/// in lower-level crates just yet.
/// Maybe this changes one day once we implement other protocols like spawning a process via `tokio` or `async-std`, or
/// provide async HTTP implementations as well.
#[cfg(any(feature = "blocking-network-client", feature = "async-network-client-async-std"))]
pub(crate) fn spawn_git_daemon_if_async(
    _base_dir: impl AsRef<std::path::Path>,
) -> std::io::Result<Option<gix_testtools::GitDaemon>> {
    #[cfg(feature = "blocking-network-client")]
    {
        Ok(None)
    }
    #[cfg(feature = "async-network-client-async-std")]
    {
        gix_testtools::spawn_git_daemon(_base_dir).map(Some)
    }
}

/// Turn `remote` into a remote that interacts with the git `daemon`, all else being the same, by creating a new stand-in remote.
#[cfg(any(feature = "blocking-network-client", feature = "async-network-client-async-std"))]
pub(crate) fn into_daemon_remote_if_async<'repo, 'a>(
    remote: gix::Remote<'repo>,
    _daemon: Option<&gix_testtools::GitDaemon>,
    _repo_name: impl Into<Option<&'a str>>,
) -> gix::Remote<'repo> {
    #[cfg(feature = "blocking-network-client")]
    {
        remote
    }
    #[cfg(feature = "async-network-client-async-std")]
    {
        let mut new_remote = remote
            .repo()
            .remote_at(format!(
                "{}/{}",
                _daemon.expect("daemon is available in async mode").url,
                _repo_name.into().unwrap_or_default()
            ))
            .expect("valid url to create remote at")
            .with_fetch_tags(remote.fetch_tags());
        for direction in [gix::remote::Direction::Fetch, gix::remote::Direction::Push] {
            new_remote
                .replace_refspecs(
                    remote.refspecs(direction).iter().map(|s| s.to_ref().to_bstring()),
                    direction,
                )
                .expect("input refspecs valid");
        }
        new_remote
    }
}

mod connect;
pub(crate) mod fetch;
mod push;
mod ref_map;
mod save;
mod name {

    #[test]
    fn origin_is_valid() {
        assert!(gix::remote::name::validated("origin").is_ok());
    }

    #[test]
    fn multiple_slashes_are_valid() {
        assert!(gix::remote::name::validated("origin/another").is_ok());
    }

    #[test]
    fn empty_is_invalid() {
        assert!(gix::remote::name::validated("").is_err());
    }
}
