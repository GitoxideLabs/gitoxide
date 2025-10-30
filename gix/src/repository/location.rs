use std::borrow::Cow;
use std::path::{Path, PathBuf};

use gix_path::realpath::MAX_SYMLINKS;

use crate::bstr::BStr;

impl crate::Repository {
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

    /// Return the path to the worktree index file, which may or may not exist.
    pub fn index_path(&self) -> PathBuf {
        self.git_dir().join("index")
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
    pub fn set_workdir(&mut self, workdir: impl Into<Option<PathBuf>>) -> Result<Option<PathBuf>, std::io::Error> {
        let workdir = workdir.into();
        Ok(match workdir {
            None => self.work_tree.take(),
            Some(new_workdir) => {
                _ = std::fs::read_dir(&new_workdir)?;

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

    /// Turn `rela_path` into a path qualified with the [`workdir()`](Self::workdir()) of this instance,
    /// if one is available.
    pub fn workdir_path(&self, rela_path: impl AsRef<BStr>) -> Option<PathBuf> {
        self.workdir().and_then(|wd| {
            gix_path::try_from_bstr(rela_path.as_ref())
                .ok()
                .map(|rela| wd.join(rela))
        })
    }

    // TODO: tests, respect precomposeUnicode
    /// The directory of the binary path of the current process.
    pub fn install_dir(&self) -> std::io::Result<PathBuf> {
        crate::path::install_dir()
    }

    /// Returns the relative path which is the components between the working tree and the current working dir (CWD).
    /// Note that it may be `None` if there is no work tree, or if CWD isn't inside of the working tree directory.
    ///
    /// Note that the CWD is obtained once upon instantiation of the repository.
    // TODO: tests, details - there is a lot about environment variables to change things around.
    pub fn prefix(&self) -> Result<Option<&Path>, gix_path::realpath::Error> {
        let (root, current_dir) = match self.workdir().zip(self.options.current_dir.as_deref()) {
            Some((work_dir, cwd)) => (work_dir, cwd),
            None => return Ok(None),
        };

        let root = gix_path::realpath_opts(root, current_dir, MAX_SYMLINKS)?;
        Ok(current_dir.strip_prefix(&root).ok())
    }

    /// Return the kind of repository, either bare or one with a work tree.
    pub fn kind(&self) -> crate::repository::Kind {
        match self.worktree() {
            Some(wt) => {
                if gix_discover::is_submodule_git_dir(self.git_dir()) {
                    crate::repository::Kind::Submodule
                } else {
                    crate::repository::Kind::WorkTree {
                        is_linked: !wt.is_main(),
                    }
                }
            }
            None => crate::repository::Kind::Bare,
        }
    }

    /// Returns `Some(true)` if the reference database [is untouched](gix_ref::file::Store::is_pristine()).
    /// This typically indicates that the repository is new and empty.
    /// Return `None` if a defect in the database makes the answer uncertain.
    #[doc(alias = "is_empty", alias = "git2")]
    pub fn is_pristine(&self) -> Option<bool> {
        let name = self
            .config
            .resolved
            .string(crate::config::tree::Init::DEFAULT_BRANCH)
            .unwrap_or(Cow::Borrowed("master".into()));
        let default_branch_ref_name: gix_ref::FullName = format!("refs/heads/{name}")
            .try_into()
            .unwrap_or_else(|_| gix_ref::FullName::try_from("refs/heads/master").expect("known to be valid"));
        self.refs.is_pristine(default_branch_ref_name.as_ref())
    }
}
