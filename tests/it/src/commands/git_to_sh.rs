pub struct Options {
    pub patterns: Vec<gix::pathspec::Pattern>,
    pub verbatim: bool,
    pub max_count: usize,
}

pub(super) mod function {
    use std::{borrow::Cow, path::Path};

    use anyhow::{Context, bail};
    use gix::{object::tree::EntryKind, objs::FindExt};

    use super::Options;

    pub fn git_to_sh(
        output_dir: &Path,
        repo_dir: &Path,
        name: &str,
        committish: &str,
        mut out: impl std::io::Write,
        Options {
            patterns,
            verbatim,
            max_count,
        }: Options,
    ) -> anyhow::Result<()> {
        let repo = gix::open(repo_dir)?;
        let commit = repo.rev_parse_single(committish)?.object()?.try_into_commit()?;

        let assets = output_dir.join(name);
        std::fs::create_dir_all(&assets)?;

        let mut commits = Vec::new();
        let mut tree_buf = Vec::new();
        let mut current = 0;
        for entry in commit.id().ancestors().first_parent_only().all()? {
            let entry = entry?;

            let commit = entry.id().object()?.into_commit();
            commits.push((commit.id, commit.message_raw_sloppy().to_owned()));
            let index = repo.index_from_tree(&commit.tree_id()?)?;

            tree_buf.clear();
            write_tree_as_update_index_format(&repo, &index, &mut tree_buf, &assets, verbatim, patterns.clone())?;

            let tree_file = assets.join(format!("{}.tree", commit.id));
            std::fs::write(tree_file, &tree_buf)?;
            current += 1;

            if current >= max_count {
                break;
            }
        }

        writeln!(
            &mut out,
            "# The following is to be executed in the receiving git repository"
        )?;
        writeln!(&mut out, "ROOT=to-be-specified-by-user")?;
        writeln!(&mut out, "index=.git/index")?;
        writeln!(&mut out, "git hash-object -w -t blob -- $ROOT/{name}/*.blob")?;
        for (commit_id, commit_msg) in commits.iter().rev() {
            writeln!(&mut out, "rm \"$index\"")?;
            writeln!(
                &mut out,
                "git update-index --index-info < \"$ROOT/{name}/{commit_id}.tree\""
            )?;
            let commit_msg_file = assets.join(format!("{commit_id}.msg"));
            std::fs::write(commit_msg_file, commit_msg)?;
            writeln!(&mut out, "git commit --allow-empty -F \"$ROOT/{name}/{commit_id}.msg\"")?;
        }

        Ok(())
    }

    fn write_tree_as_update_index_format(
        repo: &gix::Repository,
        index: &gix::index::State,
        out: &mut dyn std::io::Write,
        output_dir: &Path,
        verbatim: bool,
        patterns: Vec<gix::pathspec::Pattern>,
    ) -> anyhow::Result<()> {
        let mut blob_buf = Vec::new();
        let mut specs = repo.pathspec(
            true,
            // TODO: ideally this could accept patterns already.
            patterns.clone().into_iter().map(|p| p.to_bstring()),
            true,
            index,
            gix::worktree::stack::state::attributes::Source::IdMapping,
        )?;

        for (rela_path, entry) in specs.index_entries_with_paths(index).into_iter().flatten() {
            if rela_path.contains(&b'\n') {
                bail!(
                    "Entry at '{rela_path}' contained a newline, which currently can't be encoded. Preferred newlines over NULL separation."
                )
            }

            let (blob_id, blob_data) = match entry.mode.to_tree_entry_mode() {
                None => {
                    bail!("Couldn't interpret mode of tree entry at '{rela_path}'")
                }
                Some(mode) => match mode.kind() {
                    EntryKind::Tree => {
                        unreachable!("Can't have trees in indices")
                    }
                    EntryKind::Blob | EntryKind::BlobExecutable => {
                        let obj = repo.objects.find(&entry.id, &mut blob_buf)?;
                        if verbatim {
                            (entry.id, Cow::Borrowed(&blob_buf))
                        } else {
                            let data = std::str::from_utf8(obj.data).with_context(|| {
                                format!("Entry at '{rela_path}' was not valid UTF8 and can't be remapped")
                            })?;
                            let mapped = crate::commands::copy_royal::remapped(data);
                            (
                                gix::objs::compute_hash(
                                    repo.object_hash(),
                                    gix::object::Kind::Blob,
                                    mapped.as_bytes(),
                                )?,
                                Cow::Owned(mapped.into()),
                            )
                        }
                    }
                    EntryKind::Link => {
                        repo.objects.find(&entry.id, &mut blob_buf)?;
                        (entry.id, Cow::Borrowed(&blob_buf))
                    }
                    EntryKind::Commit => continue,
                },
            };
            let blob_path = output_dir.join(format!("{blob_id}.blob"));
            std::fs::write(blob_path, blob_data.as_ref())?;

            writeln!(out, "{mode:06o} {blob_id}\t{rela_path}", mode = entry.mode)?;
        }
        Ok(())
    }
}
