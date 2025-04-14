use crate::{
    config,
    config::tree::{keys, Gitoxide, Key, Section},
};

impl Gitoxide {
    /// The `gitoxide.allow` section.
    pub const ALLOW: Allow = Allow;
    /// The `gitoxide.author` section.
    pub const AUTHOR: Author = Author;
    /// The `gitoxide.core` section.
    pub const CORE: Core = Core;
    /// The `gitoxide.commit` section.
    pub const COMMIT: Commit = Commit;
    /// The `gitoxide.committer` section.
    pub const COMMITTER: Committer = Committer;
    /// The `gitoxide.credentials` section.
    pub const CREDENTIALS: Credentials = Credentials;
    /// The `gitoxide.http` section.
    pub const HTTP: Http = Http;
    /// The `gitoxide.https` section.
    pub const HTTPS: Https = Https;
    /// The `gitoxide.objects` section.
    pub const OBJECTS: Objects = Objects;
    /// The `gitoxide.ssh` section.
    pub const SSH: Ssh = Ssh;
    /// The `gitoxide.user` section.
    pub const USER: User = User;
    /// The `gitoxide.pathspec` section.
    pub const PATHSPEC: Pathspec = Pathspec;

    /// The `gitoxide.userAgent` Key.
    pub const USER_AGENT: keys::Any = keys::Any::new("userAgent", &config::Tree::GITOXIDE).with_note(
        "The user agent presented on the git protocol layer, serving as fallback for when no `http.userAgent` is set",
    );
    /// The `gitoxide.tracePacket` Key.
    pub const TRACE_PACKET: keys::Boolean = keys::Boolean::new_boolean("tracePacket", &config::Tree::GITOXIDE)
        .with_environment_override("GIT_TRACE_PACKET");
}

impl Section for Gitoxide {
    fn name(&self) -> &str {
        "gitoxide"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::USER_AGENT, &Self::TRACE_PACKET]
    }

    fn sub_sections(&self) -> &[&dyn Section] {
        &[
            &Self::ALLOW,
            &Self::AUTHOR,
            &Self::CORE,
            &Self::COMMIT,
            &Self::COMMITTER,
            &Self::CREDENTIALS,
            &Self::HTTP,
            &Self::HTTPS,
            &Self::OBJECTS,
            &Self::SSH,
            &Self::USER,
            &Self::PATHSPEC,
        ]
    }
}

mod subsections {
    use crate::config::{
        tree::{http, keys, Gitoxide, Key, Section},
        Tree,
    };

    /// The `Core` sub-section.
    #[derive(Copy, Clone, Default)]
    pub struct Core;

    /// The `gitoxide.allow.protocolFromUser` key.
    pub type RefsNamespace = keys::Any<super::validate::RefsNamespace>;

    impl RefsNamespace {
        /// Derive the negotiation algorithm identified by `name`, case-sensitively.
        pub fn try_into_refs_namespace(
            &'static self,
            name: std::borrow::Cow<'_, crate::bstr::BStr>,
        ) -> Result<gix_ref::Namespace, crate::config::refs_namespace::Error> {
            gix_ref::namespace::expand(name.as_ref())
                .map_err(|err| crate::config::key::Error::from_value(self, name.into_owned()).with_source(err))
        }
    }

    impl Core {
        /// The `gitoxide.core.defaultPackCacheMemoryLimit` key.
        pub const DEFAULT_PACK_CACHE_MEMORY_LIMIT: keys::UnsignedInteger =
            keys::UnsignedInteger::new_unsigned_integer("defaultPackCacheMemoryLimit", &Gitoxide::CORE).with_note(
                "If unset, we default to 96MB memory cap for the default 64 slot LRU cache for object deltas.",
            );
        /// The `gitoxide.core.useNsec` key.
        pub const USE_NSEC: keys::Boolean = keys::Boolean::new_boolean("useNsec", &Gitoxide::CORE)
            .with_note("A runtime version of the USE_NSEC build flag.");

        /// The `gitoxide.core.useStdev` key.
        pub const USE_STDEV: keys::Boolean = keys::Boolean::new_boolean("useStdev", &Gitoxide::CORE)
            .with_note("A runtime version of the USE_STDEV build flag.");

        /// The `gitoxide.core.protectWindows` key.
        pub const PROTECT_WINDOWS: keys::Boolean = keys::Boolean::new_boolean("protectWindows", &Gitoxide::CORE)
            .with_note("enable protections that are enabled by default on Windows");

