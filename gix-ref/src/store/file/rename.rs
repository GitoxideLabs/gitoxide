use std::{
    borrow::Borrow,
    fs, io,
    path::{Path, PathBuf},
};

use gix_error::{ErrorExt, Message, OptionExt, Result, ResultExt, bail, message};
use gix_hash::ObjectId;
use gix_lock::acquire::Fail;
use gix_object::bstr::{BStr, BString, ByteSlice};

use crate::{
    FullNameRef, Target,
    file::{Store, Transaction},
    store::WriteReflog,
    transaction::{Change, PreviousValue, RefEdit},
};

impl Store {
    /// Rename a direct `source` reference to `destination`, replacing an existing destination without
    /// following symbolic refs. Both directions of directory/file conflicts and same-name renames work.
    /// `destination_constraint` checks the observed destination; concurrent changes are rejected under its lock.
    /// The source's reflog history replaces the destination's, followed by a `source target -> source target`
    /// entry with whitespace-normalized `log_message`. Without source history, an orphan destination log is retained.
    /// `WriteReflog::Disable` preserves moved history but suppresses new entries.
    ///
    /// Return the source deletion and destination update, in that order, with the targets observed before
    /// renaming as their previous values. Same-name renames return two edits for the same name.
    /// Branch configuration and symbolic `HEAD` targets are not changed. The current worktree's `HEAD`
    /// reflog records deletion if it points directly to the source or an overwritten destination.
    /// A conflicting `HEAD` lock fails immediately to avoid waiting in the opposite order to HEAD transactions.
    ///
    /// This operation is **not atomic**: the source is deleted before the destination is published.
    /// On failure, restoring the source and its history is attempted without overwriting concurrent changes.
    /// An overwritten destination and entries already appended to `HEAD`'s log are not restored.
    /// Recovery failures retain both errors, the `source_target`, and a `recovery_log` path when history remains.
    /// Crashes can leave the source unreferenced and its reflog at that temporary path.
    #[expect(clippy::too_many_arguments)]
    pub fn rename(
        &self,
        source: &FullNameRef,
        destination: &FullNameRef,
        destination_constraint: PreviousValue,
        committer: Option<gix_actor::SignatureRef<'_>>,
        log_message: &BStr,
        ref_files_lock_fail_mode: Fail,
        packed_refs_lock_fail_mode: Fail,
    ) -> Result<Vec<RefEdit>> {
        self.rename_inner(
            source,
            destination,
            destination_constraint,
            committer,
            log_message,
            ref_files_lock_fail_mode,
            packed_refs_lock_fail_mode,
        )
        .or_raise(|| {
            Message::new("Could not rename reference")
                .with("source", source.as_bstr())
                .with("destination", destination.as_bstr())
        })
    }

