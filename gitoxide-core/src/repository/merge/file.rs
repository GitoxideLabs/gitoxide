use crate::OutputFormat;
use anyhow::{anyhow, bail, Context};
use gix::bstr::BString;
use gix::bstr::ByteSlice;
use gix::merge::blob::builtin_driver::binary;
use gix::merge::blob::builtin_driver::text::Conflict;
use gix::merge::blob::pipeline::WorktreeRoots;
use gix::merge::blob::{Resolution, ResourceKind};
use gix::object::tree::EntryKind;
use gix::Id;
use std::path::Path;

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
    let index = &repo.index_or_load_from_head()?;
    let specs = repo.pathspec(
        false,
        [base, ours, theirs],
        true,
        index,
        gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping.adjust_for_bare(repo.is_bare()),
    )?;
    // TODO: there should be a way to normalize paths without going through patterns, at least in this case maybe?
    //       `Search` actually sorts patterns by excluding or not, all that can lead to strange results.
    let mut patterns = specs.search().patterns().map(|p| p.path().to_owned());
    let base = patterns.next().unwrap();
    let ours = patterns.next().unwrap();
    let theirs = patterns.next().unwrap();

    let base_id = repo.rev_parse_single(base.as_bstr()).ok();
    let ours_id = repo.rev_parse_single(ours.as_bstr()).ok();
    let theirs_id = repo.rev_parse_single(theirs.as_bstr()).ok();
    let roots = worktree_roots(base_id, ours_id, theirs_id, repo.workdir())?;

    let mut cache = repo.merge_resource_cache(roots)?;
    let null = repo.object_hash().null();
    cache.set_resource(
        base_id.map_or(null, Id::detach),
        EntryKind::Blob,
        base.as_bstr(),
        ResourceKind::CommonAncestorOrBase,
        &repo.objects,
    )?;
    cache.set_resource(
        ours_id.map_or(null, Id::detach),
        EntryKind::Blob,
        ours.as_bstr(),
        ResourceKind::CurrentOrOurs,
        &repo.objects,
    )?;
    cache.set_resource(
        theirs_id.map_or(null, Id::detach),
        EntryKind::Blob,
        theirs.as_bstr(),
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
        ancestor: Some(base.as_bstr()),
        current: Some(ours.as_bstr()),
        other: Some(theirs.as_bstr()),
    };
    let mut buf = repo.empty_reusable_buffer();
    let (pick, resolution) = platform.merge(&mut buf, labels, &repo.command_context()?)?;
    let buf = platform
        .buffer_by_pick(pick)
        .map_err(|_| anyhow!("Participating object was too large"))?
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
