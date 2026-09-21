use super::*;
use crate::config::cache::util::ApplyLeniency;

pub(super) struct Child {
    pub mount: BString,
    pub top_level: BString,
    pub monitor: RepositoryMonitor,
    pub open_options: crate::open::Options,
    pub worktree: bool,
}

impl Child {
    pub fn service(&mut self, now: Instant) -> crate::notify::Outcome {
        let git_dir = self.monitor.layout().git_dir.clone();
        let worktree = self.monitor.layout().worktree.clone();
        let options = self.open_options.clone();
        self.monitor.service(now, || {
            let mut repo = crate::open_opts(&git_dir, options.clone()).or_erased()?;
            if repo.workdir().is_none() {
                repo.set_workdir(worktree.clone()).or_erased()?;
            }
            Ok(repo)
        })
    }
}

#[derive(Default)]
pub(super) struct Discovery {
    pub children: BTreeMap<PathBuf, PreparedChild>,
    pub aliased: bool,
    pub paths: Vec<BString>,
}

pub(super) struct PreparedChild {
    pub child: Child,
    pub repository: Repository,
}

pub(super) fn discover(repo: &Repository, options: &Options, interrupt: &AtomicBool) -> Result<Discovery, Error> {
    let mut out = Discovery::default();
    let mut ancestors = HashSet::new();
    visit(
        repo,
        &BString::default(),
        None,
        options.submodules,
        options,
        interrupt,
        &mut ancestors,
        &mut out,
    )?;
    Ok(out)
}

#[expect(
    clippy::too_many_arguments,
    reason = "recursive discovery passes borrowed options and accumulated detached results"
)]
fn visit(
    repo: &Repository,
    mount: &BString,
    top_level: Option<&BString>,
    mode: Submodule,
    options: &Options,
    interrupt: &AtomicBool,
    ancestors: &mut HashSet<PathBuf>,
    out: &mut Discovery,
) -> Result<(), Error> {
    check_interrupted(interrupt)?;
    let git_dir = crate::notify::absolute_path(repo.git_dir(), repo.current_dir(), false)?;
    if ancestors.len() >= 64 || !ancestors.insert(git_dir.clone()) {
        return Err(
            message("submodule repository cycle or excessive nesting while discovering status monitors").raise(),
        );
    }
    if let Some(modules) = repo
        .submodules()
        .or_raise(|| message("could not discover submodules for cached status"))?
    {
        for module in modules {
            check_interrupted(interrupt)?;
            if module
                .index_id()
                .or_raise(|| message("could not read indexed submodule"))?
                .is_none()
            {
                continue;
            }
            let path = module
                .path()
                .or_raise(|| message("could not read monitored submodule path"))?;
            let child_mount = join(mount, &path);
            out.paths.push(child_mount.clone());
            let ignore = match mode {
                Submodule::Given { ignore, .. } => ignore,
                Submodule::AsConfigured { .. } => {
                    let global = repo
                        .config_snapshot()
                        .string(crate::config::tree::Diff::IGNORE_SUBMODULES)
                        .map(|value| crate::config::tree::Diff::IGNORE_SUBMODULES.try_into_ignore(value))
                        .transpose()
                        .with_leniency(repo.config.lenient_config)
                        .or_raise(|| message("could not read global submodule ignore mode"))?;
                    match global {
                        Some(ignore) => ignore,
                        None => module
                            .ignore()
                            .or_raise(|| message("could not read submodule ignore mode"))?
                            .unwrap_or_default(),
                    }
                }
            };
            if ignore == crate::submodule::config::Ignore::All {
                continue;
            }
            let state = module
                .state()
                .or_raise(|| message("could not inspect initialized submodule"))?;
            if !state.worktree_checkout || !state.repository_exists {
                continue;
            }
            let Some(child_repo) = module
                .open()
                .or_raise(|| message("could not open monitored submodule"))?
            else {
                continue;
            };
            let worktree = ignore != crate::submodule::config::Ignore::Dirty;
            let mut monitor_options = options.repository.clone();
            monitor_options.worktree = worktree;
            let child_top = top_level.unwrap_or(&path);
            let monitor = child_repo.monitor(monitor_options)?;
            let key = crate::notify::absolute_path(child_repo.git_dir(), child_repo.current_dir(), false)?;
            if ancestors.contains(&key) {
                return Err(message("submodule repository cycle while discovering status monitors").raise());
            }
            if let Some(existing) = out.children.get_mut(&key) {
                if existing.child.monitor.layout().worktree != monitor.layout().worktree {
                    return Err(
                        message("one submodule administrative directory maps to multiple physical worktrees").raise(),
                    );
                }
                out.aliased = true;
                let needs_descendants = worktree && !existing.child.worktree;
                if needs_descendants {
                    existing.child.worktree = true;
                    existing.child.monitor.set_worktree_enabled(true);
                    visit(
                        &child_repo,
                        &child_mount,
                        Some(child_top),
                        Submodule::default(),
                        options,
                        interrupt,
                        ancestors,
                        out,
                    )?;
                }
                continue;
            }
            if worktree {
                visit(
                    &child_repo,
                    &child_mount,
                    Some(child_top),
                    Submodule::default(),
                    options,
                    interrupt,
                    ancestors,
                    out,
                )?;
            }
            out.children.insert(
                key,
                PreparedChild {
                    child: Child {
                        mount: child_mount,
                        top_level: child_top.clone(),
                        monitor,
                        open_options: child_repo.open_options().clone(),
                        worktree,
                    },
                    repository: child_repo,
                },
            );
        }
    }
    ancestors.remove(&git_dir);
    Ok(())
}

pub(super) fn join(parent: &BString, child: &BString) -> BString {
    if parent.is_empty() {
        return child.clone();
    }
    let mut out = parent.clone();
    out.push(b'/');
    out.extend_from_slice(child);
    out
}