        /// The `gitoxide.core.shallowFile` key.
        pub const SHALLOW_FILE: keys::Path = keys::Path::new_path("shallowFile", &Gitoxide::CORE)
            .with_environment_override("GIT_SHALLOW_FILE")
            .with_deviation(
                "relative file paths will always be made relative to the git-common-dir, whereas `git` keeps them as is.",
            );

        /// The `gitoxide.core.filterProcessDelay` key (default `true`).
        ///
        /// It controls whether or not long running filter driver processes can use the 'delay' capability.
        pub const FILTER_PROCESS_DELAY: keys::Boolean =
            keys::Boolean::new_boolean("filterProcessDelay", &Gitoxide::CORE);

        /// The `gitoxide.core.externalCommandStderr` key (default `true`).
        ///
        /// If `true`, the default, `stderr` of worktree filter programs, or any other git-context bearing command
        /// invoked will be inherited.
        /// If `false`, it will be suppressed completely.
        pub const EXTERNAL_COMMAND_STDERR: keys::Boolean =
            keys::Boolean::new_boolean("externalCommandStderr", &Gitoxide::CORE)
                .with_environment_override("GIX_EXTERNAL_COMMAND_STDERR");

        /// The `gitoxide.core.refsNamespace` key.
        pub const REFS_NAMESPACE: RefsNamespace =
            keys::Any::new_with_validate("refsNamespace", &Gitoxide::CORE, super::validate::RefsNamespace)
                .with_environment_override("GIT_NAMESPACE");
    }

    impl Section for Core {
        fn name(&self) -> &str {
            "core"
        }

        fn keys(&self) -> &[&dyn Key] {
            &[
                &Self::DEFAULT_PACK_CACHE_MEMORY_LIMIT,
                &Self::USE_NSEC,
                &Self::USE_STDEV,
                &Self::SHALLOW_FILE,
                &Self::PROTECT_WINDOWS,
                &Self::FILTER_PROCESS_DELAY,
                &Self::EXTERNAL_COMMAND_STDERR,
                &Self::REFS_NAMESPACE,
            ]
        }

        fn parent(&self) -> Option<&dyn Section> {
            Some(&Tree::GITOXIDE)
        }
    }

    /// The `Http` sub-section.
    #[derive(Copy, Clone, Default)]
    pub struct Http;

    impl Http {
        /// The `gitoxide.http.proxy` key.
        pub const PROXY: keys::String =
            keys::String::new_string("proxy", &Gitoxide::HTTP).with_environment_override("http_proxy");
        /// The `gitoxide.http.allProxy` key.
        pub const ALL_PROXY: keys::String = keys::String::new_string("allProxy", &Gitoxide::HTTP)
            .with_environment_override("all_proxy")
            .with_note("fallback environment is `ALL_PROXY`");
        /// The `gitoxide.http.verbose` key.
        ///
        /// If set, curl will be configured to log verbosely.
        pub const VERBOSE: keys::Boolean = keys::Boolean::new_boolean("verbose", &Gitoxide::HTTP)
            .with_environment_override("GIT_CURL_VERBOSE")
            .with_deviation("we parse it as boolean for convenience (infallible) but git only checks the presence");
        /// The `gitoxide.http.noProxy` key.
        pub const NO_PROXY: keys::String = keys::String::new_string("noProxy", &Gitoxide::HTTP)
            .with_environment_override("no_proxy")
            .with_note("fallback environment is `NO_PROXY`");
        /// The `gitoxide.http.connectTimeout` key.
        pub const CONNECT_TIMEOUT: keys::DurationInMilliseconds =
            keys::DurationInMilliseconds::new_duration("connectTimeout", &Gitoxide::HTTP).with_note(
                "entirely new, and in milliseconds, to describe how long to wait until a connection attempt is aborted",
            );
        /// The `gitoxide.http.sslVersionMin` key.
        pub const SSL_VERSION_MIN: http::SslVersion =
            http::SslVersion::new_ssl_version("sslVersionMin", &Gitoxide::HTTP).with_note(
                "entirely new to set the lower bound for the allowed ssl version range. Overwrites the min bound of `http.sslVersion` if set. Min and Max must be set to become effective.",
            );
        /// The `gitoxide.http.sslVersionMax` key.
        pub const SSL_VERSION_MAX: http::SslVersion =
            http::SslVersion::new_ssl_version("sslVersionMax", &Gitoxide::HTTP).with_note(
                "entirely new to set the upper bound for the allowed ssl version range. Overwrites the max bound of `http.sslVersion` if set. Min and Max must be set to become effective.",
            );
        /// The `gitoxide.http.sslNoVerify` key.
        ///
        /// If set, disable SSL verification. Using this is discouraged as it can lead to
        /// various security risks. An example where this may be needed is when an internal
        /// git server uses a self-signed certificate and the user accepts the associated security risks.
        pub const SSL_NO_VERIFY: keys::Boolean = keys::Boolean::new_boolean("sslNoVerify", &Gitoxide::HTTP)
            .with_environment_override("GIT_SSL_NO_VERIFY")
            .with_note("used to disable SSL verification. When this is enabled it takes priority over http.sslVerify");
        /// The `gitoxide.http.proxyAuthMethod` key.
        pub const PROXY_AUTH_METHOD: http::ProxyAuthMethod =
            http::ProxyAuthMethod::new_proxy_auth_method("proxyAuthMethod", &Gitoxide::HTTP)
                .with_environment_override("GIT_HTTP_PROXY_AUTHMETHOD");
    }

