use super::*;
use crate::config::{cache::util::ApplyLeniency, tree::Diff};

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
        let options = self.open_options.clone_with_trust();
        self.monitor.service(now, || {
            let mut repo = crate::open_opts(&git_dir, options.clone_with_trust())?;
            if repo.workdir().is_none() {
                repo.set_workdir(worktree.clone())?;
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

pub(super) fn discover(
    repo: &Repository,
    mode: Submodule,
    options: &crate::notify::Options,
    interrupt: &AtomicBool,
) -> Result<Discovery> {
    let mut out = Discovery::default();
    let mut ancestors = HashSet::new();
    visit(
        repo,
        &BString::default(),
        None,
        mode,
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
    options: &crate::notify::Options,
    interrupt: &AtomicBool,
    ancestors: &mut HashSet<PathBuf>,
    out: &mut Discovery,
) -> Result {
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
                        .config
                        .resolved
                        .string_filter(Diff::IGNORE_SUBMODULES, repo.filter_config_section())
                        .map(|value| Diff::IGNORE_SUBMODULES.try_into_ignore(value))
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
            let mut monitor_options = crate::notify::Options {
                all_worktrees: false,
                worktree_filter: None,
                checkout_filter: None,
                additional_paths: Vec::new(),
                ..*options
            };
            monitor_options.worktree_coverage = worktree.then(|| options.worktree_coverage.unwrap_or_default());
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
                    existing.child.monitor = monitor;
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
                        open_options: child_repo.open_options().clone_with_trust(),
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

/// Native subscriptions and gitlink mappings, independent of the status baseline.
#[derive(Default)]
pub(crate) struct Watches {
    pub(super) children: BTreeMap<PathBuf, Child>,
    pub(super) cursor: Option<PathBuf>,
    pub(super) paths: Vec<BString>,
    aliased: bool,
}

impl Watches {
    pub(crate) fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    pub(crate) fn has_gitlinks(&self) -> bool {
        !self.paths.is_empty() || self.has_children()
    }

    pub(crate) fn topology_changed(&self, changes: &Changes) -> bool {
        changes.configuration
            || changes.attributes
            || changes.layout
            || scope_touches_modules(changes.worktree.as_ref(), &self.paths)
    }

    pub(crate) fn rescan(&mut self) {
        for child in self.children.values_mut() {
            child.monitor.rescan();
        }
    }

    pub(crate) fn coverage(&self, root: CoverageStatus) -> CoverageStatus {
        self.children.values().fold(root, |coverage, child| {
            coverage.combine(child.monitor.coverage_status())
        })
    }

    pub(crate) fn deadline(&self, now: Instant) -> Option<Instant> {
        self.children
            .values()
            .filter_map(|child| child.monitor.notification_deadline(now))
            .min()
    }

    pub(crate) fn service(&mut self, now: Instant, quiet: Option<Duration>) -> (crate::notify::Outcome, bool) {
        let mut outcome = crate::notify::Outcome::default();
        let mut topology_changed = false;
        let mut omitted_errors = 0usize;
        for child in self.children.values_mut() {
            child.monitor.apply_loss_debounce(quiet);
        }
        let after = self.cursor.as_ref().map_or(Bound::Unbounded, Bound::Excluded);
        let keys: Vec<_> = self
            .children
            .range::<PathBuf, _>((after, Bound::Unbounded))
            .chain(
                self.children
                    .iter()
                    .take_while(|(key, _)| self.cursor.as_ref().is_some_and(|cursor| *key <= cursor)),
            )
            .filter(|(_, child)| {
                child
                    .monitor
                    .notification_deadline(now)
                    .is_some_and(|deadline| deadline <= now)
            })
            .take(CHILDREN_PER_SERVICE)
            .map(|(key, _)| key.clone())
            .collect();
        for key in keys {
            let child = self
                .children
                .get_mut(&key)
                .expect("selected child keys exist during service");
            let local = child.service(now);
            if !local.changes.is_empty() || !local.errors.is_empty() {
                if self.aliased {
                    outcome.changes.worktree = Some(Scope::All);
                } else {
                    outcome
                        .changes
                        .insert_worktree_path(path_change(child.top_level.clone()));
                }
            }
            topology_changed |= local.changes.index
                || local.changes.configuration
                || local.changes.attributes
                || local.changes.layout
                || scope_touches_modules_at(local.changes.worktree.as_ref(), &child.mount, &self.paths);
            for error in local.errors {
                if outcome.errors.len() < MAX_ERRORS - 1 {
                    outcome.errors.push(error);
                } else {
                    omitted_errors = omitted_errors.saturating_add(1);
                }
            }
            outcome.statistics.merge(local.statistics);
            self.cursor = Some(key);
        }
        if omitted_errors != 0 {
            outcome
                .errors
                .push(message!("{omitted_errors} additional submodule monitoring errors were omitted").raise());
        }
        if let Some(Scope::Paths(paths)) = &mut outcome.changes.worktree {
            paths.sort_by(|a, b| a.path.cmp(&b.path));
        }
        (outcome, topology_changed)
    }

    pub(crate) fn reconcile(
        &mut self,
        repo: &Repository,
        mode: Submodule,
        options: &crate::notify::Options,
        subscription: &Subscription,
        interrupt: &AtomicBool,
    ) -> Result {
        let discovered = discover(repo, mode, options, interrupt)?;
        let mut retain = HashSet::new();
        let mut first_error = None;
        let loss_debounce = lock(&subscription.pending).loss_debounce;
        for (key, prepared) in discovered.children {
            check_interrupted(interrupt)?;
            retain.insert(key.clone());
            let mut child = prepared.child;
            child.monitor.apply_loss_debounce(loss_debounce);
            #[cfg(feature = "async-notify")]
            child.monitor.set_async_wake(subscription.wake.clone());
            if let Some(old) = self.children.get_mut(&key)
                && old.monitor.layout() == child.monitor.layout()
                && old.worktree == child.worktree
            {
                old.worktree = child.worktree;
                old.open_options = child.open_options;
                old.mount = child.mount;
                old.top_level = child.top_level;
                if let Err(error) = old.monitor.prepare(&prepared.repository, Instant::now()) {
                    first_error.get_or_insert(error);
                }
            } else {
                match child.monitor.prepare(&prepared.repository, Instant::now()) {
                    Ok(()) => {
                        self.children.insert(key, child);
                    }
                    Err(error) => {
                        first_error.get_or_insert(error);
                    }
                }
            }
        }
        if let Some(error) = first_error {
            return Err(error);
        }
        self.children.retain(|key, _| retain.contains(key));
        self.paths = discovered.paths;
        self.aliased = discovered.aliased;
        self.cursor = None;
        Ok(())
    }
}
