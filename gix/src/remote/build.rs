use crate::{bstr::BStr, remote, Remote};

/// Builder methods
impl Remote<'_> {
    /// Set the `url` to be used when pushing data to a remote.
    pub fn push_url<Url, E>(self, url: Url) -> Result<Self, remote::init::Error>
    where
        Url: TryInto<gix_url::Url, Error = E>,
        gix_url::parse::Error: From<E>,
    {
        self.push_url_inner(
            url.try_into().map_err(|err| remote::init::Error::Url(err.into()))?,
            true,
        )
    }

    /// Set the `url` to be used when pushing data to a remote, without applying rewrite rules in case these could be faulty,
    /// eliminating one failure mode.
    pub fn push_url_without_url_rewrite<Url, E>(self, url: Url) -> Result<Self, remote::init::Error>
    where
        Url: TryInto<gix_url::Url, Error = E>,
        gix_url::parse::Error: From<E>,
    {
        self.push_url_inner(
            url.try_into().map_err(|err| remote::init::Error::Url(err.into()))?,
            false,
        )
    }

    /// Configure how tags should be handled when fetching from the remote.
    pub fn with_fetch_tags(mut self, tags: remote::fetch::Tags) -> Self {
        self.fetch_tags = tags;
        self
    }

    fn push_url_inner(
        mut self,
        push_url: gix_url::Url,
        should_rewrite_urls: bool,
    ) -> Result<Self, remote::init::Error> {
        self.push_url = push_url.into();

        let (_, push_url_alias) = if should_rewrite_urls {
            remote::init::rewrite_urls(&self.repo.config, None, self.push_url.as_ref())
        } else {
            Ok((None, None))
        }?;
        self.push_url_alias = push_url_alias;

        Ok(self)
    }

    /// Add `specs` as refspecs for `direction` to our list if they are unique, or ignore them otherwise.
    pub fn with_refspecs<Spec>(
        mut self,
        specs: impl IntoIterator<Item = Spec>,
        direction: remote::Direction,
    ) -> Result<Self, gix_refspec::parse::Error>
    where
        Spec: AsRef<BStr>,
    {
        use remote::Direction::*;
        let new_specs = specs
            .into_iter()
            .map(|spec| {
                gix_refspec::parse(
                    spec.as_ref(),
                    match direction {
                        Push => gix_refspec::parse::Operation::Push,
                        Fetch => gix_refspec::parse::Operation::Fetch,
                    },
                )
                .map(|s| s.to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let specs = match direction {
            Push => &mut self.push_specs,
            Fetch => &mut self.fetch_specs,
        };
        for spec in new_specs {
            if !specs.contains(&spec) {
                specs.push(spec);
            }
        }
        Ok(self)
    }
}
