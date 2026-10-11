use gix_error::{ResultExt, bail};
use gix_ref::FullName;
use gix_refspec::{MatchGroup, RefSpec, match_group};

#[cfg(any(feature = "blocking-network-client", feature = "async-network-client"))]
use crate::types::RemoteDetached;
use crate::{
    Remote, Result,
    bstr::{BStr, BString, ByteVec},
    remote,
};

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

    /// Return the name of the default branch on the remote, like `refs/heads/main`, as recorded locally,
    /// or `None` if it isn't known.
    ///
    /// The record is the symbolic reference `refs/remotes/<name>/HEAD`. `git clone` and `git remote set-head`
    /// point it to the remote-tracking branch of the default branch, like `refs/remotes/origin/main`,
    /// and since Git 2.48, `git fetch` creates it if it's missing. The fetch refspecs of this remote then
    /// map this remote-tracking branch back to the branch on the remote, and it's an error if they map
    /// more than one remote reference to it.
    ///
    /// `None` is returned if this remote has no name, if `refs/remotes/<name>/HEAD` doesn't exist or isn't symbolic,
    /// or if no fetch refspec maps a remote reference to its target. Note that clones made with `gix` currently store
    /// `refs/remotes/<name>/HEAD` as a direct reference, so `None` is returned for them.
    ///
    /// As the remote isn't contacted, it may have changed its default branch since, and the returned branch
    /// may not exist anymore. `git2::Remote::default_branch()`, on the other hand, asks the connected remote
    /// for its current `HEAD`.
    #[doc(alias = "git2")]
    pub fn default_branch(&self) -> Result<Option<FullName>> {
        let Some(name) = self.name() else {
            return Ok(None);
        };
        let mut head_name = BString::from("refs/remotes/");
        head_name.push_str(name.as_bstr());
        head_name.push_str("/HEAD");
        // Names that can't be part of a reference name can't have remote-tracking branches either.
        let Ok(head_name) = FullName::try_from(head_name) else {
            return Ok(None);
        };
        let Some(head) = self.repo.try_find_reference(head_name.as_bstr())? else {
            return Ok(None);
        };
        let target = head.target();
        let Some(tracking_branch) = target.try_name() else {
            return Ok(None);
        };

        let null_id = self.repo.object_hash().null();
        let mut mappings = MatchGroup::from_fetch_specs(self.fetch_specs.iter().map(RefSpec::to_ref))
            .match_rhs(std::iter::once(match_group::Item {
                full_ref_name: tracking_branch.as_bstr(),
                target: &null_id,
                object: None,
            }))
            .mappings
            .into_iter();
        let Some(mapping) = mappings.next() else {
            return Ok(None);
        };
        if let Some(other_mapping) = mappings.next() {
            bail!(gix_error::validation(format!(
                "Both '{}' and '{}' map to '{}', so the default branch is ambiguous",
                mapping.lhs,
                other_mapping.lhs,
                tracking_branch.as_bstr()
            )));
        }
        let match_group::SourceRef::FullName(remote_branch) = mapping.lhs else {
            return Ok(None);
        };
        FullName::try_from(remote_branch.into_owned())
            .map(Some)
            .or_raise(|| gix_error::validation("The remote reference that the remote HEAD maps to has an invalid name"))
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
    pub fn rewrite_urls(&mut self) -> Result<&mut Self> {
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
    ) -> Result
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
            .collect::<std::result::Result<_, _>>()?;
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
