#![allow(clippy::result_large_err)]
use std::{borrow::Cow, ffi::OsStr};

use gix_features::threading::OwnShared;
use gix_macros::momo;

use crate::{
    bstr::{BStr, BString, ByteSlice},
    config::{CommitAutoRollback, Snapshot, SnapshotMut},
};

/// Access configuration values, frozen in time, using a `key` which is a `.` separated string of up to
/// three tokens, namely `section_name.[subsection_name.]value_name`, like `core.bare` or `remote.origin.url`.
///
/// Note that single-value methods always return the last value found, which is the one set most recently in the
/// hierarchy of configuration files, aka 'last one wins'.
impl<'repo> Snapshot<'repo> {
    /// Return the boolean at `key`, or `None` if there is no such value or if the value can't be interpreted as
    /// boolean.
    ///
    /// For a non-degenerating version, use [`try_boolean(…)`][Self::try_boolean()].
    ///
    /// Note that this method takes the most recent value at `key` even if it is from a file with reduced trust.
    pub fn boolean(&self, key: impl gix_config::AsKey) -> Option<bool> {
        self.try_boolean(key).and_then(Result::ok)
    }

    /// Like [`boolean()`][Self::boolean()], but it will report an error if the value couldn't be interpreted as boolean.
    #[momo]
    pub fn try_boolean(&self, key: impl gix_config::AsKey) -> Option<Result<bool, gix_config::value::Error>> {
        self.repo.config.resolved.boolean(key)
    }

    /// Return the resolved integer at `key`, or `None` if there is no such value or if the value can't be interpreted as
    /// integer or exceeded the value range.
    ///
    /// For a non-degenerating version, use [`try_integer(…)`][Self::try_integer()].
    ///
    /// Note that this method takes the most recent value at `key` even if it is from a file with reduced trust.
    pub fn integer(&self, key: impl gix_config::AsKey) -> Option<i64> {
        self.try_integer(key).and_then(Result::ok)
    }

    /// Like [`integer()`][Self::integer()], but it will report an error if the value couldn't be interpreted as boolean.
    #[momo]
    pub fn try_integer(&self, key: impl gix_config::AsKey) -> Option<Result<i64, gix_config::value::Error>> {
        self.repo.config.resolved.integer(key)
    }

    /// Return the string at `key`, or `None` if there is no such value.
    ///
    /// Note that this method takes the most recent value at `key` even if it is from a file with reduced trust.
    #[momo]
    pub fn string(&self, key: impl gix_config::AsKey) -> Option<Cow<'repo, BStr>> {
        self.repo.config.resolved.string(key)
    }

    /// Return the trusted and fully interpolated path at `key`, or `None` if there is no such value
    /// or if no value was found in a trusted file.
    /// An error occurs if the path could not be interpolated to its final value.
    #[momo]
    pub fn trusted_path(
        &self,
        key: impl gix_config::AsKey,
    ) -> Option<Result<Cow<'repo, std::path::Path>, gix_config::path::interpolate::Error>> {
        self.repo.config.trusted_file_path(key)
    }

    /// Return the trusted string at `key` for launching using [command::prepare()](gix_command::prepare()),
    /// or `None` if there is no such value or if no value was found in a trusted file.
    #[momo]
    pub fn trusted_program(&self, key: impl gix_config::AsKey) -> Option<Cow<'repo, OsStr>> {
        let value = self
            .repo
            .config
            .resolved
            .string_filter(key, &mut self.repo.config.filter_config_section.clone())?;
        Some(match gix_path::from_bstr(value) {
            Cow::Borrowed(v) => Cow::Borrowed(v.as_os_str()),
            Cow::Owned(v) => Cow::Owned(v.into_os_string()),
        })
    }
}

/// Utilities and additional access
impl Snapshot<'_> {
    /// Returns the underlying configuration implementation for a complete API, despite being a little less convenient.
    ///
    /// It's expected that more functionality will move up depending on demand.
    pub fn plumbing(&self) -> &gix_config::File<'static> {
        &self.repo.config.resolved
    }
}

