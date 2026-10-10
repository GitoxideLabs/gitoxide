use crate::error::{ResultExt, bail, message};
use gix_error::validation;
use gix_path::realpath::MAX_SYMLINKS;
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use crate::{Result, bstr::BStr};

impl crate::Repository {
    /// Make the Git directory, common directory, worktree, and object database path absolute,
    /// using [`current_dir()`][Self::current_dir()] as captured when this repository was opened.
    /// Use it if the repository may have been opened with a relative directory, while it's handle
    /// should be re-used after a CWD change.
    ///
    /// Paths are joined lexically, without resolving symlinks or removing `..` components. Missing worktrees
    /// are allowed. Already-absolute paths and the captured current directory remain unchanged.
    /// The selected index path is already absolute from opening, so its cached snapshots are retained.
    /// This does not rewrite paths in configuration, repository files, or previously returned handles and snapshots.
    ///
    /// Unlike [`reload()`][Self::reload()], this preserves in-memory configuration, the reference namespace,
    /// and in-memory objects. Stores with relative paths are reopened, and their caches are reset; previously
    /// obtained pack IDs and locations must not be reused, and mapped packs deleted from disk are not retained.
    /// Other repository clones are unchanged. If all paths are absolute, this is a no-op.
    ///
    /// Return this repository for chaining, or an error if its relative object database cannot be reopened
    /// or a path cannot be anchored to the captured directory, such as a drive-relative path on Windows.
    /// No changes are made on error.
    pub fn make_paths_absolute(&mut self) -> Result<&mut Self> {
        if self.has_absolute_paths() {
            return Ok(self);
        }
        let current_dir = self.current_dir();
        let anchor = |path: &Path| -> Result<PathBuf> {
            let absolute = current_dir.join(path);
            if !absolute.is_absolute() {
                bail!(validation(format!(
                    "Cannot make path '{}' absolute using captured directory '{}'",
                    path.display(),
                    current_dir.display()
                )));
            }
            Ok(absolute)
        };
        let git_dir = anchor(self.git_dir())?;
        let common_dir = self.common_dir.as_deref().map(anchor).transpose()?;
        let work_tree = self.workdir().map(anchor).transpose()?;

        let refs = if self.refs.git_dir().is_relative() || self.refs.common_dir().is_some_and(Path::is_relative) {
            let options = gix_ref::store::init::Options {
                write_reflog: self.refs.write_reflog,
                precompose_unicode: self.refs.precompose_unicode,
                prohibit_windows_device_names: self.refs.prohibit_windows_device_names,
            };
            let mut refs = match self.refs.common_dir() {
                Some(common_dir) => {
                    crate::RefStore::for_linked_worktree_opts(git_dir, anchor(common_dir)?, self.object_hash(), options)
                }
                None => crate::RefStore::at_opts(git_dir, self.object_hash(), options),
            };
            refs.namespace.clone_from(&self.refs.namespace);
            let threshold = self.refs.clone().set_packed_buffer_mmap_threshold(0);
            refs.set_packed_buffer_mmap_threshold(threshold);
            Some(refs)
        } else {
            None
        };
        let store = self.objects.store_ref();
        let objects = if store.path().is_relative() {
            let mut handle = gix_odb::at_opts(
                anchor(store.path())?,
                store.object_hash(),
                store.replacements(),
                gix_odb::store::init::Options {
                    slots: self.options.object_store_slots,
                    use_multi_pack_index: store.use_multi_pack_index(),
                    alloc_limit_bytes: self.config.alloc_limit_bytes,
                    current_dir: Some(current_dir.to_owned()),
                    loose_compression: self.objects.loose_compression,
                },
            )
            .or_raise(|| message("Could not reopen the object database with an absolute path"))?
            .into_inner();
            handle.refresh = self.objects.refresh;
            handle.max_recursion_depth = self.objects.max_recursion_depth;
            handle.ignore_replacements = self.objects.ignore_replacements;
            // Cloning resets pack caches while retaining their constructors and the enclosing object memory.
            let mut objects = self.objects.clone();
            **objects = handle;
            Some(objects)
        } else {
            None
        };

        if let Some(refs) = refs {
            self.refs = refs;
        }
        if let Some(objects) = objects {
            self.objects = objects;
        }

        self.common_dir = common_dir;
        self.work_tree = work_tree;
        Ok(self)
    }

