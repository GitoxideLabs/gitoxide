use std::{collections::BTreeMap, path::PathBuf};

use crate::bstr::{BStr, ByteSlice};
use crate::{Worktree, worktree};
#[cfg(feature = "worktree-archive")]
use gix_error::ResultExt;

#[derive(Debug, thiserror::Error)]
pub(crate) enum CheckedOutBranchesError {
    #[error("Failed to read worktree directories or operation state")]
    WorktreeListing(#[from] std::io::Error),
    #[error("Could not open a worktree repository")]
    OpenWorktreeRepo(#[from] crate::open::Error),
    #[error("Failed to follow a symbolic reference")]
    FollowSymref(#[from] gix_ref::file::find::existing::Error),
}

/// Interact with individual worktrees and their information.
impl crate::Repository {
    /// Return all references checked out or reserved by bisect or rebase in the main and linked worktrees.
    ///
    /// Each key maps to the worktree directories in which it is checked out. Besides branch names,
    /// `HEAD` is recorded for every worktree with a readable head, and all symbolic references in
    /// the chain from `HEAD` to its referent are included. Bisect and rebase also reserve their original
    /// branch while `HEAD` is detached. Bare repositories and worktrees whose head cannot be read are ignored.
    /// `namespace` selects the reference namespace for all inspected worktrees; `None` uses the default namespace.
    pub(crate) fn checked_out_branches(
        &self,
        namespace: Option<&gix_ref::Namespace>,
    ) -> Result<BTreeMap<gix_ref::FullName, Vec<PathBuf>>, CheckedOutBranchesError> {
        let mut map = BTreeMap::new();
        for repo in self.worktrees_including_main()? {
            let mut repo = repo?;
            repo.refs.namespace = namespace.cloned();
            insert_head(repo.head().ok(), &mut map)?;
        }
        Ok(map)
    }

    /// Return a list of all **linked** worktrees sorted by private git dir path as a lightweight proxy.
    ///
    /// This means the number is `0` even if there is the main worktree, as it is not counted as linked worktree.
    /// This also means it will be `1` if there is one linked worktree next to the main worktree.
    /// It's worth noting that a *bare* repository may have one or more linked worktrees, but has no *main* worktree,
    /// which is the reason why the *possibly* available main worktree isn't listed here.
    ///
    /// Use [`worktrees_including_main()`][Self::worktrees_including_main()] to open the main and linked repositories.
    ///
    /// Note that these need additional processing to become usable, but provide a first glimpse a typical worktree information.
    pub fn worktrees(&self) -> std::io::Result<Vec<worktree::Proxy<'_>>> {
        let mut res = Vec::new();
        let iter = match std::fs::read_dir(self.common_dir().join("worktrees")) {
            Ok(iter) => iter,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(res),
            Err(err) => return Err(err),
        };
        for entry in iter {
            let entry = entry?;
            let worktree_git_dir = entry.path();
            res.extend(worktree::Proxy::new_if_gitdir_file_exists(self, worktree_git_dir));
        }
        res.sort_by(|a, b| a.git_dir.cmp(&b.git_dir));
        Ok(res)
    }

    /// Return an iterator over the main repository, then each linked worktree repository in private Git directory order.
    ///
    /// Each repository is yielded once, regardless of which worktree this method is called from. The main repository
    /// is included even if it is bare, matching `git worktree list`.
    ///
    /// Linked worktrees are listed immediately and opened during iteration. Opening errors are yielded
    /// per entry. Missing or inaccessible checkout directories are allowed, as with
    /// [`Proxy::into_repo_with_possibly_inaccessible_worktree()`][worktree::Proxy::into_repo_with_possibly_inaccessible_worktree()].
    pub fn worktrees_including_main(
        &self,
    ) -> std::io::Result<impl Iterator<Item = Result<crate::Repository, crate::open::Error>> + '_> {
        Ok(std::iter::once_with(|| self.main_repo()).chain(
            self.worktrees()?
                .into_iter()
                .map(worktree::Proxy::into_repo_with_possibly_inaccessible_worktree),
        ))
    }

    /// Return the worktree that [is identified](Worktree::id) by the given `id`, if it exists at
    /// `.git/worktrees/<id>` and its `gitdir` file exists.
    /// Return `None` otherwise.
    pub fn worktree_proxy_by_id<'a>(&self, id: impl Into<&'a BStr>) -> Option<worktree::Proxy<'_>> {
        worktree::Proxy::new_if_gitdir_file_exists(
            self,
            self.common_dir().join("worktrees").join(gix_path::from_bstr(id.into())),
        )
    }

    /// Return the repository owning the main worktree, typically from a linked worktree.
    ///
    /// If this repository isn't a linked worktree, return a clone preserving its known worktree directory.
    /// The main repository may be bare.
    #[expect(
        clippy::result_large_err,
        reason = "will be removed once `gix-error` is used consistently"
    )]
    pub fn main_repo(&self) -> Result<crate::Repository, crate::open::Error> {
        if self.kind() != crate::repository::Kind::LinkedWorkTree {
            return Ok(self.clone());
        }
        let options = self.options.clone().without_repository_environment_overrides();
        crate::ThreadSafeRepository::open_opts(self.common_dir(), options).map(Into::into)
    }

    /// Return the currently set worktree if there is one, acting as platform providing a validated worktree base path.
    ///
    /// Note that this would be `None` if this repository is `bare` and the parent [`Repository`](crate::Repository)
    /// was instantiated without registered worktree in the current working dir, even if no `.git` file or directory exists.
    /// It's merely based on configuration, see [Worktree::dot_git_exists()] for a way to perform more validation.
    pub fn worktree(&self) -> Option<Worktree<'_>> {
        self.workdir().map(|path| Worktree { parent: self, path })
    }

    /// Return true if this repository is bare, or in absence of a known configuration value, if it has no work tree.
    ///
    /// This is not to be confused with the [`worktree()`](crate::Repository::worktree()) method, which may exist if this instance
    /// was opened in a worktree that was created separately.
    pub fn is_bare(&self) -> bool {
        self.config.is_bare.unwrap_or_else(|| self.workdir().is_none())
    }

    /// If `id` points to a tree, produce a stream that yields one worktree entry after the other. The index of the tree at `id`
    /// is returned as well as it is an intermediate byproduct that might be useful to callers.
    ///
    /// The entries will look exactly like they would if one would check them out, with filters applied.
    /// The `export-ignore` attribute is used to skip blobs or directories to which it applies.
    #[cfg(feature = "worktree-stream")]
    pub fn worktree_stream(
        &self,
        id: impl Into<gix_hash::ObjectId>,
    ) -> Result<(gix_worktree_stream::Stream, gix_index::File), crate::repository::worktree_stream::Error> {
        use gix_odb::HeaderExt;
        let id = id.into();
        let header = self.objects.header(id)?;
        if !header.kind().is_tree() {
            return Err(crate::repository::worktree_stream::Error::NotATree {
                id,
                actual: header.kind(),
            });
        }

        // TODO(perf): potential performance improvements could be to use the index at `HEAD` if possible (`index_from_head_tree…()`)
        // TODO(perf): when loading a non-HEAD tree, we effectively traverse the tree twice. This is usually fast though, and sharing
        //             an object cache between the copies of the ODB handles isn't trivial and needs a lock.
        let index = self.index_from_tree(&id)?;
        let mut cache = self
            .attributes_only(&index, gix_worktree::stack::state::attributes::Source::IdMapping)?
            .detach();
        let pipeline = gix_filter::Pipeline::new(self.command_context()?, crate::filter::Pipeline::options(self)?);
        let objects = self.objects.clone().into_arc().expect("TBD error handling");
        let stream = gix_worktree_stream::from_tree(
            id,
            objects.clone(),
            pipeline,
            move |path, mode, attrs| -> std::io::Result<()> {
                let entry = cache.at_entry(path, Some(mode.into()), &objects)?;
                entry.matching_attributes(attrs);
                Ok(())
            },
        );
        Ok((stream, index))
    }

    /// Produce an archive from the `stream` and write it to `out` according to `options`.
    /// Use `blob` to provide progress for each entry written to `out`, and note that it should already be initialized to the amount
    /// of expected entries, with `should_interrupt` being queried between each entry to abort if needed, and on each write to `out`.
    ///
    /// ### Performance
    ///
    /// Be sure that `out` is able to handle a lot of write calls. Otherwise wrap it in a [`BufWriter`][std::io::BufWriter].
    ///
    /// ### Additional progress and fine-grained interrupt handling
    ///
    /// For additional progress reporting, wrap `out` into a writer that counts throughput on each write.
    /// This can also be used to react to interrupts on each write, instead of only for each entry.
    #[cfg(feature = "worktree-archive")]
    pub fn worktree_archive(
        &self,
        mut stream: gix_worktree_stream::Stream,
        out: impl std::io::Write + std::io::Seek,
        blobs: impl gix_features::progress::Count,
        should_interrupt: &std::sync::atomic::AtomicBool,
        options: gix_archive::Options,
    ) -> Result<(), crate::repository::worktree_archive::Error> {
        let mut out = gix_features::interrupt::Write {
            inner: out,
            should_interrupt,
        };
        if options.format == gix_archive::Format::InternalTransientNonPersistable {
            std::io::copy(&mut stream.into_read(), &mut out)
                .or_raise(|| gix_error::message("Could not copy stream"))?;
            return Ok(());
        }
        gix_archive::write_stream_seek(
            &mut stream,
            |stream| {
                if should_interrupt.load(std::sync::atomic::Ordering::Relaxed) {
                    return Err(gix_error::ErrorExt::raise_erased(gix_error::message(
                        "Cancelled by user",
                    )));
                }
                let res = stream.next_entry().or_erased();
                blobs.inc();
                res
            },
            out,
            options,
        )?;
        Ok(())
    }
}