    impl Section for Http {
        fn name(&self) -> &str {
            "http"
        }

        fn keys(&self) -> &[&dyn Key] {
            &[
                &Self::PROXY,
                &Self::ALL_PROXY,
                &Self::VERBOSE,
                &Self::NO_PROXY,
                &Self::CONNECT_TIMEOUT,
                &Self::SSL_VERSION_MIN,
                &Self::SSL_VERSION_MAX,
                &Self::SSL_NO_VERIFY,
                &Self::PROXY_AUTH_METHOD,
            ]
        }

        fn parent(&self) -> Option<&dyn Section> {
            Some(&Tree::GITOXIDE)
        }
    }

    /// The `Https` sub-section.
    #[derive(Copy, Clone, Default)]
    pub struct Https;

    impl Https {
        /// The `gitoxide.https.proxy` key.
        pub const PROXY: keys::String = keys::String::new_string("proxy", &Gitoxide::HTTPS)
            .with_environment_override("HTTPS_PROXY")
            .with_note("fallback environment variable is `https_proxy`");
    }

    impl Section for Https {
        fn name(&self) -> &str {
            "https"
        }

        fn keys(&self) -> &[&dyn Key] {
            &[&Self::PROXY]
        }

        fn parent(&self) -> Option<&dyn Section> {
            Some(&Tree::GITOXIDE)
        }
    }

    /// The `allow` sub-section.
    #[derive(Copy, Clone, Default)]
    pub struct Allow;

    /// The `gitoxide.allow.protocolFromUser` key.
    pub type ProtocolFromUser = keys::Any<super::validate::ProtocolFromUser>;

    impl Allow {
        /// The `gitoxide.allow.protocolFromUser` key.
        pub const PROTOCOL_FROM_USER: ProtocolFromUser = ProtocolFromUser::new_with_validate(
            "protocolFromUser",
            &Gitoxide::ALLOW,
            super::validate::ProtocolFromUser,
        )
        .with_environment_override("GIT_PROTOCOL_FROM_USER");
    }

    impl Section for Allow {
        fn name(&self) -> &str {
            "allow"
        }

        fn keys(&self) -> &[&dyn Key] {
            &[&Self::PROTOCOL_FROM_USER]
        }

        fn parent(&self) -> Option<&dyn Section> {
            Some(&Tree::GITOXIDE)
        }
    }

    /// The `author` sub-section.
    #[derive(Copy, Clone, Default)]
    pub struct Author;

    impl Author {
        /// The `gitoxide.author.nameFallback` key.
        pub const NAME_FALLBACK: keys::Any =
            keys::Any::new("nameFallback", &Gitoxide::AUTHOR).with_environment_override("GIT_AUTHOR_NAME");
        /// The `gitoxide.author.emailFallback` key.
        pub const EMAIL_FALLBACK: keys::Any =
            keys::Any::new("emailFallback", &Gitoxide::AUTHOR).with_environment_override("GIT_AUTHOR_EMAIL");
    }

    impl Section for Author {
        fn name(&self) -> &str {
            "author"
        }

        fn keys(&self) -> &[&dyn Key] {
            &[&Self::NAME_FALLBACK, &Self::EMAIL_FALLBACK]
        }

        fn parent(&self) -> Option<&dyn Section> {
            Some(&Tree::GITOXIDE)
        }
    }

    /// The `user` sub-section.
    #[derive(Copy, Clone, Default)]
    pub struct User;

    impl User {
        /// The `gitoxide.user.emailFallback` key.
        pub const EMAIL_FALLBACK: keys::Any =
            keys::Any::new("emailFallback", &Gitoxide::USER).with_environment_override("EMAIL");
    }