/// Utilities
impl<'repo> SnapshotMut<'repo> {
    /// Append configuration values of the form `core.abbrev=5` or `remote.origin.url = foo` or `core.bool-implicit-true`
    /// to the end of the repository configuration, with each section marked with the given `source`.
    ///
    /// Note that doing so applies the configuration at the very end, so it will always override what came before it
    /// even though the `source` is of lower priority as what's there.
    pub fn append_config(
        &mut self,
        values: impl IntoIterator<Item = impl AsRef<BStr>>,
        source: gix_config::Source,
    ) -> Result<&mut Self, crate::config::overrides::Error> {
        crate::config::overrides::append(&mut self.config, values, source, |v| Some(format!("-c {v}").into()))?;
        Ok(self)
    }
    /// Apply all changes made to this instance.
    ///
    /// Note that this would also happen once this instance is dropped, but using this method may be more intuitive and won't squelch errors
    /// in case the new configuration is partially invalid.
    pub fn commit(mut self) -> Result<&'repo mut crate::Repository, crate::config::Error> {
        let repo = self.repo.take().expect("always present here");
        self.commit_inner(repo)
    }

    /// Set the value at `key` to `new_value`, possibly creating the section if it doesn't exist yet, or overriding the most recent existing
    /// value, which will be returned.
    #[momo]
    pub fn set_value<'b>(
        &mut self,
        key: &'static dyn crate::config::tree::Key,
        new_value: impl Into<&'b BStr>,
    ) -> Result<Option<BString>, crate::config::set_value::Error> {
        if let Some(crate::config::tree::SubSectionRequirement::Parameter(_)) = key.subsection_requirement() {
            return Err(crate::config::set_value::Error::SubSectionRequired);
        }
        let value = new_value.into();
        key.validate(value)?;
        let section = key.section();
        let current = match section.parent() {
            Some(parent) => {
                self.config
                    .set_raw_value_by(parent.name(), Some(section.name().into()), key.name(), value)?
            }
            None => self.config.set_raw_value_by(section.name(), None, key.name(), value)?,
        };
        Ok(current.map(std::borrow::Cow::into_owned))
    }

    /// Set the value at `key` to `new_value` in the given `subsection`, possibly creating the section and sub-section if it doesn't exist yet,
    /// or overriding the most recent existing value, which will be returned.
    #[momo]
    pub fn set_subsection_value<'a, 'b>(
        &mut self,
        key: &'static dyn crate::config::tree::Key,
        subsection: impl Into<&'a BStr>,
        new_value: impl Into<&'b BStr>,
    ) -> Result<Option<BString>, crate::config::set_value::Error> {
        if let Some(crate::config::tree::SubSectionRequirement::Never) = key.subsection_requirement() {
            return Err(crate::config::set_value::Error::SubSectionForbidden);
        }
        let value = new_value.into();
        key.validate(value)?;

        let name = key
            .full_name(Some(subsection.into()))
            .expect("we know it needs a subsection");
        let key = gix_config::KeyRef::parse_unvalidated((**name).as_bstr())
            .expect("statically known keys can always be parsed");
        let current =
            self.config
                .set_raw_value_by(key.section_name, key.subsection_name, key.value_name.to_owned(), value)?;
        Ok(current.map(std::borrow::Cow::into_owned))
    }

    pub(crate) fn commit_inner(
        &mut self,
        repo: &'repo mut crate::Repository,
    ) -> Result<&'repo mut crate::Repository, crate::config::Error> {
        repo.reread_values_and_clear_caches_replacing_config(std::mem::take(&mut self.config).into())?;
        Ok(repo)
    }

    /// Create a structure the temporarily commits the changes, but rolls them back when dropped.
    pub fn commit_auto_rollback(mut self) -> Result<CommitAutoRollback<'repo>, crate::config::Error> {
        let repo = self.repo.take().expect("this only runs once on consumption");
        let prev_config = OwnShared::clone(&repo.config.resolved);

        Ok(CommitAutoRollback {
            repo: self.commit_inner(repo)?.into(),
            prev_config,
        })
    }

    /// Don't apply any of the changes after consuming this instance, effectively forgetting them, returning the changed configuration.
    pub fn forget(mut self) -> gix_config::File<'static> {
        self.repo.take();
        std::mem::take(&mut self.config)
    }
}

/// Utilities
impl<'repo> CommitAutoRollback<'repo> {
    /// Rollback the changes previously applied and all values before the change.
    pub fn rollback(mut self) -> Result<&'repo mut crate::Repository, crate::config::Error> {
        let repo = self.repo.take().expect("still present, consumed only once");
        self.rollback_inner(repo)
    }

    pub(crate) fn rollback_inner(
        &mut self,
        repo: &'repo mut crate::Repository,
    ) -> Result<&'repo mut crate::Repository, crate::config::Error> {
        repo.reread_values_and_clear_caches_replacing_config(OwnShared::clone(&self.prev_config))?;
        Ok(repo)
    }
}