/// Record the worktree directory under `HEAD`, its symbolic referents, and branches reserved by bisect or rebase.
///
/// Do nothing if `head` is absent or its repository has no worktree, and fail if a symbolic
/// reference in the chain cannot be followed or an operation's branch file cannot be read.
fn insert_head(
    head: Option<crate::Head<'_>>,
    out: &mut BTreeMap<gix_ref::FullName, Vec<PathBuf>>,
) -> Result<(), CheckedOutBranchesError> {
    let Some((head, workdir)) = head.and_then(|head| head.repo.workdir().map(|workdir| (head, workdir))) else {
        return Ok(());
    };
    out.entry("HEAD".try_into().expect("valid reference name"))
        .or_default()
        .push(workdir.to_owned());
    let repo = head.repo;
    let mut cursor = head.try_into_referent();
    while let Some(reference) = cursor {
        out.entry(reference.name().to_owned())
            .or_default()
            .push(workdir.to_owned());
        cursor = reference.follow().transpose()?;
    }

    let git_dir = repo.git_dir();
    let rebase = if git_dir.join("rebase-apply").is_dir() {
        // `git am` uses the same directory but doesn't reserve a branch for rebase.
        (!git_dir.join("rebase-apply/applying").is_file()).then_some("rebase-apply/head-name")
    } else {
        git_dir
            .join("rebase-merge")
            .is_dir()
            .then_some("rebase-merge/head-name")
    };
    let bisect = git_dir.join("BISECT_LOG").is_file().then_some("BISECT_START");
    for path in rebase.into_iter().chain(bisect) {
        let contents = match std::fs::read(git_dir.join(path)) {
            Ok(contents) => contents,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(err.into()),
        };
        let name = contents.trim_end_with(|c| c == '\n');
        let name = if name.starts_with(b"refs/heads/") {
            name.to_owned()
        } else {
            // An operation started from detached HEAD doesn't reserve a branch.
            if name.is_empty() || name == b"detached HEAD" || gix_hash::ObjectId::from_hex(name).is_ok() {
                continue;
            }
            [b"refs/heads/".as_slice(), name].concat()
        };
        let Ok(name) = gix_ref::FullName::try_from(name.as_bstr()) else {
            continue;
        };
        let workdirs = out.entry(name).or_default();
        if !workdirs.iter().any(|path| path == workdir) {
            workdirs.push(workdir.to_owned());
        }
    }
    Ok(())
}
