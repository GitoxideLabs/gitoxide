use std::{fs, io, path::Path};

use gix_ref::{
    Category, FullName, FullNameRef, Target,
    file::ReferenceExt,
    transaction::{PreviousValue, RefEdit},
};

use crate::{
    Result,
    bstr::{BString, ByteSlice},
    error::{ErrorExt, Message, ResultExt, bail, message, validation},
};

impl crate::Repository {
    /// Rename `source` to `destination`, preserving its target and reflog history.
    /// Both names must be fully qualified references under `refs/`. Local branches can only be renamed to
    /// local branches; other reference categories use the reference store's rename semantics.
    /// `force` permits replacing an existing destination, except one used by a non-bare worktree.
    /// Same-name renames are allowed without `force`; unresolvable symbolic destinations count as missing.
    ///
    /// Matching symbolic `HEAD`s in the main and linked worktrees are retargeted, with Git-compatible reflog
    /// entries. Local branch renames also rename every matching section in the common `config`, preserving
    /// destination sections, included files, and `config.worktree`. The affected local sections in this
    /// repository's configuration snapshot are updated without discarding in-memory overrides.
    /// A checked-out unborn branch can be renamed without creating a reference or a reflog.
    /// Branches being rebased or bisected in a detached worktree cannot be renamed.
    ///
    /// Return the performed reference edits, followed by worktree `HEAD` edits using qualified names such as
    /// `main-worktree/HEAD` or `worktrees/<id>/HEAD`. An unborn rename has no source/destination reference edits.
    ///
    /// This operation is **not atomic**. Configuration is locked before changing refs; after the storage rename,
    /// all matching `HEAD` updates are attempted. A HEAD failure leaves successful updates intact and skips
    /// configuration publication. Configuration commit failures leave the references and HEADs renamed.
    /// Errors identify the failed phase and both names. Storage failures retain the recovery information from
    /// [`gix_ref::file::Store::rename()`]. Concurrent checkout or worktree registration must be serialized by callers.
    pub fn rename_reference<Source, Destination, SourceError, DestinationError>(
        &mut self,
        source: Source,
        destination: Destination,
        force: bool,
    ) -> Result<Vec<RefEdit>>
    where
        Source: TryInto<FullName, Error = SourceError>,
        Destination: TryInto<FullName, Error = DestinationError>,
        gix_validate::reference::name::Error: From<SourceError> + From<DestinationError>,
    {
        let source = source
            .try_into()
            .map_err(gix_validate::reference::name::Error::from)
            .or_raise(|| validation("The source reference name is invalid"))?;
        let destination = destination
            .try_into()
            .map_err(gix_validate::reference::name::Error::from)
            .or_raise(|| validation("The destination reference name is invalid"))?;
        self.rename_reference_inner(source.as_ref(), destination.as_ref(), force)
            .or_raise(|| {
                Message::new("Could not rename reference completely")
                    .with("source", source.as_bstr())
                    .with("destination", destination.as_bstr())
            })
    }

