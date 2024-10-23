use gix::bstr::{BStr, BString, ByteSlice};
use gix::prelude::FindExt;
use gix::ObjectId;

pub fn log(mut repo: gix::Repository, out: &mut dyn std::io::Write, pathspec: BString) -> anyhow::Result<()> {
    repo.object_cache_size_if_unset(repo.compute_object_cache_size_for_tree_diffs(&**repo.index_or_empty()?));

    let head = repo.head()?.peel_to_commit_in_place()?;
    let infos: Vec<_> =
        gix::traverse::commit::topo::Builder::from_iters(&repo.objects, [head.id], None::<Vec<gix::ObjectId>>)
            .build()?
            .collect();

    let infos: Vec<_> = infos
        .into_iter()
        .filter(|info| {
            let commit = repo.find_commit(info.as_ref().unwrap().id).unwrap();

            let mut buffer = Vec::new();
            let tree = repo.objects.find_tree(&commit.tree_id().unwrap(), &mut buffer).unwrap();

            let Some(entry) = tree.bisect_entry(pathspec.as_ref(), false) else {
                return false;
            };

            let parent_ids: Vec<_> = commit.parent_ids().collect();

            if parent_ids.is_empty() {
                // We confirmed above that the file is in `commit`'s tree. If `parent_ids` is
                // empty, the file was added in `commit`.

                return true;
            }

            let parent_ids_with_changes: Vec<_> = parent_ids
                .clone()
                .into_iter()
                .filter(|parent_id| {
                    let mut buffer = Vec::new();
                    let parent_commit = repo.find_commit(*parent_id).unwrap();
                    let parent_tree = repo
                        .objects
                        .find_tree(&parent_commit.tree_id().unwrap(), &mut buffer)
                        .unwrap();

                    if let Some(parent_entry) = parent_tree.bisect_entry(pathspec.as_ref(), false) {
                        if entry.oid == parent_entry.oid {
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

                return false;
            }

            for parent_id in parent_ids_with_changes {
                let modifications =
                    get_modifications_for_file_path(&repo.objects, pathspec.as_ref(), commit.id, parent_id.into());

                return !modifications.is_empty();
            }

            return false;
        })
        .collect();

    write_infos(&repo, out, infos)?;

    Ok(())
}

fn write_infos(
    repo: &gix::Repository,
    mut out: impl std::io::Write,
    infos: Vec<Result<gix::traverse::commit::Info, gix::traverse::commit::topo::Error>>,
) -> Result<(), std::io::Error> {
    for info in infos {
        let info = info.unwrap();
        let commit = repo.find_commit(info.id).unwrap();

        let message = commit.message_raw_sloppy();
        let title = message.lines().next();

        writeln!(
            out,
            "{} {}",
            info.id.to_hex_with_len(8),
            title.map(BString::from).unwrap_or_else(|| "<no message>".into())
        )?;
    }

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
