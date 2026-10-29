use std::{io, sync::atomic::AtomicBool};

use gix_object::{Exists, FindExt, Write};

use crate::{
    Progress, Repository, Result,
    bstr::ByteSlice,
    error::{ResultExt, bail, message},
    push::{self, Options, Outcome, Rejection, Status, Update},
};

mod hooks;
mod policy;
mod quarantine;
mod shallow;
mod transaction;

pub(crate) fn push(
    remote: &crate::Remote<'_>,
    destination: &mut Repository,
    updates: impl IntoIterator<Item = Update>,
    options: Options<'_>,
    progress: impl Progress,
    mut remote_progress: impl FnMut(bool, &[u8]) -> io::Result<()>,
    should_interrupt: &AtomicBool,
) -> Result<Outcome> {
    let source = remote.repo;
    push::plan::check_interrupt(should_interrupt)?;
    if source.object_hash() != destination.object_hash() {
        bail!(message("Source and destination repositories use different object formats").unsupported());
    }
    let mut source = source.clone();
    source.objects.ignore_replacements = true;
    destination.objects.ignore_replacements = true;
    let policy = policy::Policy::new(destination)?;
    let handshake = advertisement(destination, &policy)?;
    let shallow = source
        .shallow_commits()?
        .map(|commits| commits.iter().copied().collect::<Vec<_>>())
        .unwrap_or_default();
    let mut updates = push::plan::updates(
        &source,
        updates,
        handshake.refs.as_deref().unwrap_or_default(),
        &shallow,
        should_interrupt,
    )?;
    if options.atomic
        && updates
            .iter()
            .any(|update| matches!(update.status, Status::Rejected(_)))
    {
        for update in &mut updates {
            if matches!(update.status, Status::WouldUpdate { .. }) {
                update.status = Status::Rejected(Rejection::Atomic);
            }
        }
    }
    let requested: Vec<_> = updates
        .iter()
        .enumerate()
        .filter_map(|(index, update)| matches!(update.status, Status::WouldUpdate { .. }).then_some(index))
        .collect();
    // Reuse wire-level validation for atomic/push-option capabilities and packet-safe options.
    gix_protocol::push::Request::new(
        requested
            .iter()
            .map(|&index| {
                let update = &updates[index];
                gix_protocol::push::Command {
                    name: update.update.destination.clone(),
                    old_id: update.old_id.unwrap_or_else(|| source.object_hash().null()),
                    new_id: update.update.new_id.unwrap_or_else(|| source.object_hash().null()),
                }
            })
            .collect(),
        source.object_hash(),
        &handshake.capabilities,
        gix_protocol::push::Options {
            atomic: options.atomic,
            push_options: options.push_options,
            agent: None,
            quiet: options.quiet,
        },
    )?;
    if options.dry_run || requested.is_empty() {
        return Ok(Outcome {
            handshake,
            updates,
            response: None,
        });
    }
    policy.ensure_supported(destination)?;
    destination.committer_or_set_generic_fallback()?;
    let quarantine = quarantine::Quarantine::new(destination)?;
    let tips = requested.iter().filter_map(|&index| updates[index].update.new_id);
    let mut buffer = Vec::new();
    for entry in gix_traverse::object::BreadthFirst::new(tips, &source).with_shallow_commits(shallow.iter().copied()) {
        push::plan::check_interrupt(should_interrupt)?;
        let entry = entry?;
        if !destination.objects.exists(&entry.id) {
            let object = source.find(&entry.id, &mut buffer)?;
            object.verify_checksum(&entry.id)?;
            quarantine
                .objects
                .write_buf_with_known_id(object.kind, object.data, entry.id)?;
        }
        progress.inc();
    }
    let shallow = shallow::Shallow::prepare(
        destination,
        &quarantine.objects,
        &shallow,
        &mut updates,
        policy.allow_shallow_updates,
        should_interrupt,
    )?;
    let hooks = hooks::Hooks::new(
        destination,
        policy.advertise_push_options.then_some(options.push_options),
        should_interrupt,
        &remote.command_environment,
    )?
    .with_shallow_file(shallow.alternate());
    for &index in &requested {
        if policy.hidden(updates[index].update.destination.as_bstr()) {
            let reason = if updates[index].update.new_id.is_some() {
                "deny updating a hidden ref"
            } else {
                "deny deleting a hidden ref"
            };
            updates[index].status = Status::Rejected(Rejection::Server(reason.into()));
        }
    }
    if !hooks.run(
        "pre-receive",
        [],
        hooks::input(requested.iter().map(|&index| &updates[index]), source.object_hash()),
        Some(quarantine.path()),
        &mut remote_progress,
    )? {
        for &index in &requested {
            if matches!(updates[index].status, Status::WouldUpdate { .. }) {
                updates[index].status = Status::Rejected(Rejection::Server("pre-receive hook declined".into()));
            }
        }
    }
    if updates
        .iter()
        .any(|update| matches!(update.status, Status::WouldUpdate { .. }))
    {
        quarantine.migrate(destination, should_interrupt)?;
    }
    drop(quarantine);
    for &index in &requested {
        let update = &mut updates[index];
        if !matches!(update.status, Status::WouldUpdate { .. }) {
            if options.atomic && matches!(update.status, Status::Rejected(_)) {
                break;
            }
            continue;
        }
        if let Some(reason) = policy.reject_before_transfer(destination, update, &mut remote_progress)? {
            update.status = Status::Rejected(Rejection::Server(reason));
        } else if policy.deny_non_fast_forwards
            && update.update.destination.as_bstr().starts_with_str("refs/heads/")
            && let (Some(old_id), Some(new_id)) = (update.old_id, update.update.new_id)
            && !push::plan::is_ancestor(
                destination,
                old_id,
                new_id,
                &shallow.boundaries(index),
                should_interrupt,
            )?
        {
            update.status = Status::Rejected(Rejection::Server("non-fast-forward".into()));
        } else if !hooks.run(
            "update",
            [
                gix_path::from_bstr(update.update.destination.as_bstr())?
                    .into_owned()
                    .into_os_string(),
                update
                    .old_id
                    .unwrap_or_else(|| source.object_hash().null())
                    .to_string()
                    .into(),
                update
                    .update
                    .new_id
                    .unwrap_or_else(|| source.object_hash().null())
                    .to_string()
                    .into(),
            ],
            Vec::new(),
            None,
            &mut remote_progress,
        )? {
            update.status = Status::Rejected(Rejection::Server("hook declined".into()));
            remote_progress(
                true,
                format!(
                    "error: hook declined to update {}\n",
                    update.update.destination.as_bstr()
                )
                .as_bytes(),
            )
            .or_error()?;
        } else if let Some(worktree) = policy.worktree_to_update(destination, update)? {
            let new_id = update.update.new_id.unwrap_or_else(|| source.object_hash().null());
            match hooks.push_to_checkout(&worktree, new_id, &mut remote_progress)? {
                Some(true) => {}
                Some(false) => {
                    update.status = Status::Rejected(Rejection::Server("push-to-checkout hook declined".into()));
                }
                None => {
                    update.status = Status::Rejected(Rejection::Server(
                        "updating checked-out worktrees without a push-to-checkout hook is not supported".into(),
                    ));
                }
            }
        }
        if options.atomic && matches!(update.status, Status::Rejected(_)) {
            break;
        }
    }
    if options.atomic {
        reject_atomic(&mut updates);
    }
    let selected: Vec<_> = requested
        .iter()
        .copied()
        .filter(|&index| matches!(updates[index].status, Status::WouldUpdate { .. }))
        .collect();
    if options.atomic && !selected.is_empty() {
        let edits = selected
            .iter()
            .map(|&index| edit(&updates[index], destination.object_hash()))
            .collect();
        let result = shallow
            .commit(destination, &selected, &updates, should_interrupt)
            .and_then(|()| transaction::commit(destination, edits, &hooks, &mut remote_progress));
        for index in selected {
            finish(&mut updates[index], &result, &mut remote_progress)?;
        }
    } else {
        for index in selected {
            push::plan::check_interrupt(should_interrupt)?;
            let result = shallow
                .commit(destination, &[index], &updates, should_interrupt)
                .and_then(|()| {
                    transaction::commit(
                        destination,
                        vec![edit(&updates[index], destination.object_hash())],
                        &hooks,
                        &mut remote_progress,
                    )
                });
            finish(&mut updates[index], &result, &mut remote_progress)?;
        }
    }
    let successful: Vec<_> = updates
        .iter()
        .filter(|update| {
            matches!(update.status, Status::Updated { .. })
                && (update.old_id.is_some() || update.update.new_id.is_some())
        })
        .collect();
    if !successful.is_empty() {
        hooks.run(
            "post-receive",
            [],
            hooks::input(successful.iter().copied(), source.object_hash()),
            None,
            &mut remote_progress,
        )?;
        let names = successful
            .iter()
            .map(|update| {
                gix_path::from_bstr(update.update.destination.as_bstr()).map(|path| path.into_owned().into_os_string())
            })
            .collect::<std::result::Result<Vec<_>, _>>()
            .or_error()?;
        hooks.run("post-update", names, Vec::new(), None, &mut remote_progress)?;
    }
    let response = gix_protocol::push::Response {
        unpack_error: None,
        refs: requested
            .into_iter()
            .map(|index| {
                let update = &updates[index];
                gix_protocol::push::RefStatus {
                    name: update.update.destination.clone(),
                    error: match &update.status {
                        Status::Rejected(Rejection::Server(reason)) => Some(reason.clone()),
                        _ => None,
                    },
                    report: Default::default(),
                }
            })
            .collect(),
    };
    Ok(Outcome {
        handshake,
        updates,
        response: Some(response),
    })
}

