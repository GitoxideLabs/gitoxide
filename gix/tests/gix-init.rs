#![allow(clippy::result_large_err)]

use std::path::Path;

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

fn discover_with_environment_overrides_isolated(
    directory: impl AsRef<Path>,
) -> Result<Repository, gix::discover::Error> {
    let mut options = gix::open::Options::isolated();
    options.permissions.env.git_prefix = Permission::Allow;
    ThreadSafeRepository::discover_with_environment_overrides_opts(
        directory,
        Default::default(),
        gix_sec::trust::Mapping {
            full: options.clone(),
            reduced: options,
        },
    )
    .map(|repo| repo.to_thread_local())
}

#[test]
#[serial]
fn globals_from_open_options_match_repository_opening() -> gix_testtools::Result {
    let temp = gix_testtools::tempfile::TempDir::new()?;
    let _cwd = gix_testtools::set_current_dir(temp.path())?;

    let repo_path = std::env::current_dir()?.join("project");
    let git_dir = repo_path.join(".git");
    let included = temp.path().join("included.config");
    let global = temp.path().join("global.config");
    std::fs::write(
        &included,
        "[marker]
            included = true",
    )?;
    std::fs::write(
        &global,
        format!(
            "[marker]
                global = true
            [includeIf \"gitdir:{git_dir}\"]
                path = {included}
            [includeIf \"gitdir:**/unrelated/.git\"]
                path = {included}",
            git_dir = git_dir.display().to_string().replace('\\', "/"),
            included = included.display().to_string().replace('\\', "/"),
        ),
    )?;
    let _env = gix_testtools::Env::new().set("GIT_CONFIG_GLOBAL", global.display().to_string());

    let mut permissions = gix::open::Permissions::isolated();
    permissions.config.user = true;
    permissions.config.includes = true;
    permissions.env.git_prefix = Permission::Allow;
    let options = gix::open::Options::isolated()
        .permissions(permissions)
        .cli_overrides(["marker.precedence=cli"])
        .config_overrides(["marker.precedence=api"]);

    let repo = gix::ThreadSafeRepository::init_opts(
        &repo_path,
        gix::create::Kind::WithWorktree,
        Default::default(),
        options.clone(),
    )?
    .to_thread_local();
    let repo = repo.config_snapshot();

    let globals = gix::config(Some(std::path::Path::new("project/.git")), &options)?;
    assert_eq!(globals.boolean("marker.global")?, Some(true), "global files are loaded");
    assert_eq!(
        globals.boolean("marker.included")?,
        Some(true),
        "git-dir conditional includes use the provided repository path"
    );
    assert_eq!(
        globals.string("marker.precedence").expect("API override is present"),
        "api",
        "API overrides retain repository-opening precedence"
    );

    let globals_without_repo = gix::config(None, &options)?;
    assert_eq!(
        globals_without_repo.boolean("marker.global")?,
        Some(true),
        "global files are loaded without repository context"
    );
    assert_eq!(
        globals_without_repo.boolean("marker.included")?,
        None,
        "git-dir conditional includes aren't loaded without repository context"
    );

    for key in ["marker.global", "marker.included"] {
        assert_eq!(
            globals.boolean(key)?,
            repo.try_boolean(key)?,
            "{key} is loaded identically before and during repository opening"
        );
    }
    assert_eq!(
        globals.string("marker.precedence"),
        repo.string("marker.precedence"),
        "overrides are loaded identically before and during repository opening"
    );
    Ok(())
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
fn git_worktree_overrides_core_worktree_and_bare() -> gix_testtools::Result {
    use std::io::Write;

    let bare = gix_testtools::tempfile::TempDir::new()?;
    gix::init_bare(bare.path())?;
    let worktree = gix_testtools::tempfile::TempDir::new()?;
    let configured_worktree = gix_testtools::tempfile::TempDir::new()?;
    writeln!(
        std::fs::OpenOptions::new()
            .append(true)
            .open(bare.path().join("config"))?,
        "\n[core]\n\tworktree = {wt_path}",
        wt_path = configured_worktree.path().to_string_lossy().replace('\\', "/")
    )?;
    let _env = gix_testtools::Env::new()
        .unset("GIT_DIR")
        .set("GIT_WORK_TREE", worktree.path().to_string_lossy());

    let repo = gix::discover_opts(bare.path(), Default::default(), gix::open::Options::isolated())?;
    assert_eq!(
        repo.workdir(),
        None,
        "without environment overrides the bare repository has no worktree even if configured"
    );
    assert!(repo.is_bare(), "without environment overrides it remains bare");

    let repo = discover_with_environment_overrides_isolated(bare.path())?;

    assert_eq!(
        repo.workdir(),
        Some(worktree.path()),
        "GIT_WORK_TREE overrides core.worktree and core.bare just like it does in Git"
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
fn git_worktree_overrides_discovered_worktree() -> gix_testtools::Result {
    let repository = gix_testtools::tempfile::TempDir::new()?;
    gix::init(repository.path())?;
    let worktree = gix_testtools::tempfile::TempDir::new()?;
    let _env = gix_testtools::Env::new()
        .unset("GIT_DIR")
        .set("GIT_WORK_TREE", worktree.path().to_string_lossy());

    let repo = discover_with_environment_overrides_isolated(repository.path())?;

    assert_eq!(
        repo.workdir(),
        Some(worktree.path()),
        "GIT_WORK_TREE takes precedence over the worktree found during discovery"
    );
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
    // Use more parent components than `current_dir` has components: the `+2` ensures that
    // at least one `..` remains after reaching the filesystem root.
    for _ in 0..current_dir.components().count() + 2 {
        relative_worktree.push("..");
    }
    // The absolute worktree path, without its leading `/`, is appended after those excess `..` components.
    // Thus Git ignores the excess parents at `/` and then resolves this suffix back to `worktree`.
    relative_worktree.push(worktree.path().strip_prefix("/")?);
    let _env = gix_testtools::Env::new()
        .unset("GIT_DIR")
        .set("GIT_WORK_TREE", relative_worktree.to_string_lossy());

    let repo = gix::discover_opts(
        fixture.join("bare-repo"),
        Default::default(),
        gix::open::Options::isolated(),
    )?;
    assert_eq!(
        repo.workdir(),
        None,
        "without environment overrides the bare repository has no worktree"
    );
    assert!(repo.is_bare(), "without environment overrides it remains bare");

    let repo = discover_with_environment_overrides_isolated(fixture.join("bare-repo"))?;

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

#[test]
#[serial]
#[cfg(unix)]
fn git_worktree_absolute_over_root_overrides_bare() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_read_only("make_config_repos.sh")?;
    let worktree = gix_testtools::tempfile::TempDir::new()?;
    let mut absolute_worktree = std::path::PathBuf::from("/");
    absolute_worktree.push("..");
    absolute_worktree.push(worktree.path().strip_prefix("/")?);
    let _env = gix_testtools::Env::new()
        .unset("GIT_DIR")
        .set("GIT_WORK_TREE", absolute_worktree.to_string_lossy());

    let repo = discover_with_environment_overrides_isolated(fixture.join("bare-repo"))?;

    assert_eq!(
        repo.workdir(),
        Some(worktree.path()),
        "an absolute path with `..` beyond the root resolves to the configured worktree"
    );
    Ok(())
}
