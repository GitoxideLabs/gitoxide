#![allow(clippy::result_large_err)]
use std::{borrow::Cow, ffi::OsString};

use gix_sec::Permission;

use super::{interpolate_context, util, Error, StageOne};
use crate::{
    bstr::BString,
    config,
    config::{
        cache::util::ApplyLeniency,
        tree::{gitoxide, Core, Gitoxide, Http},
        Cache,
    },
    open,
    repository::init::setup_objects,
};

/// Initialization
impl Cache {
    #[allow(clippy::too_many_arguments)]
    pub fn from_stage_one(
        StageOne {
            git_dir_config,
            mut buf,
            lossy,
            is_bare,
            object_hash,
            reflog: _,
            precompose_unicode: _,
            protect_windows: _,
        }: StageOne,
        git_dir: &std::path::Path,
        branch_name: Option<&gix_ref::FullNameRef>,
        filter_config_section: fn(&gix_config::file::Metadata) -> bool,
        git_install_dir: Option<&std::path::Path>,
        home: Option<&std::path::Path>,
        environment @ open::permissions::Environment {
            git_prefix,
            ssh_prefix: _,
            xdg_config_home: _,
            home: _,
            http_transport,
            identity,
            objects,
        }: open::permissions::Environment,
        attributes: open::permissions::Attributes,
        open::permissions::Config {
            git_binary: use_installation,
            system: use_system,
            git: use_git,
            user: use_user,
            env: use_env,
            includes: use_includes,
        }: open::permissions::Config,
        lenient_config: bool,
        api_config_overrides: &[BString],
        cli_config_overrides: &[BString],
    ) -> Result<Self, Error> {
        let options = gix_config::file::init::Options {
            includes: if use_includes {
                gix_config::file::includes::Options::follow(
                    interpolate_context(git_install_dir, home),
                    gix_config::file::includes::conditional::Context {
                        git_dir: git_dir.into(),
                        branch_name,
                    },
                )
            } else {
                gix_config::file::includes::Options::no_follow()
            },
            ..util::base_options(lossy, lenient_config)
        };

        let config = {
            let git_prefix = &git_prefix;
            let mut metas = [
                gix_config::source::Kind::GitInstallation,
                gix_config::source::Kind::System,
                gix_config::source::Kind::Global,
            ]
            .iter()
            .flat_map(|kind| kind.sources())
            .filter_map(|source| {
                match source {
                    gix_config::Source::GitInstallation if !use_installation => return None,
                    gix_config::Source::System if !use_system => return None,
                    gix_config::Source::Git if !use_git => return None,
                    gix_config::Source::User if !use_user => return None,
                    _ => {}
                }
                source
                    .storage_location(&mut Self::make_source_env(environment))
                    .map(|p| (source, p.into_owned()))
            })
            .map(|(source, path)| gix_config::file::Metadata {
                path: Some(path),
                source: *source,
                level: 0,
                trust: gix_sec::Trust::Full,
            });

            let err_on_nonexisting_paths = false;
            let mut globals = gix_config::File::from_paths_metadata_buf(
                &mut metas,
                &mut buf,
                err_on_nonexisting_paths,
                gix_config::file::init::Options {
                    includes: gix_config::file::includes::Options::no_follow(),
                    ..options
                },
            )
            .map_err(|err| match err {
                gix_config::file::init::from_paths::Error::Init(err) => Error::from(err),
                gix_config::file::init::from_paths::Error::Io { source, path } => Error::Io { source, path },
            })?
            .unwrap_or_default();

            let local_meta = git_dir_config.meta_owned();
            globals.append(git_dir_config);
            globals.resolve_includes(options)?;
            if use_env {
                globals.append(gix_config::File::from_env(options)?.unwrap_or_default());
            }
            if !cli_config_overrides.is_empty() {
                config::overrides::append(&mut globals, cli_config_overrides, gix_config::Source::Cli, |_| None)
                    .map_err(|err| Error::ConfigOverrides {
                        err,
                        source: gix_config::Source::Cli,
                    })?;
            }
            if !api_config_overrides.is_empty() {
                config::overrides::append(&mut globals, api_config_overrides, gix_config::Source::Api, |_| None)
                    .map_err(|err| Error::ConfigOverrides {
                        err,
                        source: gix_config::Source::Api,
                    })?;
            }
            apply_environment_overrides(&mut globals, *git_prefix, http_transport, identity, objects)?;
            globals.set_meta(local_meta);
            globals
        };

        let hex_len = util::parse_core_abbrev(&config, object_hash).with_leniency(lenient_config)?;

        use util::config_bool;
        let reflog = util::query_refupdates(&config, lenient_config)?;
        let refs_namespace = util::query_refs_namespace(&config, lenient_config)?;
        let ignore_case = config_bool(&config, &Core::IGNORE_CASE, "core.ignoreCase", false, lenient_config)?;
        let use_multi_pack_index = config_bool(
            &config,
            &Core::MULTIPACK_INDEX,
            "core.multiPackIndex",
            true,
            lenient_config,
        )?;
        #[cfg(feature = "revision")]
        let object_kind_hint = util::disambiguate_hint(&config, lenient_config)?;
        let (static_pack_cache_limit_bytes, pack_cache_bytes, object_cache_bytes) =
            util::parse_object_caches(&config, lenient_config, filter_config_section)?;
        // NOTE: When adding a new initial cache, consider adjusting `reread_values_and_clear_caches()` as well.
        Ok(Cache {
            resolved: config.into(),
            use_multi_pack_index,
            object_hash,
            #[cfg(feature = "revision")]
            object_kind_hint,
            static_pack_cache_limit_bytes,
            pack_cache_bytes,
            object_cache_bytes,
            reflog,
            refs_namespace,
            is_bare,
            ignore_case,
            hex_len,
            filter_config_section,
            environment,
            lenient_config,
            attributes,
            user_agent: Default::default(),
            personas: Default::default(),
            url_rewrite: Default::default(),
            #[cfg(feature = "blob-diff")]
            diff_renames: Default::default(),
            #[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
            url_scheme: Default::default(),
            #[cfg(feature = "blob-diff")]
            diff_algorithm: Default::default(),
        })
    }