    impl Section for User {
        fn name(&self) -> &str {
            "user"
        }

        fn keys(&self) -> &[&dyn Key] {
            &[&Self::EMAIL_FALLBACK]
        }

        fn parent(&self) -> Option<&dyn Section> {
            Some(&Tree::GITOXIDE)
        }
    }

    /// The `ssh` sub-section.
    #[derive(Copy, Clone, Default)]
    pub struct Ssh;

    impl Ssh {
        /// The `gitoxide.ssh.commandWithoutShellFallback` key.
        pub const COMMAND_WITHOUT_SHELL_FALLBACK: keys::Executable =
            keys::Executable::new_executable("commandWithoutShellFallback", &Gitoxide::SSH)
                .with_environment_override("GIT_SSH")
                .with_note("is always executed without shell and treated as fallback");
    }

    impl Section for Ssh {
        fn name(&self) -> &str {
            "ssh"
        }

        fn keys(&self) -> &[&dyn Key] {
            &[&Self::COMMAND_WITHOUT_SHELL_FALLBACK]
        }

        fn parent(&self) -> Option<&dyn Section> {
            Some(&Tree::GITOXIDE)
        }
    }

    /// The `pathspec` sub-section.
    #[derive(Copy, Clone, Default)]
    pub struct Pathspec;

    impl Pathspec {
        /// The `gitoxide.pathspec.glob` key.
        pub const GLOB: keys::Boolean = keys::Boolean::new_boolean("glob", &Gitoxide::PATHSPEC)
            .with_environment_override("GIT_GLOB_PATHSPECS")
            .with_note("pathspec wildcards don't match the slash character, then needing '**' to get past them");
        /// The `gitoxide.pathspec.noglob` key.
        pub const NOGLOB: keys::Boolean = keys::Boolean::new_boolean("noglob", &Gitoxide::PATHSPEC)
            .with_environment_override("GIT_NOGLOB_PATHSPECS")
            .with_note("Enable literal matching for glob patterns, effectively disabling globbing");
        /// The `gitoxide.pathspec.literal` key.
        pub const LITERAL: keys::Boolean = keys::Boolean::new_boolean("literal", &Gitoxide::PATHSPEC)
            .with_environment_override("GIT_LITERAL_PATHSPECS")
            .with_note("Make the entire spec used verbatim, the only way to get ':()name' verbatim for instance");
        /// The `gitoxide.pathspec.icase` key.
        pub const ICASE: keys::Boolean = keys::Boolean::new_boolean("icase", &Gitoxide::PATHSPEC)
            .with_environment_override("GIT_ICASE_PATHSPECS")
            .with_note("Compare string in a case-insensitive manner");
        /// The `gitoxide.pathspec.inheritIgnoreCase` key, defaulting to `true` if unspecified.
        /// If set, pathspecs will automatically be match case-insensitively if the underlying filesystem is configured that way.
        pub const INHERIT_IGNORE_CASE: keys::Boolean =
            keys::Boolean::new_boolean("inheritIgnoreCase", &Gitoxide::PATHSPEC)
                .with_note("Inherit `core.ignoreCase` for defaults in pathspecs");
        /// The default value for `gitoxide.pathspec.inheritIgnoreCase`.
        pub const INHERIT_IGNORE_CASE_DEFAULT: bool = true;
    }

    impl Section for Pathspec {
        fn name(&self) -> &str {
            "pathspec"
        }

        fn keys(&self) -> &[&dyn Key] {
            &[
                &Self::GLOB,
                &Self::NOGLOB,
                &Self::LITERAL,
                &Self::ICASE,
                &Self::INHERIT_IGNORE_CASE,
            ]
        }

        fn parent(&self) -> Option<&dyn Section> {
            Some(&Tree::GITOXIDE)
        }
    }

    /// The `objects` sub-section.
    #[derive(Copy, Clone, Default)]
    pub struct Objects;

    impl Objects {
        /// The `gitoxide.objects.cacheLimit` key.
        pub const CACHE_LIMIT: keys::UnsignedInteger =
            keys::UnsignedInteger::new_unsigned_integer("cacheLimit", &Gitoxide::OBJECTS)
                .with_note("If unset or 0, there is no object cache")
                .with_environment_override("GIX_OBJECT_CACHE_MEMORY");
        /// The `gitoxide.objects.noReplace` key.
        pub const NO_REPLACE: keys::Boolean = keys::Boolean::new_boolean("noReplace", &Gitoxide::OBJECTS);
        /// The `gitoxide.objects.replaceRefBase` key.
        pub const REPLACE_REF_BASE: keys::Any =
            keys::Any::new("replaceRefBase", &Gitoxide::OBJECTS).with_environment_override("GIT_REPLACE_REF_BASE");
    }

