use std::{
    borrow::Cow,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use gix::{objs::bstr::ByteSlice, progress, NestedProgress, Progress};

#[derive(Default, Copy, Clone, Eq, PartialEq)]
pub enum Mode {
    Execute,
    #[default]
    Simulate,
}

pub fn find_git_repository_workdirs(
    root: impl AsRef<Path>,
    mut progress: impl Progress,
    debug: bool,
    threads: Option<usize>,
) -> impl Iterator<Item = (PathBuf, gix::repository::Kind)> {
    progress.init(None, progress::count("filesystem items"));

    #[derive(Debug, Clone, Copy)]
    struct RepoInfo {
        kind: gix::repository::Kind,
        is_bare: bool,
    }

    fn is_repository(path: &Path) -> Option<RepoInfo> {
        // Can be git dir or worktree checkout (file)
        if path.file_name() != Some(OsStr::new(".git")) && path.extension() != Some(OsStr::new("git")) {
            return None;
        }

        if path.is_dir() {
            if path.join("HEAD").is_file() && path.join("config").is_file() {
                gix::discover::is_git(path).ok().map(|discovered_kind| {
                    let is_bare = discovered_kind.is_bare();
                    let kind = match discovered_kind {
                        gix::discover::repository::Kind::PossiblyBare => gix::repository::Kind::Common,
                        gix::discover::repository::Kind::WorkTree { linked_git_dir: None } => {
                            gix::repository::Kind::Common
                        }
                        gix::discover::repository::Kind::WorkTree {
                            linked_git_dir: Some(_),
                        } => gix::repository::Kind::LinkedWorkTree,
                        gix::discover::repository::Kind::WorkTreeGitDir { .. } => gix::repository::Kind::LinkedWorkTree,
                        gix::discover::repository::Kind::Submodule { .. } => gix::repository::Kind::Submodule,
                        gix::discover::repository::Kind::SubmoduleGitDir => gix::repository::Kind::Submodule,
                    };
                    RepoInfo { kind, is_bare }
                })
            } else {
                None
            }
        } else {
            // git files are always linked worktrees
            Some(RepoInfo {
                kind: gix::repository::Kind::LinkedWorkTree,
                is_bare: false,
            })
        }
    }
    fn into_workdir(git_dir: PathBuf, info: &RepoInfo) -> PathBuf {
        if info.is_bare {
            git_dir
        } else {
            git_dir.parent().expect("git is never in the root").to_owned()
        }
    }

    #[derive(Debug, Default)]
    struct State {
        info: Option<RepoInfo>,
    }

    let walk = jwalk::WalkDirGeneric::<((), State)>::new(root)
        .follow_links(false)
        .sort(true)
        .skip_hidden(false)
        .parallelism(jwalk::Parallelism::RayonNewPool(threads.unwrap_or(0)));

    walk.process_read_dir(move |_depth, path, _read_dir_state, siblings| {
        if debug {
            eprintln!("{}", path.display());
        }
        let mut found_any_repo = false;
        let mut found_bare_repo = false;
        for entry in siblings.iter_mut().flatten() {
            let path = entry.path();
            if let Some(info) = is_repository(&path) {
                let is_bare = info.is_bare;
                entry.client_state = State { info: info.into() };
                entry.read_children_path = None;

                found_any_repo = true;
                found_bare_repo = is_bare;
            }
        }
        // Only return paths which are repositories are further participating in the traversal
        // Don't let bare repositories cause siblings to be pruned.
        if found_any_repo && !found_bare_repo {
            siblings.retain(|e| e.as_ref().map(|e| e.client_state.info.is_some()).unwrap_or(false));
        }
    })
    .into_iter()
    .inspect(move |_| progress.inc())
    .filter_map(Result::ok)
    .filter_map(|mut e| {
        e.client_state
            .info
            .take()
            .map(|info| (into_workdir(e.path(), &info), info.kind))
    })
}

fn find_origin_remote(repo: &Path) -> anyhow::Result<Option<gix_url::Url>> {
    let non_bare = repo.join(".git").join("config");
    let local = gix::config::Source::Local;
    let config = gix::config::File::from_path_no_includes(non_bare.as_path().into(), local)
        .or_else(|_| gix::config::File::from_path_no_includes(repo.join("config"), local))?;
    Ok(config
        .string("remote.origin.url")
        .map(|url| gix_url::Url::from_bytes(url.as_ref()))
        .transpose()?)
}

fn handle(
    mode: Mode,
    kind: gix::repository::Kind,
    git_workdir: &Path,
    canonicalized_destination: &Path,
    progress: &mut impl Progress,
) -> anyhow::Result<()> {
    // Skip linked worktrees - we only handle Common and Submodule kinds
    if matches!(kind, gix::repository::Kind::LinkedWorkTree) {
        return Ok(());
    }
    fn to_relative(path: PathBuf) -> PathBuf {
        path.components()
            .skip_while(|c| c == &std::path::Component::RootDir)
            .collect()
    }

    fn find_parent_repo(mut git_workdir: &Path) -> Option<PathBuf> {
        while let Some(parent) = git_workdir.parent() {
            let has_contained_git_folder_or_file = std::fs::read_dir(parent).ok()?.any(|e| {
                e.ok()
                    .and_then(|e| {
                        e.file_name()
                            .to_str()
                            .map(|name| name == ".git" && e.path() != git_workdir)
                    })
                    .unwrap_or(false)
            });
            if has_contained_git_folder_or_file {
                return Some(parent.to_owned());
            }
            git_workdir = parent;
        }
        None
    }

    if let Some(parent_repo_path) = find_parent_repo(git_workdir) {
        progress.fail(format!(
            "Skipping repository at '{}' as it is nested within repository '{}'",
            git_workdir.display(),
            parent_repo_path.display()
        ));
        return Ok(());
    }

    let url = match find_origin_remote(git_workdir)? {
        None => {
            progress.info(format!(
                "Skipping repository {:?} without 'origin' remote",
                git_workdir.display()
            ));
            return Ok(());
        }
        Some(url) => url,
    };
    if url.path.is_empty() {
        progress.info(format!(
            "Skipping repository at '{}' whose remote does not have a path: {}",
            git_workdir.display(),
            url.to_bstring()
        ));
        return Ok(());
    }

    let destination = canonicalized_destination
        .join(match url.host() {
            Some(h) => h,
            None => return Ok(()),
        })
        .join(to_relative({
            let mut path = gix_url::expand_path(None, url.path.as_bstr())?;
            match kind {
                gix::repository::Kind::Submodule => {
                    unreachable!("BUG: We should not try to relocate submodules and not find them the first place")
                }
                gix::repository::Kind::LinkedWorkTree => {
                    unreachable!("BUG: LinkedWorkTree should have been skipped earlier")
                }
                gix::repository::Kind::Common => {
                    // For Common kind, check if it's bare
                    let git_dir = if git_workdir.join(".git").is_dir() {
                        git_workdir.join(".git")
                    } else {
                        git_workdir.to_owned()
                    };
                    if !gix::discover::is_bare(&git_dir) {
                        // Non-bare repository - strip .git extension if present
                        if let Some(ext) = path.extension() {
                            if ext == "git" {
                                path.set_extension("");
                            }
                        }
                    }
                    path
                }
            }
        }));

    if let Ok(destination) = destination.canonicalize() {
        if git_workdir.canonicalize()? == destination {
            return Ok(());
        }
    }
    match mode {
        Mode::Simulate => progress.info(format!(
            "WOULD move {} to {}",
            git_workdir.display(),
            destination.display()
        )),
        Mode::Execute => {
            if destination.starts_with(
                git_workdir
                    .canonicalize()
                    .ok()
                    .map(Cow::Owned)
                    .unwrap_or(Cow::Borrowed(git_workdir)),
            ) {
                let tempdir = tempfile::tempdir_in(canonicalized_destination)?;
                let tempdest = tempdir
                    .path()
                    .join(destination.file_name().expect("repo destination is not the root"));
                std::fs::rename(git_workdir, &tempdest)?;
                std::fs::create_dir_all(destination.parent().expect("repo destination is not the root"))?;
                std::fs::rename(&tempdest, &destination)?;
            } else {
                std::fs::create_dir_all(destination.parent().expect("repo destination is not the root"))?;
                std::fs::rename(git_workdir, &destination)?;
            }
            progress.done(format!("Moving {} to {}", git_workdir.display(), destination.display()));
        }
    }
    Ok(())
}

/// Find all working directories in the given `source_dir` and print them to `out` while providing `progress`.
pub fn discover<P: NestedProgress>(
    source_dir: impl AsRef<Path>,
    mut out: impl std::io::Write,
    mut progress: P,
    debug: bool,
    threads: Option<usize>,
) -> anyhow::Result<()> {
    for (git_workdir, _kind) in
        find_git_repository_workdirs(source_dir, progress.add_child("Searching repositories"), debug, threads)
    {
        writeln!(&mut out, "{}", git_workdir.display())?;
    }
    Ok(())
}

pub fn run<P: NestedProgress>(
    mode: Mode,
    source_dir: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    mut progress: P,
    threads: Option<usize>,
) -> anyhow::Result<()> {
    let mut num_errors = 0usize;
    let destination = destination.as_ref().canonicalize()?;
    for (path_to_move, kind) in
        find_git_repository_workdirs(source_dir, progress.add_child("Searching repositories"), false, threads)
    {
        if let Err(err) = handle(mode, kind, &path_to_move, &destination, &mut progress) {
            progress.fail(format!(
                "Error when handling directory {:?}: {}",
                path_to_move.display(),
                err
            ));
            num_errors += 1;
        }
    }

    if num_errors > 0 {
        anyhow::bail!("Failed to handle {num_errors} repositories")
    } else {
        Ok(())
    }
}