    #[expect(clippy::too_many_arguments)]
    fn rename_inner(
        &self,
        source: &FullNameRef,
        destination: &FullNameRef,
        destination_constraint: PreviousValue,
        committer: Option<gix_actor::SignatureRef<'_>>,
        log_message: &BStr,
        ref_lock: Fail,
        packed_lock: Fail,
    ) -> Result<Vec<RefEdit>> {
        self.check_windows_device_name(source).or_error()?;
        self.check_windows_device_name(destination).or_error()?;
        let log_message = normalize_message(log_message)?;
        let original = self.find(source)?;
        let Target::Object(source_target) = original.target else {
            bail!(gix_error::validation("Cannot rename a symbolic reference"));
        };
        let (destination_previous, malformed_destination) = match self.try_find(destination) {
            Ok(reference) => (reference.map(|reference| reference.target), None),
            Err(err) if err.downcast_any_ref::<super::find::ReferenceDecode>().is_some() => {
                let contents = self
                    .ref_contents(destination)
                    .or_error()?
                    .ok_or_raise(|| message("Destination changed while reading it"))?;
                if super::loose::Reference::try_from_path(destination.to_owned(), &contents, self.object_hash).is_ok() {
                    bail!(gix_error::validation("Destination changed while reading it"));
                }
                (None, Some(contents))
            }
            Err(err) => return Err(err),
        };
        match (&destination_constraint, &destination_previous) {
            (PreviousValue::Any, _)
            | (PreviousValue::MustExist, Some(_))
            | (PreviousValue::MustNotExist | PreviousValue::ExistingMustMatch(_), None) => {}
            (PreviousValue::MustNotExist, Some(actual)) => {
                return Err(super::transaction::prepare::MustNotExist {
                    full_name: destination.as_bstr().to_owned(),
                    actual: actual.clone(),
                }
                .and_raise(message("The rename destination must not exist")));
            }
            (PreviousValue::MustExist | PreviousValue::MustExistAndMatch(_), None) => {
                bail!(
                    gix_error::not_found("The rename destination must exist").with("reference", destination.as_bstr())
                );
            }
            (PreviousValue::MustExistAndMatch(expected) | PreviousValue::ExistingMustMatch(expected), Some(actual)) => {
                if actual != expected {
                    return Err(super::transaction::prepare::ReferenceOutOfDate {
                        full_name: destination.as_bstr().to_owned(),
                        actual: actual.clone(),
                    }
                    .and_raise(message!("Expected rename destination content {expected}")));
                }
            }
        }
        let destination_must_exist = matches!(
            destination_constraint,
            PreviousValue::MustExist | PreviousValue::MustExistAndMatch(_)
        );
        let source_path = self.reference_path(source)?;
        let destination_path = self.reference_path(destination)?;
        let packed = self.assure_packed_refs_uptodate()?;
        self.check_namespace_conflicts(
            Some(&source_path),
            &[(destination_path.clone(), destination.as_bstr().to_owned())],
            packed.as_deref().map(|p| &**p),
        )?;
        drop(packed);

        // Ref transactions provide CAS checks, locks, packed deletion, and cleanup; rename owns the log semantics.
        let mut silent = self.clone();
        silent.write_reflog = WriteReflog::Disable;
        let deletion = RefEdit::delete(
            source.to_owned(),
            PreviousValue::MustExistAndMatch(source_target.into()),
        );
        let source_transaction = self.transaction().reflog_message(log_message.clone()).prepare(
            [deletion.clone()],
            ref_lock,
            packed_lock,
        )?;
        let source_log = self.reflog_path(source)?;
        let log_exists = match fs::symlink_metadata(&source_log) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    bail!(
                        gix_error::validation("Cannot rename a reference with a symlink reflog")
                            .with("path", source_log)
                    );
                }
                if metadata.is_dir()
                    && self.write_reflog != WriteReflog::Disable
                    && fs::read_dir(&source_log)
                        .or_error()?
                        .next()
                        .transpose()
                        .or_error()?
                        .is_some()
                {
                    bail!(gix_error::validation("Source reflog directory is not empty").with("path", source_log));
                }
                true
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => false,
            Err(err) => return Err(err.and_raise(message("Could not inspect source reflog"))),
        };
        let (_, destination_relative) = self.reflog_base_and_relative_path(destination)?;
        if committer.is_none()
            && self.write_reflog != WriteReflog::Disable
            && (log_exists
                || self.reflog_exists(destination)?
                || self.write_reflog == WriteReflog::Always
                || self.should_autocreate_reflog(&destination_relative))
        {
            bail!(super::log::create_or_update::MissingCommitter);
        }

        let mut recovery_log = if log_exists {
            Some(move_log_aside(self, source, &source_log)?)
        } else {
            None
        };
        let mut source_deleted = false;
        let result = (|| -> Result<Vec<RefEdit>> {
            source_transaction.commit(committer)?;
            source_deleted = true;
            if source_path != destination_path
                && let Some(previous) = &destination_previous
                && (destination_must_exist || self.try_find(destination)?.is_some())
            {
                let transaction = self.transaction().prepare(
                    [RefEdit::delete(destination.to_owned(), PreviousValue::Any)],
                    ref_lock,
                    packed_lock,
                )?;
                // Compare raw targets under lock, allowing replacement of symrefs with broken referents.
                let edit: &RefEdit = transaction.updates.as_ref().expect("prepared")[0].borrow();
                if edit.change.previous_value() != Some(previous.to_ref()) {
                    bail!(gix_error::validation("Destination changed concurrently"));
                }
                transaction.commit(committer)?;
            }
            let update = RefEdit::update(
                destination.to_owned(),
                source_target,
                PreviousValue::MustNotExist,
                log_message.clone(),
            );
            let mut transaction = silent.transaction().prepare_for_rename(update, ref_lock, packed_lock)?;
            require_absent(&transaction)?;
            if let Some(contents) = &malformed_destination {
                // A malformed loose destination shadows any packed value, and Git preserves its orphan log.
                if self.ref_contents(destination).or_error()?.as_ref() != Some(contents) {
                    bail!(gix_error::validation("Destination changed concurrently"));
                }
            }
            self.check_namespace_conflicts(
                None,
                &[(destination_path.clone(), destination.as_bstr().to_owned())],
                transaction
                    .packed_transaction
                    .as_ref()
                    .and_then(crate::packed::Transaction::buffer),
            )?;
            if let Some(backup) = &recovery_log {
                copy_log(self, destination, backup, ref_lock)?;
            }
            let publish = self
                .reflog_create_or_append(
                    destination,
                    Some(source_target),
                    &source_target,
                    committer,
                    log_message.as_bstr(),
                    false,
                )
                .and_then(|()| {
                    // This direct update changes no packed records; retain its locks until log recovery is done.
                    transaction.updates.as_mut().expect("prepared")[0].commit_ref()
                });
            if let Err(primary) = publish {
                if recovery_log.is_some()
                    && let Err(cleanup) = remove_installed_log(&self.reflog_path(destination)?)
                {
                    // The destination ref is still locked and absent: only our installed history is removed.
                    return Err(message("Could not remove renamed reflog during recovery")
                        .raise_all([primary.into_exn(), cleanup.raise().into_exn()])
                        .into_error());
                }
                return Err(primary);
            }
            Ok(vec![
                deletion,
                RefEdit::update(
                    destination.to_owned(),
                    source_target,
                    destination_previous.map_or(PreviousValue::MustNotExist, PreviousValue::MustExistAndMatch),
                    log_message,
                ),
            ])
        })();
        match result {
            Ok(edits) => {
                if let Some(backup) = recovery_log.take() {
                    remove_log_backup(&backup).or_raise(|| {
                        Message::new("Reference renamed but reflog backup could not be removed")
                            .with("recovery_log", backup)
                    })?;
                }
                Ok(edits)
            }
            Err(primary) => {
                let recovered = restore_source(
                    &silent,
                    source,
                    source_target,
                    &mut recovery_log,
                    source_deleted,
                    ref_lock,
                    packed_lock,
                );
                let mut context =
                    Message::new("Reference rename failed").with("source_target", source_target.to_string());
                if let Some(backup) = recovery_log {
                    context = context.with("recovery_log", backup);
                }
                Err(match recovered {
                    Ok(()) => primary.and_raise(context),
                    Err(recovery) => context
                        .raise_all([primary.into_exn(), recovery.into_exn()])
                        .into_error(),
                })
            }
        }
    }

    pub(in crate::store_impl::file) fn check_namespace_conflicts(
        &self,
        source: Option<&Path>,
        destinations: &[(PathBuf, BString)],
        packed: Option<&crate::packed::Buffer>,
    ) -> Result {
        for (destination_path, destination_name) in destinations {
            let destination = <&FullNameRef>::try_from(destination_name.as_bstr()).or_error()?;
            if matches!(
                destination.category(),
                Some(
                    crate::Category::PseudoRef
                        | crate::Category::MainPseudoRef
                        | crate::Category::LinkedPseudoRef { .. }
                )
            ) {
                continue;
            }
            let conflict = |name: &BStr| {
                gix_error::validation("The reference name conflicts with another reference")
                    .with("reference", name)
                    .and_raise(
                        Message::new("Could not prepare reference edit").with("reference", destination_name.as_bstr()),
                    )
            };
            for (end, _) in destination_name.iter().enumerate().filter(|(_, byte)| **byte == b'/') {
                let Ok(ancestor) = <&FullNameRef>::try_from(destination_name[..end].as_bstr()) else {
                    continue;
                };
                let path = self.reference_path(ancestor)?;
                if Some(path.as_path()) == source || !destination_path.starts_with(&path) {
                    continue;
                }
                match self.find_existing_inner(ancestor, packed) {
                    Ok(_) => return Err(conflict(ancestor.as_bstr())),
                    Err(err) if err.downcast_any_ref::<super::find::ReferenceDecode>().is_some() => {
                        return Err(conflict(ancestor.as_bstr()));
                    }
                    Err(err) if err.is_not_found() => {}
                    Err(err) => return Err(err),
                }
            }

            // Only visit descendant names; their contents are irrelevant to namespace conflicts.
            let (base, _) = self.reference_path_with_base(destination)?;
            for entry in super::loose::iter::SortedLoosePaths::at(
                destination_path,
                base.into_owned(),
                None,
                None,
                self.precompose_unicode,
            ) {
                let (path, mut name) = entry.or_error()?;
                if Some(path.as_path()) != source {
                    if let Some(namespace) = &self.namespace {
                        name.strip_namespace(namespace);
                    }
                    return Err(conflict(name.as_bstr()));
                }
            }
            if let Some((packed, name)) = packed.zip(crate::packed::find::transform_full_name_for_lookup(destination)) {
                let mut prefix = self
                    .namespace
                    .as_ref()
                    .map_or_else(BString::default, |ns| ns.as_bstr().to_owned());
                if self.precompose_unicode {
                    prefix.extend_from_slice(&gix_path::into_bstr(gix_utils::str::precompose_path(
                        gix_path::from_bstr(name.as_bstr())?,
                    ))?);
                } else {
                    prefix.extend_from_slice(name.as_bstr());
                }
                prefix.push(b'/');
                let prefix_len = prefix.len();
                for reference in packed.iter_prefixed(prefix)? {
                    let mut name = reference?.name.to_owned();
                    let path = destination_path.join(gix_path::from_bstr(name.as_bstr()[prefix_len..].as_bstr())?);
                    if Some(path.as_path()) != source {
                        if let Some(namespace) = &self.namespace {
                            name.strip_namespace(namespace);
                        }
                        return Err(conflict(name.as_bstr()));
                    }
                }
            }
        }
        Ok(())
    }
}