    fn rename_reference_inner(
        &mut self,
        source: &FullNameRef,
        destination: &FullNameRef,
        force: bool,
    ) -> Result<Vec<RefEdit>> {
        for name in [source, destination] {
            if !name.as_bstr().starts_with(b"refs/") {
                bail!(
                    validation("Reference renaming requires a fully qualified name under refs/")
                        .with("reference", name.as_bstr())
                );
            }
        }
        let branch = source.category() == Some(Category::LocalBranch);
        if branch != (destination.category() == Some(Category::LocalBranch)) {
            bail!(validation("Local branches can only be renamed to local branches"));
        }
        if branch {
            gix_validate::reference::branch_name(destination.as_bstr()).or_error()?;
            if destination.shorten().starts_with(b"-") {
                bail!(validation("A branch name cannot start with a dash"));
            }
        }
        let original = self.refs.try_find(source)?;
        if original
            .as_ref()
            .is_some_and(|reference| matches!(reference.target, Target::Symbolic(_)))
        {
            bail!(validation("Cannot rename a symbolic source reference"));
        }

        let main = self.main_repo()?;
        let mut worktrees = vec![(
            FullName::try_from("main-worktree/HEAD").or_error()?,
            main.git_dir().to_owned(),
            main.is_bare(),
        )];
        for worktree in self.worktrees()? {
            let mut name = BString::from("worktrees/");
            name.extend_from_slice(worktree.id()?);
            name.extend_from_slice(b"/HEAD");
            worktrees.push((
                FullName::try_from(name).or_error()?,
                worktree.git_dir().to_owned(),
                false,
            ));
        }
        let destination_target = Target::Symbolic(destination.to_owned());
        let (destination_previous, destination_exists) = destination_state(&self.refs, destination)?;
        if source != destination && destination_exists && !force {
            bail!(validation("The destination reference already exists").with("reference", destination.as_bstr()));
        }
        let mut heads = Vec::new();
        for (name, git_dir, bare) in &worktrees {
            let reference = self.refs.try_find(name.as_ref())?;
            let head = reference.as_ref().map(|reference| reference.target.clone());
            let referent = reference
                .map(|reference| symbolic_referent(&self.refs, reference))
                .transpose()?
                .flatten();
            if branch && matches!(head, Some(Target::Object(_))) && operation_uses_branch(git_dir, source, false)? {
                bail!(validation("The source branch is being rebased or bisected").with("git_dir", git_dir.as_path()));
            }
            if source != destination
                && destination_exists
                && !bare
                && (referent.as_ref().map(AsRef::as_ref) == Some(destination)
                    || (branch && operation_uses_branch(git_dir, destination, true)?))
            {
                bail!(
                    validation("The destination reference is in use by a worktree").with("git_dir", git_dir.as_path())
                );
            }
            if referent.as_ref().map(AsRef::as_ref) == Some(source) {
                heads.push((name.clone(), head.expect("a resolved HEAD has an original target")));
            }
        }
        let unborn = original.is_none() && branch && !heads.is_empty();
        if original.is_none() && !unborn {
            bail!(message("The source reference does not exist").not_found());
        }

        let config_path = self.config_path(gix_config::Source::Local)?;
        let config = if branch && source != destination {
            let mut config = self.config_file_mut(&config_path)?;
            match config.rename_section(
                "branch",
                Some(source.shorten()),
                "branch",
                Some(destination.shorten().to_owned()),
            ) {
                Ok(()) => Some(config),
                Err(err) if err.is_not_found() => None,
                Err(err) => return Err(err),
            }
        } else {
            None
        };
        let committer = self.committer().transpose()?;
        let (ref_lock, packed_lock) = self.config.lock_timeout()?;
        let mut message = BString::from(if branch {
            "Branch: renamed "
        } else {
            "Reference: renamed "
        });
        message.extend_from_slice(source.as_bstr());
        message.extend_from_slice(b" to ");
        message.extend_from_slice(destination.as_bstr());
        let destination_constraint = if force || source == destination {
            PreviousValue::Any
        } else {
            destination_previous.map_or(PreviousValue::MustNotExist, PreviousValue::MustExistAndMatch)
        };
        let mut edits = if unborn {
            Vec::new()
        } else {
            self.refs.rename(
                source,
                destination,
                destination_constraint,
                committer,
                message.as_bstr(),
                ref_lock,
                packed_lock,
            )?
        };
        let mut failures = Vec::new();
        for (head, previous) in heads {
            let edit = RefEdit::update(
                head,
                destination_target.clone(),
                PreviousValue::MustExistAndMatch(previous),
                message.clone(),
            );
            match self
                .refs
                .transaction()
                .prepare([edit], ref_lock, packed_lock)
                .and_then(|transaction| transaction.commit(committer))
            {
                Ok(performed) => edits.extend(performed),
                Err(err) => failures.push(err.into_exn()),
            }
        }
        if !failures.is_empty() {
            return Err(
                Message::new("Reference renamed, but some worktree HEADs could not be updated")
                    .with("phase", "HEAD updates")
                    .raise_all(failures)
                    .into_error(),
            );
        }
        if let Some(config) = config {
            config.commit().or_raise(|| {
                Message::new("Reference and HEADs renamed, but configuration could not be committed")
                    .with("phase", "configuration")
            })?;
            let current_dir = self.current_dir().to_owned();
            let config = gix_parallel::OwnShared::make_mut(&mut self.config.resolved);
            match config.rename_section_filter(
                "branch",
                Some(source.shorten()),
                "branch",
                Some(destination.shorten().to_owned()),
                |meta| {
                    meta.source == gix_config::Source::Local
                        && meta.level == 0
                        && meta
                            .path
                            .as_ref()
                            .is_some_and(|path| current_dir.join(path) == config_path)
                },
            ) {
                Ok(()) => {}
                Err(err) if err.is_not_found() => {}
                Err(err) => return Err(err),
            }
        }
        Ok(edits)
    }
}

