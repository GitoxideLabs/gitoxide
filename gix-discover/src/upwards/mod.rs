mod types;
pub use types::{Error, Options, TrustPolicy};

mod util;

pub(crate) mod function {
    use gix_error::Result;
    use std::{
        borrow::Cow,
        cell::OnceCell,
        ffi::OsStr,
        path::{Path, PathBuf},
    };

    use gix_error::{ErrorExt, OptionExt, ResultExt, bail, message, validation};
    use gix_sec::Trust;

    use super::{Error, Options, TrustPolicy};
    #[cfg(unix)]
    use crate::upwards::util::device_id;
    use crate::{
        DOT_GIT_DIR,
        is::{git_with_cwd as is_git_with_cwd, git_with_metadata as is_git_with_metadata},
        upwards::util::{find_ceiling_height, shorten_path_with_cwd},
    };

    /// Resolve `directory` before lexical normalization so a symlink followed by `..` ascends from its target.
    ///
    /// Return the resolved path only when it differs from the normalized absolute spelling. `None` means either that
    /// no switch to a physical cursor is necessary or that resolution failed, in which case traversal keeps its
    /// logical cursor.
    fn resolved_directory_for_parent_traversal(directory: &Path, cwd: &Path) -> Option<PathBuf> {
        let resolved = gix_path::realpath_opts(directory, cwd, gix_path::realpath::MAX_SYMLINKS).ok()?;
        let absolute = if directory.is_absolute() {
            Cow::Borrowed(directory)
        } else {
            Cow::Owned(cwd.join(directory))
        };
        let absolute = gix_path::normalize_and_clean(absolute, cwd)?;
        (absolute.as_ref() != resolved).then_some(resolved)
    }

    /// The caller-facing path and the cursor used for filesystem access.
    struct SearchPath {
        /// The normalized caller-provided spelling, i.e. without relative path components, but with leading `.`.
        /// It never moves and is used to reconstruct the returned path.
        logical: PathBuf,
        /// Initially a copy of `logical`. [`SearchPath::use_physical_start()`] replaces it with the resolved path
        /// before the first probe when the input contains `..`; otherwise it does so after the direct probe and
        /// before ascending.
        current: PathBuf,
        /// Cached metadata for the filesystem object identified by `current`.
        current_metadata: Option<std::fs::Metadata>,
        /// `Some(n)` records how many parents `current` has ascended since switching to the resolved path, allowing
        /// the same ancestor to be reconstructed from `logical`. `None` means no distinct physical traversal began.
        physical_parent_steps: Option<usize>,
    }

    // Discovery-facing API.
    impl SearchPath {
        pub fn new(logical: PathBuf, current_metadata: std::fs::Metadata) -> Self {
            let current = logical.clone();
            SearchPath {
                logical,
                current,
                current_metadata: Some(current_metadata),
                physical_parent_steps: None,
            }
        }

        /// Use the distinct physical path returned by [`resolved_directory_for_parent_traversal()`] as the cursor.
        ///
        /// `Some` starts physical traversal and resets its ascent count. `None` leaves the logical cursor unchanged
        /// because resolution either failed or produced the same normalized absolute path.
        pub fn use_physical_start(&mut self, resolved: Option<&PathBuf>) {
            if let Some(resolved) = resolved {
                // Resolution changes only the spelling, so the cached metadata still describes this directory.
                self.current.clone_from(resolved);
                self.physical_parent_steps = Some(0);
            }
        }

        pub fn traverses_resolved_path(&self) -> bool {
            self.physical_parent_steps.is_some()
        }

        #[cfg(unix)]
        pub fn metadata(&mut self) -> Result<&std::fs::Metadata> {
            if self.current_metadata.is_none() {
                let path = if self.current.as_os_str().is_empty() {
                    Path::new(".")
                } else {
                    self.current.as_ref()
                };
                let inaccessible_path = self.current.clone();
                self.current_metadata = Some(path.metadata().or_raise(|| {
                    message!(
                        "Failed to access a directory, or path is not a directory: \"{}\"",
                        inaccessible_path.display()
                    )
                })?);
            }
            Ok(self
                .current_metadata
                .as_ref()
                .expect("metadata was initialized immediately above"))
        }

        /// Move `current` to its parent, invalidating its metadata and recording a resolved-path ascent.
        ///
        /// Return `false` if `current` has no component to remove.
        pub fn ascend(&mut self) -> bool {
            let popped = self.current.pop();
            if popped {
                self.current_metadata = None;
                if let Some(parent_steps) = self.physical_parent_steps.as_mut() {
                    *parent_steps += 1;
                }
            }
            popped
        }

        /// Replace a cursor denoting the current directory with `cwd` so its parent can be traversed.
        pub fn make_absolute_if_needed(&mut self, cwd: &Path) {
            if self.current.as_os_str().is_empty() || self.current == Path::new(".") {
                self.set_current(cwd.to_path_buf());
            }
        }

