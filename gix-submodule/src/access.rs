use gix_error::Result;
use std::{collections::HashSet, path::Path};

use bstr::{BStr, BString, ByteSlice};
use gix_error::{ErrorExt, OptionExt, ResultExt};

use crate::{
    File, IsActivePlatform,
    config::{Branch, FetchRecurse, Ignore, Update},
};

/// High-Level Access
///
/// Note that all methods perform validation of the requested value and report issues right away.
/// If a bypass is needed, use [`config()`](File::config()) for direct access.
impl File {
    /// Return the underlying configuration file.
    ///
    /// Note that it might have been merged with values from another configuration file and may
    /// thus not be accurately reflecting that state of a `.gitmodules` file anymore.
    pub fn config(&self) -> &gix_config::File {
        &self.config
    }

    /// Return the path at which the `.gitmodules` file lives, if it is known.
    pub fn config_path(&self) -> Option<&Path> {
        self.config.sections().filter_map(|s| s.meta().path.as_deref()).next()
    }

    /// Return the unvalidated names of the submodules for which configuration is present.
    ///
    /// Note that these exact names have to be used for querying submodule values.
    pub fn names(&self) -> impl Iterator<Item = &BStr> {
        let mut seen = HashSet::<&BStr>::default();
        self.config
            .sections_by_name("submodule")
            .into_iter()
            .flatten()
            .filter_map(move |s| {
                s.header()
                    .subsection_name()
                    .filter(|_| s.meta().source == crate::init::META_MARKER)
                    .filter(|name| seen.insert(*name))
            })
    }

    /// Similar to [Self::is_active_platform()], but automatically applies it to each name to learn if a submodule is active or not.
    pub fn names_and_active_state<'a>(
        &'a self,
        config: &'a gix_config::File,
        defaults: gix_pathspec::Defaults,
        attributes: &'a mut (
                    dyn FnMut(
            &BStr,
            gix_pathspec::attributes::glob::pattern::Case,
            bool,
            &mut gix_pathspec::attributes::search::Outcome,
        ) -> bool
                        + 'a
                ),
    ) -> Result<impl Iterator<Item = (&'a BStr, Result<bool>)> + 'a> {
        let mut platform = self.is_active_platform(config, defaults)?;
        let iter = self
            .names()
            .map(move |name| (name, platform.is_active(config, name, attributes)));
        Ok(iter)
    }

    /// Return a platform which allows to check if a submodule name is active or inactive.
    /// Use `defaults` for parsing the pathspecs used to later match on names via `submodule.active` configuration retrieved from `config`.
    ///
    /// All `submodule.active` pathspecs are considered to be top-level specs and match the name of submodules, which are active
    /// on inclusive match.
    /// The full algorithm is described as [hierarchy of rules](https://git-scm.com/docs/gitsubmodules#_active_submodules).
    pub fn is_active_platform(
        &self,
        config: &gix_config::File,
        defaults: gix_pathspec::Defaults,
    ) -> Result<IsActivePlatform> {
        let search = config
            .strings("submodule.active")
            .map(|patterns| -> Result<_> {
                let patterns = patterns
                    .into_iter()
                    .map(|pattern| gix_pathspec::parse(&pattern, defaults))
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                gix_pathspec::Search::from_specs(patterns, None, std::path::Path::new(""))
            })
            .transpose()?;
        Ok(IsActivePlatform { search })
    }

    /// Given the `relative_path` (as seen from the root of the worktree) of a submodule with possibly platform-specific
    /// component separators, find the submodule's name associated with this path, or `None` if none was found.
    ///
    /// Note that this does a linear search and compares `relative_path` in a normalized form to the same form of the path
    /// associated with the submodule.
    pub fn name_by_path(&self, relative_path: &BStr) -> Option<&BStr> {
        self.names()
            .filter_map(|n| self.path(n).ok().map(|p| (n, p)))
            .find_map(|(n, p)| (p == relative_path).then_some(n))
    }
}

/// Per-Submodule Access
impl File {
    /// Return the path relative to the root directory of the working tree at which the submodule is expected to be checked out.
    /// It's an error if the path doesn't exist as it's the only way to associate a path in the index with additional submodule
    /// information, like the URL to fetch from.
    /// Invalid path bytes are stored as `input` in [`gix_error::Message::values`].
    /// After [wrapping](gix_error::Error::from_error()), inspect them with [metadata](gix_error::Error::metadata()).
    ///
    /// ### Deviation
    ///
    /// Git currently allows absolute paths to be used when adding submodules, but fails later as it can't find the submodule by
    /// relative path anymore. Let's play it safe here.
    pub fn path(&self, name: &BStr) -> Result<BString> {
        let path_bstr = self.config.string(&format!("submodule.{name}.path")).ok_or_raise(|| {
            gix_error::validation(format!(
                "The submodule '{name}' was missing its 'path' field or it was empty"
            ))
        })?;
        if path_bstr.is_empty() {
            return Err(gix_error::validation(format!(
                "The submodule '{name}' was missing its 'path' field or it was empty"
            ))
            .raise()
            .into());
        }
        let path = gix_path::from_bstr(path_bstr.as_bstr());
        if path.is_absolute() {
            return Err(
                gix_error::validation(format!("The path of submodule '{name}' needs to be relative"))
                    .with("input", path_bstr)
                    .raise()
                    .into(),
            );
        }
        if gix_path::normalize(path, "".as_ref()).is_none() {
            return Err(
                gix_error::validation("The path would lead outside of the repository worktree")
                    .with("input", path_bstr)
                    .raise()
                    .into(),
            );
        }
        Ok(path_bstr)
    }

