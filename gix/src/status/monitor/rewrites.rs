use std::sync::atomic::AtomicBool;

use gix_error::{ErrorExt, Result, message};
use gix_status::index_as_worktree_with_renames::{Entry, Recorder, Sorting, track_rewrites};

use crate::{
    Repository,
    status::{Item, index_worktree::Item as WorktreeItem},
};

pub(super) fn apply(
    repo: &Repository,
    index: &gix_index::State,
    items: &[Item],
    rewrites: gix_diff::Rewrites,
    interrupt: &AtomicBool,
) -> Result<Vec<Item>> {
    let worktree = repo
        .workdir()
        .ok_or_else(|| message("worktree rewrite tracking requires a worktree").raise())?;
    let attributes = repo.attributes_only(
        index,
        crate::worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
    )?;
    let mut resources = crate::diff::resource_cache(
        repo,
        gix_diff::blob::pipeline::Mode::ToGit,
        attributes.inner,
        gix_diff::blob::pipeline::WorktreeRoots {
            old_root: None,
            new_root: Some(worktree.to_owned()),
        },
    )?;
    let mut next = Vec::new();
    let mut entries = Vec::new();
    for item in items {
        super::check_interrupted(interrupt)?;
        match item {
            Item::TreeIndex(_) => next.push(item.clone()),
            Item::IndexWorktree(WorktreeItem::Modification {
                entry,
                entry_index,
                rela_path,
                status,
            }) => entries.push(Entry::Modification {
                entries: index.entries(),
                entry,
                entry_index: *entry_index,
                rela_path: rela_path.as_ref(),
                status: status.clone(),
            }),
            Item::IndexWorktree(WorktreeItem::DirectoryContents {
                entry,
                collapsed_directory_status,
            }) => entries.push(Entry::DirectoryContents {
                entry: entry.clone(),
                collapsed_directory_status: *collapsed_directory_status,
            }),
            Item::IndexWorktree(WorktreeItem::Rewrite { .. }) => {
                return Err(message("cached rewrite candidates must contain underlying status entries").raise());
            }
        }
    }
    let mut recorder = Recorder::default();
    track_rewrites(
        index,
        worktree,
        entries,
        &mut recorder,
        &mut resources,
        &repo.objects,
        interrupt,
        Some(rewrites),
        Some(Sorting::ByPathCaseSensitive),
    )?;
    next.extend(
        recorder
            .records
            .into_iter()
            .filter(|entry| {
                !matches!(
                    entry,
                    Entry::Modification {
                        status: gix_status::index_as_worktree::EntryStatus::NeedsUpdate(_),
                        ..
                    }
                )
            })
            .map(WorktreeItem::from)
            .map(Item::IndexWorktree),
    );
    Ok(next)
}
