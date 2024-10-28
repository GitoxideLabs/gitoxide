use gix::bstr::{BStr, BString, ByteSlice};
use gix::prelude::FindExt;
use gix::ObjectId;

pub fn log(mut repo: gix::Repository, out: &mut dyn std::io::Write, path: Option<BString>) -> anyhow::Result<()> {
    repo.object_cache_size_if_unset(repo.compute_object_cache_size_for_tree_diffs(&**repo.index_or_empty()?));

    if let Some(path) = path {
        log_file(repo, out, path)
    } else {
        log_all(repo, out)
    }
}

fn log_all(repo: gix::Repository, out: &mut dyn std::io::Write) -> Result<(), anyhow::Error> {
    let head = repo.head()?.peel_to_commit_in_place()?;
    let topo = gix::traverse::commit::topo::Builder::from_iters(&repo.objects, [head.id], None::<Vec<gix::ObjectId>>)
        .build()?;

    for info in topo {
        let info = info?;

        write_info(&repo, &mut *out, &info)?;
    }

    Ok(())
}

fn log_file(repo: gix::Repository, out: &mut dyn std::io::Write, path: BString) -> anyhow::Result<()> {
    let head = repo.head()?.peel_to_commit_in_place()?;
    let topo = gix::traverse::commit::topo::Builder::from_iters(&repo.objects, [head.id], None::<Vec<gix::ObjectId>>)
        .build()?;

    'outer: for info in topo {
        let info = info?;
        let commit = repo.find_commit(info.id).unwrap();

        let tree = repo.find_tree(commit.tree_id().unwrap()).unwrap();

        let entry = tree.lookup_entry_by_path(path.to_path().unwrap()).unwrap();

        let Some(entry) = entry else {
            continue;
        };

        let parent_ids: Vec<_> = commit.parent_ids().collect();

        if parent_ids.is_empty() {
            // We confirmed above that the file is in `commit`'s tree. If `parent_ids` is
            // empty, the file was added in `commit`.

            write_info(&repo, out, &info)?;

            break;
        }

        let parent_ids_with_changes: Vec<_> = parent_ids
            .clone()
            .into_iter()
            .filter(|parent_id| {
                let parent_commit = repo.find_commit(*parent_id).unwrap();
                let parent_tree = repo.find_tree(parent_commit.tree_id().unwrap()).unwrap();
                let parent_entry = parent_tree.lookup_entry_by_path(path.to_path().unwrap()).unwrap();

                if let Some(parent_entry) = parent_entry {
                    if entry.oid() == parent_entry.oid() {
                        // The blobs storing the file in `entry` and `parent_entry` are
                        // identical which means the file was not changed in `commit`.

                        return false;
                    }
                }

                true
            })
            .collect();

        if parent_ids.len() != parent_ids_with_changes.len() {
            // At least one parent had an identical version of the file which means it was not
            // changed in `commit`.

            continue;
        }

        for parent_id in parent_ids_with_changes {
            let modifications =
                get_modifications_for_file_path(&repo.objects, path.as_ref(), commit.id, parent_id.into());

            if !modifications.is_empty() {
                write_info(&repo, &mut *out, &info)?;

                // We continue because we’ve already determined that this commit is part of the
                // file’s history, so there’s no need to compare it to its other parents.

                continue 'outer;
            }
        }
    }

    Ok(())
}

fn write_info(
    repo: &gix::Repository,
    mut out: impl std::io::Write,
    info: &gix::traverse::commit::Info,
) -> Result<(), std::io::Error> {
    let commit = repo.find_commit(info.id).unwrap();

    let message = commit.message_raw_sloppy();
    let title = message.lines().next();

    writeln!(
        out,
        "{} {}",
        info.id.to_hex_with_len(8),
        title.map_or_else(|| "<no message>".into(), BString::from)
    )?;

    Ok(())
}

fn get_modifications_for_file_path(
    odb: impl gix::objs::Find + gix::objs::FindHeader,
    file_path: &BStr,
    id: ObjectId,
    parent_id: ObjectId,
) -> Vec<gix::diff::tree::recorder::Change> {
    let mut buffer = Vec::new();

    let parent = odb.find_commit(&parent_id, &mut buffer).unwrap();

    let mut buffer = Vec::new();
    let parent_tree_iter = odb
        .find(&parent.tree(), &mut buffer)
        .unwrap()
        .try_into_tree_iter()
        .unwrap();

    let mut buffer = Vec::new();
    let commit = odb.find_commit(&id, &mut buffer).unwrap();

    let mut buffer = Vec::new();
    let tree_iter = odb
        .find(&commit.tree(), &mut buffer)
        .unwrap()
        .try_into_tree_iter()
        .unwrap();

    let mut recorder = gix::diff::tree::Recorder::default();
    gix::diff::tree(
        parent_tree_iter,
        tree_iter,
        gix::diff::tree::State::default(),
        &odb,
        &mut recorder,
    )
    .unwrap();

    recorder
        .records
        .iter()
        .filter(|change| match change {
            gix::diff::tree::recorder::Change::Modification { path, .. } => path == file_path,
            gix::diff::tree::recorder::Change::Addition { path, .. } => path == file_path,
            _ => false,
        })
        .cloned()
        .collect()
}
