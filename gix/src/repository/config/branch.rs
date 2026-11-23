use std::collections::BTreeSet;

use crate::error::{ResultExt, bail, message, validation};
use gix_ref::{FullName, FullNameRef};
use gix_refspec::{RefSpec, RefSpecRef, parse::Operation};

use crate::{
    Result,
    bstr::{BStr, BString, ByteSlice},
    config::{
        cache::util::ApplyLeniencyDefault,
        tree::{Branch, Push, Remote},
    },
    push, remote,
};

/// Query configuration related to branches.
impl crate::Repository {
    /// Return a set of unique short branch names for which custom configuration exists in the configuration,
    /// if we deem them [trustworthy][crate::open::Options::filter_config_section()].
    ///
    /// ### Note
    ///
    /// Branch names that have illformed UTF-8 will silently be skipped.
    pub fn branch_names(&self) -> BTreeSet<&str> {
        self.subsection_str_names_of("branch")
    }

    /// Returns the validated reference name of the upstream branch on the remote associated with the given `name`,
    /// which will be used when *merging*.
    /// The returned value corresponds to the `branch.<short_branch_name>.merge` configuration key for [`remote::Direction::Fetch`].
    /// For the [push direction](`remote::Direction::Push`) the Git configuration is used for a variety of different outcomes,
    /// similar to what would happen when running `git push <name>`.
    /// Only [trusted configuration sections](crate::open::Options::filter_config_section()) are considered.
    ///
    /// Returns `None` if there is nothing configured, or if no remote or remote ref is configured.
    ///
    /// ### Note
    ///
    /// The returned name refers to what Git calls upstream branch (as opposed to upstream *tracking* branch).
    /// The value is also fast to retrieve compared to its tracking branch.
    ///
    /// See also [`Reference::remote_ref_name()`](crate::Reference::remote_ref_name()).
    ///
    /// # Example
    ///
    /// Given this Git configuration:
    ///
    /// ```gitconfig
    /// [branch "main"]
    ///     remote = origin
    ///     merge = refs/heads/topic
    /// [remote "origin"]
    ///     url = https://example.com/project.git
    ///     fetch = +refs/heads/*:refs/remotes/origin/*
    ///     push = refs/heads/main:refs/heads/publish
    /// ```
    ///
    /// Passing `refs/heads/main` as `name` returns `Some(Ok(...))` with a different name for each direction:
    ///
    /// * `Fetch` returns `refs/heads/topic`: the configured merge ref.
    /// * `Push` returns `refs/heads/publish`: the destination selected by the push refspec.
    ///
    /// Neither result is mapped to a local `refs/remotes/origin/*` tracking ref.
    #[doc(alias = "branch_upstream_name", alias = "git2")]
    pub fn branch_remote_ref_name(&self, name: &FullNameRef, direction: remote::Direction) -> Option<Result<FullName>> {
        match direction {
            remote::Direction::Fetch => self.branch_merge_ref_name(name).transpose(),
            remote::Direction::Push => {
                let remote = match self.branch_remote_or_default(name, direction)? {
                    Ok(r) => r,
                    Err(err) => return Some(Err(err)),
                };
                self.push_remote_ref_name(name, remote.push_specs.iter().map(RefSpec::to_ref))
                    .map(|out| out.map(|(remote_ref, _)| remote_ref))
                    .transpose()
            }
        }
    }

