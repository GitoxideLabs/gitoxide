#![allow(clippy::result_large_err)]

use gix::open::Permissions;
use gix::{Repository, ThreadSafeRepository};
use gix_sec::Permission;
use serial_test::serial;

pub fn named_subrepo_opts(
    fixture: &str,
    name: &str,
    opts: gix::open::Options,
) -> std::result::Result<Repository, gix::open::Error> {
    let repo_path = gix_testtools::scripted_fixture_read_only(fixture).unwrap().join(name);
    Ok(ThreadSafeRepository::open_opts(repo_path, opts)?.to_thread_local())
}

mod with_overrides {
    use crate::named_subrepo_opts;
    use gix_sec::Permission;
    use gix_testtools::Env;
    use serial_test::serial;

    #[test]
    #[serial]
    fn order_from_api_and_cli_and_environment() -> gix_testtools::Result {
        let default_date = "42 +0030";
        let _env = Env::new()
            .set("GIT_HTTP_USER_AGENT", "agent-from-env")
            .set("GIT_HTTP_LOW_SPEED_LIMIT", "1")
            .set("GIT_HTTP_LOW_SPEED_TIME", "1")
            .set("GIT_HTTP_PROXY_AUTHMETHOD", "proxy-auth-method-env")
            .set("GIT_SSL_NO_VERIFY", "true")
            .set("GIT_CURL_VERBOSE", "true")
            .set("https_proxy", "https-lower-override")
            .set("HTTPS_PROXY", "https-upper")
            .set("http_proxy", "http-lower")
            .set("all_proxy", "all-proxy-lower")
            .set("ALL_PROXY", "all-proxy")
            .set("no_proxy", "no-proxy-lower")
            .set("NO_PROXY", "no-proxy")
            .set("GIT_PROTOCOL_FROM_USER", "file-allowed")
            .set("GIT_REPLACE_REF_BASE", "refs/replace-mine")
            .set("GIT_NO_REPLACE_OBJECTS", "no-replace")
            .set("GIT_ALLOC_LIMIT", "7m")
            .set("GIT_COMMITTER_NAME", "committer name")
            .set("GIT_COMMITTER_EMAIL", "committer email")
            .set("GIT_COMMITTER_DATE", default_date)
            .set("GIT_AUTHOR_NAME", "author name")
            .set("GIT_AUTHOR_EMAIL", "author email")
            .set("GIT_AUTHOR_DATE", default_date)
            .set("EMAIL", "user email")
            .set("GIX_PACK_CACHE_MEMORY", "0")
            .set("GIX_PARSE_PRECIOUS", "1")
            .set("GIX_OBJECT_CACHE_MEMORY", "5m")
            .set("GIX_CREDENTIALS_HELPER_STDERR", "creds-stderr")
            .set("GIX_EXTERNAL_COMMAND_STDERR", "filter-stderr")
            .set("GIT_SSL_CAINFO", "./env.pem")
            .set("GIT_SSL_VERSION", "tlsv1.3")
            .set("GIT_SSH_VARIANT", "ssh-variant-env")
            .set("GIT_SSH_COMMAND", "ssh-command-env")
            .set("GIT_SSH", "ssh-command-fallback-env")
            .set("GIT_LITERAL_PATHSPECS", "pathspecs-literal")
            .set("GIT_GLOB_PATHSPECS", "pathspecs-glob")
            .set("GIT_NOGLOB_PATHSPECS", "pathspecs-noglob")
            .set("GIT_ICASE_PATHSPECS", "pathspecs-icase")
            .set("GIT_TERMINAL_PROMPT", "42")
            .set("GIT_SHALLOW_FILE", "shallow-file-env")
            .set("GIT_NAMESPACE", "namespace-env")
            .set("GIT_EXTERNAL_DIFF", "external-diff-env");
        let mut opts = gix::open::Options::isolated()
            .cli_overrides([
                "http.userAgent=agent-from-cli",
                "http.lowSpeedLimit=3",
                "http.lowSpeedTime=3",
                "http.sslCAInfo=./cli.pem",
                "http.sslVersion=sslv3",
                "ssh.variant=ssh-variant-cli",
                "core.sshCommand=ssh-command-cli",
                "gitoxide.ssh.commandWithoutShellFallback=ssh-command-fallback-cli",
                "gitoxide.http.proxyAuthMethod=proxy-auth-method-cli",
                "gitoxide.core.shallowFile=shallow-file-cli",
                "gitoxide.core.refsNamespace=namespace-cli",
            ])
            .config_overrides([
                "http.userAgent=agent-from-api",
                "http.lowSpeedLimit=2",
                "http.lowSpeedTime=2",
                "http.sslCAInfo=./api.pem",
                "http.sslVersion=tlsv1",
                "ssh.variant=ssh-variant-api",
                "core.sshCommand=ssh-command-api",
                "gitoxide.ssh.commandWithoutShellFallback=ssh-command-fallback-api",
                "gitoxide.http.proxyAuthMethod=proxy-auth-method-api",
                "gitoxide.core.shallowFile=shallow-file-api",
                "gitoxide.core.refsNamespace=namespace-api",
            ]);
        opts.permissions.env.git_prefix = Permission::Allow;
        opts.permissions.env.http_transport = Permission::Allow;
        opts.permissions.env.identity = Permission::Allow;
        opts.permissions.env.objects = Permission::Allow;
        let repo = named_subrepo_opts("make_config_repos.sh", "http-config", opts)?;
        assert_eq!(
            repo.config_snapshot().meta().source,
            gix::config::Source::Local,
            "config always refers to the local one for safety"
        );
        let config = repo.config_snapshot();
        assert_eq!(
            config.strings("gitoxide.core.shallowFile").expect("at least one value"),
            ["shallow-file-cli", "shallow-file-api", "shallow-file-env"]
        );
        assert_eq!(
            config
                .strings("gitoxide.core.refsNamespace")
                .expect("at least one value"),
            ["namespace-cli", "namespace-api", "namespace-env"]
        );
        assert_eq!(
            config.strings("http.userAgent").expect("at least one value"),
            ["agentJustForHttp", "agent-from-cli", "agent-from-api", "agent-from-env"]
        );
        assert_eq!(
            config.integers("http.lowSpeedLimit")?.expect("many values"),
            [5120, 3, 2, 1]
        );
        assert_eq!(
            config.integers("http.lowSpeedTime")?.expect("many values"),
            [10, 3, 2, 1]
        );
        assert_eq!(
            config.strings("http.proxyAuthMethod").expect("at least one value"),
            ["basic"],
            "this value isn't overridden directly"
        );
        assert_eq!(
            config.strings("gitoxide.https.proxy").expect("at least one value"),
            [
                "https-upper",
                if cfg!(windows) {
                    "https-upper" // on windows, environment variables are case-insensitive
                } else {
                    "https-lower-override"
                }
            ]
        );
        assert_eq!(
            config.strings("gitoxide.http.proxy").expect("at least one value"),
            ["http-lower"]
        );
        assert_eq!(
            config.strings("gitoxide.http.allProxy").expect("at least one value"),
            [
                "all-proxy", // on windows, environment variables are case-insensitive
                if cfg!(windows) { "all-proxy" } else { "all-proxy-lower" }
            ]
        );
        assert_eq!(
            config.strings("gitoxide.http.noProxy").expect("at least one value"),
            [
                "no-proxy", // on windows, environment variables are case-insensitive
                if cfg!(windows) { "no-proxy" } else { "no-proxy-lower" }
            ]
        );
        assert_eq!(
            config.strings("http.sslCAInfo").expect("at least one value"),
            ["./CA.pem", "./cli.pem", "./api.pem", "./env.pem"]
        );
        assert_eq!(
            config.strings("http.sslVersion").expect("at least one value"),
            ["sslv2", "sslv3", "tlsv1", "tlsv1.3"]
        );
        assert_eq!(
            config.strings("ssh.variant").expect("at least one value"),
            ["ssh-variant-cli", "ssh-variant-api", "ssh-variant-env"]
        );
        assert_eq!(
            config.strings("core.sshCommand").expect("at least one value"),
            ["ssh-command-cli", "ssh-command-api", "ssh-command-env"]
        );
        assert_eq!(
            config
                .strings("gitoxide.ssh.commandWithoutShellFallback")
                .expect("at least one value"),
            [
                "ssh-command-fallback-cli",
                "ssh-command-fallback-api",
                "ssh-command-fallback-env",
            ]
        );
        assert_eq!(
            config
                .strings("gitoxide.http.proxyAuthMethod")
                .expect("at least one value"),
            [
                "proxy-auth-method-cli",
                "proxy-auth-method-api",
                "proxy-auth-method-env"
            ]
        );
        for (key, expected) in [
            ("gitoxide.http.sslNoVerify", "true"),
            ("gitoxide.http.verbose", "true"),
            ("gitoxide.allow.protocolFromUser", "file-allowed"),
            ("core.useReplaceRefs", "no-replace"),
            #[cfg(feature = "blob-diff")]
            ("diff.external", "external-diff-env"),
            ("gitoxide.objects.replaceRefBase", "refs/replace-mine"),
            ("gitoxide.committer.nameFallback", "committer name"),
            ("gitoxide.committer.emailFallback", "committer email"),
            ("gitoxide.author.nameFallback", "author name"),
            ("gitoxide.author.emailFallback", "author email"),
            ("gitoxide.commit.authorDate", default_date),
            ("gitoxide.commit.committerDate", default_date),
            ("gitoxide.user.emailFallback", "user email"),
            ("gitoxide.parsePrecious", "1"),
            ("core.deltaBaseCacheLimit", "0"),
            ("gitoxide.objects.cacheLimit", "5m"),
            ("gitoxide.objects.allocLimit", "7m"),
            ("gitoxide.pathspec.icase", "pathspecs-icase"),
            ("gitoxide.pathspec.glob", "pathspecs-glob"),
            ("gitoxide.pathspec.noglob", "pathspecs-noglob"),
            ("gitoxide.pathspec.literal", "pathspecs-literal"),
            ("gitoxide.credentials.terminalPrompt", "42"),
            ("gitoxide.credentials.helperStderr", "creds-stderr"),
            ("gitoxide.core.externalCommandStderr", "filter-stderr"),
        ] {
            assert_eq!(
                config.string(key).unwrap_or_else(|| panic!("no value for {key}")),
                expected,
                "{key} == {expected}"
            );
        }
        Ok(())
    }
}