    /// Call this with new `config` to update values and clear caches. Note that none of the values will be applied if a single
    /// one is invalid.
    /// However, those that are lazily read won't be re-evaluated right away and might thus pass now but fail later.
    ///
    /// Note that we unconditionally re-read all values.
    pub fn reread_values_and_clear_caches_replacing_config(&mut self, config: crate::Config) -> Result<(), Error> {
        let prev = std::mem::replace(&mut self.resolved, config);
        match self.reread_values_and_clear_caches() {
            Err(err) => {
                drop(std::mem::replace(&mut self.resolved, prev));
                Err(err)
            }
            Ok(()) => Ok(()),
        }
    }

    /// Similar to `reread_values_and_clear_caches_replacing_config()`, but works on the existing configuration instead of a passed
    /// in one that it them makes the default.
    pub fn reread_values_and_clear_caches(&mut self) -> Result<(), Error> {
        let config = &self.resolved;
        let hex_len = util::parse_core_abbrev(config, self.object_hash).with_leniency(self.lenient_config)?;

        use util::config_bool;
        let ignore_case = config_bool(
            config,
            &Core::IGNORE_CASE,
            "core.ignoreCase",
            false,
            self.lenient_config,
        )?;

        #[cfg(feature = "revision")]
        {
            let object_kind_hint = util::disambiguate_hint(config, self.lenient_config)?;
            self.object_kind_hint = object_kind_hint;
        }
        let reflog = util::query_refupdates(config, self.lenient_config)?;
        let refs_namespace = util::query_refs_namespace(config, self.lenient_config)?;

        self.hex_len = hex_len;
        self.ignore_case = ignore_case;
        self.reflog = reflog;
        self.refs_namespace = refs_namespace;

        self.user_agent = Default::default();
        self.personas = Default::default();
        self.url_rewrite = Default::default();
        #[cfg(feature = "blob-diff")]
        {
            self.diff_renames = Default::default();
            self.diff_algorithm = Default::default();
        }
        (
            self.static_pack_cache_limit_bytes,
            self.pack_cache_bytes,
            self.object_cache_bytes,
        ) = util::parse_object_caches(config, self.lenient_config, self.filter_config_section)?;
        #[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
        {
            self.url_scheme = Default::default();
        }

        Ok(())
    }

    pub(crate) fn make_source_env(
        crate::open::permissions::Environment {
            xdg_config_home,
            git_prefix,
            home,
            ..
        }: open::permissions::Environment,
    ) -> impl FnMut(&str) -> Option<OsString> {
        move |name| {
            match name {
                git_ if git_.starts_with("GIT_") => Some(git_prefix),
                "XDG_CONFIG_HOME" => Some(xdg_config_home),
                "HOME" => {
                    return if home.is_allowed() {
                        gix_path::env::home_dir().map(Into::into)
                    } else {
                        None
                    }
                }
                _ => None,
            }
            .and_then(|perm| perm.check_opt(name).and_then(gix_path::env::var))
        }
    }
}

