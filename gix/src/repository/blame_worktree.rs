use std::{borrow::Cow, io::Read, ops::ControlFlow};

use gix_error::{ErrorExt, OptionExt, ResultExt, bail, message, validation};
use gix_object::{Write, tree::EntryKind};
use gix_ref::bstr::{BStr, ByteSlice};

use crate::{Repository, Result, repository::blame_file};

impl Repository {
    /// Attribute the current worktree contents of `file_path`, following staged renames when
    /// rewrite tracking is enabled in `options`.
    ///
    /// During a merge, `MERGE_HEAD` commits also contribute their matching lines.
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
        let mut parents = vec![self.head_id()?.detach()];
        match std::fs::read(self.git_dir().join("MERGE_HEAD")) {
            Ok(contents) => {
                for line in contents.lines() {
                    let id = gix_hash::ObjectId::from_hex(line).or_error()?;
                    let commit_id = self.find_object(id)?.peel_to_commit()?.id;
                    if !parents.contains(&commit_id) {
                        parents.push(commit_id);
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.raise()),
        }
        let (mut filters, index) = self.filter_pipeline(None)?;
        let relative_path = gix_path::from_bstr(file_path.as_ref())?;
        let mut tracked = index.entry_range(&file_path).is_some();
        if !tracked {
            for parent in &parents {
                if self
                    .find_commit(*parent)?
                    .tree()?
                    .lookup_entry_by_path(&relative_path)?
                    .is_some_and(|entry| entry.mode().is_blob_or_symlink())
                {
                    tracked = true;
                    break;
                }
            }
        }
        if !tracked {
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

        let mut origins = Vec::with_capacity(parents.len());
        for commit_id in parents {
            let tree = self.find_commit(commit_id)?.tree()?;
            let head_entry = tree.lookup_entry_by_path(&relative_path)?;
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
                if current.is_sparse() {
                    let validate = self.config.protect_options()?;
                    let tree_id = current.write_tree(&objects, validate)?;
                    current = gix_index::State::from_tree(&tree_id, &objects, validate)?;
                }
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
            origins.push(gix_blame::PreviousOrigin {
                commit_id,
                source_file_name: source_path.into_owned(),
            });
        }
        let outcome = self.blame_file_from(
            &file_path,
            gix_blame::Start::ContentsWithParents {
                parents: origins,
                contents: contents.into(),
            },
            options,
        )?;
        Ok(outcome)
    }
}
