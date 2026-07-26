use gix_refspec::RefSpec;

use crate::{Remote, Repository, config, remote};

mod error {
    use crate::bstr::BString;

    // TODO(review): kept concrete. Erasing this would give `clone::fetch::Error`
    //                (`gix/src/clone/fetch/mod.rs`) a second `From<gix_error::Error>` impl: it embeds
    //                this type via `RemoteInit(#[from] crate::remote::init::Error)`, but has already
    //                used its one erased slot via `ParseConfig(#[from] crate::config::overrides::Error)`
    //                (`config::overrides::Error` is already erased) — E0119.
    /// The error returned by [`Repository::remote_at(…)`][crate::Repository::remote_at()].
    #[derive(Debug, thiserror::Error)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Url(#[from] gix_url::parse::Error),
        #[error("The rewritten {kind} url {rewritten_url:?} failed to parse")]
        RewrittenUrlInvalid {
            kind: &'static str,
            rewritten_url: BString,
            source: gix_url::parse::Error,
        },
    }
}
pub use error::Error;

use crate::bstr::BString;

type UrlAliases = Vec<Option<gix_url::Url>>;
type UrlRewriteAliases = (UrlAliases, UrlAliases, UrlAliases);

/// Initialization
impl<'repo> Remote<'repo> {
    #[expect(clippy::too_many_arguments)]
    pub(crate) fn from_preparsed_config(
        name_or_url: Option<BString>,
        urls: Vec<gix_url::Url>,
        push_urls: Vec<gix_url::Url>,
        fetch_specs: Vec<RefSpec>,
        push_specs: Vec<RefSpec>,
        should_rewrite_urls: bool,
        fetch_tags: remote::fetch::Tags,
        repo: &'repo Repository,
    ) -> Result<Self, Error> {
        let (url_aliases, url_push_aliases, push_url_aliases) = if should_rewrite_urls {
            rewrite_urls(&repo.config, &urls, &push_urls)
        } else {
            Ok((
                vec![None; urls.len()],
                vec![None; urls.len()],
                vec![None; push_urls.len()],
            ))
        }?;
        Ok(Remote {
            name: name_or_url.map(Into::into),
            urls,
            url_aliases,
            url_push_aliases,
            push_urls,
            push_url_aliases,
            fetch_specs,
            push_specs,
            fetch_tags,
            repo,
        })
    }

    pub(crate) fn from_fetch_url<Url, E>(
        url: Url,
        should_rewrite_urls: bool,
        repo: &'repo Repository,
    ) -> Result<Self, Error>
    where
        Url: TryInto<gix_url::Url, Error = E>,
        gix_url::parse::Error: From<E>,
    {
        Self::from_fetch_url_inner(
            url.try_into().map_err(|err| Error::Url(err.into()))?,
            should_rewrite_urls,
            repo,
        )
    }

    fn from_fetch_url_inner(
        url: gix_url::Url,
        should_rewrite_urls: bool,
        repo: &'repo Repository,
    ) -> Result<Self, Error> {
        let urls = vec![url];
        let (url_aliases, url_push_aliases, _) = if should_rewrite_urls {
            rewrite_urls(&repo.config, &urls, &[])
        } else {
            Ok((vec![None; urls.len()], vec![None; urls.len()], Vec::new()))
        }?;
        Ok(Remote {
            name: None,
            urls,
            url_aliases,
            url_push_aliases,
            push_urls: Vec::new(),
            push_url_aliases: Vec::new(),
            fetch_specs: Vec::new(),
            push_specs: Vec::new(),
            fetch_tags: Default::default(),
            repo,
        })
    }
}

pub(crate) fn rewrite_url(
    config: &config::Cache,
    url: &gix_url::Url,
    direction: remote::Direction,
    error_kind: remote::Direction,
) -> Result<Option<gix_url::Url>, Error> {
    config
        .url_rewrite()
        .longest(url, direction)
        .map(|url| {
            gix_url::parse(&url).map_err(|err| Error::RewrittenUrlInvalid {
                kind: match error_kind {
                    remote::Direction::Fetch => "fetch",
                    remote::Direction::Push => "push",
                },
                source: err,
                rewritten_url: url,
            })
        })
        .transpose()
}

pub(crate) fn rewrite_url_aliases(
    config: &config::Cache,
    urls: &[gix_url::Url],
    direction: remote::Direction,
) -> Result<Vec<Option<gix_url::Url>>, Error> {
    rewrite_url_aliases_with_error_kind(config, urls, direction, direction)
}

pub(crate) fn rewrite_url_aliases_with_error_kind(
    config: &config::Cache,
    urls: &[gix_url::Url],
    direction: remote::Direction,
    error_kind: remote::Direction,
) -> Result<Vec<Option<gix_url::Url>>, Error> {
    urls.iter()
        .map(|url| rewrite_url(config, url, direction, error_kind))
        .collect()
}

pub(crate) fn rewrite_url_aliases_non_destructive(
    config: &config::Cache,
    urls: &[gix_url::Url],
    direction: remote::Direction,
) -> (Vec<Option<gix_url::Url>>, Option<Error>) {
    rewrite_url_aliases_non_destructive_with_error_kind(config, urls, direction, direction)
}

pub(crate) fn rewrite_url_aliases_non_destructive_with_error_kind(
    config: &config::Cache,
    urls: &[gix_url::Url],
    direction: remote::Direction,
    error_kind: remote::Direction,
) -> (Vec<Option<gix_url::Url>>, Option<Error>) {
    let mut first_error = None;
    let aliases = urls
        .iter()
        .map(|url| match rewrite_url(config, url, direction, error_kind) {
            Ok(alias) => alias,
            Err(err) => {
                first_error.get_or_insert(err);
                None
            }
        })
        .collect();
    (aliases, first_error)
}

pub(crate) fn rewrite_url_aliases_with_fallback_non_destructive(
    config: &config::Cache,
    urls: &[gix_url::Url],
    direction: remote::Direction,
    fallback: remote::Direction,
) -> (Vec<Option<gix_url::Url>>, Option<Error>) {
    let mut first_error = None;
    let aliases = urls
        .iter()
        .map(|url| match rewrite_url(config, url, direction, direction) {
            Ok(Some(alias)) => Some(alias),
            Ok(None) => match rewrite_url(config, url, fallback, direction) {
                Ok(alias) => alias,
                Err(err) => {
                    first_error.get_or_insert(err);
                    None
                }
            },
            Err(err) => {
                first_error.get_or_insert(err);
                None
            }
        })
        .collect();
    (aliases, first_error)
}

pub(crate) fn rewrite_urls(
    config: &config::Cache,
    urls: &[gix_url::Url],
    push_urls: &[gix_url::Url],
) -> Result<UrlRewriteAliases, Error> {
    let url_aliases = rewrite_url_aliases(config, urls, remote::Direction::Fetch)?;
    let url_push_aliases = if push_urls.is_empty() {
        rewrite_url_aliases_with_fallback_non_destructive(
            config,
            urls,
            remote::Direction::Push,
            remote::Direction::Fetch,
        )
        .0
    } else {
        vec![None; urls.len()]
    };
    let push_url_aliases =
        rewrite_url_aliases_with_error_kind(config, push_urls, remote::Direction::Fetch, remote::Direction::Push)?;

    Ok((url_aliases, url_push_aliases, push_url_aliases))
}
