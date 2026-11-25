use crate::{Error, Remote, Result, bstr::BStr, config, remote};
use gix_utils::AsBStr;

/// Serialize into git-config.
impl Remote<'_> {
    /// Save this remote to `config` under its [configured subsection name](Self::configured_name).
    ///
    /// The name is preserved as configured, including `.` or names like `backup mirror`.
    /// A fetch URL inferred from the subsection name is not written as an explicit `url`.
    /// An anonymous remote cannot be saved; use [`Self::save_as_to()`] to give it a name.
    ///
    /// Note that all sections named `remote "<name>"` will be cleared of all values we are about to write,
    /// and the last `remote "<name>"` section will be containing all relevant values so that reloading the remote
    /// from `config` would yield the same in-memory state.
    pub fn save_to(&self, config: &mut gix_config::File) -> Result<()> {
        let Some(name) = self.configured_name().map(remote::ConfiguredName::as_config_key) else {
            let url = self
                .urls
                .first()
                .or_else(|| self.push_urls.first())
                .expect("anonymous remotes always have a URL")
                .to_bstring();
            return Err(Error::from_error(gix_error::message!(
                "The remote pointing to {url} is anonymous and can't be saved."
            )));
        };

        let target_meta = config.meta().clone();
        let mut needs_url_reset = false;
        let mut needs_push_url_reset = false;
        if let Some(section_ids) = config.sections_and_ids_by_name("remote").map(|it| {
            it.filter_map(|(s, id)| (s.header().subsection_name() == Some(name)).then_some(id))
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
            config.new_section("remote", name)?
        } else {
            config.section_mut_or_create_new_filter("remote", name, |meta| *meta == target_meta)?
        };
        if needs_url_reset {
            section.push(config::tree::Remote::URL.name, "")?;
        }
        if !self.fetch_url_from_subsection {
            for url in &self.urls {
                section.push("url", url.to_bstring())?;
            }
        }
        if needs_push_url_reset {
            section.push(config::tree::Remote::PUSH_URL.name, "")?;
        }
        for url in &self.push_urls {
            section.push("pushurl", url.to_bstring())?;
        }
        if self.fetch_tags != Default::default() {
            section.push(
                config::tree::Remote::TAG_OPT.name,
                BStr::new(match self.fetch_tags {
                    remote::fetch::Tags::All => "--tags",
                    remote::fetch::Tags::None => "--no-tags",
                    remote::fetch::Tags::Included => {
                        unreachable!("BUG: the default shouldn't be written and we try")
                    }
                }),
            )?;
        }
        for (key, spec) in self
            .fetch_specs
            .iter()
            .map(|spec| ("fetch", spec))
            .chain(self.push_specs.iter().map(|spec| ("push", spec)))
        {
            section.push(key, spec.to_ref().to_bstring())?;
        }
        Ok(())
    }

    /// Assign `name` to this remote and save it to `config` as in [`Self::save_to()`].
    ///
    /// `name` must be a [valid symbolic name](remote::Name::symbol); `.` is not accepted.
    /// This can give an anonymous remote a name. When replacing an existing name, the old subsection remains
    /// in `config`; remove it separately if needed.
    pub fn save_as_to(&mut self, name: impl AsBStr, config: &mut gix_config::File) -> Result<()> {
        self.save_as_validated(remote::Name::symbol(name.as_bstr().to_owned())?, config)
    }

    pub(crate) fn save_as_validated(&mut self, name: remote::Name, config: &mut gix_config::File) -> Result<()> {
        let prev_fetch_url_from_subsection = self.fetch_url_from_subsection;
        if self.configured_name().map(remote::ConfiguredName::as_config_key) != Some(name.as_bstr()) {
            // A different subsection might not imply the same URL, so save the old destination explicitly.
            self.fetch_url_from_subsection = false;
        }
        let prev_name = self.name.replace(remote::ConfiguredName::Symbolic(name));
        self.save_to(config).inspect_err(|_| {
            self.name = prev_name;
            self.fetch_url_from_subsection = prev_fetch_url_from_subsection;
        })
    }
}
