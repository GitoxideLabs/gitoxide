use std::{
    io::Write,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

use gix::{
    Repository, Result,
    error::{ResultExt, bail, validation},
    notify::{Layout, RepositoryMonitor, WorktreeChange},
};

/// Print repository invalidations until interrupted, reopening only when the monitor needs maintenance.
/// The caller creates the detached monitor and controls repository discovery through `opener`.
/// This logs hints; consumers still need to reconcile their baselines after loss or full invalidation.
pub fn function(
    mut monitor: RepositoryMonitor,
    poll_interval: Duration,
    mut opener: impl FnMut() -> Result<Repository>,
    mut out: impl Write,
    mut err: impl Write,
    should_interrupt: &AtomicBool,
) -> Result<()> {
    if poll_interval.is_zero() {
        bail!(validation("the maximum idle wait must be positive"));
    }
    writeln!(
        out,
        "Initial selected layout (before service; selected repository only): {:#?}",
        monitor.layout()
    )
    .or_error()?;
    writeln!(out, "Starting monitoring. Press Ctrl-C to stop.").or_error()?;
    writeln!(out, "Maximum idle wait: {poll_interval:?}").or_error()?;
    writeln!(out, "Monitor options: {:#?}", monitor.options()).or_error()?;
    out.flush().or_error()?;

    let started = Instant::now();
    let mut inventory = Inventory::default();
    while !should_interrupt.load(Ordering::Relaxed) {
        let outcome = monitor.service(Instant::now(), &mut opener);
        if inventory.update(monitor.layout(), monitor.other_worktrees()) {
            writeln!(
                out,
                "[{:.3}s] Discovered inventory after service (selected repository + {} other registered worktrees):",
                started.elapsed().as_secs_f64(),
                inventory.other_worktrees.len()
            )
            .or_error()?;
            writeln!(out, "Selected layout: {:#?}", monitor.layout()).or_error()?;
            writeln!(out, "Other discovered layouts: {:#?}", inventory.other_worktrees).or_error()?;
            writeln!(
                out,
                "Registered but inaccessible checkouts remain listed; discovery does not guarantee coverage."
            )
            .or_error()?;
        }
        if !outcome.changes.is_empty() || !outcome.other_worktrees.is_empty() {
            writeln!(
                out,
                "[{:.3}s] Actionable repository updates:",
                started.elapsed().as_secs_f64()
            )
            .or_error()?;
            if !outcome.changes.is_empty() {
                writeln!(out, "Selected worktree invalidations: {:#?}", outcome.changes).or_error()?;
            }
            for worktree in &outcome.other_worktrees {
                writeln!(
                    out,
                    "Other worktree: {} (Git directory: {})",
                    worktree.worktree.display(),
                    worktree.git_dir.display()
                )
                .or_error()?;
                match &worktree.kind {
                    WorktreeChange::Added => writeln!(
                        out,
                        "Lifecycle: Added (discovered now, including initial enumeration); consumers must build a full baseline."
                    ),
                    WorktreeChange::Moved { previous_worktree } => writeln!(
                        out,
                        "Lifecycle: Moved from {}; consumers must rebuild a full baseline.",
                        previous_worktree.display()
                    ),
                    WorktreeChange::Removed => writeln!(out, "Lifecycle: Removed (registration disappeared)."),
                    WorktreeChange::Changed(changes) => writeln!(out, "Invalidations: {changes:#?}"),
                }
                .or_error()?;
            }
        }
        let statistics = &outcome.statistics;
        if statistics.received != 0 || statistics.rescans != 0 || statistics.added != 0 || statistics.removed != 0 {
            writeln!(
                out,
                "[{:.3}s] Raw watcher statistics (native events/directory watches, not invalidations or worktree lifecycle): {statistics:#?}",
                started.elapsed().as_secs_f64()
            )
            .or_error()?;
        }
        for error in outcome.errors {
            writeln!(
                err,
                "[{:.3}s] Monitor error (maintenance will retry): {error:?}",
                started.elapsed().as_secs_f64()
            )
            .or_error()?;
        }
        out.flush().or_error()?;
        err.flush().or_error()?;
        thread::sleep(
            monitor
                .next_timeout(Instant::now())
                .unwrap_or(poll_interval)
                .min(poll_interval)
                .min(Duration::from_millis(250)),
        );
    }
    Ok(())
}

#[derive(Default)]
struct Inventory {
    selected: Option<Layout>,
    other_worktrees: Vec<Layout>,
}

impl Inventory {
    fn update<'a>(&mut self, selected: &Layout, other_worktrees: impl Iterator<Item = &'a Layout>) -> bool {
        let mut other_worktrees: Vec<_> = other_worktrees.cloned().collect();
        other_worktrees.sort_by(|a, b| a.git_dir.cmp(&b.git_dir));
        if self.selected.as_ref() == Some(selected) && self.other_worktrees == other_worktrees {
            return false;
        }
        self.selected = Some(selected.clone());
        self.other_worktrees = other_worktrees;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gix::error::TestResult;

    #[test]
    fn inventory_reports_layout_and_membership_changes_without_repeating_reordered_entries() -> TestResult<()> {
        let selected = layout("selected");
        let first = layout("first");
        let mut second = layout("second");
        let mut inventory = Inventory::default();
        assert!(
            inventory.update(&selected, [&second, &first].into_iter()),
            "initial inventory includes every discovered checkout"
        );
        assert!(
            !inventory.update(&selected, [&first, &second].into_iter()),
            "iteration order alone does not repeat the same discovered inventory"
        );
        second.worktree = Some("moved".into());
        assert!(
            inventory.update(&selected, [&first, &second].into_iter()),
            "a changed checkout layout is printed even with the same Git directory"
        );
        assert!(
            inventory.update(&selected, [&first].into_iter()),
            "retired worktrees are removed from the printed inventory"
        );
        let mut selected = selected;
        selected.index = "alternate-index".into();
        assert!(
            inventory.update(&selected, [&first].into_iter()),
            "selected layout changes remain visible independently of other worktrees"
        );
        Ok(())
    }

    fn layout(name: &str) -> Layout {
        Layout {
            git_dir: name.into(),
            common_dir: "common".into(),
            worktree: Some(name.into()),
            index: "index".into(),
            dependencies: Vec::new(),
        }
    }
}