        /// Convert the discovered physical cursor into a caller-facing repository candidate.
        ///
        /// After physical traversal, replay the recorded ascents on `logical` and mirror an appended `.git`
        /// as per `append_dot_git`. Use that caller-provided spelling only if it resolves to the same
        /// location as `current`; otherwise retain `current`.
        pub fn into_candidate(self, cwd: &Path, append_dot_git: bool) -> PathBuf {
            let Some(parent_steps) = self.physical_parent_steps else {
                return self.current;
            };

            let mut logical = self.logical;
            for _ in 0..parent_steps {
                if logical.as_os_str().is_empty() || logical.as_os_str() == OsStr::new(".") {
                    cwd.clone_into(&mut logical);
                }
                if !logical.pop() {
                    return self.current;
                }
            }
            if append_dot_git {
                logical.push(DOT_GIT_DIR);
            }

            let logical_resolved = gix_path::realpath_opts(&logical, cwd, gix_path::realpath::MAX_SYMLINKS).ok();
            let same_candidate = logical_resolved.as_deref() == Some(self.current.as_path())
                || logical_resolved.is_some_and(|logical_resolved| {
                    gix_path::realpath_opts(&self.current, cwd, gix_path::realpath::MAX_SYMLINKS)
                        .is_ok_and(|physical| logical_resolved == physical)
                });
            if same_candidate { logical } else { self.current }
        }

        /// Probe `current/.git`, then `current` as a possible bare repository unless `dot_git_only` is set.
        ///
        /// If `current` itself is named `.git`, it is probed directly.
        /// On success, the boolean indicates whether `.git` was appended to `current`,
        /// which remains at the discovered repository.
        pub fn probe_repository(&mut self, cwd: &Path, dot_git_only: bool) -> Option<(crate::repository::Kind, bool)> {
            let started_as_dot_git = self.current.file_name() == Some(OsStr::new(DOT_GIT_DIR));
            if started_as_dot_git {
                let kind = match self.current_metadata.as_ref() {
                    Some(metadata) => is_git_with_metadata(&self.current, metadata, cwd),
                    None => is_git_with_cwd(&self.current, cwd),
                };
                return kind.ok().map(|kind| (kind, false));
            }

            let dot_git = self.current.join(DOT_GIT_DIR);
            if let Ok(kind) = is_git_with_cwd(&dot_git, cwd) {
                self.set_current(dot_git);
                return Some((kind, true));
            }
            if !dot_git_only {
                let kind = match self.current_metadata.as_ref() {
                    Some(metadata) => is_git_with_metadata(&self.current, metadata, cwd),
                    None => is_git_with_cwd(&self.current, cwd),
                };
                if let Ok(kind) = kind {
                    return Some((kind, false));
                }
            }
            None
        }

        fn set_current(&mut self, current: PathBuf) {
            self.current = current;
            self.current_metadata = None;
        }
    }

    /// Find the location of the git repository directly in `directory` or in any of its parent directories,
    /// controlling discovery using `options`, and return its path and associated trust level.
    ///
    /// [`Options::trust`] determines whether trust is [derived from ownership of all relevant repository paths][TrustPolicy::Required]
    /// or [supplied by the caller without ownership checks][TrustPolicy::Assume]. The returned trust is not
    /// necessarily derived only from the returned path; see [`TrustPolicy`] for details.
    ///
    /// Fail if no valid-looking git repository could be found.
    /// Downcast to [`Error`] to distinguish a missing repository or a search limit from an untrusted candidate.
    /// Filesystem and other operational failures retain their original causes.
    pub fn discover_opts(directory: &Path, options: Options<'_>) -> Result<(crate::repository::Path, gix_sec::Trust)> {
        discover_opts_with_trust(directory, options, Trust::from_path_ownership)
    }

