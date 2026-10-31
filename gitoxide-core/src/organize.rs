use std::{
    borrow::Cow,
    collections::HashMap,
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::Arc,
};

use gix::{
    NestedProgress, Progress, Result,
    error::{ErrorExt, Exn, ResultExt, bail, message},
    objs::bstr::ByteSlice,
    progress,
};
use parking_lot::Mutex;

fn walk_threads(requested: Option<usize>) -> usize {
    requested.filter(|threads| *threads != 0).unwrap_or_else(|| {
        if cfg!(target_os = "macos") {
            4
        } else {
            std::thread::available_parallelism().map_or(1, usize::from)
        }
    })
}

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

    fn is_repository(path: &Path, is_dir: bool, is_file: bool) -> Option<RepoInfo> {
        // Can be git dir or worktree checkout (file)
        if path.file_name() != Some(OsStr::new(".git")) && path.extension() != Some(OsStr::new("git")) {
            return None;
        }

        if is_dir {
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
        } else if is_file {
            // git files are always linked worktrees
            Some(RepoInfo {
                kind: gix::repository::Kind::LinkedWorkTree,
                is_bare: false,
            })
        } else {
            None
        }
    }
    fn into_workdir(git_dir: PathBuf, info: &RepoInfo) -> PathBuf {
        if info.is_bare {
            git_dir
        } else {
            git_dir.parent().expect("git is never in the root").to_owned()
        }
    }

    fn repository_at(entry: &dua_core::Entry) -> Option<(PathBuf, RepoInfo)> {
        let path = entry.path();
        if let Some(info) = is_repository(&path, entry.file_type.is_dir(), entry.file_type.is_file()) {
            return Some((path, info));
        }
        let git_dir = path.join(".git");
        let metadata = git_dir.metadata().ok()?;
        is_repository(&git_dir, metadata.is_dir(), metadata.is_file()).map(|info| (git_dir, info))
    }

    let repositories = Arc::new(Mutex::new(HashMap::new()));
    let repositories_in_workers = Arc::clone(&repositories);
    dua_core::walk(
        root.as_ref(),
        walk_threads(threads),
        dua_core::Order::ParentFirst,
        dua_core::Options::default().skip_metadata(),
        move |entry| {
            let Some(repository) = repository_at(entry) else {
                return true;
            };
            let path = entry.path();
            repositories_in_workers.lock().insert(path, repository);
            false
        },
    )
    .inspect(move |entry| {
        if debug && let Ok(entry) = entry {
            eprintln!("{}", entry.path().display());
        }
    })
    .inspect(move |_| progress.inc())
    .filter_map(std::result::Result::ok)
    .filter_map(move |entry| {
        let path = entry.path();
        let repository = if entry.file_type.is_dir() {
            repositories.lock().remove(&path)
        } else {
            is_repository(&path, false, entry.file_type.is_file()).map(|info| (path, info))
        };
        repository.map(|(git_dir, info)| (into_workdir(git_dir, &info), info.kind))
    })
}

fn find_origin_remote(repo: &Path) -> Result<Option<gix_url::Url>> {
    let non_bare = repo.join(".git").join("config");
    let local = gix::config::Source::Local;
    let config = gix::config::File::from_path_no_includes(non_bare.as_path().into(), local)
        .or_else(|_| gix::config::File::from_path_no_includes(repo.join("config"), local))?;
    config
        .string("remote.origin.url")
        .map(|url| gix_url::Url::from_bytes(url.as_ref()))
        .transpose()
}

/// Confinement validation errors record `destination` (the resolved path) and `destination_root`
/// (the canonicalized allowed root) as path metadata.
fn handle(
    mode: Mode,
    kind: gix::repository::Kind,
    git_workdir: &Path,
    canonicalized_destination: &Path,
    progress: &mut impl Progress,
) -> Result<()> {
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
            "Skipping repository at \"{}\" as it is nested within repository \"{}\"",
            git_workdir.display(),
            parent_repo_path.display()
        ));
        return Ok(());
    }

    let url = match find_origin_remote(git_workdir).or_raise(|| message("Failed to read repository origin URL"))? {
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
            "Skipping repository at \"{}\" whose remote does not have a path: {}",
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
            let mut path = gix_url::expand_path(None, url.path.as_bstr())
                .or_raise(|| message("Failed to expand repository origin URL path"))?;
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
                        if path.extension().is_some_and(|ext| ext == "git") {
                            path.set_extension("");
                        }
                    }
                    path
                }
                _ => bail!(message!("cannot relocate repository of unsupported kind {kind:?}")),
            }
        }));

    // Unlike canonicalize(), realpath also resolves existing symlinks when the destination leaf
    // does not exist yet. Check confinement before creating directories or moving the repository.
    let destination_metadata = std::fs::symlink_metadata(&destination);
    let destination = gix::path::realpath(&destination)
        .or_raise(|| message("Failed to resolve repository destination").with("destination", destination.as_path()))?;
    if !destination.starts_with(canonicalized_destination) || destination == canonicalized_destination {
        bail!(
            "the origin URL resolves outside the repository destination"
                .validation()
                .with("destination", destination.as_path())
                .with("destination_root", canonicalized_destination)
        );
    }

    match destination_metadata {
        Ok(_) => {
            if git_workdir.canonicalize().or_error()? == destination {
                return Ok(());
            }
            progress.info(format!(
                "Skipping repository \"{}\": destination \"{}\" already exists",
                git_workdir.display(),
                destination.display()
            ));
            return Ok(());
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            return Err(err.and_raise(
                message("Failed to inspect repository destination").with("destination", destination.as_path()),
            ));
        }
    }
    match mode {
        Mode::Simulate => progress.info(format!(
            "WOULD move {} to {}",
            git_workdir.display(),
            destination.display()
        )),
        Mode::Execute => {
            move_repository(git_workdir, &destination, canonicalized_destination)?;
            progress.done(format!("Moving {} to {}", git_workdir.display(), destination.display()));
        }
    }
    Ok(())
}

