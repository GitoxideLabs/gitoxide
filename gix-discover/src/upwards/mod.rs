mod types;
pub use types::{Error, Options, TrustPolicy};

mod util;

pub(crate) mod function {
    use std::{
        borrow::Cow,
        cell::OnceCell,
        ffi::OsStr,
        path::{Path, PathBuf},
    };

    use gix_sec::Trust;

    use super::{Error, Options, TrustPolicy};
    #[cfg(unix)]
    use crate::upwards::util::device_id;
    use crate::{
        DOT_GIT_DIR,
        is::git_with_metadata as is_git_with_metadata,
        is_git,
        upwards::util::{find_ceiling_height, shorten_path_with_cwd},
    };

    fn resolved_directory_for_parent_traversal(directory: &Path, cwd: &Path) -> Option<PathBuf> {
        let resolved = gix_path::realpath_opts(directory, cwd, gix_path::realpath::MAX_SYMLINKS).ok()?;
        let absolute_directory = if directory.is_absolute() {
            Cow::Borrowed(directory)
        } else {
            Cow::Owned(cwd.join(directory))
        };
        let absolute_directory = gix_path::normalize_and_clean(absolute_directory, cwd)?;
        (absolute_directory.as_ref() != resolved).then_some(resolved)
    }

    /// Find the location of the git repository directly in `directory` or in any of its parent directories and provide
    /// an associated Trust level by looking at the git directory's ownership, and control discovery using `options`.
    ///
    /// Fail if no valid-looking git repository could be found.
    // TODO: tests for trust-based discovery
    #[cfg_attr(not(unix), allow(unused_variables))]
    pub fn discover_opts(
        directory: &Path,
        Options {
            trust,
            ceiling_dirs,
            match_ceiling_dir_or_error,
            cross_fs,
            current_dir,
            dot_git_only,
        }: Options<'_>,
    ) -> Result<(crate::repository::Path, gix_sec::Trust), Error> {
        // Normalize the path so that `Path::parent()` _actually_ gives
        // us the parent directory. (`Path::parent` just strips off the last
        // path component, which means it will not do what you expect when
        // working with paths that contain '..'.)
        let cwd = current_dir.map_or_else(
            || {
                // The paths we return are relevant to the repository, but at this time it's impossible to know
                // what `core.precomposeUnicode` is going to be. Hence, the one using these paths will have to
                // transform the paths as needed, because we can't. `false` means to leave the obtained path as is.
                gix_fs::current_dir(false).map(Cow::Owned)
            },
            |cwd| Ok(Cow::Borrowed(cwd)),
        )?;
        #[cfg(windows)]
        let directory = dunce::simplified(directory);
        let dir = gix_path::normalize(directory.into(), cwd.as_ref()).ok_or_else(|| Error::InvalidInput {
            directory: directory.into(),
        })?;
        let dir_metadata = dir.metadata().map_err(|_| Error::InaccessibleDirectory {
            path: dir.to_path_buf(),
        })?;

        if !dir_metadata.is_dir() {
            return Err(Error::InaccessibleDirectory { path: dir.into_owned() });
        }
        let resolved_dir_for_parent_traversal = OnceCell::<Option<PathBuf>>::new();
        let mut dir_made_absolute = !directory.is_absolute()
            && cwd
                .as_ref()
                .strip_prefix(dir.as_ref())
                .or_else(|_| dir.as_ref().strip_prefix(cwd.as_ref()))
                .is_ok();

        let filter_by_trust = |x: &Path| -> Result<Result<Trust, Trust>, Error> {
            match trust {
                TrustPolicy::Required(required) => {
                    let trust =
                        Trust::from_path_ownership(x).map_err(|err| Error::CheckTrust { path: x.into(), err })?;
                    Ok(if trust >= required { Ok(trust) } else { Err(required) })
                }
                TrustPolicy::Assume(trust) => Ok(Ok(trust)),
            }
        };

        let max_height = if !ceiling_dirs.is_empty() {
            let resolved_dir = resolved_dir_for_parent_traversal
                .get_or_init(|| resolved_directory_for_parent_traversal(dir.as_ref(), cwd.as_ref()));
            let max_height = find_ceiling_height(
                resolved_dir.as_deref().unwrap_or(dir.as_ref()),
                &ceiling_dirs,
                cwd.as_ref(),
            );
            if max_height.is_none() && match_ceiling_dir_or_error {
                return Err(Error::NoMatchingCeilingDir);
            }
            max_height
        } else {
            None
        };

        #[cfg(unix)]
        let initial_device = device_id(&dir_metadata);

        let mut cursor = dir.clone().into_owned();
        let mut current_height = 0;
        let mut cursor_metadata = Some(dir_metadata);
        'outer: loop {
            if max_height.is_some_and(|x| current_height > x) {
                return Err(Error::NoGitRepositoryWithinCeiling {
                    path: dir.into_owned(),
                    ceiling_height: current_height,
                });
            }
            current_height += 1;

            #[cfg(unix)]
            if current_height != 0 && !cross_fs {
                let metadata = cursor_metadata.take().map_or_else(
                    || {
                        if cursor.as_os_str().is_empty() {
                            Path::new(".")
                        } else {
                            cursor.as_ref()
                        }
                        .metadata()
                        .map_err(|_| Error::InaccessibleDirectory { path: cursor.clone() })
                    },
                    Ok,
                )?;

                if device_id(&metadata) != initial_device {
                    return Err(Error::NoGitRepositoryWithinFs {
                        path: dir.into_owned(),
                        limit: cursor.clone(),
                    });
                }
                cursor_metadata = Some(metadata);
            }

            let mut cursor_metadata_backup = None;
            let started_as_dot_git = cursor.file_name() == Some(OsStr::new(DOT_GIT_DIR));
            let dir_manipulation = if dot_git_only { &[true] as &[_] } else { &[true, false] };
            for append_dot_git in dir_manipulation {
                if *append_dot_git && !started_as_dot_git {
                    cursor.push(DOT_GIT_DIR);
                    cursor_metadata_backup = cursor_metadata.take();
                }
                if let Ok(kind) = match cursor_metadata.take() {
                    Some(metadata) => is_git_with_metadata(&cursor, metadata, &cwd),
                    None => is_git(&cursor),
                } {
                    match filter_by_trust(&cursor)? {
                        Ok(trust) => {
                            // TODO: test this more, it definitely doesn't always find the shortest path to a directory
                            let path = if dir_made_absolute {
                                shorten_path_with_cwd(cursor, cwd.as_ref())
                            } else {
                                cursor
                            };
                            break 'outer Ok((
                                crate::repository::Path::from_dot_git_dir(path, kind, cwd.as_ref()).ok_or_else(
                                    || Error::InvalidInput {
                                        directory: directory.into(),
                                    },
                                )?,
                                trust,
                            ));
                        }
                        Err(required) => {
                            break 'outer Err(Error::NoTrustedGitRepository {
                                path: dir.into_owned(),
                                candidate: cursor,
                                required,
                            });
                        }
                    }
                }

                // Usually `.git` (started_as_dot_git == true) will be a git dir, but if not we can quickly skip over it.
                if *append_dot_git || started_as_dot_git {
                    cursor.pop();
                    if let Some(metadata) = cursor_metadata_backup.take() {
                        cursor_metadata = Some(metadata);
                    }
                }
            }
            // Keep the caller's path if it directly identifies a repository. Once we ascend, use
            // physical parents so a symlink to `repo/nested` continues at `repo`, just like Git.
            if current_height == 1 {
                if let Some(resolved_dir) = resolved_dir_for_parent_traversal
                    .get_or_init(|| resolved_directory_for_parent_traversal(dir.as_ref(), cwd.as_ref()))
                {
                    cursor.clone_from(resolved_dir);
                    dir_made_absolute |= !directory.is_absolute();
                }
            }
            if cursor.as_os_str().is_empty() || cursor.as_os_str() == OsStr::new(".") {
                cursor = cwd.to_path_buf();
                dir_made_absolute = true;
            }
            if !cursor.pop() {
                if dir_made_absolute
                    || matches!(
                        cursor.components().next(),
                        Some(std::path::Component::RootDir | std::path::Component::Prefix(_))
                    )
                {
                    break Err(Error::NoGitRepository { path: dir.into_owned() });
                } else {
                    dir_made_absolute = true;
                    debug_assert!(!cursor.as_os_str().is_empty());
                    // Symlinks in the starting directory were resolved before parent traversal,
                    // so only lexical normalization remains here.
                    cursor = gix_path::normalize(cursor.clone().into(), cwd.as_ref())
                        .ok_or_else(|| Error::InvalidInput {
                            directory: cursor.clone(),
                        })?
                        .into_owned();
                }
            }
        }
    }

    /// Find the location of the git repository directly in `directory` or in any of its parent directories, and provide
    /// the trust level derived from Path ownership.
    ///
    /// Fail if no valid-looking git repository could be found.
    pub fn discover(directory: &Path) -> Result<(crate::repository::Path, gix_sec::Trust), Error> {
        discover_opts(directory, Default::default())
    }
}