    /// Note that `trust_from_path` is really only needed for testability.
    #[cfg_attr(not(unix), allow(unused_variables))]
    fn discover_opts_with_trust(
        directory: &Path,
        Options {
            trust,
            ceiling_dirs,
            match_ceiling_dir_or_error,
            cross_fs,
            current_dir,
            dot_git_only,
        }: Options<'_>,
        mut trust_from_path: impl FnMut(&Path) -> std::io::Result<Trust>,
    ) -> Result<(crate::repository::Path, gix_sec::Trust)> {
        // Normalize the path so that `Path::parent()` _actually_ gives
        // us the parent directory. (`Path::parent` just strips off the last
        // path component, which means it will not do what you expect when
        // working with paths that contain '..'.)
        let cwd = current_dir
            .map_or_else(
                || {
                    // The paths we return are relevant to the repository, but at this time it's impossible to know
                    // what `core.precomposeUnicode` is going to be. Hence, the one using these paths will have to
                    // transform the paths as needed, because we can't. `false` means to leave the obtained path as is.
                    gix_fs::current_dir(false).map(Cow::Owned)
                },
                |cwd| Ok(Cow::Borrowed(cwd)),
            )
            .or_raise(|| message("Could not obtain the current working directory"))?;
        #[cfg(windows)]
        let directory = dunce::simplified(directory);
        let logical = gix_path::normalize(directory.into(), cwd.as_ref())
            .ok_or_raise(|| {
                validation(format!(
                    "Relative path \"{}\" tries to reach beyond root filesystem",
                    directory.display()
                ))
            })?
            .into_owned();
        let directory_to_access = if directory.is_absolute() {
            Cow::Borrowed(directory)
        } else {
            Cow::Owned(cwd.join(directory))
        };
        let dir_metadata = directory_to_access.metadata().or_raise(|| {
            message!(
                "Failed to access a directory, or path is not a directory: \"{}\"",
                logical.display()
            )
        })?;

        if !dir_metadata.is_dir() {
            bail!(validation(format!(
                "Failed to access a directory, or path is not a directory: '{}'",
                logical.display()
            )));
        }
        #[cfg(unix)]
        let initial_device = device_id(&dir_metadata);
        let resolved = OnceCell::<Option<PathBuf>>::new();
        let resolved = || resolved.get_or_init(|| resolved_directory_for_parent_traversal(directory, cwd.as_ref()));
        let mut filter_by_trust =
            |dir: &Path, kind: &crate::repository::Kind| -> Result<std::result::Result<Trust, (Trust, Trust)>> {
                match trust {
                    TrustPolicy::Required(required) => {
                        use crate::repository::Kind;
                        let git_dir = match kind {
                            Kind::Submodule { git_dir }
                            | Kind::WorkTree {
                                linked_git_dir: Some(git_dir),
                            } => git_dir.as_path(),
                            _ => dir,
                        };
                        let work_dir = match kind {
                            Kind::WorkTree { .. } | Kind::Submodule { .. } => dir.parent(),
                            Kind::WorkTreeGitDir { work_dir } => Some(work_dir.as_path()),
                            _ => None,
                        };
                        let trust = crate::repository::trust_with(
                            git_dir,
                            work_dir,
                            cwd.as_ref(),
                            (dir, None),
                            &mut trust_from_path,
                        )?;
                        Ok(if trust >= required {
                            Ok(trust)
                        } else {
                            Err((required, trust))
                        })
                    }
                    TrustPolicy::Assume(trust) => Ok(Ok(trust)),
                }
            };

        // A preceding symlink makes `..` ascend from its target rather than its lexical parent.
        // Resolve any input containing `..` before probing because only the filesystem can distinguish these cases.
        let mut search = SearchPath::new(logical, dir_metadata);
        if directory
            .components()
            .any(|component| component == std::path::Component::ParentDir)
        {
            search.use_physical_start(resolved().as_ref());
        }

        let max_height = if !ceiling_dirs.is_empty() {
            let max_height = find_ceiling_height(
                resolved().as_deref().unwrap_or(&search.logical),
                &ceiling_dirs,
                cwd.as_ref(),
            );
            if max_height.is_none() && match_ceiling_dir_or_error {
                bail!(validation(
                    "None of the passed ceiling directories prefixed the git-dir candidate, making them ineffective."
                ));
            }
            max_height
        } else {
            None
        };

        let mut height = 0;
        'outer: loop {
            if max_height.is_some_and(|max| height > max) {
                bail!(Error::NoGitRepositoryWithinCeiling {
                    path: search.logical,
                    ceiling_height: height,
                });
            }

            #[cfg(unix)]
            if !cross_fs && device_id(search.metadata()?) != initial_device {
                bail!(Error::NoGitRepositoryWithinFs {
                    path: search.logical,
                    limit: search.current,
                });
            }

            if let Some((kind, appended_dot_git)) = search.probe_repository(cwd.as_ref(), dot_git_only) {
                match filter_by_trust(&search.current, &kind)? {
                    Err((required, trust)) => {
                        break 'outer Err(Error::NoTrustedGitRepository {
                            path: search.logical,
                            candidate: search.current,
                            required,
                            trust,
                        }
                        .raise());
                    }
                    Ok(trust) => {
                        let cursor = search.into_candidate(cwd.as_ref(), appended_dot_git);
                        // Prefer a shorter `../…/.git` spelling when the repository is a lexical ancestor of `cwd`;
                        // otherwise retain the discovered spelling.
                        let path = if directory.is_relative() && cursor.is_absolute() {
                            shorten_path_with_cwd(cursor, cwd.as_ref())
                        } else {
                            cursor
                        };
                        break 'outer Ok((
                            crate::repository::Path::from_dot_git_dir(path, kind, cwd.as_ref()).ok_or_raise(|| {
                                validation(format!(
                                    "Relative path \"{}\" tries to reach beyond root filesystem",
                                    directory.display()
                                ))
                            })?,
                            trust,
                        ));
                    }
                }
            }
            if height == 0 {
                // The first probe keeps the caller's spelling when possible. All ascent is physical.
                if !search.traverses_resolved_path() {
                    search.use_physical_start(resolved().as_ref());
                }
            }

            search.make_absolute_if_needed(cwd.as_ref());
            if !search.ascend() {
                if matches!(
                    search.current.components().next(),
                    Some(std::path::Component::RootDir | std::path::Component::Prefix(_))
                ) {
                    break Err(Error::NoGitRepository { path: search.logical }.raise());
                } else {
                    debug_assert!(
                        !search.current.as_os_str().is_empty(),
                        "only a non-empty relative cursor can require normalization after ascent stalls"
                    );
                    let current = gix_path::normalize(search.current.clone().into(), cwd.as_ref())
                        .ok_or_raise(|| {
                            validation(format!(
                                "Relative path \"{}\" tries to reach beyond root filesystem",
                                search.current.display()
                            ))
                        })?
                        .into_owned();
                    search.set_current(current);
                }
            }
            height += 1;
        }
    }

    /// Find the location of the git repository directly in `directory` or in any of its parent directories,
    /// and return its path and ownership-derived trust level.
    ///
    /// Uses default options with [`TrustPolicy::Required`] set to [`Trust::Reduced`]. The returned trust is the
    /// minimum ownership-derived trust of all paths described by that policy, not just the returned path,
    /// and can be [`Trust::Full`]. See [`crate::upwards_opts()`] to customize discovery and its trust policy.
    ///
    /// Fail if no valid-looking git repository could be found.
    pub fn discover(directory: &Path) -> Result<(crate::repository::Path, gix_sec::Trust)> {
        discover_opts(directory, Default::default())
    }

    #[cfg(test)]
    mod tests {
        use super::{Options, Trust, TrustPolicy, discover_opts_with_trust};

        #[test]
        fn ownership_checks_skip_identical_candidate_and_git_directory() -> gix_testtools::TestResult {
            let root = gix_testtools::scripted_fixture_read_only("make_ownership_repos.sh")?;
            for (name, bare) in [("worktree", false), ("bare", true)] {
                let repository = root.join(name);
                let git_dir = if bare {
                    repository.clone()
                } else {
                    repository.join(".git")
                };
                let mut expected_paths = vec![git_dir];
                if !bare {
                    expected_paths.push(repository.clone());
                }
                let mut checked_paths = Vec::new();
                let (_, trust) = discover_opts_with_trust(&repository, Options::default(), |path| {
                    checked_paths.push(path.to_path_buf());
                    Ok(Trust::Reduced)
                })?;
                assert_eq!(
                    checked_paths, expected_paths,
                    "{name} discovery checks each distinct repository path exactly once"
                );
                assert_eq!(
                    trust,
                    Trust::Reduced,
                    "skipping a duplicate must preserve ownership-derived trust"
                );
            }
            Ok(())
        }

        #[test]
        fn ownership_of_every_linked_repository_path_limits_trust() -> gix_testtools::TestResult {
            let root = gix_testtools::scripted_fixture_read_only("make_linked_ownership_repo.sh")?;
            let main = root.join("main");
            let worktree = root.join("linked");

            for foreign_path in [
                worktree.clone(),
                worktree.join(".git"),
                main.join(".git/worktrees/linked"),
                main.join(".git"),
            ] {
                let foreign_path = foreign_path.canonicalize()?;
                let mut ownership = |path: &std::path::Path| {
                    Ok(if path.canonicalize()? == foreign_path {
                        Trust::Reduced
                    } else {
                        Trust::Full
                    })
                };
                let (_, trust) = discover_opts_with_trust(&worktree, Options::default(), &mut ownership)?;
                assert_eq!(
                    trust,
                    Trust::Reduced,
                    "untrusted {} must constrain discovery",
                    foreign_path.display()
                );
                assert!(
                    discover_opts_with_trust(
                        &worktree,
                        Options {
                            trust: TrustPolicy::Required(Trust::Full),
                            ..Options::default()
                        },
                        &mut ownership,
                    )
                    .is_err(),
                    "requiring full trust rejects every untrusted link in the repository paths"
                );
            }
            let (_, trust) = discover_opts_with_trust(
                &worktree,
                Options {
                    trust: TrustPolicy::Assume(Trust::Full),
                    ..Options::default()
                },
                |_| panic!("explicit trust overrides must not inspect ownership"),
            )?;
            assert_eq!(trust, Trust::Full, "explicit trust overrides remain authoritative");
            Ok(())
        }
    }
}
