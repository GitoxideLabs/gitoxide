use std::path::Path;

use anyhow::{Context, anyhow, bail};
use gix::{
    Id,
    bstr::BString,
    merge::blob::{
        Resolution, ResourceKind,
        builtin_driver::{binary, text::Conflict},
        pipeline::WorktreeRoots,
    },
    object::tree::EntryKind,
};

use crate::OutputFormat;

pub fn file(
    repo: gix::Repository,
    out: &mut dyn std::io::Write,
    format: OutputFormat,
    conflict: Option<gix::merge::blob::builtin_driver::text::Conflict>,
    base: BString,
    ours: BString,
    theirs: BString,
) -> anyhow::Result<()> {
    if format != OutputFormat::Human {
        bail!("JSON output isn't implemented yet");
    }
    let base = repo.normalize_path(&base)?;
    let ours = repo.normalize_path(&ours)?;
    let theirs = repo.normalize_path(&theirs)?;

    let base_id = repo.rev_parse_single(base.as_ref()).ok();
    let ours_id = repo.rev_parse_single(ours.as_ref()).ok();
    let theirs_id = repo.rev_parse_single(theirs.as_ref()).ok();
    let roots = worktree_roots(base_id, ours_id, theirs_id, repo.workdir())?;

    let mut cache = repo.merge_resource_cache(roots)?;
    let null = repo.object_hash().null();
    cache.set_resource(
        base_id.map_or(null, Id::detach),
        EntryKind::Blob,
        base.as_ref(),
        ResourceKind::CommonAncestorOrBase,
        &repo.objects,
    )?;
    cache.set_resource(
        ours_id.map_or(null, Id::detach),
        EntryKind::Blob,
        ours.as_ref(),
        ResourceKind::CurrentOrOurs,
        &repo.objects,
    )?;
    cache.set_resource(
        theirs_id.map_or(null, Id::detach),
        EntryKind::Blob,
        theirs.as_ref(),
        ResourceKind::OtherOrTheirs,
        &repo.objects,
    )?;

    let mut options = repo.blob_merge_options()?;
    if let Some(conflict) = conflict {
        options.text.conflict = conflict;
        options.resolve_binary_with = match conflict {
            Conflict::Keep { .. } => None,
            Conflict::ResolveWithOurs => Some(binary::ResolveWith::Ours),
            Conflict::ResolveWithTheirs => Some(binary::ResolveWith::Theirs),
            Conflict::ResolveWithUnion => None,
        };
    }
    let platform = cache.prepare_merge(&repo.objects, options)?;
    let labels = gix::merge::blob::builtin_driver::text::Labels {
        ancestor: Some(base.as_ref()),
        current: Some(ours.as_ref()),
        other: Some(theirs.as_ref()),
    };
    let mut buf = repo.empty_reusable_buffer();
    let (pick, resolution) = platform.merge(&mut buf, labels, &repo.command_context()?)?;
    let buf = platform
        .buffer_by_pick(pick)
        .map_err(|()| anyhow!("Participating object was too large"))?
        .unwrap_or(&buf);
    out.write_all(buf)?;

    if resolution == Resolution::Conflict {
        bail!("File conflicted")
    }
    Ok(())
}

fn worktree_roots(
    base: Option<gix::Id<'_>>,
    ours: Option<gix::Id<'_>>,
    theirs: Option<gix::Id<'_>>,
    workdir: Option<&Path>,
) -> anyhow::Result<gix::merge::blob::pipeline::WorktreeRoots> {
    let roots = if base.is_none() || ours.is_none() || theirs.is_none() {
        let workdir = workdir.context("A workdir is required if one of the bases are provided as path.")?;
        gix::merge::blob::pipeline::WorktreeRoots {
            current_root: ours.is_none().then(|| workdir.to_owned()),
            other_root: theirs.is_none().then(|| workdir.to_owned()),
            common_ancestor_root: base.is_none().then(|| workdir.to_owned()),
        }
    } else {
        WorktreeRoots::default()
    };
    Ok(roots)
}