fn require_absent(transaction: &Transaction<'_, '_>) -> Result {
    let edit: &RefEdit = transaction.updates.as_ref().expect("prepared")[0].borrow();
    if !matches!(
        edit.change,
        Change::Update {
            expected: PreviousValue::MustNotExist,
            ..
        }
    ) {
        bail!(gix_error::validation("Reference was created concurrently").with("reference", edit.name.as_bstr()));
    }
    Ok(())
}

fn normalize_message(input: &BStr) -> Result<BString> {
    if input.contains(&0) {
        bail!(gix_error::validation("Reflog messages cannot contain NUL"));
    }
    let mut result = BString::default();
    for word in input.split(u8::is_ascii_whitespace).filter(|word| !word.is_empty()) {
        if !result.is_empty() {
            result.push(b' ');
        }
        result.extend_from_slice(word);
    }
    Ok(result)
}

fn move_log_aside(store: &Store, name: &FullNameRef, source: &Path) -> Result<PathBuf> {
    let (mut root, relative) = store.reflog_base_and_relative_path(name)?;
    if relative.starts_with("refs") {
        root.push("refs"); // logs/refs may live on a different filesystem than logs/.
    }
    let temporary = gix_tempfile::new(
        root,
        gix_tempfile::ContainingDirectory::Exists,
        gix_tempfile::AutoRemove::Tempfile,
    )
    .or_raise(|| message("Could not create reflog recovery file"))?
    .take()
    .ok_or_raise(|| message("Reflog recovery file disappeared"))?;
    let (file, path) = temporary
        .keep()
        .or_raise(|| message("Could not retain reflog recovery file"))?;
    drop(file);
    if source.is_dir() {
        fs::remove_file(&path).or_error()?;
    }
    if let Err(err) = fs::rename(source, &path) {
        if !source.is_dir()
            && let Err(cleanup) = fs::remove_file(&path)
        {
            return Err(Message::new("Could not move source reflog or remove unused backup")
                .with("recovery_log", path)
                .raise_all([err.raise().into_exn(), cleanup.raise().into_exn()])
                .into_error());
        }
        return Err(err.and_raise(message("Could not move source reflog aside")));
    }
    Ok(path)
}