    /// Return the validated name of the reference that tracks the corresponding reference of `name` on the remote for
    /// `direction`. Note that a branch with that name might not actually exist.
    ///
    /// * For [remote::Direction::Fetch], we return the tracking branch that is on the destination
    ///   side of a `src:dest` refspec. For instance, with `name` being `main` and the default refspec
    ///   `refs/heads/*:refs/remotes/origin/*`, `refs/heads/main` would match and produce `refs/remotes/origin/main`.
    ///   Use the first configured `branch.<name>.merge` entry. If the remote is `.` and there is no mapping,
    ///   use that entry as a local reference name.
    /// * For [remote::Direction::Push], we return the tracking branch that corresponds to the remote
    ///   branch that we would push to. For instance, with `name` being `main`, `push.default=current`,
    ///   and an `origin` remote but no branch-specific configuration, we
    ///   would push to `refs/heads/main` on the remote. And that one would be fetched matching the
    ///   `refs/heads/*:refs/remotes/origin/*` fetch refspec, hence `refs/remotes/origin/main` is returned.
    ///   Note that `push` refspecs can be used to map `main` to `other` (using a push refspec `refs/heads/main:refs/heads/other`),
    ///   which would then lead to `refs/remotes/origin/other` to be returned instead.
    ///
    /// The repository's [configuration section filter](crate::open::Options::filter_config_section()) applies, which removes
    /// remotes in untrusted repositories.
    ///
    /// Note that if there is an ambiguity, that is if `name` maps to multiple tracking branches, the first matching mapping
    /// is returned, according to the order in which the fetch or push refspecs occur in the configuration file.
    ///
    /// See also [`Reference::remote_tracking_ref_name()`](crate::Reference::remote_tracking_ref_name()).
    ///
    /// # Example
    ///
    /// Given this Git configuration:
    ///
    /// ```gitconfig
    /// [branch "main"]
    ///     remote = origin
    ///     merge = refs/heads/topic
    /// [remote "origin"]
    ///     url = https://example.com/project.git
    ///     fetch = +refs/heads/*:refs/remotes/origin/*
    ///     push = refs/heads/main:refs/heads/publish
    /// ```
    ///
    /// Passing `refs/heads/main` as `name` returns `Some(Ok(...))` with a different name for each direction:
    ///
    /// * `Fetch` returns `refs/remotes/origin/topic`: the fetch refspec maps the configured merge ref.
    /// * `Push` returns `refs/remotes/origin/publish`: the push refspec selects `refs/heads/publish` on the remote,
    ///   then the fetch refspec maps it to its local tracking ref.
    #[doc(alias = "branch_upstream_name", alias = "git2")]
    pub fn branch_remote_tracking_ref_name(
        &self,
        name: &FullNameRef,
        direction: remote::Direction,
    ) -> Option<Result<FullName>> {
        match direction {
            remote::Direction::Fetch => self.upstream_tracking_ref_name(name),
            remote::Direction::Push => self.push_tracking_ref_name(name),
        }
        .transpose()
    }

    fn push_tracking_ref_name(&self, name: &FullNameRef) -> Result<Option<FullName>> {
        let Some(remote) = self
            .branch_remote_name_raw(name.shorten(), remote::Direction::Push)
            .or_else(|| self.remote_default_name(remote::Direction::Fetch))
        else {
            return Ok(None);
        };
        let config = &self.config.resolved;
        let filter = self.filter_config_section();
        let remote = remote.as_bstr();
        let push_specs = config
            .strings_filter_by("remote", remote, Remote::PUSH.name, filter)
            .unwrap_or_default();
        let Some((remote_ref, push_default)) = self
            .push_remote_ref_name(name, parse_tracking_refspecs(&push_specs, remote, Operation::Push)?)
            .or_raise(|| message("Could not get the remote reference to translate into the local tracking branch"))?
        else {
            return Ok(None);
        };
        if push_default == Some(push::Default::Upstream) {
            return self.upstream_tracking_ref_name(name);
        }
        let fetch_specs = config
            .strings_filter_by("remote", remote, Remote::FETCH.name, filter)
            .unwrap_or_default();
        let Some(tracking) = matching_remote(
            remote_ref.as_ref(),
            parse_tracking_refspecs(&fetch_specs, remote, Operation::Fetch)?,
        )
        .transpose()
        .or_raise(|| message("The name of the tracking reference was invalid"))?
        else {
            return Ok(None);
        };
        if push_default == Some(push::Default::Simple)
            && self.upstream_tracking_ref_name(name)?.as_ref() != Some(&tracking)
        {
            return Ok(None);
        }
        Ok(Some(tracking))
    }

    fn upstream_tracking_ref_name(&self, branch: &FullNameRef) -> Result<Option<FullName>> {
        let config = &self.config.resolved;
        let filter = self.filter_config_section();
        let branch = branch.shorten();
        let Some(remote) = config.string_filter_by("branch", branch, Branch::REMOTE.name, filter) else {
            return Ok(None);
        };
        let Some(merge) = config
            .strings_filter_by("branch", branch, Branch::MERGE.name, filter)
            .and_then(|values| values.into_iter().next())
        else {
            return Ok(None);
        };
        let merge = merge_ref_name(merge)?;
        let specs = config
            .strings_filter_by("remote", remote.as_bstr(), Remote::FETCH.name, filter)
            .unwrap_or_default();
        let tracking = matching_remote(
            merge.as_ref(),
            parse_tracking_refspecs(&specs, remote.as_bstr(), Operation::Fetch)?,
        )
        .transpose()
        .or_raise(|| message("The name of the tracking reference was invalid"))?;
        Ok(tracking.or_else(|| (remote == ".").then_some(merge)))
    }