#[test]
#[serial]
fn git_worktree_and_strict_config() -> gix_testtools::Result {
    let _restore_env_on_drop = gix_testtools::Env::new().set("GIT_WORK_TREE", ".");
    let _repo = named_subrepo_opts(
        "make_empty_repo.sh",
        "",
        gix::open::Options::isolated()
            .permissions({
                let mut perm = Permissions::isolated();
                perm.env.git_prefix = Permission::Allow;
                perm
            })
            .strict_config(true),
    )?;
    Ok(())
}

#[test]
#[serial]
fn git_worktree_overrides_bare() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_read_only("make_config_repos.sh")?;
    let worktree = gix_testtools::tempfile::TempDir::new()?;
    let _env = gix_testtools::Env::new()
        .unset("GIT_DIR")
        .set("GIT_WORK_TREE", worktree.path().to_string_lossy());

    let repo =
        gix::ThreadSafeRepository::discover_with_environment_overrides(fixture.join("bare-repo"))?.to_thread_local();

    assert_eq!(
        repo.workdir(),
        Some(worktree.path()),
        "GIT_WORK_TREE overrides core.bare just like it does in Git"
    );
    assert!(
        !repo.is_bare(),
        "a repository with an explicit GIT_WORK_TREE is not bare according to Git"
    );
    #[cfg(feature = "status")]
    {
        std::fs::write(worktree.path().join("untracked"), b"content")?;
        assert_eq!(
            repo.status(gix::progress::Discard)?
                .into_index_worktree_iter(None)?
                .count(),
            1,
            "status observes files in the explicit worktree"
        );
    }
    Ok(())
}

