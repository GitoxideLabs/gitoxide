use crate::{Remote, bstr::BStr, config, remote};
use gix_utils::AsBStr;

/// The error returned by [`Remote::save_to()`].
pub type Error = gix_error::Error;

// TODO(review): kept concrete due to an E0119 collision. `clone::fetch::Error`
//                (`gix/src/clone/fetch/mod.rs`) embeds this type via
//                `SaveConfig(#[from] crate::remote::save::AsError)`, and already embeds the
//                erased `config::overrides::Error`, via `ParseConfig(#[from] ...)`. Erasing
//                `AsError` would give `clone::fetch::Error` two `From<gix_error::Error>` impls.
/// The error returned by [`Remote::save_as_to()`].
///
/// Note that this type should rather be in the `as` module, but cannot be as it's part of the Rust syntax.
#[derive(Debug, thiserror::Error)]
#[expect(missing_docs)]
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
    pub fn save_to(&self, config: &mut gix_config::File) -> Result<(), Error> {
        let name = self.name().ok_or_else(|| {
            let url = self
                .urls
                .first()
                .or_else(|| self.push_urls.first())
                .expect("one url is always set")
                .to_bstring();
            gix_error::Error::from_error(gix_error::message!(
                "The remote pointing to {url} is anonymous and can't be saved."
            ))
        })?;
        let target_meta = config.meta().clone();
        let mut needs_url_reset = false;
        let mut needs_push_url_reset = false;
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
                let url_values = section.values(config::tree::Remote::URL.name);
                let push_url_values = section.values(config::tree::Remote::PUSH_URL.name);
                if *section.meta() != target_meta {
                    needs_url_reset |= !url_values.is_empty();
                    needs_push_url_reset |= !push_url_values.is_empty();
                } else {
                    needs_url_reset |= url_values.iter().any(|url| url.is_empty());
                    needs_push_url_reset |= push_url_values.iter().any(|url| url.is_empty());
                }

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
        // Only reuse an existing section that belongs to the file we are writing to. Otherwise a
        // section provided by another source (e.g. global config like `remote.<name>.prune`) would
        // be mutated in place, mixing foreign metadata with our values and getting lost when the
        // caller writes back only the local sections. In that case, create a fresh local section.
        // We assume that `config.meta()` is truly the 'identity' of the configuration file.
        let mut section = if needs_url_reset || needs_push_url_reset {
            // A foreign section may occur after the last local one, notably when an include follows
            // it. The reset must follow all such values or reopening the written file would append
            // the foreign values once more.
            config
                .new_section("remote", name.as_bstr())
                .expect("section name is validated and 'remote' is acceptable")
        } else {
            config
                .section_mut_or_create_new_filter("remote", name.as_bstr(), |meta| *meta == target_meta)
                .expect("section name is validated and 'remote' is acceptable")
        };
        if needs_url_reset {
            section
                .push(config::tree::Remote::URL.name, "")
                .map_err(gix_error::Error::from_error)?;
        }
        for url in &self.urls {
            section
                .push("url", url.to_bstring())
                .map_err(gix_error::Error::from_error)?;
        }
        if needs_push_url_reset {
            section
                .push(config::tree::Remote::PUSH_URL.name, "")
                .map_err(gix_error::Error::from_error)?;
        }
        for url in &self.push_urls {
            section
                .push("pushurl", url.to_bstring())
                .map_err(gix_error::Error::from_error)?;
        }
        if self.fetch_tags != Default::default() {
            section
                .push(
                    config::tree::Remote::TAG_OPT.name,
                    BStr::new(match self.fetch_tags {
                        remote::fetch::Tags::All => "--tags",
                        remote::fetch::Tags::None => "--no-tags",
                        remote::fetch::Tags::Included => {
                            unreachable!("BUG: the default shouldn't be written and we try")
                        }
                    }),
                )
                .map_err(gix_error::Error::from_error)?;
        }
        for (key, spec) in self
            .fetch_specs
            .iter()
            .map(|spec| ("fetch", spec))
            .chain(self.push_specs.iter().map(|spec| ("push", spec)))
        {
            section
                .push(key, spec.to_ref().to_bstring())
                .map_err(gix_error::Error::from_error)?;
        }
        Ok(())
    }

    /// Forcefully set our name to `name` and write our state to `config` similar to [`save_to()`][Self::save_to()].
    ///
    /// Note that this sets a name for anonymous remotes, but overwrites the name for those who were named before.
    /// If this name is different from the current one, the git configuration will still contain the previous name,
    /// and the caller should account for that.
    pub fn save_as_to(&mut self, name: impl AsBStr, config: &mut gix_config::File) -> Result<(), AsError> {
        let name = crate::remote::name::validated(name.as_bstr().to_owned())?;
        let prev_name = self.name.take();
        self.name = Some(name.into());
        self.save_to(config).map_err(|err| {
            self.name = prev_name;
            err.into()
        })
    }
}
