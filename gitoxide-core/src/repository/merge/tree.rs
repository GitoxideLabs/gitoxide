use crate::OutputFormat;

pub struct Options {
    pub format: OutputFormat,
    pub file_favor: Option<gix::merge::tree::FileFavor>,
    pub tree_favor: Option<gix::merge::tree::TreeFavor>,
    pub in_memory: bool,
    pub debug: bool,
    pub message: Option<String>,
    pub update_head: bool,
}

pub(super) mod function {

    use std::collections::BTreeSet;

    use anyhow::{Context, anyhow, bail};
    use gix::{
        bstr::{BString, ByteSlice},
        merge::tree::TreatAsUnresolved,
        prelude::Write,
    };

    use super::Options;
    use crate::OutputFormat;

    #[allow(clippy::too_many_arguments)]
    pub fn tree(
        mut repo: gix::Repository,
        out: &mut dyn std::io::Write,
        err: &mut dyn std::io::Write,
        base: BString,
        ours: BString,
        theirs: BString,
        Options {
            format,
            file_favor,
            tree_favor,
            in_memory,
            debug,
            message,
            update_head,
        }: Options,
    ) -> anyhow::Result<()> {
        if format != OutputFormat::Human {
            bail!("JSON output isn't implemented yet");
        }
        if update_head && in_memory {
            bail!("`--update-head` cannot be used with `--in-memory` - cannot set head to nothing");
        }
        if update_head && message.is_none() {
            bail!("`--update-head` requires `--message`");
        }
        repo.object_cache_size_if_unset(repo.compute_object_cache_size_for_tree_diffs(&**repo.index_or_empty()?));
        if in_memory || message.is_some() {
            repo.objects.enable_object_memory();
        }
        let (base_ref, base_id) = refname_and_tree(&repo, base)?;
        let (ours_ref, ours_id) = refname_and_tree(&repo, ours)?;
        let (theirs_ref, theirs_id) = refname_and_tree(&repo, theirs)?;

        let options = repo
            .tree_merge_options()?
            .with_file_favor(file_favor)
            .with_tree_favor(tree_favor);
        let base_id_str = base_id.to_string();
        let ours_id_str = ours_id.to_string();
        let theirs_id_str = theirs_id.to_string();
        let labels = gix::merge::blob::builtin_driver::text::Labels {
            ancestor: base_ref
                .as_ref()
                .map_or(base_id_str.as_str().into(), |n| n.as_bstr())
                .into(),
            current: ours_ref
                .as_ref()
                .map_or(ours_id_str.as_str().into(), |n| n.as_bstr())
                .into(),
            other: theirs_ref
                .as_ref()
                .map_or(theirs_id_str.as_str().into(), |n| n.as_bstr())
                .into(),
        };
        let res = repo.merge_trees(base_id, ours_id, theirs_id, labels, options)?;
        let has_conflicts = !res.conflicts.is_empty();
        let has_unresolved_conflicts = res.has_unresolved_conflicts(TreatAsUnresolved::default());
        if message.is_some() && has_unresolved_conflicts {
            write_unresolved_conflict_paths(err, &res.conflicts)?;
            if debug {
                writeln!(err, "{:#?}", &res.conflicts)?;
            }
            bail!("Tree conflicted, refusing to write commit");
        }

        let tree_id = {
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
            tree_id
        };

        let conflicts = res.conflicts;
        if message.is_some() && !in_memory {
            persist_in_memory_objects(&mut repo)?;
        }

        if let Some(message) = message {
            let head_id = repo.head_id()?;
            let commit_id = if update_head {
                let commit_id = repo.commit("HEAD", message, tree_id, Some(head_id))?;
                let mut index = repo.index_from_tree(&tree_id)?;
                index.write(Default::default())?;
                commit_id
            } else {
                repo.new_commit(message, tree_id, Some(head_id))?.id()
            };
            writeln!(out, "{commit_id} (commit)")?;
            return Ok(());
        }

        if debug {
            writeln!(err, "{conflicts:#?}")?;
        }
        if has_conflicts {
            writeln!(err, "{} possibly resolved conflicts", conflicts.len())?;
        }
        if has_unresolved_conflicts {
            bail!("Tree conflicted")
        }
        Ok(())
    }

    fn persist_in_memory_objects(repo: &mut gix::Repository) -> anyhow::Result<()> {
        let objects = repo.objects.take_object_memory().expect("always write in memory first");
        for (_id, (kind, data)) in objects.iter() {
            repo.write_buf(*kind, data).map_err(|err| anyhow!("{err}"))?;
        }
        Ok(())
    }

    fn write_unresolved_conflict_paths(
        err: &mut dyn std::io::Write,
        conflicts: &[gix::merge::tree::Conflict],
    ) -> std::io::Result<()> {
        let how = TreatAsUnresolved::default();
        let mut paths = BTreeSet::new();
        for conflict in conflicts.iter().filter(|conflict| conflict.is_unresolved(how)) {
            let (ours, theirs) = conflict.changes_in_resolution();
            for path in [
                ours.source_location(),
                ours.location(),
                theirs.source_location(),
                theirs.location(),
            ] {
                if !path.is_empty() {
                    paths.insert(path);
                }
            }
        }
        for path in paths {
            err.write_all(path.as_ref())?;
            err.write_all(b"\n")?;
        }
        Ok(())
    }

    fn refname_and_tree(
        repo: &gix::Repository,
        revspec: BString,
    ) -> anyhow::Result<(Option<BString>, gix::hash::ObjectId)> {
        let spec = repo.rev_parse(revspec.as_bstr())?;
        let tree_id = spec
            .single()
            .context("Expected revspec to expand to a single rev only")?
            .object()?
            .peel_to_tree()?
            .id;
        let refname = spec.first_reference().map(|r| r.name.shorten().as_bstr().to_owned());
        Ok((refname, tree_id))
    }
}