    impl Section for Objects {
        fn name(&self) -> &str {
            "objects"
        }

        fn keys(&self) -> &[&dyn Key] {
            &[&Self::CACHE_LIMIT, &Self::REPLACE_REF_BASE]
        }

        fn parent(&self) -> Option<&dyn Section> {
            Some(&Tree::GITOXIDE)
        }
    }

    /// The `committer` sub-section.
    #[derive(Copy, Clone, Default)]
    pub struct Committer;

    impl Committer {
        /// The `gitoxide.committer.nameFallback` key.
        pub const NAME_FALLBACK: keys::Any =
            keys::Any::new("nameFallback", &Gitoxide::COMMITTER).with_environment_override("GIT_COMMITTER_NAME");
        /// The `gitoxide.committer.emailFallback` key.
        pub const EMAIL_FALLBACK: keys::Any =
            keys::Any::new("emailFallback", &Gitoxide::COMMITTER).with_environment_override("GIT_COMMITTER_EMAIL");
    }

    impl Section for Committer {
        fn name(&self) -> &str {
            "committer"
        }

        fn keys(&self) -> &[&dyn Key] {
            &[&Self::NAME_FALLBACK, &Self::EMAIL_FALLBACK]
        }

        fn parent(&self) -> Option<&dyn Section> {
            Some(&Tree::GITOXIDE)
        }
    }

    /// The `credentials` sub-section.
    #[derive(Copy, Clone, Default)]
    pub struct Credentials;
    impl Credentials {
        /// The `gitoxide.credentials.terminalPrompt` key.
        pub const TERMINAL_PROMPT: keys::Boolean = keys::Boolean::new_boolean("terminalPrompt", &Gitoxide::CREDENTIALS)
            .with_note("This is a custom addition to provide an alternative to the respective environment variable.")
            .with_environment_override("GIT_TERMINAL_PROMPT");

        /// The `gitoxide.credentials.helperStderr` key to control what happens with the credential helpers `stderr`.
        ///
        /// If `true`, the default, `stderr` of credential helper programs will be inherited, just like with `git`.
        /// If `false`, will be suppressed completely.
        pub const HELPER_STDERR: keys::Boolean = keys::Boolean::new_boolean("helperStderr", &Gitoxide::CREDENTIALS)
            .with_environment_override("GIX_CREDENTIALS_HELPER_STDERR");
    }

    impl Section for Credentials {
        fn name(&self) -> &str {
            "credentials"
        }

        fn keys(&self) -> &[&dyn Key] {
            &[&Self::TERMINAL_PROMPT, &Self::HELPER_STDERR]
        }

        fn parent(&self) -> Option<&dyn Section> {
            Some(&Tree::GITOXIDE)
        }
    }

    /// The `commit` sub-section.
    #[derive(Copy, Clone, Default)]
    pub struct Commit;

    impl Commit {
        /// The `gitoxide.commit.authorDate` key.
        pub const AUTHOR_DATE: keys::Any =
            keys::Any::new("authorDate", &Gitoxide::COMMIT).with_environment_override("GIT_AUTHOR_DATE");
        /// The `gitoxide.commit.committerDate` key.
        pub const COMMITTER_DATE: keys::Any =
            keys::Any::new("committerDate", &Gitoxide::COMMIT).with_environment_override("GIT_COMMITTER_DATE");
    }

    impl Section for Commit {
        fn name(&self) -> &str {
            "commit"
        }

        fn keys(&self) -> &[&dyn Key] {
            &[]
        }

        fn parent(&self) -> Option<&dyn Section> {
            Some(&Tree::GITOXIDE)
        }
    }
}
pub use subsections::{Allow, Author, Commit, Committer, Core, Credentials, Http, Https, Objects, Pathspec, Ssh, User};

pub mod validate {
    use std::error::Error;

    use crate::{bstr::BStr, config::tree::keys::Validate};

    pub struct ProtocolFromUser;
    impl Validate for ProtocolFromUser {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
            if value != "1" {
                return Err("GIT_PROTOCOL_FROM_USER is either unset or as the value '1'".into());
            }
            Ok(())
        }
    }

    pub struct RefsNamespace;
    impl Validate for RefsNamespace {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
            super::Core::REFS_NAMESPACE.try_into_refs_namespace(value.into())?;
            Ok(())
        }
    }
}