fn move_repository(source: &Path, destination: &Path, destination_root: &Path) -> Result<()> {
    let move_error = || {
        message("Failed to move repository")
            .with("source", source)
            .with("destination", destination)
    };
    let create_parent = || {
        std::fs::create_dir_all(destination.parent().expect("repo destination is not the root"))
            .or_raise(|| message("Failed to create repository destination directory").with("destination", destination))
    };
    let destination_is_within_source = destination.starts_with(
        source
            .canonicalize()
            .ok()
            .map(Cow::Owned)
            .unwrap_or(Cow::Borrowed(source)),
    );
    if destination_is_within_source {
        let mut tempdir = tempfile::tempdir_in(destination_root).or_raise(|| {
            message("Failed to create temporary relocation directory").with("destination_root", destination_root)
        })?;
        let tempdest = tempdir
            .path()
            .join(destination.file_name().expect("repo destination is not the root"));
        std::fs::rename(source, &tempdest).or_raise(|| {
            message("Failed to stage repository for relocation")
                .with("source", source)
                .with("destination", tempdest.as_path())
        })?;
        // After the first move, cleanup must never delete the only remaining repository copy on failure.
        tempdir.disable_cleanup(true);
        create_parent().or_raise(|| move_error().with("recovery", tempdest.as_path()))?;
        std::fs::rename(&tempdest, destination).or_raise(|| move_error().with("recovery", tempdest.as_path()))?;
        tempdir.disable_cleanup(false);
    } else {
        create_parent()?;
        std::fs::rename(source, destination).or_raise(move_error)?;
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
) -> Result<()> {
    let mut repositories =
        find_git_repository_workdirs(source_dir, progress.add_child("Searching repositories"), debug, threads)
            .collect::<Vec<_>>();
    repositories.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    for (git_workdir, _kind) in repositories {
        writeln!(&mut out, "{}", git_workdir.display()).or_error()?;
    }
    Ok(())
}

/// Relocate repositories under `source_dir` to paths derived from their origin URLs below `destination`.
///
/// Skip destinations that already exist, preserving both paths, in execution and simulation modes.
/// Errors retain per-repository causes and `repository` path metadata, including validation classifications and
/// the `destination` and `destination_root` path metadata for origin URLs that resolve outside the allowed root.
/// Move failures identify `source` and `destination`; failed moves through a temporary directory retain the
/// repository there and identify its `recovery` path.
pub fn run<P: NestedProgress>(
    mode: Mode,
    source_dir: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    mut progress: P,
    threads: Option<usize>,
) -> Result<()> {
    let mut errors = Vec::new();
    let destination = destination.as_ref().canonicalize().or_raise(|| {
        message("Failed to resolve repository destination root").with("destination_root", destination.as_ref())
    })?;
    let mut repositories =
        find_git_repository_workdirs(source_dir, progress.add_child("Searching repositories"), false, threads)
            .collect::<Vec<_>>();
    repositories.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    for (path_to_move, kind) in repositories {
        if let Err(err) = handle(mode, kind, &path_to_move, &destination, &mut progress) {
            progress.fail(format!(
                "Failed to organize repository {:?}: {}",
                path_to_move.display(),
                err
            ));
            errors.push(err.and_raise(message("Failed to organize repository").with("repository", path_to_move)));
        }
    }

    if !errors.is_empty() {
        let num_errors = errors.len();
        let repositories = if num_errors == 1 { "repository" } else { "repositories" };
        Err(Exn::raise_all(errors, message!("Failed to organize {num_errors} {repositories}")).into())
    } else {
        Ok(())
    }
}
