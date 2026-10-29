use std::{borrow::Cow, io::Read, ops::ControlFlow};

use gix_error::{OptionExt, ResultExt, bail, message, validation};
use gix_object::{Write, tree::EntryKind};
use gix_ref::bstr::BStr;

use crate::{Repository, Result, repository::blame_file};

impl Repository {
    /// Attribute the current worktree contents of `file_path`, following staged renames when
    /// rewrite tracking is enabled in `options`.
    ///
    /// The path is resolved with [`normalize_path()`](Self::normalize_path). The file must exist
    /// in the index or `HEAD`. Worktree filters and, optionally, text conversion are applied before
    /// blaming; lines changed since `HEAD` carry the null id. Neither objects nor the index are written.
    ///
    /// Returns an error for bare repositories, untracked or unreadable files, unsupported file types,
    /// failed filters, or failures while following history.
    pub fn blame_worktree_file(
        &self,
        file_path: &BStr,
        mut options: blame_file::Options,
    ) -> Result<gix_blame::Outcome> {
        let file_path = self.normalize_path(file_path)?;
        let workdir = self
            .workdir()
            .ok_or_raise(|| validation("Worktree blame requires a worktree"))?;
        let head = self.head_commit()?;
        let tree = head.tree()?;
        let (mut filters, index) = self.filter_pipeline(None)?;
        let relative_path = gix_path::from_bstr(file_path.as_ref())?;
        let head_entry = tree.lookup_entry_by_path(&relative_path)?;
        if index.entry_by_path(&file_path).is_none() && head_entry.is_none() {
            bail!("The path {file_path:?} is not tracked".not_found());
        }
        let worktree_path = workdir.join(relative_path.as_ref());
        let metadata = std::fs::symlink_metadata(&worktree_path)
            .or_raise(|| message!("Could not read worktree file {file_path:?}"))?;
        let mode = if metadata.file_type().is_symlink() {
            EntryKind::Link
        } else if metadata.is_file() {
            EntryKind::Blob
        } else {
            bail!("Unsupported worktree file type at {file_path:?}".validation());
        };
        options.contents_mode = Some(mode);
        let contents = if mode == EntryKind::Link {
            gix_path::into_bstr(std::fs::read_link(&worktree_path).or_error()?)?
                .into_owned()
                .into()
        } else {
            let converted = if options.textconv {
                let mut cache = self.diff_resource_cache(
                    gix_diff::blob::pipeline::Mode::ToGitUnlessBinaryToTextIsPresent,
                    gix_diff::blob::pipeline::WorktreeRoots {
                        old_root: None,
                        new_root: Some(workdir.to_owned()),
                    },
                )?;
                cache.set_resource(
                    self.object_hash().null(),
                    mode,
                    &file_path,
                    gix_diff::blob::ResourceKind::NewOrDestination,
                    &self.objects,
                )?;
                cache
                    .resource(gix_diff::blob::ResourceKind::NewOrDestination)
                    .and_then(|resource| {
                        resource.data.as_slice().map(|data| {
                            let textconv = resource
                                .driver_index
                                .and_then(|index| cache.filter.drivers().get(index))
                                .is_some_and(|driver| driver.binary_to_text_command.is_some());
                            (data.to_vec(), textconv)
                        })
                    })
            } else {
                None
            };
            match converted {
                Some((data, false)) => data,
                Some((data, true)) => {
                    let mut contents = Vec::new();
                    filters
                        .convert_to_git(data.as_slice(), &relative_path, &index)?
                        .read_to_end(&mut contents)
                        .or_error()?;
                    contents
                }
                None => {
                    let mut contents = Vec::new();
                    filters
                        .convert_to_git(std::fs::File::open(&worktree_path).or_error()?, &relative_path, &index)?
                        .read_to_end(&mut contents)
                        .or_error()?;
                    contents
                }
            }
        };

        let mut source_path = Cow::Borrowed(file_path.as_ref());
        if let Some(rewrites) = options.rewrites
            && head_entry
                .as_ref()
                .is_none_or(|entry| entry.mode().is_link() != (mode == EntryKind::Link))
        {
            let baseline = self.index_from_tree(tree.id.as_ref())?;
            let mut current = gix_index::State::clone(&index);
            let objects = gix_odb::memory::Proxy::new(&self.objects, self.object_hash());
            let blob_id = objects.write_buf(gix_object::Kind::Blob, &contents)?;
            // Git compares the working file with HEAD while retaining the staged deletions that
            // identify rename sources. Keep that synthetic blob and index entirely in memory.
            current.remove_entries(|_, path, _| path == file_path.as_ref());
            current.dangerously_push_entry(
                Default::default(),
                blob_id,
                gix_index::entry::Flags::empty(),
                gix_object::tree::EntryMode::from(mode).into(),
                &file_path,
            );
            current.sort_entries();
            let mut cache = self.diff_resource_cache_for_tree_diff()?;
            gix_diff::index(
                &baseline,
                &current,
                |change| {
                    if let gix_diff::index::ChangeRef::Rewrite {
                        source_location,
                        location,
                        ..
                    } = change
                        && location == file_path
                    {
                        source_path = Cow::Owned(source_location.into_owned());
                        return Ok(ControlFlow::Break(()));
                    }
                    Ok(ControlFlow::Continue(()))
                },
                Some(gix_diff::index::RewriteOptions {
                    resource_cache: &mut cache,
                    find: &objects,
                    rewrites,
                }),
                &mut gix_pathspec::Search::from_specs([], None, workdir)?,
                &mut |_, _, _, _| false,
            )?;
        }
        let mut outcome = self.blame_file_from(
            &source_path,
            gix_blame::Start::Contents {
                first_suspect: head.id,
                contents: contents.into(),
            },
            options,
        )?;
        if source_path != file_path {
            for entry in &mut outcome.entries {
                if !entry.commit_id.is_null() && entry.source_file_name.is_none() {
                    entry.source_file_name = Some(source_path.as_ref().to_owned());
                }
            }
            outcome.previous_origins = outcome
                .previous_origins
                .into_iter()
                .map(|((commit, path), origin)| {
                    let path = if !commit.is_null() && path.is_none() {
                        Some(source_path.as_ref().to_owned())
                    } else {
                        path
                    };
                    ((commit, path), origin)
                })
                .collect();
        }
        Ok(outcome)
    }
}
