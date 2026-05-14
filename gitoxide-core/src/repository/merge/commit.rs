use anyhow::{Context, anyhow, bail};
use gix::{
    bstr::{BString, ByteSlice},
    merge::tree::TreatAsUnresolved,
    prelude::Write,
};

use super::tree::Options;
use crate::OutputFormat;

#[allow(clippy::too_many_arguments)]
pub fn commit(
    mut repo: gix::Repository,
    out: &mut dyn std::io::Write,
    err: &mut dyn std::io::Write,
    ours: BString,
    theirs: BString,
    Options {
        format,
        file_favor,
        tree_favor,
        in_memory,
        debug,
        message: _,
        update_head: _,
    }: Options,
) -> anyhow::Result<()> {
    if format != OutputFormat::Human {
        bail!("JSON output isn't implemented yet");
    }
    repo.object_cache_size_if_unset(repo.compute_object_cache_size_for_tree_diffs(&**repo.index_or_empty()?));
    if in_memory {
        repo.objects.enable_object_memory();
    }
    let (ours_ref, ours_id) = refname_and_commit(&repo, ours)?;
    let (theirs_ref, theirs_id) = refname_and_commit(&repo, theirs)?;

    let options = repo
        .tree_merge_options()?
        .with_file_favor(file_favor)
        .with_tree_favor(tree_favor);
    let ours_id_str = ours_id.to_string();
    let theirs_id_str = theirs_id.to_string();
    let labels = gix::merge::blob::builtin_driver::text::Labels {
        ancestor: None,
        current: ours_ref
            .as_ref()
            .map_or(ours_id_str.as_str().into(), |n| n.as_bstr())
            .into(),
        other: theirs_ref
            .as_ref()
            .map_or(theirs_id_str.as_str().into(), |n| n.as_bstr())
            .into(),
    };
    let res = repo
        .merge_commits(ours_id, theirs_id, labels, options.into())?
        .tree_merge;
    let has_conflicts = res.conflicts.is_empty();
    let has_unresolved_conflicts = res.has_unresolved_conflicts(TreatAsUnresolved::default());
    {
        let _span = gix::trace::detail!("Writing merged tree");
        let mut written = 0;
        let tree_id = res
            .tree
            .detach()
            .write(|tree| {
                written += 1;
                repo.write(tree)
            })
            .map_err(|err| anyhow!("{err}"))?;
        writeln!(out, "{tree_id} (wrote {written} trees)")?;
    }

    if debug {
        writeln!(err, "{:#?}", &res.conflicts)?;
    }
    if !has_conflicts {
        writeln!(err, "{} possibly resolved conflicts", res.conflicts.len())?;
    }
    if has_unresolved_conflicts {
        bail!("Tree conflicted")
    }
    Ok(())
}

fn refname_and_commit(
    repo: &gix::Repository,
    revspec: BString,
) -> anyhow::Result<(Option<BString>, gix::hash::ObjectId)> {
    let spec = repo.rev_parse(revspec.as_bstr())?;
    let commit_id = spec
        .single()
        .context("Expected revspec to expand to a single rev only")?
        .object()?
        .peel_to_commit()?
        .id;
    let refname = spec.first_reference().map(|r| r.name.shorten().as_bstr().to_owned());
    Ok((refname, commit_id))
}