fn copy_log(store: &Store, destination: &FullNameRef, backup: &Path, lock_fail: Fail) -> Result {
    let path = store.reflog_path(destination)?;
    if backup.is_dir() {
        fs::create_dir_all(path.parent().expect("reflog has a parent")).or_error()?;
        if path.is_dir() {
            gix_tempfile::remove_dir::empty_depth_first(path.clone()).or_error()?;
        }
        copy_log_directory(backup, &path).or_raise(|| message("Could not install source reflog directory"))?;
        return Ok(());
    }
    let (root, _) = store.reflog_base_and_relative_path(destination)?;
    let mut lock = gix_lock::File::acquire_to_update_resource(&path, lock_fail, Some(root), 0)?;
    let mut input = fs::File::open(backup).or_raise(|| message("Could not read reflog backup"))?;
    io::copy(&mut input, &mut lock).or_raise(|| message("Could not copy source reflog history"))?;
    lock.with_mut(|file| file.set_permissions(input.metadata()?.permissions()))
        .or_error()?;
    if path.is_dir() {
        gix_tempfile::remove_dir::empty_depth_first(path)
            .or_raise(|| message("Destination reflog directory is not empty"))?;
    }
    lock.commit().or_raise(|| message("Could not install renamed reflog"))?;
    Ok(())
}

