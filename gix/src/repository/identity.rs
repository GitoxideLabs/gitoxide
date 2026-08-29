use crate::{
    bstr::{BString, ByteSlice},
    config,
    config::tree::{Author, Committer, Key, User, gitoxide, keys},
};

/// Identity handling.
///
/// # Deviation
///
/// There is no notion of a default user like in git, and instead failing to provide a user
/// is fatal. That way, we enforce correctness and force application developers to take care
/// of this issue. Use [`Repository::committer_or_set_fallback()`][crate::Repository::committer_or_set_fallback]
/// to install an application fallback without overriding an already configured user identity.
impl crate::Repository {
    /// Return the committer as configured by this repository, which is determined by…
    ///
    /// * …the `GIT_COMMITTER_(NAME|EMAIL)` environment variables…
    /// * …the git configuration `committer.name|email`…
    /// * …the configuration for `user.name|email` as fallback…
    /// * …the `EMAIL` environment variable as an email-only fallback…
    /// * …the `gitoxide.committer.(name|email)Fallback` configuration as a last resort…
    ///
    /// …and in that order, with name and email resolved independently. `GIT_COMMITTER_DATE` controls the time separately.
    /// Returns `None` if no committer name or email was configured, or `Some(Err(…))` if the committer date could not be
    /// parsed.
    ///
    /// # Note
    ///
    /// The values are cached when the repository is instantiated.
    pub fn committer(&self) -> Option<Result<gix_actor::SignatureRef<'_>, config::time::Error>> {
        let p = self.config.personas();

        Ok(gix_actor::SignatureRef {
            name: p
                .committer
                .name
                .as_ref()
                .or(p.user.name.as_ref())
                .or(p.committer_fallback.name.as_ref())
                .map(AsRef::as_ref)?,
            email: p
                .committer
                .email
                .as_ref()
                .or(p.user.email.as_ref())
                .or(p.committer_fallback.email.as_ref())
                .map(AsRef::as_ref)?,
            time: p.committer.time.as_ref().map(AsRef::as_ref)?,
        })
        .into()
    }

    /// Return the configured committer, or install `name` and `email` as fallback in memory on this instance if no complete
    /// committer can be resolved. The fallback is then returned and future calls to [`committer()`](Self::committer()) will
    /// return it as well.
    ///
    /// If either part of the committer is missing, both fallback values are installed. Environment, `committer.*`, and
    /// `user.*` values still take precedence individually.
    pub fn committer_or_set_fallback(
        &mut self,
        name: impl gix_utils::AsBStr,
        email: impl gix_utils::AsBStr,
    ) -> Result<gix_actor::SignatureRef<'_>, config::commit_signature::Error> {
        if self.committer().is_none() {
            let mut config = gix_config::File::new(gix_config::file::Metadata::api());
            config.set_raw_value(gitoxide::Committer::NAME_FALLBACK, name)?;
            config.set_raw_value(gitoxide::Committer::EMAIL_FALLBACK, email)?;
            let mut repo_config = self.config_snapshot_mut();
            repo_config.append(config)?;
        }
        Ok(self.committer().expect("committer was just set")?)
    }

    /// Return the configured committer or install a generic fallback in memory on this instance.
    ///
    /// This is equivalent to calling [`committer_or_set_fallback()`](Self::committer_or_set_fallback()) with
    /// `no name configured <noEmailAvailable@example.com>`.
    pub fn committer_or_set_generic_fallback(
        &mut self,
    ) -> Result<gix_actor::SignatureRef<'_>, config::commit_signature::Error> {
        self.committer_or_set_fallback("no name configured", "noEmailAvailable@example.com")
    }

    /// Return the author as configured by this repository, which is determined by…
    ///
    /// * …the `GIT_AUTHOR_(NAME|EMAIL)` environment variables…
    /// * …the git configuration `author.name|email`…
    /// * …the configuration for `user.name|email` as fallback…
    /// * …the `EMAIL` environment variable as an email-only fallback…
    /// * …the `gitoxide.author.(name|email)Fallback` configuration as a last resort…
    ///
    /// …and in that order, with name and email resolved independently. `GIT_AUTHOR_DATE` controls the time separately.
    /// Returns `None` if there was nothing configured.
    ///
    /// # Note
    ///
    /// The values are cached when the repository is instantiated.
    pub fn author(&self) -> Option<Result<gix_actor::SignatureRef<'_>, config::time::Error>> {
        let p = self.config.personas();

        Ok(gix_actor::SignatureRef {
            name: p
                .author
                .name
                .as_ref()
                .or(p.user.name.as_ref())
                .or(p.author_fallback.name.as_ref())
                .map(AsRef::as_ref)?,
            email: p
                .author
                .email
                .as_ref()
                .or(p.user.email.as_ref())
                .or(p.author_fallback.email.as_ref())
                .map(AsRef::as_ref)?,
            time: p.author.time.as_ref().map(AsRef::as_ref)?,
        })
        .into()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Entity {
    pub name: Option<BString>,
    pub email: Option<BString>,
    pub time: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct Personas {
    user: Entity,
    committer: Entity,
    committer_fallback: Entity,
    author: Entity,
    author_fallback: Entity,
}

impl Personas {
    pub fn from_config_and_env(config: &gix_config::File) -> Self {
        let parse_date = |key: &str, date: &keys::Any| -> Option<String> {
            debug_assert_eq!(
                key,
                date.logical_name(),
                "BUG: drift of expected name and actual name of the key (we hardcode it to save an allocation)"
            );
            config
                .string(key)
                .and_then(|config_date| {
                    config_date
                        .to_str()
                        .ok()
                        .and_then(|date| gix_date::parse(date, Some(gix_date::Zoned::now())).ok())
                })
                .or_else(|| Some(gix_date::Time::now_local_or_utc()))
                .map(|time| time.format_or_unix(gix_date::time::Format::Raw))
        };

        let committer_date = parse_date("gitoxide.commit.committerDate", &gitoxide::Commit::COMMITTER_DATE);
        let author_date = parse_date("gitoxide.commit.authorDate", &gitoxide::Commit::AUTHOR_DATE);

        Personas {
            user: Entity {
                name: config.string(User::NAME),
                email: config
                    .string(User::EMAIL)
                    .or_else(|| config.string(gitoxide::User::EMAIL_FALLBACK)),
                time: None,
            },
            committer: Entity {
                name: config.string(Committer::NAME),
                email: config.string(Committer::EMAIL),
                time: committer_date,
            },
            committer_fallback: Entity {
                name: config.string(gitoxide::Committer::NAME_FALLBACK),
                email: config.string(gitoxide::Committer::EMAIL_FALLBACK),
                time: None,
            },
            author: Entity {
                name: config.string(Author::NAME),
                email: config.string(Author::EMAIL),
                time: author_date,
            },
            author_fallback: Entity {
                name: config.string(gitoxide::Author::NAME_FALLBACK),
                email: config.string(gitoxide::Author::EMAIL_FALLBACK),
                time: None,
            },
        }
    }
}