impl crate::Repository {
    /// Replace our own configuration with `config` and re-read all cached values, and apply them to select in-memory instances.
    pub(crate) fn reread_values_and_clear_caches_replacing_config(
        &mut self,
        config: crate::Config,
    ) -> Result<(), Error> {
        let (a, b, c) = (
            self.config.static_pack_cache_limit_bytes,
            self.config.pack_cache_bytes,
            self.config.object_cache_bytes,
        );
        self.config.reread_values_and_clear_caches_replacing_config(config)?;
        self.apply_changed_values();
        if a != self.config.static_pack_cache_limit_bytes
            || b != self.config.pack_cache_bytes
            || c != self.config.object_cache_bytes
        {
            setup_objects(&mut self.objects, &self.config);
        }
        Ok(())
    }

    fn apply_changed_values(&mut self) {
        self.refs.write_reflog = util::reflog_or_default(self.config.reflog, self.workdir().is_some());
        self.refs.namespace.clone_from(&self.config.refs_namespace);
    }
}

fn apply_environment_overrides(
    config: &mut gix_config::File<'static>,
    git_prefix: Permission,
    http_transport: Permission,
    identity: Permission,
    objects: Permission,
) -> Result<(), Error> {
    fn env(key: &'static dyn config::tree::Key) -> &'static str {
        key.the_environment_override()
    }
    fn var_as_bstring(var: &str, perm: Permission) -> Option<BString> {
        perm.check_opt(var)
            .and_then(std::env::var_os)
            .and_then(|val| gix_path::os_string_into_bstring(val).ok())
    }

    let mut env_override = gix_config::File::new(gix_config::file::Metadata::from(gix_config::Source::EnvOverride));
    for (section_name, subsection_name, permission, data) in [
        (
            "core",
            None,
            git_prefix,
            &[{
                let key = &Core::WORKTREE;
                (env(key), key.name)
            }][..],
        ),
        (
            "http",
            None,
            http_transport,
            &[
                ("GIT_HTTP_LOW_SPEED_LIMIT", "lowSpeedLimit"),
                ("GIT_HTTP_LOW_SPEED_TIME", "lowSpeedTime"),
                ("GIT_HTTP_USER_AGENT", "userAgent"),
                {
                    let key = &Http::SSL_CA_INFO;
                    (env(key), key.name)
                },
                {
                    let key = &Http::SSL_VERSION;
                    (env(key), key.name)
                },
            ][..],
        ),
        (
            "gitoxide",
            None,
            git_prefix,
            &[{
                let key = &Gitoxide::TRACE_PACKET;
                (env(key), key.name)
            }],
        ),
        (
            "gitoxide",
            Some(Cow::Borrowed("https".into())),
            http_transport,
            &[
                ("HTTPS_PROXY", gitoxide::Https::PROXY.name),
                ("https_proxy", gitoxide::Https::PROXY.name),
            ],
        ),
        (
            "gitoxide",
            Some(Cow::Borrowed("http".into())),
            http_transport,
            &[
                ("ALL_PROXY", "allProxy"),
                {
                    let key = &gitoxide::Http::ALL_PROXY;
                    (env(key), key.name)
                },
                ("NO_PROXY", "noProxy"),
                {
                    let key = &gitoxide::Http::NO_PROXY;
                    (env(key), key.name)
                },
                {
                    let key = &gitoxide::Http::PROXY;
                    (env(key), key.name)
                },
                {
                    let key = &gitoxide::Http::VERBOSE;
                    (env(key), key.name)
                },
                {
                    let key = &gitoxide::Http::PROXY_AUTH_METHOD;
                    (env(key), key.name)
                },
            ],
        ),
        (
            "gitoxide",
            Some(Cow::Borrowed("http".into())),
            git_prefix,
            &[{
                let key = &gitoxide::Http::SSL_NO_VERIFY;
                (env(key), key.name)
            }],
        ),
        (
            "gitoxide",
            Some(Cow::Borrowed("credentials".into())),
            git_prefix,
            &[
                {
                    let key = &gitoxide::Credentials::TERMINAL_PROMPT;
                    (env(key), key.name)
                },
                {
                    let key = &gitoxide::Credentials::HELPER_STDERR;
                    (env(key), key.name)
                },
            ],
        ),
        (
            "gitoxide",
            Some(Cow::Borrowed("committer".into())),
            identity,
            &[
                {
                    let key = &gitoxide::Committer::NAME_FALLBACK;
                    (env(key), key.name)
                },
                {
                    let key = &gitoxide::Committer::EMAIL_FALLBACK;
                    (env(key), key.name)
                },
            ],
        ),
        (
            "gitoxide",
            Some(Cow::Borrowed("core".into())),
            git_prefix,
            &[
                {
                    let key = &gitoxide::Core::SHALLOW_FILE;
                    (env(key), key.name)
                },
                {
                    let key = &gitoxide::Core::REFS_NAMESPACE;
                    (env(key), key.name)
                },
                {
                    let key = &gitoxide::Core::EXTERNAL_COMMAND_STDERR;
                    (env(key), key.name)
                },
            ],
        ),
        (
            "gitoxide",
            Some(Cow::Borrowed("author".into())),
            identity,
            &[
                {
                    let key = &gitoxide::Author::NAME_FALLBACK;
                    (env(key), key.name)
                },
                {
                    let key = &gitoxide::Author::EMAIL_FALLBACK;
                    (env(key), key.name)
                },
            ],
        ),
        (
            "gitoxide",
            Some(Cow::Borrowed("commit".into())),
            git_prefix,
            &[
                {
                    let key = &gitoxide::Commit::COMMITTER_DATE;
                    (env(key), key.name)
                },
                {
                    let key = &gitoxide::Commit::AUTHOR_DATE;
                    (env(key), key.name)
                },
            ],
        ),
        (
            "gitoxide",
            Some(Cow::Borrowed("allow".into())),
            http_transport,
            &[("GIT_PROTOCOL_FROM_USER", "protocolFromUser")],
        ),
        (
            "gitoxide",
            Some(Cow::Borrowed("user".into())),
            identity,
            &[{
                let key = &gitoxide::User::EMAIL_FALLBACK;
                (env(key), key.name)
            }],
        ),
        (
            "gitoxide",
            Some(Cow::Borrowed("objects".into())),
            objects,
            &[
                {
                    let key = &gitoxide::Objects::REPLACE_REF_BASE;
                    (env(key), key.name)
                },
                {
                    let key = &gitoxide::Objects::CACHE_LIMIT;
                    (env(key), key.name)
                },
            ],
        ),
        (
            "gitoxide",
            Some(Cow::Borrowed("ssh".into())),
            git_prefix,
            &[{
                let key = &gitoxide::Ssh::COMMAND_WITHOUT_SHELL_FALLBACK;
                (env(key), key.name)
            }],
        ),
        (
            "gitoxide",
            Some(Cow::Borrowed("pathspec".into())),
            git_prefix,
            &[
                {
                    let key = &gitoxide::Pathspec::LITERAL;
                    (env(key), key.name)
                },
                {
                    let key = &gitoxide::Pathspec::GLOB;
                    (env(key), key.name)
                },
                {
                    let key = &gitoxide::Pathspec::NOGLOB;
                    (env(key), key.name)
                },
                {
                    let key = &gitoxide::Pathspec::ICASE;
                    (env(key), key.name)
                },
            ],
        ),
        (
            "ssh",
            None,
            git_prefix,
            &[{
                let key = &config::tree::Ssh::VARIANT;
                (env(key), key.name)
            }],
        ),
        #[cfg(feature = "blob-diff")]
        (
            "diff",
            None,
            git_prefix,
            &[{
                let key = &config::tree::Diff::EXTERNAL;
                (env(key), key.name)
            }],
        ),
    ] {
        let mut section = env_override
            .new_section(section_name, subsection_name)
            .expect("statically known valid section name");
        for (var, key) in data {
            if let Some(value) = var_as_bstring(var, permission) {
                section.push_with_comment(
                    (*key).try_into().expect("statically known to be valid"),
                    Some(value.as_ref()),
                    format!("from {var}").as_str(),
                );
            }
        }
        if section.num_values() == 0 {
            let id = section.id();
            env_override.remove_section_by_id(id);
        }
    }

    {
        let mut section = env_override
            .new_section("core", None)
            .expect("statically known valid section name");

        for (var, key, permission) in [
            {
                let key = &Core::DELTA_BASE_CACHE_LIMIT;
                (env(key), key.name, objects)
            },
            {
                let key = &Core::SSH_COMMAND;
                (env(key), key.name, git_prefix)
            },
            {
                let key = &Core::USE_REPLACE_REFS;
                (env(key), key.name, objects)
            },
        ] {
            if let Some(value) = var_as_bstring(var, permission) {
                section.push_with_comment(
                    key.try_into().expect("statically known to be valid"),
                    Some(value.as_ref()),
                    format!("from {var}").as_str(),
                );
            }
        }

        if section.num_values() == 0 {
            let id = section.id();
            env_override.remove_section_by_id(id);
        }
    }

    if !env_override.is_void() {
        config.append(env_override);
    }
    Ok(())
}