fn symbolic_referent(store: &gix_ref::file::Store, mut reference: gix_ref::Reference) -> Result<Option<FullName>> {
    if matches!(reference.target, Target::Object(_)) {
        return Ok(None);
    }
    let packed = store.cached_packed_buffer()?;
    match reference.follow_to_object_packed(store, packed.as_deref().map(|buffer| &**buffer)) {
        Ok(_) => Ok(Some(reference.name)),
        Err(err) if err.is_not_found() => Ok(reference.target.try_name().map(ToOwned::to_owned)),
        Err(err) => Err(err),
    }
}

fn destination_state(store: &gix_ref::file::Store, name: &FullNameRef) -> Result<(Option<Target>, bool)> {
    let mut reference = match store.try_find(name) {
        Ok(Some(reference)) => reference,
        Ok(None) => return Ok((None, false)),
        Err(err) if err.downcast_any_ref::<gix_ref::file::find::ReferenceDecode>().is_some() => {
            return Ok((None, false));
        }
        Err(err) => return Err(err),
    };
    let previous = Some(reference.target.clone());
    let packed = store.cached_packed_buffer()?;
    // Git treats unresolvable symbolic destinations as absent; replacement still checks their raw targets.
    let exists = reference
        .follow_to_object_packed(store, packed.as_deref().map(|buffer| &**buffer))
        .is_ok();
    Ok((previous, exists))
}

fn operation_uses_branch(git_dir: &Path, name: &FullNameRef, include_update_refs: bool) -> Result<bool> {
    let rebase = if git_dir.join("rebase-apply").is_dir() {
        (!git_dir.join("rebase-apply/applying").is_file()).then_some("rebase-apply/head-name")
    } else {
        git_dir
            .join("rebase-merge")
            .is_dir()
            .then_some("rebase-merge/head-name")
    };
    let bisect = git_dir.join("BISECT_LOG").is_file().then_some("BISECT_START");
    for path in rebase.into_iter().chain(bisect) {
        if let Some(contents) = read_optional(&git_dir.join(path))? {
            let contents = contents.trim_end_with(|c| c == '\n' || c == '\r');
            if contents == b"detached HEAD" || gix_hash::ObjectId::from_hex(contents).is_ok() {
                continue;
            }
            if contents == name.as_bstr() || contents == name.shorten() {
                return Ok(true);
            }
        }
    }
    if include_update_refs && let Some(contents) = read_optional(&git_dir.join("rebase-merge/update-refs"))? {
        // Git stores one ref name followed by its before and after object IDs.
        let mut lines = contents.lines();
        let mut found = false;
        while let Some(reference) = lines.next() {
            for _ in 0..2 {
                let Some(object_id) = lines.next() else {
                    return Ok(false);
                };
                if gix_hash::ObjectId::from_hex(object_id).is_err() {
                    return Ok(false);
                }
            }
            found |= reference == name.as_bstr();
        }
        return Ok(found);
    }
    Ok(false)
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.and_raise(message!("Could not read worktree operation state at {path:?}"))),
    }
}