#[test]
#[serial]
#[cfg(unix)]
fn git_worktree_over_root_overrides_bare() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_read_only("make_config_repos.sh")?;
    let worktree = gix_testtools::tempfile::TempDir::new()?;
    let current_dir = std::env::current_dir()?;
    let mut relative_worktree = std::path::PathBuf::new();
    for _ in 0..current_dir.components().count() + 2 {
        relative_worktree.push("..");
    }
    relative_worktree.push(worktree.path().strip_prefix("/")?);
    let _env = gix_testtools::Env::new()
        .unset("GIT_DIR")
        .set("GIT_WORK_TREE", relative_worktree.to_string_lossy());

    let repo =
        gix::ThreadSafeRepository::discover_with_environment_overrides(fixture.join("bare-repo"))?.to_thread_local();

    assert_eq!(
        repo.workdir(),
        Some(worktree.path()),
        "parent components beyond the root saturate there, just like they do in Git"
    );
    assert!(!repo.is_bare(), "the explicit worktree makes the repository non-bare");
    #[cfg(feature = "status")]
    {
        std::fs::write(worktree.path().join("untracked"), b"content")?;
        assert_eq!(
            repo.status(gix::progress::Discard)?
                .into_index_worktree_iter(None)?
                .count(),
            1,
            "status observes files through the over-root worktree path"
        );
    }
    Ok(())
}
