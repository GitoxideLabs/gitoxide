use std::io;

use gix_ref::{
    TargetRef,
    transaction::{Change, RefEdit, RefLog},
};

use crate::{
    Repository, Result,
    error::{bail, message},
};

pub(super) fn commit(
    repo: &Repository,
    edits: Vec<RefEdit>,
    hooks: &super::hooks::Hooks<'_>,
    progress: &mut impl FnMut(bool, &[u8]) -> io::Result<()>,
) -> Result<Vec<RefEdit>> {
    let (loose, packed) = repo.config.lock_timeout()?;
    let queued_input = input(edits.iter(), repo.object_hash(), repo.namespace());
    if !hooks.run(
        "reference-transaction",
        ["preparing".into()],
        queued_input.clone(),
        None,
        progress,
    )? {
        hooks.run(
            "reference-transaction",
            ["aborted".into()],
            queued_input,
            None,
            progress,
        )?;
        bail!(message("Reference transaction hook declined"));
    }
    let transaction = repo.refs.transaction().prepare(edits, loose, packed)?;
    let input = input(transaction.edits(), repo.object_hash(), repo.namespace());
    if !hooks.run(
        "reference-transaction",
        ["prepared".into()],
        input.clone(),
        None,
        progress,
    )? {
        transaction.rollback();
        hooks.run("reference-transaction", ["aborted".into()], input, None, progress)?;
        bail!(message("Reference transaction hook declined"));
    }
    let result = transaction.commit(repo.committer().transpose()?);
    hooks.run(
        "reference-transaction",
        [if result.is_ok() { "committed" } else { "aborted" }.into()],
        input,
        None,
        progress,
    )?;
    result
}

fn input<'a>(
    edits: impl Iterator<Item = &'a RefEdit>,
    hash: gix_hash::Kind,
    namespace: Option<&gix_ref::Namespace>,
) -> Vec<u8> {
    let mut out = Vec::new();
    for edit in edits {
        if matches!(
            edit.change,
            Change::Update {
                log: gix_ref::transaction::LogChange { mode: RefLog::Only, .. },
                ..
            } | Change::Delete { log: RefLog::Only, .. }
        ) {
            continue;
        }
        for target in [edit.change.previous_value(), edit.change.new_value()] {
            match target {
                Some(TargetRef::Object(object_id)) => out.extend_from_slice(object_id.to_string().as_bytes()),
                Some(TargetRef::Symbolic(name)) => {
                    out.extend_from_slice(b"ref:");
                    out.extend_from_slice(name.as_bstr());
                }
                None => out.extend_from_slice(hash.null().to_string().as_bytes()),
            }
            out.push(b' ');
        }
        if let Some(namespace) = namespace {
            out.extend_from_slice(namespace.as_bstr());
        }
        out.extend_from_slice(edit.name.as_bstr());
        out.push(b'\n');
    }
    out
}