fn advertisement(destination: &Repository, policy: &policy::Policy) -> Result<gix_protocol::Handshake> {
    let mut refs = Vec::new();
    for reference in destination.references()?.all()? {
        let mut reference = reference?;
        let name = reference.name().as_bstr().to_owned();
        if policy.hidden(&name) {
            continue;
        }
        match reference.follow_to_object() {
            Ok(object_id) => refs.push(gix_protocol::handshake::Ref::Direct {
                full_ref_name: name,
                object: object_id.detach(),
            }),
            Err(err) if err.is_not_found() => {}
            Err(err) => return Err(err),
        }
    }
    let mut caps = format!(
        "\0report-status report-status-v2 delete-refs object-format={}",
        destination.object_hash()
    );
    if policy.advertise_atomic {
        caps.push_str(" atomic");
    }
    if policy.advertise_push_options {
        caps.push_str(" push-options");
    }
    Ok(gix_protocol::Handshake {
        server_protocol_version: gix_transport::Protocol::V1,
        refs: Some(refs),
        capabilities: gix_transport::client::Capabilities::from_bytes(caps.as_bytes())?.0,
        v1_shallow_updates: destination
            .shallow_commits()?
            .map(|ids| ids.iter().copied().map(gix_shallow::Update::Shallow).collect()),
    })
}

