use std::collections::BTreeSet;

use crate::{bstr::ByteSlice, config};

/// General Configuration
impl crate::Repository {
    /// Return a snapshot of the configuration as seen upon opening the repository.
    pub fn config_snapshot(&self) -> config::Snapshot<'_> {
        config::Snapshot { repo: self }
    }

    /// Return a mutable snapshot of the configuration as seen upon opening the repository, starting a transaction.
    /// When the returned instance is dropped, it is applied in full, even if the reason for the drop is an error.
    ///
    /// Note that changes to the configuration are in-memory only and are observed only this instance
    /// of the [`Repository`](crate::Repository).
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

    /// Obtain options for use when connecting via `ssh`.
    #[cfg(feature = "blocking-network-client")]
    pub fn ssh_connect_options(
        &self,
    ) -> Result<gix_protocol::transport::client::ssh::connect::Options, config::ssh_connect_options::Error> {
        use crate::config::{
            cache::util::ApplyLeniency,
            tree::{gitoxide, Core, Ssh},
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
        let opts = gix_protocol::transport::client::ssh::connect::Options {
            disallow_shell: fallback_active,
            command: ssh_command,
            kind: config
                .string_filter("ssh.variant", &mut trusted)
                .and_then(|variant| Ssh::VARIANT.try_into_variant(variant).transpose())
                .transpose()
                .with_leniency(self.options.lenient_config)?,
        };
        Ok(opts)
    }

    /// Return the context to be passed to any spawned program that is supposed to interact with the repository, like
    /// hooks or filters.
    #[cfg(feature = "attributes")]
    pub fn command_context(&self) -> Result<gix_command::Context, config::command_context::Error> {
        use crate::config::{cache::util::ApplyLeniency, tree::gitoxide};

        let pathspec_boolean = |key: &'static config::tree::keys::Boolean| {
            self.config
                .resolved
                .boolean(key)
                .map(|value| key.enrich_error(value))
                .transpose()
                .with_leniency(self.config.lenient_config)
        };

        Ok(gix_command::Context {
            stderr: {
                self.config
                    .resolved
                    .boolean(gitoxide::Core::EXTERNAL_COMMAND_STDERR)
                    .map(|value| gitoxide::Core::EXTERNAL_COMMAND_STDERR.enrich_error(value))
                    .transpose()
                    .with_leniency(self.config.lenient_config)?
                    .unwrap_or(true)
                    .into()
            },
            git_dir: self.git_dir().to_owned().into(),
            worktree_dir: self.workdir().map(ToOwned::to_owned),
            no_replace_objects: config::shared::is_replace_refs_enabled(
                &self.config.resolved,
                self.config.lenient_config,
                self.filter_config_section(),
            )?
            .map(|enabled| !enabled),
            ref_namespace: self.refs.namespace.as_ref().map(|ns| ns.as_bstr().to_owned()),
            literal_pathspecs: pathspec_boolean(&gitoxide::Pathspec::LITERAL)?,
            glob_pathspecs: pathspec_boolean(&gitoxide::Pathspec::GLOB)?
                .or(pathspec_boolean(&gitoxide::Pathspec::NOGLOB)?),
            icase_pathspecs: pathspec_boolean(&gitoxide::Pathspec::ICASE)?,
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