    #[cfg(feature = "worktree-mutation")]
    pub(crate) fn with_absolute_paths(&self) -> Result<Cow<'_, Self>> {
        if self.has_absolute_paths() {
            return Ok(Cow::Borrowed(self));
        }
        let mut repo = self.clone();
        repo.make_paths_absolute()?;
        Ok(Cow::Owned(repo))
    }

    fn has_absolute_paths(&self) -> bool {
        self.git_dir().is_absolute()
            && self.common_dir().is_absolute()
            && self.refs.common_dir().is_none_or(Path::is_absolute)
            && self.workdir().is_none_or(Path::is_absolute)
            && self.index_path.is_absolute()
            && self.objects.store_ref().path().is_absolute()
    }

    /// Return the path to the repository itself, containing objects, references, configuration, and more.
    ///
    /// Synonymous to [`path()`][crate::Repository::path()].
    pub fn git_dir(&self) -> &std::path::Path {
        self.refs.git_dir()
    }

    /// The trust we place in the git-dir, with lower amounts of trust causing access to configuration to be limited.
    /// Note that if the git-dir is trusted but the worktree is not, the result is that the git-dir is also less trusted.
    pub fn git_dir_trust(&self) -> gix_sec::Trust {
        self.options.git_dir_trust.expect("definitely set by now")
    }

    /// Return the current working directory as present during the instantiation of this repository.
    ///
    /// Note that this should be preferred over manually obtaining it as this may have been adjusted to
    /// deal with `core.precomposeUnicode`.
    pub fn current_dir(&self) -> &Path {
        self.options
            .current_dir
            .as_deref()
            .expect("BUG: cwd is always set after instantiation")
    }

    /// Returns the main git repository if this is a repository on a linked work-tree, or the `git_dir` itself.
    pub fn common_dir(&self) -> &std::path::Path {
        self.common_dir.as_deref().unwrap_or_else(|| self.git_dir())
    }

    /// Return the absolute path to the worktree index file, which may or may not exist.
    ///
    /// It may have been overridden with
    /// [gitoxide.core.indexFile][crate::config::tree::gitoxide::Core::INDEX_FILE]. Relative configured paths use the
    /// private Git directory; relative `GIT_INDEX_FILE` overrides use [`current_dir()`][Self::current_dir()].
    /// The path is anchored lexically when opening, without canonicalization, and only re-evaluated by
    /// [`reload()`](Self::reload()). Configuration text remains unchanged.
    pub fn index_path(&self) -> PathBuf {
        self.index_path.clone()
    }

    /// The path to the `.gitmodules` file in the worktree, if a worktree is available.
    #[cfg(feature = "attributes")]
    pub fn modules_path(&self) -> Option<PathBuf> {
        self.workdir().map(|wtd| wtd.join(crate::submodule::MODULES_FILE))
    }

    /// The path to the `.git` directory itself, or equivalent if this is a bare repository.
    pub fn path(&self) -> &std::path::Path {
        self.git_dir()
    }

    /// Return the work tree containing all checked out files, if there is one.
    #[deprecated = "Use `workdir()` instead"]
    #[doc(alias = "workdir", alias = "git2")]
    pub fn work_dir(&self) -> Option<&std::path::Path> {
        self.work_tree.as_deref()
    }

    /// Forcefully set the given `workdir` to be the worktree of this repository, *in memory*,
    /// no matter if it had one or not, or unset it with `None`.
    /// Return the previous working directory if one existed.
    ///
    /// Fail if the `workdir`, if not `None`, isn't accessible or isn't a directory.
    /// No change is performed on error.
    ///
    /// ### About Worktrees
    ///
    /// * When setting a main worktree to a linked worktree directory, this repository instance
    ///   will still claim that it is the [main worktree](crate::Worktree::is_main()) as that depends
    ///   on the `git_dir`, not the worktree dir.
    /// * When setting a linked worktree to a main worktree directory, this repository instance
    ///   will still claim that it is *not* a [main worktree](crate::Worktree::is_main()) as that depends
    ///   on the `git_dir`, not the worktree dir.
    #[doc(alias = "git2")]
    pub fn set_workdir(&mut self, workdir: impl Into<Option<PathBuf>>) -> Result<Option<PathBuf>> {
        let workdir = workdir.into();
        Ok(match workdir {
            None => self.work_tree.take(),
            Some(new_workdir) => {
                _ = std::fs::read_dir(&new_workdir).or_error()?;

                let old = self.work_tree.take();
                self.work_tree = Some(new_workdir);
                old
            }
        })
    }

    /// Return the work tree containing all checked out files, if there is one.
    pub fn workdir(&self) -> Option<&std::path::Path> {
        self.work_tree.as_deref()
    }

    /// Turn the repository-relative Git path `rela_path` into a filesystem path qualified with the
    /// [`workdir()`](Self::workdir()) of this instance, if one is available.
    ///
    /// This is useful for accessing a path obtained from repository data, such as an index or tree entry, in the
    /// worktree. It performs no normalization or containment checks. Use [`normalize_path()`](Self::normalize_path)
    /// first for paths supplied relative to the current working directory, absolute paths, or paths with `.` or `..`
    /// components.
    pub fn workdir_path(&self, rela_path: impl AsRef<BStr>) -> Option<PathBuf> {
        self.workdir()
            .and_then(|wd| gix_path::from_bstr(rela_path.as_ref()).ok().map(|rela| wd.join(rela)))
    }

    /// Normalize `path` into a repository-relative Git path with slash separators.
    ///
    /// This is useful for turning user-supplied filesystem paths into paths suitable for querying repository data.
    /// To access the corresponding file in a non-bare repository, pass the result to
    /// [`workdir_path()`](Self::workdir_path).
    ///
    /// Relative paths are interpreted from [`current_dir()`](Self::current_dir), as captured when this repository was
    /// instantiated, if it is inside the worktree. Otherwise they are considered repository-relative.
    /// An empty path or `.` refers to that directory, and the result is empty if that directory is the repository root.
    ///
    /// Absolute paths must be within the worktree, or within the Git directory for bare repositories.
    /// Paths which traverse outside of the repository are rejected. Note that passing absolute paths is expensive
    /// as their realpath has to be determined.
    pub fn normalize_path<'a>(&self, path: &'a (impl gix_utils::AsBStr + ?Sized)) -> Result<Cow<'a, BStr>> {
        let path = gix_path::from_bstr(Cow::Borrowed(path.as_bstr()))?;
        let path = if gix_path::is_absolute(path.as_ref()) {
            let root = gix_path::realpath_opts(
                self.workdir().unwrap_or_else(|| self.git_dir()),
                self.current_dir(),
                MAX_SYMLINKS,
            )?;
            let absolute = path.into_owned();
            let relative = if let Ok(relative) = absolute.strip_prefix(&root) {
                relative.to_owned()
            } else {
                gix_path::realpath_opts(&absolute, self.current_dir(), MAX_SYMLINKS)?
                    .strip_prefix(&root)
                    .or_raise(|| {
                        gix_error::validation(format!(
                            "The absolute path \"{}\" is not inside the repository at \"{}\"",
                            absolute.display(),
                            root.display()
                        ))
                    })?
                    .to_owned()
            };
            Cow::Owned(relative)
        } else if let Some(prefix) = self.prefix()?.filter(|prefix| !prefix.as_os_str().is_empty()) {
            Cow::Owned(prefix.join(path.as_ref()))
        } else {
            path
        };

        let path = match path {
            Cow::Borrowed(path) => gix_path::normalize_and_clean(Cow::Borrowed(path), Path::new(""))
                .ok_or_else(|| message!("The path \"{}\" leaves the repository", path.display()).validation_error())?,
            Cow::Owned(path) => {
                if gix_path::normalize_and_clean(Cow::Borrowed(path.as_path()), Path::new("")).is_none() {
                    bail!("The path \"{}\" leaves the repository".validation(), path.display());
                }
                gix_path::normalize_and_clean(Cow::Owned(path), Path::new(""))
                    .expect("path was just validated as normalizable")
            }
        };
        Ok(gix_path::to_unix_separators_on_windows(gix_path::into_bstr(path)?))
    }

    // TODO: tests, respect precomposeUnicode
    /// The directory of the binary path of the current process.
    pub fn install_dir(&self) -> Result<PathBuf> {
        crate::path::install_dir().or_error()
    }

    /// Returns the relative path which is the components between the working tree and the current working dir (CWD).
    /// Note that it may be `None` if there is no work tree, or if CWD isn't inside of the working tree directory.
    ///
    /// Note that the CWD is obtained once upon instantiation of the repository.
    // TODO: tests, details - there is a lot about environment variables to change things around.
    pub fn prefix(&self) -> Result<Option<&Path>> {
        let (root, current_dir) = match self.workdir().zip(self.options.current_dir.as_deref()) {
            Some((work_dir, cwd)) => (work_dir, cwd),
            None => return Ok(None),
        };

        let root = gix_path::realpath_opts(root, current_dir, MAX_SYMLINKS)?;
        Ok(current_dir.strip_prefix(&root).ok())
    }

    /// Return the kind of repository as measured by its Git directory location.
    pub fn kind(&self) -> crate::repository::Kind {
        use gix_discover::path::RepositoryKind::*;
        match gix_discover::path::repository_kind(self.git_dir()) {
            Some(Submodule) => crate::repository::Kind::Submodule,
            Some(LinkedWorktree) => crate::repository::Kind::LinkedWorkTree,
            // Opening a linked worktree resolves its `commondir` file, making its private Git directory
            // differ from the shared one. This also detects worktrees of natively bare repositories, whose
            // private Git directory isn't below a directory named `.git` and thus isn't recognized by the path
            // heuristic in `repository_kind()`.
            None if self.git_dir() != self.common_dir() => crate::repository::Kind::LinkedWorkTree,
            None | Some(Common) => crate::repository::Kind::Common,
        }
    }

    /// Returns `Some(true)` if the reference database [is untouched](gix_ref::file::Store::is_pristine()).
    /// This typically indicates that the repository is new and empty.
    /// Return `None` if a defect in the database makes the answer uncertain.
    #[doc(alias = "is_empty", alias = "git2")]
    pub fn is_pristine(&self) -> Option<bool> {
        use gix_utils::AsBStr;
        let name = self
            .config
            .resolved
            .string(crate::config::tree::Init::DEFAULT_BRANCH)
            .unwrap_or_else(|| "master".into());
        let default_branch_ref_name = gix_ref::Category::LocalBranch
            .to_full_name(name.as_bstr())
            .unwrap_or_else(|_| gix_ref::FullName::try_from("refs/heads/master").expect("known to be valid"));
        self.refs.is_pristine(default_branch_ref_name.as_ref())
    }
}
