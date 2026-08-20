use crate::{bstr::ByteSlice, config};
use std::{collections::BTreeSet, ffi::OsString};

/// General Configuration
impl crate::Repository {
    /// Return the compression level used when writing loose objects.
    pub fn loose_compression(&self) -> gix_zlib::Compression {
        self.config.loose_compression
    }

    /// Return the effective compression level used when writing pack entries.
    pub fn pack_compression(&self) -> Result<gix_zlib::Compression, config::Error> {
        config::cache::access::pack_compression(
            &self.config.resolved,
            self.config.lenient_config,
            self.filter_config_section(),
        )
    }

    /// Return a snapshot of the configuration as seen upon opening the repository.
    ///
    /// Use [`reload()`](Self::reload()) to refresh it from disk.
    pub fn config_snapshot(&self) -> config::Snapshot<'_> {
        config::Snapshot { repo: self }
    }

    /// Lock and open `path` as one physical configuration file without expanding its includes.
    ///
    /// Relative paths are resolved against the current directory captured when this repository was opened. Dropping the
    /// returned transaction releases the lock and discards its changes. Committing it only updates the file; call
    /// [`reload()`](Self::reload()) explicitly to rebuild this repository from the changed configuration.
    pub fn config_file_mut(
        &self,
        path: impl Into<std::path::PathBuf>,
    ) -> Result<config::FileTransaction, config::file_mut::Error> {
        let path = path.into();
        let path = if path.is_absolute() {
            path
        } else {
            self.current_dir().join(path)
        };
        let lock_mode = self.config.config_lock_timeout()?;
        let shared_repository_permissions =
            config::file_mut::shared_repository_permissions(&self.config.resolved, self.filter_config_section())?;
        config::FileTransaction::open(path, self.git_dir_trust(), lock_mode, shared_repository_permissions)
    }

    /// Return the editor program selected by Git's precedence rules.
    ///
    /// `GIT_EDITOR` takes precedence over `core.editor`. If the terminal isn't dumb, `VISUAL` is considered next,
    /// followed by `EDITOR`. If none are set, a bundled `vi` (or its `vim` implementation) is returned when available
    /// unless `TERM` is unset or `dumb`, in which case there is no usable editor.
    ///
    /// Use [`editor_command()`](Self::editor_command) to obtain a command prepared for execution.
    pub fn editor(&self) -> Option<OsString> {
        use crate::config::tree::{Core, Gitoxide};

        let config = self.config_snapshot();
        let terminal_is_dumb = config.string(Gitoxide::TERM).is_none_or(|terminal| terminal == "dumb");
        config
            .trusted_program(Core::EDITOR)
            .or_else(|| {
                (!terminal_is_dumb)
                    .then(|| config.trusted_program(Gitoxide::VISUAL))
                    .flatten()
            })
            .or_else(|| config.trusted_program(Gitoxide::EDITOR))
            .or_else(|| {
                (!terminal_is_dumb).then(|| {
                    gix_path::env::installation_program("vi")
                        // Current Git for Windows versions provide `vi` as a shell script that delegates to `vim.exe`.
                        // Select the directly executable implementation when no `vi.exe` is installed.
                        .or_else(|| {
                            cfg!(windows)
                                .then(|| gix_path::env::installation_program("vim"))
                                .flatten()
                        })
                        .unwrap_or_else(|| "vi".into())
                        .into_os_string()
                })
            })
            .filter(|editor| !editor.is_empty())
    }

    /// Return the prepared [`editor`](Self::editor) command.
    ///
    /// The returned command has repository context and inherited standard streams. Add the paths to edit as arguments
    /// before spawning it.
    #[cfg(feature = "command")]
    pub fn editor_command(&self) -> Result<Option<gix_command::Prepare>, config::command_context::Error> {
        use std::{path::Path, process::Stdio};

        let Some(editor) = self.editor() else {
            return Ok(None);
        };

        let mut command = gix_command::prepare(&editor);
        if editor.to_string_lossy().trim_ascii() == ":" {
            command = command.with_shell();
        } else if !Path::new(&editor).is_file() {
            command = command.command_may_be_shell_script();
        }
        Ok(Some(
            command
                .with_context(self.command_context()?)
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit()),
        ))
    }

    /// Resolve all Git configuration needed to sign a commit with [`gix_object::Commit::sign()`].
    ///
    /// The returned plumbing options may be adjusted before use, for example to disable GPG pinentry by adding
    /// `--pinentry-mode=error` to `program_arguments`.
    #[cfg(feature = "command")]
    pub fn commit_signing_options(
        &self,
    ) -> Result<gix_object::signature::sign::Options, crate::commit::sign::options::Error> {
        crate::commit::sign::signing_options(self)
    }

    /// Resolve all Git configuration needed to sign a commit if `commit.gpgSign` enables signing.
    ///
    /// If signing is disabled, signer-specific configuration isn't resolved or validated.
    #[cfg(feature = "command")]
    pub fn commit_signing_options_if_enabled(
        &self,
    ) -> Result<Option<gix_object::signature::sign::Options>, crate::commit::sign::options::Error> {
        crate::commit::sign::signing_options_if_enabled(self)
    }

    /// Return a mutable snapshot of the configuration as seen upon opening the repository, starting a transaction.
    /// When the returned instance is dropped, it is applied in full, even if the reason for the drop is an error.
    ///
    /// Note that changes to the configuration are in-memory only and are observed only this instance
    /// of the [`Repository`](crate::Repository). Use [`reload()`](Self::reload()) to discard them and
    /// refresh the snapshot from disk.
    ///
    /// Values used to locate repository files are fixed when the repository is opened and aren't reapplied by
    /// committing changes here. This includes `core.worktree` and `gitoxide.core.indexFile`; `GIT_DIR` likewise
    /// cannot retarget an existing repository. Reload after changing persisted configuration or open another
    /// repository to change these locations.
    pub fn config_snapshot_mut(&mut self) -> config::SnapshotMut<'_> {
        let config = self.config.resolved.as_ref().clone();
        config::SnapshotMut {
            repo: Some(self),
            config,
        }
    }

    /// Return filesystem options as retrieved from the repository configuration.
    ///
    /// Note that these values have not been [probed](gix_fs::Capabilities::probe()).
    pub fn filesystem_options(&self) -> Result<gix_fs::Capabilities, config::boolean::Error> {
        self.config.fs_capabilities()
    }

    /// Return filesystem options on how to perform stat-checks, typically in relation to the index.
    ///
    /// Note that these values have not been [probed](gix_fs::Capabilities::probe()).
    #[cfg(feature = "index")]
    pub fn stat_options(&self) -> Result<gix_index::entry::stat::Options, config::stat_options::Error> {
        self.config.stat_options()
    }

    /// The options used to open the repository.
    pub fn open_options(&self) -> &crate::open::Options {
        &self.options
    }

    /// Return the big-file threshold above which Git will not perform a diff anymore or try to delta-diff packs,
    /// as configured by `core.bigFileThreshold`, or the default value.
    pub fn big_file_threshold(&self) -> Result<u64, config::unsigned_integer::Error> {
        self.config.big_file_threshold()
    }

    /// Create a low-level parser for ignore patterns, for instance for use in [`excludes()`](crate::Repository::excludes()).
    ///
    /// Depending on the configuration, precious-file parsing in `.gitignore-files` is supported.
    /// This means that `$` prefixed files will be interpreted as precious, which is a backwards-incompatible change.
    #[cfg(feature = "excludes")]
    pub fn ignore_pattern_parser(&self) -> Result<gix_ignore::search::Ignore, config::boolean::Error> {
        self.config.ignore_pattern_parser()
    }

    /// Obtain options for use when connecting via `ssh`.
    #[cfg(feature = "blocking-network-client")]
    pub fn ssh_connect_options(
        &self,
    ) -> Result<gix_protocol::transport::client::blocking_io::ssh::connect::Options, config::ssh_connect_options::Error>
    {
        use crate::config::{
            cache::util::ApplyLeniency,
            tree::{Core, Ssh, gitoxide},
        };

        let config = &self.config.resolved;
        let mut trusted = self.filter_config_section();
        let mut fallback_active = false;
        let ssh_command = config
            .string_filter(Core::SSH_COMMAND, &mut trusted)
            .or_else(|| {
                fallback_active = true;
                config.string_filter(gitoxide::Ssh::COMMAND_WITHOUT_SHELL_FALLBACK, &mut trusted)
            })
            .map(|cmd| gix_path::from_bstr(cmd).into_owned().into());
        let opts = gix_protocol::transport::client::blocking_io::ssh::connect::Options {
            disallow_shell: fallback_active,
            command: ssh_command,
            kind: config
                .string_filter("ssh.variant", &mut trusted)
                .and_then(|variant| Ssh::VARIANT.try_into_variant(variant).transpose())
                .transpose()
                .with_leniency(self.options.lenient_config)
                .map_err(gix_error::Error::from)?,
        };
        Ok(opts)
    }

    /// Return the context to be passed to any spawned program that is supposed to interact with the repository, like
    /// hooks or filters.
    #[cfg(feature = "command")]
    pub fn command_context(&self) -> Result<gix_command::Context, config::command_context::Error> {
        use crate::config::{cache::util::ApplyLeniency, tree::gitoxide};

        let pathspec_boolean = |key: &'static config::tree::keys::Boolean| {
            key.enrich_error(self.config.resolved.boolean(key))
                .with_leniency(self.config.lenient_config)
        };

        Ok(gix_command::Context {
            stderr: {
                gitoxide::Core::EXTERNAL_COMMAND_STDERR
                    .enrich_error(self.config.resolved.boolean(gitoxide::Core::EXTERNAL_COMMAND_STDERR))
                    .with_leniency(self.config.lenient_config)
                    .map_err(gix_error::Error::from)?
                    .unwrap_or(true)
                    .into()
            },
            git_dir: self.git_dir().to_owned().into(),
            worktree_dir: self.workdir().map(ToOwned::to_owned),
            no_replace_objects: config::shared::is_replace_refs_enabled(
                &self.config.resolved,
                self.config.lenient_config,
                self.filter_config_section(),
            )
            .map_err(gix_error::Error::from)?
            .map(|enabled| !enabled),
            ref_namespace: self.refs.namespace.as_ref().map(|ns| ns.as_bstr().to_owned()),
            literal_pathspecs: pathspec_boolean(&gitoxide::Pathspec::LITERAL).map_err(gix_error::Error::from)?,
            glob_pathspecs: pathspec_boolean(&gitoxide::Pathspec::GLOB)
                .map_err(gix_error::Error::from)?
                .or(pathspec_boolean(&gitoxide::Pathspec::NOGLOB).map_err(gix_error::Error::from)?),
            icase_pathspecs: pathspec_boolean(&gitoxide::Pathspec::ICASE).map_err(gix_error::Error::from)?,
        })
    }

    /// The kind of object hash the repository is configured to use.
    pub fn object_hash(&self) -> gix_hash::Kind {
        self.config.object_hash
    }

    /// Return the algorithm to perform diffs or merges with.
    ///
    /// In case of merges, a diff is performed under the hood in order to learn which hunks need merging.
    #[cfg(feature = "blob-diff")]
    pub fn diff_algorithm(&self) -> Result<gix_diff::blob::Algorithm, config::diff::algorithm::Error> {
        self.config.diff_algorithm()
    }
}

mod branch;
mod remote;
#[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
mod transport;

impl crate::Repository {
    pub(crate) fn filter_config_section(&self) -> fn(&gix_config::file::Metadata) -> bool {
        self.options
            .filter_config_section
            .unwrap_or(config::section::is_trusted)
    }

    fn subsection_str_names_of<'a>(&'a self, header_name: &'a str) -> BTreeSet<&'a str> {
        self.config
            .resolved
            .sections_by_name(header_name)
            .map(|it| {
                let filter = self.filter_config_section();
                it.filter(move |s| filter(s.meta()))
                    .filter_map(|section| section.header().subsection_name().and_then(|b| b.to_str().ok()))
                    .collect()
            })
            .unwrap_or_default()
    }
}