    /// Given a local `tracking_branch` name, find the remote that maps to it along with the name of the branch on
    /// the side of the remote, also called upstream branch.
    ///
    /// Return `Ok(None)` if there is no remote with fetch-refspecs that would match `tracking_branch` on the right-hand side,
    /// or `Err` if the matches were ambiguous. All configured remotes are searched, and their fetch refspecs are reverse-mapped.
    /// A single valid mapping is required: mappings from multiple remotes or multiple mappings within one remote are ambiguous.
    /// Destinations match literally, and the source must be a valid full reference name; abbreviated sources
    /// aren't expanded into branch names.
    pub fn upstream_branch_and_remote_for_tracking_branch(
        &self,
        tracking_branch: &FullNameRef,
    ) -> Result<Option<(FullName, crate::Remote<'_>)>> {
        if tracking_branch.category() != Some(gix_ref::Category::RemoteBranch) {
            bail!(validation(format!(
                "The input branch '{}' needs to be a remote tracking branch",
                tracking_branch.as_bstr()
            )));
        }

        let mut candidates = Vec::new();
        let mut ambiguous_remotes = Vec::new();
        for remote_name in self.remote_names() {
            let remote = self.find_remote(remote_name)?;
            let out = remote.reverse_map_tracking_ref(tracking_branch);
            match &out.mappings[..] {
                [] => {}
                [one] => candidates.push((remote.clone(), one.lhs.clone().into_owned())),
                [..] => ambiguous_remotes.push(remote),
            }
        }

        if candidates.len() == 1 {
            let (remote, candidate) = candidates.pop().expect("just checked for one entry");
            let upstream_branch = source_ref_to_full_name(candidate)?;
            return Ok(Some((upstream_branch, remote)));
        }
        if !ambiguous_remotes.is_empty() || candidates.len() > 1 {
            let remotes = ambiguous_remotes
                .into_iter()
                .map(|r| r.name)
                .chain(candidates.into_iter().map(|(r, _)| r.name))
                .flatten()
                .map(|name| name.as_bstr().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            bail!("Found ambiguous remotes without 1:1 mapping or more than one match: {remotes}".validation());
        }
        Ok(None)
    }

    /// Returns the unvalidated name of the remote associated with the given `short_branch_name`,
    /// typically `main` instead of `refs/heads/main`.
    /// In some cases, the returned name will be an URL.
    /// Returns `None` if the remote was not found or if the name contained illformed UTF-8.
    /// Only [trusted configuration sections](crate::open::Options::filter_config_section()) are considered.
    ///
    /// * if `direction` is [remote::Direction::Fetch], we will query the `branch.<short_name>.remote` configuration.
    /// * if `direction` is [remote::Direction::Push], the push remote will be queried by means of `branch.<short_name>.pushRemote`
    ///   or `remote.pushDefault` as fallback.
    ///
    /// See also [`Reference::remote_name()`](crate::Reference::remote_name()) for a more typesafe version
    /// to be used when a `Reference` is available.
    ///
    /// `short_branch_name` can typically be obtained by [shortening a full branch name](FullNameRef::shorten()).
    #[doc(alias = "branch_upstream_remote", alias = "git2")]
    pub fn branch_remote_name<'a>(
        &self,
        short_branch_name: impl Into<&'a BStr>,
        direction: remote::Direction,
    ) -> Option<remote::Name<'_>> {
        self.branch_remote_name_raw(short_branch_name.into(), direction)
            .map(Into::into)
    }

    fn branch_remote_name_raw(&self, name: &BStr, direction: remote::Direction) -> Option<BString> {
        let config = &self.config.resolved;
        let filter = self.filter_config_section();
        (direction == remote::Direction::Push)
            .then(|| {
                config
                    .string_filter_by("branch", Some(name), Branch::PUSH_REMOTE.name, filter)
                    .or_else(|| config.string_filter(Remote::PUSH_DEFAULT, filter))
            })
            .flatten()
            .or_else(|| config.string_filter_by("branch", Some(name), Branch::REMOTE.name, filter))
    }

    /// Like [`branch_remote_name(…)`](Self::branch_remote_name()), but returns a [Remote](crate::Remote).
    /// `short_branch_name` is the name to use for looking up `branch.<short_branch_name>.*` values in the
    /// configuration.
    ///
    /// See also [`Reference::remote()`](crate::Reference::remote()).
    pub fn branch_remote<'a>(
        &self,
        short_branch_name: impl Into<&'a BStr>,
        direction: remote::Direction,
    ) -> Option<Result<crate::Remote<'_>>> {
        let name = self.branch_remote_name(short_branch_name, direction)?;
        self.try_find_remote(name.as_bstr()).or_else(|| match name {
            remote::Name::Url(url) => gix_url::parse(&url).and_then(|url| self.remote_at(url)).into(),
            remote::Name::Symbol(_) => None,
        })
    }

    fn branch_remote_or_default(
        &self,
        name: &FullNameRef,
        direction: remote::Direction,
    ) -> Option<Result<crate::Remote<'_>>> {
        if direction == remote::Direction::Push && self.branch_remote_name(name.shorten(), direction).is_none() {
            self.try_find_remote(self.remote_default_name(direction)?.as_bstr())
        } else {
            self.branch_remote(name.shorten(), direction)
        }
    }

    fn branch_merge_ref_name(&self, name: &FullNameRef) -> Result<Option<FullName>> {
        self.config
            .resolved
            .string_filter_by(
                "branch",
                name.shorten(),
                Branch::MERGE.name,
                self.filter_config_section(),
            )
            .map(merge_ref_name)
            .transpose()
    }

    fn push_remote_ref_name<'a>(
        &self,
        name: &FullNameRef,
        push_specs: impl IntoIterator<Item = RefSpecRef<'a>>,
    ) -> Result<Option<(FullName, Option<push::Default>)>> {
        let mut push_specs = push_specs.into_iter().peekable();
        if push_specs.peek().is_some() {
            return matching_remote(name, push_specs)
                .transpose()
                .map(|out| out.map(|remote_ref| (remote_ref, None)))
                .or_error();
        }
        let mode = self.push_default()?;
        let remote_ref = match mode {
            push::Default::Nothing => None,
            push::Default::Current | push::Default::Matching => Some(name.to_owned()),
            push::Default::Upstream => self.branch_merge_ref_name(name)?,
            push::Default::Simple => self.branch_merge_ref_name(name)?.filter(|merge| merge.as_ref() == name),
        };
        Ok(remote_ref.map(|remote_ref| (remote_ref, Some(mode))))
    }

    fn push_default(&self) -> Result<push::Default> {
        self.config
            .resolved
            .string_filter(Push::DEFAULT, self.filter_config_section())
            .map_or(Ok(Default::default()), |v| {
                Push::DEFAULT
                    .try_into_default(v)
                    .with_lenient_default(self.config.lenient_config)
            })
    }
}

