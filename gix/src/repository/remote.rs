use std::borrow::Cow;

use crate::{Remote, Result, bstr::BStr, config, remote};
use gix_error::{ErrorExt, ResultExt};

impl crate::Repository {
    /// Create a new remote available at the given `url`.
    ///
    /// It's configured to fetch included tags by default, similar to git.
    /// See [`with_fetch_tags(…)`][Remote::with_fetch_tags()] for a way to change it.
    ///
    /// URL rewrite rules are applied immediately. A malformed `pushInsteadOf` result is ignored when this fetch URL is merely
    /// used as the push fallback, keeping the original URL usable; malformed `insteadOf` results are reported as errors.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::basic_repo_dir()?)?;
    /// let remote = repo.remote_at("https://github.com/byron/gitoxide")?;
    ///
    /// assert_eq!(remote.name(), None);
    /// assert_eq!(
    ///     remote
    ///         .url(gix::remote::Direction::Fetch)
    ///         .expect("fetch URL")
    ///         .to_bstring(),
    ///     "https://github.com/byron/gitoxide"
    /// );
    /// # Ok(()) }
    /// ```
    pub fn remote_at<Url, E>(&self, url: Url) -> Result<Remote<'_>>
    where
        Url: TryInto<gix_url::Url, Error = E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        Remote::from_fetch_url(url, true, self)
    }

    /// Create a new remote available at the given `url` similarly to [`remote_at()`][crate::Repository::remote_at()],
    /// but don't rewrite the url according to rewrite rules.
    /// This eliminates a failure mode in case the rewritten URL is faulty, allowing to selectively [apply rewrite
    /// rules][Remote::rewrite_urls()] later and do so non-destructively.
    pub fn remote_at_without_url_rewrite<Url, E>(&self, url: Url) -> Result<Remote<'_>>
    where
        Url: TryInto<gix_url::Url, Error = E>,
        E: std::error::Error + Send + Sync + 'static,
    {
        Remote::from_fetch_url(url, false, self)
    }

    /// Find the configured remote with the given `name_or_url` or report an error,
    /// similar to [`try_find_remote(…)`][Self::try_find_remote()].
    ///
    /// Note that we will obtain remotes only if we deem them [trustworthy][crate::open::Options::filter_config_section()].
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::remote_repo_dir("clone")?)?;
    /// let remote = repo.find_remote("origin")?;
    ///
    /// assert_eq!(remote.name().expect("configured"), "origin");
    /// assert_eq!(remote.refspecs(gix::remote::Direction::Fetch).len(), 1);
    /// # Ok(()) }
    /// ```
    pub fn find_remote(&self, name_or_url: impl gix_utils::AsBStr) -> Result<Remote<'_>> {
        let name_or_url = name_or_url.as_bstr();
        self.try_find_remote(name_or_url)
            .ok_or_else(|| gix_error::message!("The remote named {name_or_url:?} did not exist").not_found_error())?
    }

    /// Find the default remote as configured, or `None` if no such configuration could be found.
    ///
    /// See [`remote_default_name()`](Self::remote_default_name()) for more information on the `direction` parameter.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::remote_repo_dir("clone")?)?;
    /// let remote = repo
    ///     .find_default_remote(gix::remote::Direction::Fetch)
    ///     .expect("configured")?;
    ///
    /// assert_eq!(remote.name().expect("named"), "origin");
    /// # Ok(()) }
    /// ```
    pub fn find_default_remote(&self, direction: remote::Direction) -> Option<Result<Remote<'_>>> {
        self.remote_default_name(direction).map(|name| self.find_remote(name))
    }

    /// Find the configured remote with the given `name_or_url` or return `None` if it doesn't exist,
    /// for the purpose of fetching or pushing data. Lookup uses the exact `[remote "…"]` subsection bytes, even when
    /// `name_or_url` looks like a URL or is `.`.
    /// Use [`Self::find_fetch_remote()`] with `Some(name_or_url)` to fall back to an anonymous remote at that URL or path
    /// when no subsection matches, or [`Self::remote_at()`] to create one without looking up a subsection.
    ///
    /// There are various error kinds related to partial information or incorrectly formatted URLs or ref-specs.
    /// Invalid configured URLs or refspecs include `name_or_url` bytes as `input`
    /// [metadata](gix_error::Error::metadata()).
    /// Also note that the created `Remote` may have neither fetch nor push ref-specs set at all.
    /// Unlike Git, a configured remote with no effective fetch URL remains without a fetch URL unless its name can also be
    /// interpreted as a URL or path. For `.` the fallback is this repository's Git directory. This applies equally to remotes
    /// whose URL list was cleared by an empty value and remotes configured only with a push URL.
    ///
    /// URL rewrite rules are applied while constructing the remote. A malformed `pushInsteadOf` result is ignored when a fetch
    /// URL is used as the push fallback, keeping the original URL usable. Malformed `insteadOf` results for fetch URLs or explicit
    /// push URLs are reported as errors. Use [`try_find_remote_without_url_rewrite()`](Self::try_find_remote_without_url_rewrite)
    /// with [`Remote::rewrite_urls()`] to defer rewriting and handle such errors after constructing the remote.
    ///
    /// Note that ref-specs are de-duplicated right away, keeping their configured order as the first configured matching spec should win.
    ///
    /// We will only include information if we deem it [trustworthy][crate::open::Options::filter_config_section()].
    pub fn try_find_remote<'a>(&self, name_or_url: impl Into<&'a BStr>) -> Option<Result<Remote<'_>>> {
        self.try_find_remote_inner(name_or_url.into(), true)
    }

    /// This method emulate what `git fetch <remote>` does in order to obtain a remote to fetch from.
    ///
    /// As such, with `name_or_url` being `Some`, it will:
    ///
    /// * use `name_or_url` verbatim if it is a URL, creating a new remote in memory as needed.
    /// * find the named remote if `name_or_url` is a remote name
    ///
    /// If `name_or_url` is `None`:
    ///
    /// * use the current `HEAD` branch to find a configured remote
    /// * fall back to either a generally configured remote or the only configured remote.
    ///
    /// Fail if no remote could be found despite all of the above.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::remote_repo_dir("clone")?)?;
    /// let remote_names: Vec<_> = repo.remote_names().into_iter().map(|name| name.to_string()).collect();
    /// assert_eq!(remote_names, vec!["myself".to_owned(), "origin".to_owned()]);
    ///
    /// let remote = repo.find_fetch_remote(None)?;
    /// assert_eq!(remote.name().expect("configured"), "origin");
    /// # Ok(()) }
    /// ```
    pub fn find_fetch_remote(&self, name_or_url: Option<&BStr>) -> Result<Remote<'_>> {
        Ok(match name_or_url {
            Some(name) => match self.try_find_remote(name).and_then(std::result::Result::ok) {
                Some(remote) => remote,
                None => self.remote_at(
                    gix_url::parse(name).or_raise(|| gix_error::message("remote name could not be parsed as URL"))?,
                )?,
            },
            None => self
                .head()?
                .into_remote(remote::Direction::Fetch)
                .transpose()?
                .map(Ok)
                .or_else(|| self.find_default_remote(remote::Direction::Fetch))
                .ok_or_else(|| {
                    gix_error::message("No configured remote could be found, or too many were available")
                        .not_found_error()
                })??,
        })
    }

    /// Similar to [`try_find_remote()`][Self::try_find_remote()], but removes a failure mode if rewritten URLs turn out to be invalid
    /// as it skips rewriting them.
    /// Use this in conjunction with [`Remote::rewrite_urls()`] to non-destructively apply the rules and keep the failed urls unchanged.
    pub fn try_find_remote_without_url_rewrite<'a>(
        &self,
        name_or_url: impl Into<&'a BStr>,
    ) -> Option<Result<Remote<'_>>> {
        self.try_find_remote_inner(name_or_url.into(), false)
    }

    pub(super) fn self_remote_url(&self) -> Result<gix_url::Url> {
        let mut url = gix_url::parse(".")?;
        // The Git directory is already a filesystem path, not URL syntax (e.g. a verbatim Windows path).
        url.path = gix_path::into_bstr(self.current_dir().join(self.git_dir()))?.into_owned();
        Ok(url)
    }

    /// Check whether an exact `[remote "…"]` subsection matches `subsection` and passes the repository's config filter.
    ///
    /// Like Git, count a remote as configured even if its subsection only sets options such as `prune`, with no URL
    /// or refspecs. This also lets callers tell whether a value like `team/origin` names a configured remote rather
    /// than a possible direct path, without parsing the remote's settings.
    pub(crate) fn has_trusted_remote_subsection(&self, subsection: &BStr) -> bool {
        let filter = self.filter_config_section();
        self.config
            .resolved
            .sections_by_name("remote")
            .is_some_and(|mut sections| {
                sections.any(|section| section.header().subsection_name() == Some(subsection) && filter(section.meta()))
            })
    }

    fn try_find_remote_inner(&self, subsection: &BStr, rewrite_urls: bool) -> Option<Result<Remote<'_>>> {
        fn config_spec<T: config::tree::keys::Validate>(
            specs: Vec<crate::bstr::BString>,
            name_or_url: &BStr,
            key: &'static config::tree::keys::Any<T>,
            op: gix_refspec::parse::Operation,
        ) -> Result<Vec<gix_refspec::RefSpec>> {
            let kind = key.name;
            // Like `git`, keep the configured order, as the first matching spec may win.
            let mut unique = Vec::with_capacity(specs.len());
            let mut seen = std::collections::HashSet::with_capacity(specs.len());
            for spec in specs {
                let spec = key.try_into_refspec(spec, op).or_raise(|| {
                    crate::error::validation(format!("{kind} ref-spec under `remote.{name_or_url}` was invalid"))
                        .with_input(name_or_url.to_owned())
                })?;
                if seen.insert(spec.clone()) {
                    unique.push(spec);
                }
            }
            Ok(unique)
        }

        let filter = self.filter_config_section();

        let config = &self.config.resolved;
        let config_urls = |key: &'static config::tree::keys::Url, kind: &'static str| {
            config
                .strings_filter_by("remote", subsection, key.name, filter)
                .map(|urls| {
                    let mut effective_urls = Vec::new();
                    for url in urls {
                        if url.is_empty() {
                            // empty urls are a sentinel, indicating all prior urls should be cleared.
                            // This makes overriding global remote configuration possible.
                            effective_urls.clear();
                        } else {
                            effective_urls.push(url);
                        }
                    }

                    effective_urls
                        .into_iter()
                        .map(|url| {
                            key.try_into_url(url).or_raise(|| {
                                gix_error::message!("The {kind} url under `remote.{subsection}` was invalid")
                                    .validation()
                                    .with_input(subsection.to_owned())
                            })
                        })
                        .collect()
                })
        };
        let urls = config_urls(&config::tree::Remote::URL, "fetch");
        let push_urls = config_urls(&config::tree::Remote::PUSH_URL, "push");

        let fetch_specs = config
            .strings_filter_by("remote", subsection, config::tree::Remote::FETCH.name, filter)
            .map(|specs| {
                config_spec(
                    specs,
                    subsection,
                    &config::tree::Remote::FETCH,
                    gix_refspec::parse::Operation::Fetch,
                )
            });
        let push_specs = config
            .strings_filter_by("remote", subsection, config::tree::Remote::PUSH.name, filter)
            .map(|specs| {
                config_spec(
                    specs,
                    subsection,
                    &config::tree::Remote::PUSH,
                    gix_refspec::parse::Operation::Push,
                )
            });
        let fetch_tags =
            config
                .string_filter_by("remote", subsection, config::tree::Remote::TAG_OPT.name, filter)
                .map(|value| {
                    config::tree::Remote::TAG_OPT.try_into_tag_opt(value).or_raise(|| gix_error::message(
                        "The value for 'remote.<name>.tagOpt` is invalid and must either be '--tags' or '--no-tags'",
                    ))
                });
        let fetch_tags = match fetch_tags {
            Some(Ok(v)) => v,
            Some(Err(err)) => return Some(Err(err)),
            None => Default::default(),
        };

        match (urls, fetch_specs, push_urls, push_specs) {
            (None, None, None, None) if !self.has_trusted_remote_subsection(subsection) => None,
            (urls, fetch_specs, push_urls, push_specs) => {
                let mut urls = match urls {
                    Some(Ok(v)) => v,
                    Some(Err(err)) => return Some(Err(err)),
                    None => Vec::new(),
                };
                let push_urls = match push_urls {
                    Some(Ok(v)) => v,
                    Some(Err(err)) => return Some(Err(err)),
                    None => Vec::new(),
                };
                let fetch_specs = match fetch_specs {
                    Some(Ok(v)) => v,
                    Some(Err(err)) => return Some(Err(err)),
                    None => Vec::new(),
                };
                let push_specs = match push_specs {
                    Some(Ok(v)) => v,
                    Some(Err(err)) => return Some(Err(err)),
                    None => Vec::new(),
                };
                let configured_name = remote::ConfiguredName::from_config(subsection);
                let fetch_url_from_subsection = if urls.is_empty() {
                    match self.configured_remote_url_fallback(&configured_name) {
                        Ok(Some(url)) => {
                            urls.push(url);
                            true
                        }
                        Ok(None) => false,
                        Err(err) => return Some(Err(err)),
                    }
                } else {
                    false
                };

                Some(Remote::from_preparsed_config(
                    configured_name,
                    urls,
                    fetch_url_from_subsection,
                    push_urls,
                    fetch_specs,
                    push_specs,
                    rewrite_urls,
                    fetch_tags,
                    self,
                ))
            }
        }
    }

    /// Derive a fetch URL from the configured remote's subsection name when no fetch URL is available.
    ///
    /// `.` points to this repository's Git directory. Names containing `/` or recognized as absolute paths
    /// must parse as URLs, and parsing errors are reported. Other names only qualify if they parse as non-file
    /// URLs, so a plain symbolic name doesn't silently become a relative filesystem path.
    fn configured_remote_url_fallback(&self, name: &remote::ConfiguredName) -> Result<Option<gix_url::Url>> {
        if matches!(name, remote::ConfiguredName::Dot) {
            return self.self_remote_url().map(Some);
        }
        let value = name.as_config_key();
        let is_path_or_url = value.contains(&b'/') || gix_path::is_absolute(gix_path::from_bstr(value)?);
        match config::tree::Remote::URL.try_into_url(Cow::Borrowed(value)) {
            Ok(url) if is_path_or_url || url.scheme != gix_url::Scheme::File => Ok(Some(url)),
            Err(err) if is_path_or_url => {
                Err(err.and_raise(gix_error::message!("The fetch url under `remote.{value}` was invalid").validation()))
            }
            Ok(_) | Err(_) => Ok(None),
        }
    }
}
