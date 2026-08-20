use crate::{Remote, bstr::BStr, remote};

/// Builder methods
impl Remote<'_> {
    /// Override the `url` to be used when fetching data from a remote.
    ///
    /// Note that this URL is typically set during instantiation with [`crate::Repository::remote_at()`].
    pub fn with_url<Url, E>(self, url: Url) -> Result<Self, remote::init::Error>
    where
        Url: TryInto<gix_url::Url, Error = E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        self.url_inner(url.try_into().map_err(gix_error::Error::from_error)?, true)
    }

    /// Set the `url` to be used when fetching data from a remote, without applying rewrite rules in case these could be faulty,
    /// eliminating one failure mode.
    ///
    /// Note that this URL is typically set during instantiation with [`crate::Repository::remote_at_without_url_rewrite()`].
    pub fn with_url_without_url_rewrite<Url, E>(self, url: Url) -> Result<Self, remote::init::Error>
    where
        Url: TryInto<gix_url::Url, Error = E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        self.url_inner(url.try_into().map_err(gix_error::Error::from_error)?, false)
    }

    /// Set the `url` to be used when pushing data to a remote.
    #[deprecated = "Use `with_push_url()` instead"]
    pub fn push_url<Url, E>(self, url: Url) -> Result<Self, remote::init::Error>
    where
        Url: TryInto<gix_url::Url, Error = E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        self.with_push_url(url)
    }

    /// Set the explicit `url` to be used when pushing data to a remote.
    ///
    /// Explicit push URLs are rewritten with `url.<base>.insteadOf`; `pushInsteadOf` only applies when a fetch URL is used
    /// as the push fallback because no explicit push URL is configured.
    pub fn with_push_url<Url, E>(self, url: Url) -> Result<Self, remote::init::Error>
    where
        Url: TryInto<gix_url::Url, Error = E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        self.push_url_inner(url.try_into().map_err(gix_error::Error::from_error)?, true)
    }

    /// Set the `url` to be used when pushing data to a remote, without applying rewrite rules in case these could be faulty,
    /// eliminating one failure mode.
    #[deprecated = "Use `with_push_url_without_rewrite()` instead"]
    pub fn push_url_without_url_rewrite<Url, E>(self, url: Url) -> Result<Self, remote::init::Error>
    where
        Url: TryInto<gix_url::Url, Error = E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        self.with_push_url_without_url_rewrite(url)
    }

    /// Set the `url` to be used when pushing data to a remote, without applying rewrite rules in case these could be faulty,
    /// eliminating one failure mode.
    pub fn with_push_url_without_url_rewrite<Url, E>(self, url: Url) -> Result<Self, remote::init::Error>
    where
        Url: TryInto<gix_url::Url, Error = E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        self.push_url_inner(url.try_into().map_err(gix_error::Error::from_error)?, false)
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
        self.push_urls = vec![push_url];

        self.push_url_aliases = if should_rewrite_urls {
            remote::init::rewrite_url_aliases_with_error_kind(
                &self.repo.config,
                &self.push_urls,
                remote::Direction::Fetch,
                remote::Direction::Push,
            )
        } else {
            Ok(vec![None; self.push_urls.len()])
        }?;
        self.url_push_aliases = vec![None; self.urls.len()];

        Ok(self)
    }

    fn url_inner(mut self, url: gix_url::Url, should_rewrite_urls: bool) -> Result<Self, remote::init::Error> {
        self.urls = vec![url];

        self.url_aliases = if should_rewrite_urls {
            remote::init::rewrite_url_aliases(&self.repo.config, &self.urls, remote::Direction::Fetch)
        } else {
            Ok(vec![None; self.urls.len()])
        }?;
        self.url_push_aliases = if should_rewrite_urls && self.push_urls.is_empty() {
            remote::init::rewrite_url_aliases_with_fallback_non_destructive(
                &self.repo.config,
                &self.urls,
                remote::Direction::Push,
                remote::Direction::Fetch,
            )
            .0
        } else {
            vec![None; self.urls.len()]
        };

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
