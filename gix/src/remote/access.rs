use gix_refspec::RefSpec;

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

    /// Return the url used for the given `direction` with rewrites from `url.<base>.insteadOf|pushInsteadOf`, unless the instance
    /// was created with one of the `_without_url_rewrite()` methods.
    /// For pushing, this is the `remote.<name>.pushUrl` or the `remote.<name>.url` used for fetching, and for fetching it's
    /// the `remote.<name>.url`.
    /// Note that it's possible to only have the push url set, in which case there will be no way to fetch from the remote as
    /// the push-url isn't used for that.
    pub fn url(&self, direction: remote::Direction) -> Option<&gix_url::Url> {
        match direction {
            remote::Direction::Fetch => self.url_alias.as_ref().or(self.url.as_ref()),
            remote::Direction::Push => self
                .push_url_alias
                .as_ref()
                .or(self.push_url.as_ref())
                .or_else(|| self.url(remote::Direction::Fetch)),
        }
    }
}

/// Modification
impl Remote<'_> {
    /// Read `url.<base>.insteadOf|pushInsteadOf` configuration variables and apply them to our urls, changing them in place.
    ///
    /// This happens only once, and one if them may be changed even when reporting an error.
    /// If both urls fail, only the first error (for fetch urls) is reported.
    pub fn rewrite_urls(&mut self) -> Result<&mut Self, remote::init::Error> {
        let url_err = match remote::init::rewrite_url(&self.repo.config, self.url.as_ref(), remote::Direction::Fetch) {
            Ok(url) => {
                self.url_alias = url;
                None
            }
            Err(err) => err.into(),
        };
        let push_url_err =
            match remote::init::rewrite_url(&self.repo.config, self.push_url.as_ref(), remote::Direction::Push) {
                Ok(url) => {
                    self.push_url_alias = url;
                    None
                }
                Err(err) => err.into(),
            };
        url_err.or(push_url_err).map(Err::<&mut Self, _>).transpose()?;
        Ok(self)
    }

    /// Replace all currently set refspecs, typically from configuration, with the given `specs` for `direction`,
    /// or `None` if one of the input specs could not be parsed.
    pub fn replace_refspecs<Spec>(
        &mut self,
        specs: impl IntoIterator<Item = Spec>,
        direction: remote::Direction,
    ) -> Result<(), gix_refspec::parse::Error>
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