fn remove_log_backup(path: &Path) -> io::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

fn remove_installed_log(path: &Path) -> io::Result<()> {
    if path.is_dir() {
        fs::remove_dir(path)
    } else {
        fs::remove_file(path)
    }
}

fn copy_log_directory(source: &Path, destination: &Path) -> Result {
    fs::create_dir(destination).or_error()?;
    for entry in fs::read_dir(source).or_error()? {
        let entry = entry.or_error()?;
        let path = destination.join(entry.file_name());
        let kind = entry.file_type().or_error()?;
        if kind.is_dir() {
            copy_log_directory(&entry.path(), &path)?;
        } else if kind.is_symlink() {
            gix_fs::symlink::create(&fs::read_link(entry.path()).or_error()?, &path).or_error()?;
        } else {
            copy_log_file(&entry.path(), &path)?;
        }
    }
    Ok(())
}

fn copy_log_file(source: &Path, destination: &Path) -> Result {
    let mut input = fs::File::open(source).or_error()?;
    let mut temporary = gix_tempfile::new(
        destination.parent().expect("reflog has a parent"),
        gix_tempfile::ContainingDirectory::Exists,
        gix_tempfile::AutoRemove::Tempfile,
    )
    .or_error()?
    .take()
    .ok_or_raise(|| message("Reflog recovery file disappeared"))?;
    io::copy(&mut input, &mut temporary).or_error()?;
    temporary
        .as_file()
        .set_permissions(input.metadata().or_error()?.permissions())
        .or_error()?;
    temporary.persist_noclobber(destination).or_error()?;
    Ok(())
}

