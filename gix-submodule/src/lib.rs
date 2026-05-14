//! Primitives for describing git submodules.
#![deny(missing_docs)]
#![forbid(unsafe_code)]

use std::{borrow::Cow, collections::BTreeMap};

/// All relevant information about a git module, typically from `.gitmodules` files.
///
/// Note that overrides from other configuration might be relevant, which is why this type
/// can be used to take these into consideration when presented with other configuration
/// from the superproject.
#[derive(Clone)]
pub struct File {
    config: gix_config::File<'static>,
}

mod access;

///
pub mod config;

///
pub mod is_active_platform;

/// A platform to keep the state necessary to perform repeated active checks, created by [File::is_active_platform()].
pub struct IsActivePlatform {
    pub(crate) search: Option<gix_pathspec::Search>,
}

/// Mutation
impl File {
    /// This can be used to let `config` override some values we know about submodules, namely…
    ///
    /// * `url`
    /// * `fetchRecurseSubmodules`
    /// * `ignore`
    /// * `update`
    /// * `branch`
    ///
    /// These values aren't validated yet, which will happen upon query.
    pub fn append_submodule_overrides(&mut self, config: &gix_config::File<'_>) -> &mut Self {
        let mut values = BTreeMap::<_, Vec<_>>::new();
        for (module_name, section) in config
            .sections_by_name("submodule")
            .into_iter()
            .flatten()
            .filter_map(|s| s.header().subsection_name().map(|n| (n, s)))
        {
            for field in ["url", "fetchRecurseSubmodules", "ignore", "update", "branch"] {
                if let Some(value) = section.value(field) {
                    values.entry((module_name, field)).or_default().push(value);
                }
            }
        }

        let values = {
            let mut v: Vec<_> = values.into_iter().collect();
            v.sort_by_key(|a| a.0.0);
            v
        };

        let mut config_to_append = gix_config::File::new(config.meta_owned());
        let mut prev_name = None;
        for ((module_name, field), values) in values {
            if prev_name != Some(module_name) {
                config_to_append
                    .new_section("submodule", Some(Cow::Owned(module_name.to_owned())))
                    .expect("all names come from valid configuration, so remain valid");
                prev_name = Some(module_name);
            }
            config_to_append
                .section_mut("submodule", Some(module_name))
                .expect("always set at this point")
                .push(
                    field.try_into().expect("statically known key"),
                    Some(values.last().expect("at least one value or we wouldn't be here")),
                );
        }

        self.config.append(config_to_append);
        self
    }
}

///
mod init {
    use std::path::PathBuf;

    use crate::File;

    impl std::fmt::Debug for File {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("File")
                .field("config_path", &self.config_path())
                .field("config", &format_args!("r#\"{}\"#", self.config))
                .finish()
        }
    }

    /// A marker we use when listing names to not pick them up from overridden sections.
    pub(crate) const META_MARKER: gix_config::Source = gix_config::Source::Api;

    /// Lifecycle
    impl File {
        /// Parse `bytes` as git configuration, typically from `.gitmodules`, without doing any further validation.
        /// `path` can be provided to keep track of where the file was read from in the underlying [`config`](Self::config())
        /// instance.
        /// `config` is used to [apply value overrides](File::append_submodule_overrides), which can be empty if overrides
        /// should be applied at a later time.
        ///
        /// Future access to the module information is lazy and configuration errors are exposed there on a per-value basis.
        ///
        /// ### Security Considerations
        ///
        /// The information itself should be used with care as it can direct the caller to fetch from remotes. It is, however,
        /// on the caller to assure the input data can be trusted.
        pub fn from_bytes(
            bytes: &[u8],
            path: impl Into<Option<PathBuf>>,
            config: &gix_config::File<'_>,
        ) -> Result<Self, gix_config::parse::Error> {
            let metadata = {
                let mut meta = gix_config::file::Metadata::from(META_MARKER);
                meta.path = path.into();
                meta
            };
            let modules = gix_config::File::from_parse_events_no_includes(
                gix_config::parse::Events::from_bytes_owned(bytes, None)?,
                metadata,
            );

            let mut res = Self { config: modules };
            res.append_submodule_overrides(config);
            Ok(res)
        }

        /// Turn ourselves into the underlying parsed configuration file.
        pub fn into_config(self) -> gix_config::File<'static> {
            self.config
        }
    }
}
