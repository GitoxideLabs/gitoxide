use gix_refspec::RefSpec;

#[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
use crate::types::RemoteDetached;
use crate::{Remote, bstr::BStr, remote};

/// Access
impl<'repo> Remote<'repo> {
    /// Return the name of this remote or `None` if it wasn't persisted to disk yet.
    pub fn name(&self) -> Option<&remote::Name<'static>> {
        self.name.as_ref()
    }

    /// Return our repository reference.
    pub fn repo(&self) -> &'repo crate::Repository {
        self.repo
    }

    /// Return the set of ref-specs used for `direction`, which may be empty, in order of occurrence in the configuration.
    pub fn refspecs(&self, direction: remote::Direction) -> &[RefSpec] {
        match direction {
            remote::Direction::Fetch => &self.fetch_specs,
            remote::Direction::Push => &self.push_specs,
        }
    }

    /// Return how we handle tags when fetching the remote.
    pub fn fetch_tags(&self) -> remote::fetch::Tags {
        self.fetch_tags
    }

    /// Return the first url used for the given `direction` with rewrites from `url.<base>.insteadOf|pushInsteadOf`, unless the instance
    /// was created with one of the `_without_url_rewrite()` methods.
    /// See [`urls()`](Self::urls()) for how rewrite rules differ between fetch URLs, explicit push URLs, and push fallbacks.
    /// For pushing, this is the first `remote.<name>.pushUrl` or the first `remote.<name>.url` used for fetching, and for
    /// fetching it's the first `remote.<name>.url`. Unlike `git remote get-url`, a missing fetch URL doesn't fall back to a
    /// symbolic remote name, but a URL-shaped remote name may itself be used as the fallback URL.
    /// Note that it's possible to only have the push url set, in which case there will be no way to fetch from the remote as
    /// the push-url isn't used for that.
    pub fn url(&self, direction: remote::Direction) -> Option<&gix_url::Url> {
        self.urls(direction).next()
    }

    /// Return all urls used for the given `direction` with rewrites from `url.<base>.insteadOf|pushInsteadOf`, unless the
    /// instance was created with one of the `_without_url_rewrite()` methods.
    ///
    /// Fetch URLs are rewritten with `url.<base>.insteadOf`. Explicit `remote.<name>.pushUrl` values are also rewritten with
    /// `insteadOf`, and `pushInsteadOf` is ignored for them. If no explicit push URL is configured, the fetch URLs are used
    /// as push fallbacks: matching `pushInsteadOf` rules take precedence, with `insteadOf` used when none match.
    ///
    /// Values are returned in configuration order.
    pub fn urls(&self, direction: remote::Direction) -> impl Iterator<Item = &gix_url::Url> + '_ {
        let (urls, aliases) = self.urls_and_aliases(direction);
        debug_assert_eq!(
            urls.len(),
            aliases.len(),
            "each URL should have a corresponding rewrite slot"
        );
        urls.iter()
            .zip(aliases)
            .map(|(url, alias)| alias.as_ref().unwrap_or(url))
    }

    fn urls_and_aliases(&self, direction: remote::Direction) -> (&[gix_url::Url], &[Option<gix_url::Url>]) {
        match direction {
            remote::Direction::Fetch => (&self.urls, &self.url_aliases),
            remote::Direction::Push if self.push_urls.is_empty() => (&self.urls, &self.url_push_aliases),
            remote::Direction::Push => (&self.push_urls, &self.push_url_aliases),
        }
    }

    /// Return a clone of this remote without its repository reference.
    #[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
    pub(crate) fn detached(&self) -> RemoteDetached {
        self.clone().into()
    }
}

/// Access
#[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
impl RemoteDetached {
    /// Return the name of this remote or `None` if it wasn't persisted to disk yet.
    pub(crate) fn name(&self) -> Option<&remote::Name<'static>> {
        self.name.as_ref()
    }

    /// Return the set of ref-specs used for fetching, which may be empty, in order of occurrence in the configuration.
    pub(crate) fn fetch_refspecs(&self) -> &[RefSpec] {
        &self.fetch_specs
    }
}

/// Modification
impl Remote<'_> {
    /// Re-read `url.<base>.insteadOf|pushInsteadOf` and recompute the effective URLs returned by [`url()`](Self::url()) and
    /// [`urls()`](Self::urls()). This may be called repeatedly to refresh rewrite rules after configuration changes.
    ///
    /// Every URL is attempted non-destructively: successful rewrites remain effective if another rewritten URL is malformed,
    /// while a failed entry keeps using its original URL. The first error is returned in fetch, push-fallback, explicit-push
    /// order. See [`urls()`](Self::urls()) for which rules apply to each category.
    pub fn rewrite_urls(&mut self) -> Result<&mut Self, remote::init::Error> {
        let (url_aliases, url_err) =
            remote::init::rewrite_url_aliases_non_destructive(&self.repo.config, &self.urls, remote::Direction::Fetch);
        self.url_aliases = url_aliases;
        let url_push_err = if self.push_urls.is_empty() {
            let (url_push_aliases, err) = remote::init::rewrite_url_aliases_with_fallback_non_destructive(
                &self.repo.config,
                &self.urls,
                remote::Direction::Push,
                remote::Direction::Fetch,
            );
            self.url_push_aliases = url_push_aliases;
            err
        } else {
            self.url_push_aliases = vec![None; self.urls.len()];
            None
        };
        let (push_url_aliases, push_url_err) = remote::init::rewrite_url_aliases_non_destructive_with_error_kind(
            &self.repo.config,
            &self.push_urls,
            remote::Direction::Fetch,
            remote::Direction::Push,
        );
        self.push_url_aliases = push_url_aliases;
        url_err
            .or(url_push_err)
            .or(push_url_err)
            .map(Err::<&mut Self, _>)
            .transpose()?;
        Ok(self)
    }

    /// Replace all currently set refspecs, typically from configuration, with the given `specs` for `direction`,
    /// or `None` if one of the input specs could not be parsed.
    pub fn replace_refspecs<Spec>(
        &mut self,
        specs: impl IntoIterator<Item = Spec>,
        direction: remote::Direction,
    ) -> Result<(), gix_error::Error>
    where
        Spec: AsRef<BStr>,
    {
        use remote::Direction::*;
        let specs: Vec<_> = specs
            .into_iter()
            .map(|spec| {
                gix_refspec::parse(
                    spec.as_ref(),
                    match direction {
                        Push => gix_refspec::parse::Operation::Push,
                        Fetch => gix_refspec::parse::Operation::Fetch,
                    },
                )
                .map(|url| url.to_owned())
            })
            .collect::<Result<_, _>>()?;
        let dst = match direction {
            Push => &mut self.push_specs,
            Fetch => &mut self.fetch_specs,
        };
        *dst = specs;
        Ok(())
    }
}

#[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
impl From<Remote<'_>> for RemoteDetached {
    fn from(
        Remote {
            name,
            urls,
            url_aliases,
            url_push_aliases: _,
            fetch_specs,
            fetch_tags,
            push_urls: _,
            push_url_aliases: _,
            push_specs: _,
            repo: _,
        }: Remote<'_>,
    ) -> Self {
        RemoteDetached {
            name,
            urls,
            url_aliases,
            fetch_specs,
            fetch_tags,
        }
    }
}