fn restore_source(
    store: &Store,
    source: &FullNameRef,
    target: ObjectId,
    recovery_log: &mut Option<PathBuf>,
    source_deleted: bool,
    ref_lock: Fail,
    packed_lock: Fail,
) -> Result {
    let restore_ref = source_deleted || store.try_find(source)?.is_none();
    let edit = if restore_ref {
        RefEdit::update(source.to_owned(), target, PreviousValue::MustNotExist, "")
    } else {
        RefEdit::delete(source.to_owned(), PreviousValue::MustExistAndMatch(target.into()))
    };
    let mut transaction = store.transaction().prepare_for_rename(edit, ref_lock, packed_lock)?;
    if restore_ref {
        require_absent(&transaction)?;
        let source_path = store.reference_path(source)?;
        match fs::symlink_metadata(&source_path) {
            Ok(metadata) if !metadata.is_dir() => {
                bail!(gix_error::validation("Reference was created concurrently").with("reference", source.as_bstr()));
            }
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.and_raise(message("Could not inspect source during recovery"))),
        }
        store.check_namespace_conflicts(
            None,
            &[(source_path, source.as_bstr().to_owned())],
            transaction
                .packed_transaction
                .as_ref()
                .and_then(crate::packed::Transaction::buffer),
        )?;
    }
    let log_result = (|| -> Result {
        if let Some(backup) = recovery_log.as_ref() {
            let source_log = store.reflog_path(source)?;
            fs::create_dir_all(source_log.parent().expect("reflog has a parent")).or_error()?;
            if source_log.is_dir() {
                gix_tempfile::remove_dir::empty_depth_first(source_log.clone()).or_error()?;
            }
            if backup.is_dir() {
                copy_log_directory(backup, &source_log)?;
            } else {
                copy_log_file(backup, &source_log)?;
            }
        }
        Ok(())
    })()
    .or_raise(|| message("Could not restore source reflog without overwriting concurrent changes"));
    let ref_result = if restore_ref {
        transaction.updates.as_mut().expect("prepared")[0].commit_ref()
    } else {
        Ok(())
    };
    drop(transaction);
    match (log_result, ref_result) {
        (Ok(()), Ok(())) => {}
        (Err(err), Ok(())) | (Ok(()), Err(err)) => return Err(err),
        (Err(log), Err(reference)) => {
            return Err(message("Could not restore source reference or reflog")
                .raise_all([log.into_exn(), reference.into_exn()])
                .into_error());
        }
    }
    if let Some(backup) = recovery_log.take() {
        remove_log_backup(&backup)
            .or_raise(|| Message::new("Could not remove restored reflog backup").with("recovery_log", backup))?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use gix_error::TestResult;
    use gix_testtools::tempfile::TempDir;

    const SOURCE: &str = "refs/heads/topic";
    const HISTORY: &[u8] = b"original reflog history\n";
    const CONCURRENT_HISTORY: &[u8] = b"history written by another operation\n";

    fn recovery_store() -> TestResult<(TempDir, Store, PathBuf)> {
        let directory = TempDir::new()?;
        fs::create_dir_all(directory.path().join("refs/heads"))?;
        fs::create_dir_all(directory.path().join("logs/refs/heads"))?;
        let mut store = Store::at(directory.path().to_owned(), gix_testtools::object_hash());
        store.write_reflog = WriteReflog::Disable;
        let backup = directory.path().join("logs/refs/recovery");
        fs::write(&backup, HISTORY)?;
        Ok((directory, store, backup))
    }

    #[test]
    fn recovery_does_not_replace_a_concurrently_recreated_source() -> TestResult {
        let tree_id = ObjectId::empty_tree(gix_testtools::object_hash());
        for concurrent_object_id in [tree_id, ObjectId::empty_blob(gix_testtools::object_hash())] {
            let (_keep, store, backup) = recovery_store()?;
            let source: &FullNameRef = SOURCE.try_into()?;
            let source_path = store.reference_path(source)?;
            let source_log = store.reflog_path(source)?;
            let concurrent_ref = format!("{concurrent_object_id}\n");
            fs::write(&source_path, &concurrent_ref)?;
            fs::write(&source_log, CONCURRENT_HISTORY)?;
            let mut recovery_log = Some(backup.clone());

            restore_source(
                &store,
                source,
                tree_id,
                &mut recovery_log,
                true,
                Fail::Immediately,
                Fail::Immediately,
            )
            .expect_err("a recreated source belongs to another operation, even with the same target");

            assert_eq!(
                fs::read(source_path)?,
                concurrent_ref.as_bytes(),
                "preserve the concurrent target"
            );
            assert_eq!(
                fs::read(source_log)?,
                CONCURRENT_HISTORY,
                "preserve the concurrent history"
            );
            assert_eq!(
                recovery_log.as_deref(),
                Some(backup.as_path()),
                "retain the recovery location"
            );
            assert_eq!(
                fs::read(backup)?,
                HISTORY,
                "keep the original history available for recovery"
            );
        }
        Ok(())
    }

    #[test]
    fn recovery_does_not_replace_a_malformed_concurrent_source() -> TestResult {
        let (_keep, store, backup) = recovery_store()?;
        let source: &FullNameRef = SOURCE.try_into()?;
        let path = store.reference_path(source)?;
        fs::write(&path, b"concurrent malformed reference\n")?;
        let mut recovery_log = Some(backup.clone());
        restore_source(
            &store,
            source,
            ObjectId::empty_tree(gix_testtools::object_hash()),
            &mut recovery_log,
            true,
            Fail::Immediately,
            Fail::Immediately,
        )
        .expect_err("unparseable contents still belong to another writer");
        assert_eq!(
            fs::read(path)?,
            b"concurrent malformed reference\n",
            "preserve the concurrent file"
        );
        assert_eq!(fs::read(backup)?, HISTORY, "retain the recovery history");
        Ok(())
    }

    #[test]
    fn recovery_restores_the_reference_without_overwriting_a_concurrent_reflog() -> TestResult {
        let (_keep, store, backup) = recovery_store()?;
        let source: &FullNameRef = SOURCE.try_into()?;
        let source_log = store.reflog_path(source)?;
        let tree_id = ObjectId::empty_tree(gix_testtools::object_hash());
        fs::write(&source_log, CONCURRENT_HISTORY)?;
        let mut recovery_log = Some(backup.clone());

        restore_source(
            &store,
            source,
            tree_id,
            &mut recovery_log,
            true,
            Fail::Immediately,
            Fail::Immediately,
        )
        .expect_err("another operation's reflog prevents complete recovery");

        assert_eq!(
            store.find(source)?.target,
            Target::Object(tree_id),
            "recover the reference despite the log failure"
        );
        assert_eq!(
            fs::read(source_log)?,
            CONCURRENT_HISTORY,
            "never overwrite the concurrent reflog"
        );
        assert_eq!(
            recovery_log.as_deref(),
            Some(backup.as_path()),
            "retain the recovery location"
        );
        assert_eq!(fs::read(backup)?, HISTORY, "preserve the original history separately");
        Ok(())
    }

    #[test]
    fn recovery_restores_history_when_source_deletion_did_not_happen() -> TestResult {
        let (_keep, store, backup) = recovery_store()?;
        let source: &FullNameRef = SOURCE.try_into()?;
        let source_path = store.reference_path(source)?;
        let tree_id = ObjectId::empty_tree(gix_testtools::object_hash());
        let original_ref = format!("{tree_id}\n");
        fs::write(&source_path, &original_ref)?;
        let mut recovery_log = Some(backup.clone());

        restore_source(
            &store,
            source,
            tree_id,
            &mut recovery_log,
            false,
            Fail::Immediately,
            Fail::Immediately,
        )?;

        assert_eq!(
            fs::read(source_path)?,
            original_ref.as_bytes(),
            "keep the original reference untouched"
        );
        assert_eq!(
            fs::read(store.reflog_path(source)?)?,
            HISTORY,
            "restore the history moved aside before deletion"
        );
        assert!(recovery_log.is_none(), "complete recovery consumes its backup");
        assert!(!backup.exists(), "remove the redundant backup after restoring history");
        Ok(())
    }

    #[test]
    fn recovery_reports_both_reference_and_reflog_failures() -> TestResult {
        let (_keep, store, backup) = recovery_store()?;
        let source: &FullNameRef = SOURCE.try_into()?;
        let source_path = store.reference_path(source)?;
        let source_log = store.reflog_path(source)?;
        let tree_id = ObjectId::empty_tree(gix_testtools::object_hash());
        fs::create_dir(&source_path)?;
        let child = source_path.join(".keep");
        fs::write(&child, b"concurrent child reference\n")?;
        fs::write(&source_log, CONCURRENT_HISTORY)?;
        let mut recovery_log = Some(backup.clone());

        let error = restore_source(
            &store,
            source,
            tree_id,
            &mut recovery_log,
            true,
            Fail::Immediately,
            Fail::Immediately,
        )
        .expect_err("the concurrent child and reflog independently block recovery");

        assert_eq!(
            error.into_exn().frame().children().len(),
            2,
            "retain both recovery failures as separate causes"
        );
        assert_eq!(
            fs::read(child)?,
            b"concurrent child reference\n",
            "preserve the blocking child"
        );
        assert_eq!(
            fs::read(source_log)?,
            CONCURRENT_HISTORY,
            "preserve the blocking reflog"
        );
        assert_eq!(
            recovery_log.as_deref(),
            Some(backup.as_path()),
            "retain the recovery location"
        );
        assert_eq!(
            fs::read(backup)?,
            HISTORY,
            "keep the source history available for manual recovery"
        );
        Ok(())
    }
}
