use gix_macros::momo;

use crate::{
    bstr::{BStr, BString},
    config, remote, Remote,
};

/// The error returned by [`Remote::save_to()`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("The remote pointing to {} is anonymous and can't be saved.", url.to_bstring())]
    NameMissing { url: gix_url::Url },
}

/// The error returned by [`Remote::save_as_to()`].
///
/// Note that this type should rather be in the `as` module, but cannot be as it's part of the Rust syntax.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum AsError {
    #[error(transparent)]
    Save(#[from] Error),
    #[error(transparent)]
    Name(#[from] crate::remote::name::Error),
}

/// Serialize into git-config.
impl Remote<'_> {
    /// Save ourselves to the given `config` if we are a named remote or fail otherwise.
    ///
    /// Note that all sections named `remote "<name>"` will be cleared of all values we are about to write,
    /// and the last `remote "<name>"` section will be containing all relevant values so that reloading the remote
    /// from `config` would yield the same in-memory state.
    #[allow(clippy::result_large_err)]
    pub fn save_to(&self, config: &mut gix_config::File<'static>) -> Result<(), Error> {
        fn as_key(name: &str) -> gix_config::parse::section::ValueName<'_> {
            name.try_into().expect("valid")
        }
        let name = self.name().ok_or_else(|| Error::NameMissing {
            url: self
                .url
                .as_ref()
                .or(self.push_url.as_ref())
                .expect("one url is always set")
                .to_owned(),
        })?;
        if let Some(section_ids) = config.sections_and_ids_by_name("remote").map(|it| {
            it.filter_map(|(s, id)| (s.header().subsection_name() == Some(name.as_bstr())).then_some(id))
                .collect::<Vec<_>>()
        }) {
            let mut sections_to_remove = Vec::new();
            const KEYS_TO_REMOVE: &[&str] = &[
                config::tree::Remote::URL.name,
                config::tree::Remote::PUSH_URL.name,
                config::tree::Remote::FETCH.name,
                config::tree::Remote::PUSH.name,
                config::tree::Remote::TAG_OPT.name,
            ];
            for id in section_ids {
                let mut section = config.section_mut_by_id(id).expect("just queried");
                let was_empty = section.num_values() == 0;

                for key in KEYS_TO_REMOVE {
                    while section.remove(key).is_some() {}
                }

                let is_empty_after_deletions_of_values_to_be_written = section.num_values() == 0;
                if !was_empty && is_empty_after_deletions_of_values_to_be_written {
                    sections_to_remove.push(id);
                }
            }
            for id in sections_to_remove {
                config.remove_section_by_id(id);
            }
        }
        let mut section = config
            .section_mut_or_create_new("remote", Some(name.as_ref()))
            .expect("section name is validated and 'remote' is acceptable");
        if let Some(url) = self.url.as_ref() {
            section.push(as_key("url"), Some(url.to_bstring().as_ref()));
        }
        if let Some(url) = self.push_url.as_ref() {
            section.push(as_key("pushurl"), Some(url.to_bstring().as_ref()));
        }
        if self.fetch_tags != Default::default() {
            section.push(
                as_key(config::tree::Remote::TAG_OPT.name),
                BStr::new(match self.fetch_tags {
                    remote::fetch::Tags::All => "--tags",
                    remote::fetch::Tags::None => "--no-tags",
                    remote::fetch::Tags::Included => unreachable!("BUG: the default shouldn't be written and we try"),
                })
                .into(),
            );
        }
        for (key, spec) in self
            .fetch_specs
            .iter()
            .map(|spec| ("fetch", spec))
            .chain(self.push_specs.iter().map(|spec| ("push", spec)))
        {
            section.push(as_key(key), Some(spec.to_ref().to_bstring().as_ref()));
        }
        Ok(())
    }

    /// Forcefully set our name to `name` and write our state to `config` similar to [`save_to()`][Self::save_to()].
    ///
    /// Note that this sets a name for anonymous remotes, but overwrites the name for those who were named before.
    /// If this name is different from the current one, the git configuration will still contain the previous name,
    /// and the caller should account for that.
    #[allow(clippy::result_large_err)]
    #[momo]
    pub fn save_as_to(
        &mut self,
        name: impl Into<BString>,
        config: &mut gix_config::File<'static>,
    ) -> Result<(), AsError> {
        let name = crate::remote::name::validated(name)?;
        let prev_name = self.name.take();
        self.name = Some(name.into());
        self.save_to(config).map_err(|err| {
            self.name = prev_name;
            err.into()
        })
    }
}
