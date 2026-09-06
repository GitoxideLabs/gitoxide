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

mod config {
    use gix_sec::Permission;
    use serial_test::serial;

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

        let globals = gix::config(Some(std::path::Path::new("project/.git")), &options)?;
        assert_eq!(globals.boolean("marker.global")?, Some(true), "global files are loaded");
        assert_eq!(
            globals.boolean("marker.included")?,
            Some(true),
            "git-dir conditional includes use the provided future repository path"
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

        let repo = gix::ThreadSafeRepository::init_opts(
            &repo_path,
            gix::create::Kind::WithWorktree,
            Default::default(),
            options,
        )?
        .to_thread_local();
        let repo = repo.config_snapshot();

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
}

mod config_mut {
    use gix::config::{Source, file_mut::Error};
    use gix_sec::Permission;
    use gix_testtools::{Env, Result};
    use serial_test::serial;

    #[test]
    #[serial]
    fn edits_one_physical_file_losslessly_without_a_repository() -> Result {
        let temp = gix_testtools::tempfile::tempdir()?;
        let _cwd = gix_testtools::set_current_dir(temp.path())?;
        let path = std::env::current_dir()?.join("global.config");
        let included = path.with_file_name("included.config");
        let original = "# leading comment
[include]
    path = included.config
[marker]
    value = before
# trailing comment
";
        let included_contents = "[marker]
    included = true
";
        std::fs::write(&path, original)?;
        std::fs::write(&included, included_contents)?;
        let _env = Env::new().set("GIT_CONFIG_GLOBAL", path.display().to_string());
        let mut options = options_for(Source::User)
            .lossy_config(true)
            .cli_overrides(["marker.cli=transient"])
            .config_overrides(["marker.api=transient"]);
        options.permissions.config.includes = true;
        options.permissions.env.git_prefix = Permission::Allow;

        let mut file = gix::config_mut(Source::User, &options)?;
        assert_eq!(
            file.meta().source,
            Source::User,
            "metadata identifies the requested source"
        );
        assert_eq!(
            file.meta().path.as_deref(),
            Some(path.as_path()),
            "metadata identifies the physical file"
        );
        assert_eq!(
            file.meta().trust,
            gix_sec::Trust::Full,
            "global configuration is fully trusted"
        );
        for key in ["marker.included", "marker.cli", "marker.api"] {
            assert!(file.string(key).is_none(), "{key} is not part of the physical file");
        }
        file.set_raw_value("marker.value", "after")?;
        assert_eq!(
            std::fs::read_to_string(&path)?,
            original,
            "changes wait for an explicit commit"
        );
        file.commit()?;
        let written = original.replace("value = before", "value = after");
        assert_eq!(
            std::fs::read_to_string(&path)?,
            written,
            "comments, whitespace and includes survive lossy options"
        );
        assert_eq!(
            std::fs::read_to_string(&included)?,
            included_contents,
            "included files are untouched"
        );
        assert_eq!(
            gix::config(None, &options)?
                .string("marker.value")
                .expect("committed marker"),
            "after",
            "standalone reads see the committed value"
        );

        let mut file = gix::config_mut(Source::User, &options)?;
        file.set_raw_value("marker.value", "discarded")?;
        drop(file);
        assert_eq!(std::fs::read_to_string(&path)?, written, "dropping discards edits");
        assert!(
            !path.with_extension("config.lock").exists(),
            "dropping releases the lock"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn source_paths_match_standalone_reads() -> Result {
        let temp = gix_testtools::tempfile::tempdir()?;
        let home = temp.path();
        let xdg = home.join("xdg");
        std::fs::create_dir_all(xdg.join("git"))?;
        std::fs::create_dir_all(home.join(".config/git"))?;
        let _env = Env::new()
            .set("HOME", home.display().to_string())
            .set("XDG_CONFIG_HOME", xdg.display().to_string());
        let installation = home.join("installation.config");
        let system = home.join("system.config");

        for (source, path) in [
            (Source::GitInstallation, installation.clone()),
            (Source::System, system.clone()),
            (Source::Git, xdg.join("git/config")),
            (Source::User, home.join(".gitconfig")),
        ] {
            let mut options = options_for(source)
                .git_installation_config_path(&installation)
                .system_config_path(&system);
            options.permissions.env.home = Permission::Allow;
            options.permissions.env.xdg_config_home = Permission::Allow;
            let mut file = gix::config_mut(source, &options)?;
            assert_eq!(
                file.meta().path.as_deref(),
                Some(path.as_path()),
                "source resolves to its own file"
            );
            assert_eq!(file.meta().source, source, "new files retain their selected source");
            file.set_raw_value("marker.value", "created")?;
            assert_eq!(
                file.section_by_key("marker")?.meta().source,
                source,
                "new sections inherit the source"
            );
            file.commit()?;
            assert!(path.is_file(), "the selected missing file is created on commit");
            assert_eq!(
                gix::config(None, &options)?
                    .string("marker.value")
                    .expect("created marker"),
                "created",
                "reading uses the same source path"
            );
        }

        let mut options = options_for(Source::Git);
        options.permissions.env.home = Permission::Allow;
        let file = gix::config_mut(Source::Git, &options)?;
        assert_eq!(
            file.meta().path.as_deref(),
            Some(home.join(".config/git/config").as_path()),
            "denying XDG environment access falls back to HOME"
        );
        drop(file);
        options.permissions.env.home = Permission::Deny;
        assert!(
            matches!(
                gix::config_mut(Source::Git, &options),
                Err(Error::SourceUnavailable(Source::Git))
            ),
            "a source with no permitted path is unavailable"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn honors_environment_and_explicit_path_overrides() -> Result {
        let temp = gix_testtools::tempfile::tempdir()?;
        let global = temp.path().join("global.config");
        let system = temp.path().join("system.config");
        let explicit = temp.path().join("explicit.config");
        let _env = Env::new()
            .set("GIT_CONFIG_GLOBAL", global.display().to_string())
            .set("GIT_CONFIG_SYSTEM", system.display().to_string())
            .set("GIT_CONFIG_NOSYSTEM", "false");
        for source in [Source::Git, Source::User, Source::System] {
            let mut options = options_for(source);
            options.permissions.env.git_prefix = Permission::Allow;
            let file = gix::config_mut(source, &options)?;
            let expected = if source == Source::System { &system } else { &global };
            assert_eq!(
                file.meta().path.as_deref(),
                Some(expected.as_path()),
                "Git environment overrides select the target"
            );
        }

        for source in [Source::GitInstallation, Source::System] {
            let mut options = options_for(source)
                .git_installation_config_path(&explicit)
                .system_config_path(&explicit);
            options.permissions.env.git_prefix = Permission::Allow;
            let file = gix::config_mut(source, &options)?;
            assert_eq!(
                file.meta().path.as_deref(),
                Some(explicit.as_path()),
                "explicit options take precedence over environment paths"
            );
            drop(file);
            let _no_system = Env::new().set("GIT_CONFIG_NOSYSTEM", "true");
            assert!(
                matches!(gix::config_mut(source, &options), Err(Error::SourceUnavailable(actual)) if actual == source),
                "GIT_CONFIG_NOSYSTEM suppresses explicit installation and system paths"
            );
            options.permissions.env.git_prefix = Permission::Deny;
            assert!(
                gix::config_mut(source, &options).is_ok(),
                "denying environment access ignores GIT_CONFIG_NOSYSTEM, so the explicit path can be opened"
            );
        }

        let mut options = options_for(Source::User).config_overrides(["core.configLockTimeout=0"]);
        options.permissions.config.git = true;
        options.permissions.env.git_prefix = Permission::Allow;
        let _file = gix::config_mut(Source::User, &options)?;
        assert!(
            matches!(gix::config_mut(Source::Git, &options), Err(Error::AcquireLock(_))),
            "sources overridden to the same file share its lock"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn rejects_unsupported_and_disabled_sources() -> Result {
        let temp = gix_testtools::tempfile::tempdir()?;
        let options = gix::open::Options::isolated()
            .git_installation_config_path(temp.path().join("installation.config"))
            .system_config_path(temp.path().join("system.config"));
        for source in [Source::GitInstallation, Source::System, Source::Git, Source::User] {
            assert!(
                matches!(gix::config_mut(source, &options), Err(Error::SourceUnavailable(actual)) if actual == source),
                "disabled sources cannot be opened for writing"
            );
        }
        for source in [
            Source::Local,
            Source::Worktree,
            Source::Env,
            Source::Cli,
            Source::Api,
            Source::EnvOverride,
        ] {
            assert!(
                matches!(gix::config_mut(source, &options), Err(Error::UnsupportedSource(actual)) if actual == source),
                "repository and in-memory sources have no standalone transaction"
            );
        }
        assert_eq!(
            std::fs::read_dir(temp.path())?.count(),
            0,
            "rejected sources do not create files or locks"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn missing_files_require_existing_parent_directories() -> Result {
        let temp = gix_testtools::tempfile::tempdir()?;
        let path = temp.path().join("missing/global.config");
        let options = options_for(Source::System).system_config_path(&path);
        assert!(
            matches!(gix::config_mut(Source::System, &options), Err(Error::AcquireLock(gix::lock::acquire::Error::Io(err))) if err.kind() == std::io::ErrorKind::NotFound),
            "the parent directory must already exist"
        );
        assert!(
            !path.parent().expect("config parent").exists(),
            "opening does not create directories"
        );
        std::fs::create_dir(path.parent().expect("config parent"))?;
        let mut file = gix::config_mut(Source::System, &options)?;
        file.set_raw_value("marker.value", "discarded")?;
        drop(file);
        assert_eq!(
            std::fs::read_dir(path.parent().expect("config parent"))?.count(),
            0,
            "dropping a new file leaves neither the file nor its lock"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn malformed_configuration_cannot_be_overwritten() -> Result {
        let temp = gix_testtools::tempfile::tempdir()?;
        let path = temp.path().join("global.config");
        let options = options_for(Source::System).system_config_path(&path);
        let malformed = "[unterminated";
        std::fs::write(&path, malformed)?;
        assert!(
            matches!(gix::config_mut(Source::System, &options), Err(Error::Config(_))),
            "malformed configuration cannot be overwritten through a transaction"
        );
        assert_eq!(
            std::fs::read_to_string(&path)?,
            malformed,
            "failed opening preserves malformed input"
        );
        assert!(
            !path.with_extension("config.lock").exists(),
            "failed opening leaves no lock"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn lock_timeout_uses_global_configuration_and_option_precedence() -> Result {
        use gix::lock::acquire::{Error as LockError, Fail};

        let temp = gix_testtools::tempfile::tempdir()?;
        let global_config_path = temp.path().join("global.config");
        std::fs::write(
            &global_config_path,
            "[core]
    configLockTimeout = 0
",
        )?;
        std::fs::write(global_config_path.with_extension("config.lock"), "held")?;
        let options = options_for(Source::System)
            .system_config_path(&global_config_path)
            .strict_config(true)
            .cli_overrides(["core.configLockTimeout=invalid"])
            .config_overrides(["core.configLockTimeout=0"]);
        assert!(
            matches!(
                gix::config_mut(Source::System, &options),
                Err(Error::AcquireLock(LockError::PermanentlyLocked {
                    mode: Fail::Immediately,
                    ..
                }))
            ),
            "API overrides take precedence over CLI and disk values"
        );
        let options = options.filter_config_section(|meta| meta.source != Source::Api);
        assert!(
            matches!(gix::config_mut(Source::System, &options), Err(Error::LockTimeout(_))),
            "section filtering exposes the invalid CLI timeout in strict mode"
        );
        let options = options.strict_config(false);
        assert!(
            matches!(gix::config_mut(Source::System, &options), Err(Error::AcquireLock(LockError::PermanentlyLocked { mode: Fail::AfterDurationWithBackoff(duration), .. })) if duration == std::time::Duration::from_millis(1000)),
            "lenient invalid timeouts use the one-second default"
        );

        std::fs::write(
            &global_config_path,
            "[core]
    configLockTimeout = invalid
[include]
    path = included.config
",
        )?;
        std::fs::write(
            global_config_path.with_file_name("included.config"),
            "[core]
    configLockTimeout = 0
",
        )?;
        let mut options = options_for(Source::System)
            .system_config_path(&global_config_path)
            .strict_config(true);
        options.permissions.config.includes = true;
        assert!(
            matches!(
                gix::config_mut(Source::System, &options),
                Err(Error::AcquireLock(LockError::PermanentlyLocked {
                    mode: Fail::Immediately,
                    ..
                }))
            ),
            "expanded global includes can supply the lock timeout"
        );
        options.permissions.config.includes = false;
        assert!(
            matches!(gix::config_mut(Source::System, &options), Err(Error::LockTimeout(_))),
            "disabling includes exposes the invalid physical timeout"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn relative_paths_remain_anchored_when_the_current_directory_changes() -> Result {
        let temp = gix_testtools::tempfile::tempdir()?;
        let _cwd = gix_testtools::set_current_dir(temp.path())?;
        let root = std::env::current_dir()?;
        let elsewhere = root.join("elsewhere");
        std::fs::create_dir(&elsewhere)?;
        let options = options_for(Source::System).system_config_path("global.config");
        let mut file = gix::config_mut(Source::System, &options)?;
        assert_eq!(
            file.meta().path.as_deref(),
            Some(root.join("global.config").as_path()),
            "relative targets capture the opening directory"
        );
        file.set_raw_value("marker.value", "anchored")?;
        let _elsewhere = gix_testtools::set_current_dir(&elsewhere)?;
        file.commit()?;
        assert!(root.join("global.config").is_file(), "commit uses the captured target");
        assert!(
            !elsewhere.join("global.config").exists(),
            "the later working directory does not affect commit"
        );
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    #[serial]
    fn global_permissions_honor_shared_repository_policy() -> Result {
        use std::os::unix::fs::PermissionsExt;

        let temp = gix_testtools::tempfile::tempdir()?;
        let path = temp.path().join("global.config");
        let installation = temp.path().join("installation.config");
        std::fs::write(
            &installation,
            "[core]
    sharedRepository = 0664
",
        )?;
        let mut options = options_for(Source::System)
            .system_config_path(&path)
            .git_installation_config_path(&installation)
            .strict_config(true);
        options.permissions.config.git_binary = true;
        gix::config_mut(Source::System, &options)?.commit()?;
        assert_eq!(
            path.metadata()?.permissions().mode() & 0o777,
            0o664,
            "new global files honor the sharing policy from the resolved configuration"
        );
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        let mut file = gix::config_mut(Source::System, &options)?;
        file.set_raw_value("marker.value", "written")?;
        file.commit()?;
        assert_eq!(
            path.metadata()?.permissions().mode() & 0o777,
            0o600,
            "editing preserves existing permissions"
        );
        let options = options.config_overrides(["core.sharedRepository=invalid"]);
        assert!(
            matches!(
                gix::config_mut(Source::System, &options),
                Err(Error::InvalidSharedRepository(_))
            ),
            "invalid sharing policies in overrides are rejected even for existing files"
        );
        Ok(())
    }

    fn options_for(source: Source) -> gix::open::Options {
        let mut options = gix::open::Options::isolated();
        match source {
            Source::GitInstallation => options.permissions.config.git_binary = true,
            Source::System => options.permissions.config.system = true,
            Source::Git => options.permissions.config.git = true,
            Source::User => options.permissions.config.user = true,
            _ => panic!("test options require a repository-independent file"),
        }
        options
    }
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
            .set("GIT_ALLOW_PROTOCOL", "file:ssh")
            .set("GIT_PROTOCOL_FROM_USER", "false")
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
            .set("GIT_NOTES_DISPLAY_REF", "refs/notes/review:refs/notes/*")
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
            .set("GIT_INDEX_FILE", "index-file-env")
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
                "gitoxide.core.indexFile=index-file-cli",
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
                "gitoxide.core.indexFile=index-file-api",
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
            config.strings("gitoxide.core.indexFile").expect("at least one value"),
            ["index-file-cli", "index-file-api", "index-file-env"]
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
            ("gitoxide.allow.protocol", "file:ssh"),
            ("gitoxide.allow.protocolFromUser", "false"),
            ("core.useReplaceRefs", "no-replace"),
            #[cfg(feature = "blob-diff")]
            ("diff.external", "external-diff-env"),
            ("gitoxide.objects.replaceRefBase", "refs/replace-mine"),
            ("committer.name", "committer name"),
            ("committer.email", "committer email"),
            ("author.name", "author name"),
            ("author.email", "author email"),
            ("gitoxide.commit.authorDate", default_date),
            ("gitoxide.commit.committerDate", default_date),
            ("gitoxide.user.emailFallback", "user email"),
            ("notes.displayRef", "refs/notes/review:refs/notes/*"),
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

#[test]
#[serial]
fn repository_transitions_do_not_inherit_repository_environment_overrides() -> gix_testtools::Result {
    fn assert_paths(
        repo: &Repository,
        git_dir: &Path,
        worktree: &Path,
        index: &Path,
        context: &str,
    ) -> gix_testtools::Result {
        assert_eq!(
            gix_path::realpath(repo.git_dir())?,
            gix_path::realpath(git_dir)?,
            "{context}: git directory"
        );
        assert_eq!(
            repo.workdir().map(gix_path::realpath).transpose()?,
            Some(gix_path::realpath(worktree)?),
            "{context}: worktree"
        );
        assert_eq!(
            gix_path::realpath(repo.index_path())?,
            gix_path::realpath(index)?,
            "{context}: index"
        );
        Ok(())
    }

    let fixture = gix_testtools::scripted_fixture_read_only_needs_archive("make_worktree_relative_linking.sh")?;
    let main = std::fs::canonicalize(fixture.join("main"))?;
    let main_git_dir = main.join(".git");
    let main_index_file = main.join(".git/index");
    let git_dir_override = main_git_dir.clone();
    let worktree_override = fixture.to_owned();
    let index_override = main.join(".git/temporary-index");
    let _env = gix_testtools::Env::new()
        .set("GIT_DIR", git_dir_override.to_string_lossy())
        .set("GIT_WORK_TREE", worktree_override.to_string_lossy())
        .set("GIT_INDEX_FILE", index_override.to_string_lossy());
    let mut options = gix::open::Options::isolated();
    options.permissions.env.git_prefix = Permission::Allow;

    let mut main_repo = discover_with_environment_overrides_isolated(fixture.join("linked"))?;
    assert_paths(
        &main_repo,
        &git_dir_override,
        &worktree_override,
        &index_override,
        "the initial open uses all overrides",
    )?;
    main_repo.reload()?;
    assert_paths(
        &main_repo,
        &git_dir_override,
        &worktree_override,
        &index_override,
        "reloading the initial repository keeps all overrides",
    )?;
    let mut reopened_main_repo = main_repo.main_repo()?;
    assert_paths(
        &reopened_main_repo,
        &git_dir_override,
        &worktree_override,
        &index_override,
        "remaining in the main repository keeps all overrides",
    )?;
    reopened_main_repo.reload()?;
    assert_paths(
        &reopened_main_repo,
        &git_dir_override,
        &worktree_override,
        &index_override,
        "reloading the reopened main repository keeps all overrides",
    )?;

    let proxy = main_repo.worktrees()?.into_iter().next().expect("one linked worktree");
    let expected_linked_worktree = proxy.base()?;
    let expected_linked_git_dir = proxy.git_dir();
    let expected_linked_index = proxy.git_dir().join("index");
    let mut linked_repo_from_proxy = proxy.clone().into_repo()?;
    assert_paths(
        &linked_repo_from_proxy,
        expected_linked_git_dir,
        &expected_linked_worktree,
        &expected_linked_index,
        "opening a linked worktree uses its own repository paths",
    )?;
    linked_repo_from_proxy.reload()?;
    assert_paths(
        &linked_repo_from_proxy,
        expected_linked_git_dir,
        &expected_linked_worktree,
        &expected_linked_index,
        "reloading keeps the linked worktree repository paths",
    )?;
    let mut linked_repo_from_permissive_proxy = proxy.clone().into_repo_with_possibly_inaccessible_worktree()?;
    assert_paths(
        &linked_repo_from_permissive_proxy,
        expected_linked_git_dir,
        &expected_linked_worktree,
        &expected_linked_index,
        "the permissive proxy open also uses the linked worktree repository paths",
    )?;
    linked_repo_from_permissive_proxy.reload()?;
    assert_paths(
        &linked_repo_from_permissive_proxy,
        expected_linked_git_dir,
        &expected_linked_worktree,
        &expected_linked_index,
        "reloading the permissively opened repository keeps the linked worktree repository paths",
    )?;

    let mut linked_repo = gix::open_opts(proxy.base()?, options)?;
    assert_paths(
        &linked_repo,
        expected_linked_git_dir,
        &worktree_override,
        &index_override,
        "a direct open still uses repository environment overrides",
    )?;
    linked_repo.reload()?;
    assert_paths(
        &linked_repo,
        expected_linked_git_dir,
        &worktree_override,
        &index_override,
        "reloading a directly opened worktree keeps repository environment overrides",
    )?;
    let mut main_repo_from_linked = linked_repo.main_repo()?;
    assert_paths(
        &main_repo_from_linked,
        &main_git_dir,
        &main,
        &main_index_file,
        "returning to the main repository uses its own repository paths",
    )?;
    main_repo_from_linked.reload()?;
    assert_paths(
        &main_repo_from_linked,
        &main_git_dir,
        &main,
        &main_index_file,
        "reloading the main repository preserves its own repository paths",
    )?;
    Ok(())
}

#[test]
#[serial]
#[cfg(feature = "attributes")]
fn git_index_file_override_is_not_inherited_by_opened_submodules() -> gix_testtools::Result {
    let fixture = gix_testtools::scripted_fixture_read_only("make_submodules.sh")?;
    let superproject = std::fs::canonicalize(fixture.join("with-submodules"))?;
    let index_file = superproject.join(".git/index");
    let _env = gix_testtools::Env::new().set("GIT_INDEX_FILE", index_file.to_string_lossy());
    let mut options = gix::open::Options::isolated();
    options.permissions.env.git_prefix = Permission::Allow;
    let repo = gix::open_opts(superproject, options)?;
    assert_eq!(repo.index_path(), index_file, "the superproject uses the override");

    let submodule = repo
        .submodules()?
        .expect("modules present")
        .next()
        .expect("one submodule");
    let mut submodule = submodule.open()?.expect("initialized submodule");
    assert_eq!(
        submodule.index_path(),
        submodule.git_dir().join("index"),
        "submodules use their own index"
    );
    submodule.reload()?;
    assert_eq!(
        submodule.index_path(),
        submodule.git_dir().join("index"),
        "reloading preserves the submodule's own index"
    );
    Ok(())
}

#[test]
#[serial]
fn git_index_file_relative_paths_use_the_cwd_when_opening() -> gix_testtools::Result {
    let repository = gix_testtools::scripted_fixture_writable("make_basic_repo.sh")?;
    let _cwd = gix_testtools::set_current_dir(repository.path())?;
    let _env = gix_testtools::Env::new()
        .unset("GIT_DIR")
        .set("GIT_INDEX_FILE", "temporary-index");
    std::fs::copy(repository.path().join(".git/index"), "temporary-index")?;

    let repo = discover_with_environment_overrides_isolated(repository.path())?;

    assert_eq!(
        repo.index_path(),
        Path::new("temporary-index"),
        "relative paths start at the captured CWD, which is where Git invokes hooks"
    );
    assert!(
        repo.index_path().is_file(),
        "the repository retains its opening CWD, so the relative index path continues to identify the same file"
    );
    assert_ne!(
        repo.index_path(),
        repo.git_dir().join("temporary-index"),
        "index file overrides notably are not relative to the git-dir"
    );
    Ok(())
}