    /// Retrieve the `url` field of the submodule named `name`. It's an error if it doesn't exist or is empty.
    /// Parse failures include the URL bytes as `input` [metadata](gix_error::Error::metadata()).
    pub fn url(&self, name: &BStr) -> Result<gix_url::Url> {
        let url = self.config.string(&format!("submodule.{name}.url")).ok_or_else(|| {
            gix_error::validation(format!(
                "The submodule '{name}' was missing its 'url' field or it was empty"
            ))
            .raise()
        })?;

        if url.is_empty() {
            return Err(gix_error::validation(format!(
                "The submodule '{name}' was missing its 'url' field or it was empty"
            ))
            .raise()
            .into());
        }
        (gix_url::Url::from_bytes(url.as_ref()).or_raise(|| {
            gix_error::validation(format!("The url of submodule '{name}' could not be parsed")).with("input", url)
        }))
        .map_err(Into::into)
    }

    /// Retrieve the `update` field of the submodule named `name`, if present.
    /// Invalid value or command bytes are stored as `input` in [`gix_error::Message::values`].
    /// After [wrapping](gix_error::Error::from_error()), inspect them with [metadata](gix_error::Error::metadata()).
    pub fn update(&self, name: &BStr) -> Result<Option<Update>> {
        let mut value_is_from_modules_file = None;
        let our_meta = self.config.meta();
        let value: Update = match self.config.string_filter(&format!("submodule.{name}.update"), |meta| {
            value_is_from_modules_file = Some(std::ptr::eq(meta, our_meta));
            true
        }) {
            Some(v) => v.as_bstr().try_into().map_err(|()| {
                gix_error::validation(format!("The 'update' field of submodule '{name}' was invalid"))
                    .with("input", v)
                    .raise()
            })?,
            None => return Ok(None),
        };

        if let Update::Command(cmd) = &value
            && value_is_from_modules_file.unwrap_or_default()
        {
            return Err(gix_error::validation(format!(
                "The 'update' field of submodule '{name}' tried to set a command to be shared"
            ))
            .with("input", cmd.to_owned())
            .raise()
            .into());
        }
        Ok(Some(value))
    }

    /// Retrieve the `branch` field of the submodule named `name`, or `None` if unset.
    ///
    /// Note that `Default` is implemented for [`Branch`].
    /// Parse failures include the branch bytes as `input` [metadata](gix_error::Error::metadata()).
    pub fn branch(&self, name: &BStr) -> Result<Option<Branch>> {
        let branch = match self.config.string(&format!("submodule.{name}.branch")) {
            Some(v) => v,
            None => return Ok(None),
        };

        (Branch::try_from(branch.as_ref()).map(Some).or_raise(|| {
            gix_error::validation(format!(
                "The 'branch' field of submodule '{name}' couldn't be turned into a valid fetch refspec"
            ))
            .with("input", branch)
        }))
        .map_err(Into::into)
    }

    /// Retrieve the `fetchRecurseSubmodules` field of the submodule named `name`, or `None` if unset.
    ///
    /// Note that if it's unset, it should be retrieved from `fetch.recurseSubmodules` in the configuration.
    /// Invalid value bytes are stored as `input` in [`gix_error::Message::values`].
    /// After [wrapping](gix_error::Error::from_error()), inspect them with [metadata](gix_error::Error::metadata()).
    pub fn fetch_recurse(&self, name: &BStr) -> Result<Option<FetchRecurse>> {
        Ok(
            FetchRecurse::new(self.config.boolean(&format!("submodule.{name}.fetchRecurseSubmodules"))).map_err(
                |value| {
                    gix_error::validation(format!(
                        "The 'fetchRecurseSubmodules' field of submodule '{name}' was invalid"
                    ))
                    .with("input", value)
                    .raise()
                },
            )?,
        )
    }

    /// Retrieve the `ignore` field of the submodule named `name`, or `None` if unset.
    /// Invalid value bytes are stored as `input` in [`gix_error::Message::values`].
    /// After [wrapping](gix_error::Error::from_error()), inspect them with [metadata](gix_error::Error::metadata()).
    pub fn ignore(&self, name: &BStr) -> Result<Option<Ignore>> {
        Ok(self
            .config
            .string(&format!("submodule.{name}.ignore"))
            .map(|value| {
                Ignore::try_from(value.as_ref()).map_err(|()| {
                    gix_error::validation(format!("The 'ignore' field of submodule '{name}' was invalid"))
                        .with("input", value)
                        .raise()
                })
            })
            .transpose()?)
    }

    /// Retrieve the `shallow` field of the submodule named `name`, or `None` if unset.
    ///
    /// If `true`, the submodule will be checked out with `depth = 1`. If unset, `false` is assumed.
    pub fn shallow(&self, name: &BStr) -> Result<Option<bool>> {
        self.config.boolean(&format!("submodule.{name}.shallow"))
    }
}
