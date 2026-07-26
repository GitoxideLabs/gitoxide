#![allow(clippy::result_large_err)]
use gix_error::ResultExt;

use crate::{Remote, bstr::BStr, config, remote, remote::find};

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
    pub fn remote_at<Url, E>(&self, url: Url) -> Result<Remote<'_>, remote::init::Error>
    where
        Url: TryInto<gix_url::Url, Error = E>,
        gix_url::parse::Error: From<E>,
    {
        Remote::from_fetch_url(url, true, self)
    }

    /// Create a new remote available at the given `url` similarly to [`remote_at()`][crate::Repository::remote_at()],
    /// but don't rewrite the url according to rewrite rules.
    /// This eliminates a failure mode in case the rewritten URL is faulty, allowing to selectively [apply rewrite
    /// rules][Remote::rewrite_urls()] later and do so non-destructively.
    pub fn remote_at_without_url_rewrite<Url, E>(&self, url: Url) -> Result<Remote<'_>, remote::init::Error>
    where
        Url: TryInto<gix_url::Url, Error = E>,
        gix_url::parse::Error: From<E>,
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
    /// assert_eq!(remote.name().expect("configured").as_bstr(), "origin");
    /// assert_eq!(remote.refspecs(gix::remote::Direction::Fetch).len(), 1);
    /// # Ok(()) }
    /// ```
    pub fn find_remote(&self, name_or_url: impl gix_utils::AsBStr) -> Result<Remote<'_>, find::existing::Error> {
        let name_or_url = name_or_url.as_bstr();
        Ok(self
            .try_find_remote(name_or_url)
            .ok_or_else(|| find::existing::Error::NotFound {
                name: name_or_url.into(),
            })??)
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
    /// assert_eq!(remote.name().expect("named").as_bstr(), "origin");
    /// # Ok(()) }
    /// ```
    pub fn find_default_remote(
        &self,
        direction: remote::Direction,
    ) -> Option<Result<Remote<'_>, find::existing::Error>> {
        self.remote_default_name(direction).map(|name| self.find_remote(name))
    }

    /// Find the configured remote with the given `name_or_url` or return `None` if it doesn't exist,
    /// for the purpose of fetching or pushing data.
    ///
    /// There are various error kinds related to partial information or incorrectly formatted URLs or ref-specs.
    /// Also note that the created `Remote` may have neither fetch nor push ref-specs set at all.
    /// Unlike Git, a configured remote with a symbolic name and no effective fetch URL remains without a fetch URL. If the
    /// requested remote name is itself a URL, it is used as the fetch URL instead. This applies equally to remotes whose URL
    /// list was cleared by an empty value and remotes configured only with a push URL.
    ///
    /// URL rewrite rules are applied while constructing the remote. A malformed `pushInsteadOf` result is ignored when a fetch
    /// URL is used as the push fallback, keeping the original URL usable. Malformed `insteadOf` results for fetch URLs or explicit
    /// push URLs are reported as errors. Use [`try_find_remote_without_url_rewrite()`](Self::try_find_remote_without_url_rewrite)
    /// with [`Remote::rewrite_urls()`] to defer rewriting and handle such errors after constructing the remote.
    ///
    /// Note that ref-specs are de-duplicated right away which may change their order. This doesn't affect matching in any way
    /// as negations/excludes are applied after includes.
    ///
    /// We will only include information if we deem it [trustworthy][crate::open::Options::filter_config_section()].
    pub fn try_find_remote<'a>(&self, name_or_url: impl Into<&'a BStr>) -> Option<Result<Remote<'_>, find::Error>> {
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
    /// assert_eq!(remote.name().expect("configured").as_bstr(), "origin");
    /// # Ok(()) }
    /// ```
    pub fn find_fetch_remote(&self, name_or_url: Option<&BStr>) -> Result<Remote<'_>, find::for_fetch::Error> {
        Ok(match name_or_url {
            Some(name) => match self.try_find_remote(name).and_then(Result::ok) {
                Some(remote) => remote,
                None => self.remote_at(gix_url::parse(name)?)?,
            },
            None => self
                .head()?
                .into_remote(remote::Direction::Fetch)
                .transpose()?
                .map(Ok)
                .or_else(|| self.find_default_remote(remote::Direction::Fetch))
                .ok_or(find::for_fetch::Error::ExactlyOneRemoteNotAvailable)??,
        })
    }

    /// Similar to [`try_find_remote()`][Self::try_find_remote()], but removes a failure mode if rewritten URLs turn out to be invalid
    /// as it skips rewriting them.
    /// Use this in conjunction with [`Remote::rewrite_urls()`] to non-destructively apply the rules and keep the failed urls unchanged.
    pub fn try_find_remote_without_url_rewrite<'a>(
        &self,
        name_or_url: impl Into<&'a BStr>,
    ) -> Option<Result<Remote<'_>, find::Error>> {
        self.try_find_remote_inner(name_or_url.into(), false)
    }

    fn try_find_remote_inner<'a>(
        &self,
        name_or_url: impl Into<&'a BStr>,
        rewrite_urls: bool,
    ) -> Option<Result<Remote<'_>, find::Error>> {
        fn config_spec<T: config::tree::keys::Validate>(
            specs: Vec<crate::bstr::BString>,
            name_or_url: &BStr,
            key: &'static config::tree::keys::Any<T>,
            op: gix_refspec::parse::Operation,
        ) -> Result<Vec<gix_refspec::RefSpec>, find::Error> {
            let kind = key.name;
            specs
                .into_iter()
                .map(|spec| {
                    key.try_into_refspec(spec, op)
                        .or_raise(|| gix_error::message!("{kind} ref-spec under `remote.{name_or_url}` was invalid"))
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(Into::into)
                .map(|mut specs| {
                    specs.sort();
                    specs.dedup();
                    specs
                })
        }

        let mut filter = self.filter_config_section();
        let name_or_url = name_or_url.into();
        // Git considers any matching remote section configured, even if it only contains unrelated keys like `prune`.
        let remote_is_configured = self
            .config
            .resolved
            .sections_by_name("remote")
            .is_some_and(|mut sections| {
                sections
                    .any(|section| section.header().subsection_name() == Some(name_or_url) && filter(section.meta()))
            });
        let mut config_urls = |key: &'static config::tree::keys::Url, kind: &'static str| {
            self.config
                .resolved
                .strings_filter(&format!("remote.{}.{}", name_or_url, key.name), &mut filter)
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
                                gix_error::message!("The {kind} url under `remote.{name_or_url}` was invalid")
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(Into::into)
                })
        };
        let urls = config_urls(&config::tree::Remote::URL, "fetch");
        let push_urls = config_urls(&config::tree::Remote::PUSH_URL, "push");
        let config = &self.config.resolved;

        let fetch_specs = config
            .strings_filter(&format!("remote.{}.{}", name_or_url, "fetch"), &mut filter)
            .map(|specs| {
                config_spec(
                    specs,
                    name_or_url,
                    &config::tree::Remote::FETCH,
                    gix_refspec::parse::Operation::Fetch,
                )
            });
        let push_specs = config
            .strings_filter(&format!("remote.{}.{}", name_or_url, "push"), &mut filter)
            .map(|specs| {
                config_spec(
                    specs,
                    name_or_url,
                    &config::tree::Remote::PUSH,
                    gix_refspec::parse::Operation::Push,
                )
            });
        let fetch_tags = config
            .string_filter(&format!("remote.{}.{}", name_or_url, "tagOpt"), &mut filter)
            .map(|value| {
                config::tree::Remote::TAG_OPT.try_into_tag_opt(value).or_raise(|| {
                    gix_error::message(
                        "The value for 'remote.<name>.tagOpt` is invalid and must either be '--tags' or '--no-tags'",
                    )
                })
                .map_err(Into::into)
            });
        let fetch_tags = match fetch_tags {
            Some(Ok(v)) => v,
            Some(Err(err)) => return Some(Err(err)),
            None => Default::default(),
        };

        match (urls, fetch_specs, push_urls, push_specs) {
            (None, None, None, None) if !remote_is_configured => None,
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
                if urls.is_empty() {
                    let name_is_url = matches!(
                        remote::Name::try_from(std::borrow::Cow::Borrowed(name_or_url)),
                        Ok(remote::Name::Url(_))
                    ) || gix_path::is_absolute(gix_path::from_bstr(name_or_url));
                    match config::tree::Remote::URL.try_into_url(std::borrow::Cow::Borrowed(name_or_url)) {
                        Ok(url) if name_is_url || url.scheme != gix_url::Scheme::File => urls.push(url),
                        Ok(_) => {}
                        Err(source) if name_is_url => {
                            return Some(
                                Err(source)
                                    .or_raise(|| {
                                        gix_error::message!("The fetch url under `remote.{name_or_url}` was invalid")
                                    })
                                    .map_err(Into::into),
                            );
                        }
                        Err(_) => {}
                    }
                }

                Some(
                    Remote::from_preparsed_config(
                        Some(name_or_url.to_owned()),
                        urls,
                        push_urls,
                        fetch_specs,
                        push_specs,
                        rewrite_urls,
                        fetch_tags,
                        self,
                    )
                    .map_err(gix_error::Error::from_error),
                )
            }
        }
    }
}