fn reject_atomic(updates: &mut [push::UpdateOutcome]) {
    if updates
        .iter()
        .any(|update| matches!(update.status, Status::Rejected(_)))
    {
        for update in updates {
            if matches!(update.status, Status::WouldUpdate { .. }) {
                update.status = Status::Rejected(Rejection::Server("atomic push failure".into()));
            }
        }
    }
}

fn edit(update: &push::UpdateOutcome, object_hash: gix_hash::Kind) -> gix_ref::transaction::RefEdit {
    use gix_ref::transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog};
    let expected = update.old_id.map_or(PreviousValue::MustNotExist, |object_id| {
        PreviousValue::MustExistAndMatch(object_id.into())
    });
    RefEdit {
        name: update.update.destination.clone(),
        change: match update.update.new_id {
            Some(new_id) => Change::Update {
                expected,
                new: new_id.into(),
                log: LogChange {
                    message: "push".into(),
                    ..Default::default()
                },
            },
            None => Change::Delete {
                expected: update.old_id.map_or_else(
                    || PreviousValue::ExistingMustMatch(object_hash.null().into()),
                    |object_id| PreviousValue::MustExistAndMatch(object_id.into()),
                ),
                log: RefLog::AndReference,
            },
        },
        deref: true,
    }
}

fn finish(
    update: &mut push::UpdateOutcome,
    result: &Result<Vec<gix_ref::transaction::RefEdit>>,
    remote_progress: &mut impl FnMut(bool, &[u8]) -> io::Result<()>,
) -> Result<()> {
    match result {
        Ok(_) => {
            let Status::WouldUpdate { forced } = update.status else {
                unreachable!("only pending updates are committed")
            };
            update.status = Status::Updated { forced };
        }
        Err(err) => {
            remote_progress(
                true,
                format!("error: {}: {err}\n", update.update.destination.as_bstr()).as_bytes(),
            )
            .or_error()?;
            update.status = Status::Rejected(Rejection::Server("failed to update ref".into()));
        }
    }
    Ok(())
}