fn parse_tracking_refspecs<'a>(specs: &'a [BString], remote: &BStr, op: Operation) -> Result<Vec<RefSpecRef<'a>>> {
    let direction = match op {
        Operation::Push => "push",
        Operation::Fetch => "fetch",
    };
    specs
        .iter()
        .map(|spec| {
            gix_refspec::parse(spec.as_bstr(), op).or_raise(|| {
                validation(format!("Could not parse {direction} refspec for remote '{remote}'"))
                    .with_input(spec.as_bstr())
            })
        })
        .collect()
}

fn merge_ref_name(name: BString) -> Result<FullName> {
    if name.starts_with(b"refs/") {
        FullName::try_from(name)
    } else {
        gix_ref::Category::LocalBranch.to_full_name(name.as_bstr())
    }
    .or_raise(|| validation("The configured name of the remote ref to merge wasn't valid"))
}

fn source_ref_to_full_name(source: gix_refspec::match_group::SourceRef<'_>) -> Result<FullName> {
    match source {
        gix_refspec::match_group::SourceRef::FullName(name) => gix_ref::FullName::try_from(name.into_owned())
            .or_raise(|| validation("The upstream branch name is invalid")),
        gix_refspec::match_group::SourceRef::ObjectId(_) => {
            unreachable!("Such a reverse mapping isn't ever produced")
        }
    }
}

fn matching_remote<'a>(
    lhs: &FullNameRef,
    specs: impl IntoIterator<Item = RefSpecRef<'a>>,
) -> Option<std::result::Result<FullName, gix_validate::reference::name::Error>> {
    let search = gix_refspec::MatchGroup {
        specs: specs.into_iter().collect(),
    };
    let out = search.match_lhs(
        Some(gix_refspec::match_group::Item {
            full_ref_name: lhs.as_bstr(),
            target: None,
            object: None,
        })
        .into_iter(),
    );
    out.mappings
        .into_iter()
        .filter(|mapping| mapping.item_index.is_some())
        .find_map(|mapping| mapping.rhs)
        .map(|name| FullName::try_from(name.into_owned()))
}
