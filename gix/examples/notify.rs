//! Print repository invalidations until Ctrl-C.
//!
//! ```sh
//! cargo run -p gix --features notify --example notify -- --debounce-ms 20 --max-latency-ms 100 /path/to/repository
//! cargo run -p gix --features notify --example notify -- --all-worktrees --worktree-coverage all /path/to/repository
//! cargo run -p gix --features notify --example notify -- --no-metadata /path/to/repository
//! ```
//!
//! `RepositoryMonitor` uses ignore-aware worktree coverage by default:
//! the index and ignore rules skip fully ignored directories, stopping at nested repositories.
//! File invalidations are unfiltered; callers check individual exclusions, including trackedness.
//! Directory scopes, ambiguous removals and full invalidations still require reconciliation.
//! Use `--worktree-coverage all` to include ignored paths and nested repositories. The API classifies metadata
//! changes and filters administrative paths out of worktree scopes. Each path includes advisory
//! event-kind and object-type hints; conflicting hints coalesce to `Any`, not an operation history.
//! Opt in to all other worktrees with `--all-worktrees`; a single monitor shares metadata subscriptions and
//! reports each other worktree's lifecycle separately from its invalidations. `Added` means
//! discovered now (including initial enumeration), not necessarily newly created; `Moved`
//! means a known registration has a new checkout path. Both require a full baseline.
//! `Removed` means the registration disappeared, not just that its checkout is inaccessible.
//! The initial layout describes only the selected repository. The example prints its
//! discovered inventory after the first service and whenever the layouts change, including
//! registered but inaccessible checkouts. Inclusion in the inventory does not prove coverage.
//! Raw watcher statistics describe native events and directory watches, not actionable
//! invalidations or worktree lifecycle events. Errors are reported separately on every failure.
//!
//! Use `--no-metadata` for worktree-only invalidations, or `--worktree-coverage none` for
//! metadata-only monitoring. These tune the same monitor, not a separate event API.
//! `--all-worktrees` requires metadata monitoring. External configuration files are not watched.
//!
//! This example only logs hints: it does not maintain a baseline or reconcile filesystem
//! state after loss. The monitor does not schedule timed full invalidations. Native events and
//! reported coverage loss drive normal recovery. For explicit manual recovery when cached
//! state appears stale, call `RepositoryMonitor::rescan()` to rebuild watches, rediscover
//! subscriptions, and invalidate every enabled domain at the next service call. This example
//! does not provide interactive input to request a rescan. `--retry-ms` controls retries of
//! failed discovery or watcher registration (default: 5000 ms). With the default `interrupt`
//! feature, Ctrl-C shuts down gracefully; without it, the operating system terminates the process.

fn main() -> gix::Result<()> {
    #[cfg(feature = "notify")]
    {
        monitor::run()
    }
    #[cfg(not(feature = "notify"))]
    {
        use gix::error::bail;
        bail!("Enable the notify feature: cargo run -p gix --features notify --example notify -- --help");
    }
}

#[cfg(feature = "notify")]
mod monitor {
    use std::{
        ffi::OsString,
        path::PathBuf,
        str::FromStr,
        thread,
        time::{Duration, Instant},
    };

    use gix::{
        Result,
        error::{OptionExt, bail, message},
        notify::{Layout, Options, RepositoryMonitor, WorktreeChange, WorktreeCoverage},
    };

    const POLL_INTERVAL: Duration = Duration::from_millis(250);
    const USAGE: &str = "Usage: notify [options] [repository=.]

Options:
  --no-metadata             Report only worktree invalidations [default: metadata enabled]
  --all-worktrees           Also monitor all other worktrees [requires metadata; default: off]
  --worktree-coverage none|ignore-aware|all
                           Directory coverage [default: ignore-aware; none disables worktree coverage]
  --debounce-ms N           Metadata quiet period [default: 100; 0 disables]
  --max-latency-ms N        Maximum metadata burst delay [default: 250; 0 publishes immediately]
  --retry-ms N              Failed discovery/watch retry interval [default: 5000; must be positive]
  --poll-ms N               Maximum idle wait [default: 250; must be positive]
  -h, --help               Show this help

All coverage and scheduling choices configure the same RepositoryMonitor.
RepositoryMonitor ignore-aware coverage uses the index and ignore rules to select directories,
stopping at nested repositories. File invalidations are unfiltered; callers check exclusions.
Tracked files remain observable even when ignored. Directory scopes, ambiguous removals and
full invalidations still require reconciliation using current repository state.
All coverage includes ignored paths and nested repositories; administrative paths remain excluded.
Use --no-metadata for worktree-only reporting, or --worktree-coverage none for metadata only.
--all-worktrees shares metadata subscriptions and reports other worktrees by path.
The initial layout is only the selected repository. The discovered inventory is printed
after the first service and when layouts change, not on every retry or invalidation.
Registered but inaccessible checkouts remain listed; discovery does not guarantee coverage.
Raw watcher statistics count native events and directory watches, not actionable
invalidations or worktree lifecycle events. Every monitor error is reported separately.
Lifecycle: Added means discovered now, including initial enumeration; Moved means a known
registration has a new checkout path. Both require a full baseline. Removed means the
registration disappeared. Changed carries invalidations, not lifecycle notifications.
Repository metadata is debounced; worktree scopes are not.
The monitor does not schedule timed full invalidations. Native events and reported coverage loss drive recovery.
RepositoryMonitor::rescan() provides explicit manual recovery when cached state appears stale:
rebuild watches, rediscover subscriptions, and invalidate every enabled domain at the next service call.
This example does not provide interactive input to request a rescan.
--retry-ms controls retries of failed discovery or watcher registration.
Repository polling is capped by the API's own deadlines (at most 250 ms).

Timestamps show elapsed time since monitoring started, not native event timestamps.
Use -- before a repository path that starts with a dash. Stop with Ctrl-C.";

    #[derive(Debug)]
    struct Args {
        options: Options,
        repository: PathBuf,
        poll_interval: Duration,
    }

    impl Default for Args {
        fn default() -> Self {
            Self {
                options: Options::default(),
                repository: ".".into(),
                poll_interval: POLL_INTERVAL,
            }
        }
    }

    impl Args {
        fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Option<Self>> {
            let mut args = args.into_iter();
            let mut out = Self::default();
            let mut repository = None;

            let mut positional_only = false;
            while let Some(arg) = args.next() {
                match arg.to_str() {
                    Some("--") if !positional_only => positional_only = true,
                    Some("--help" | "-h") if !positional_only => return Ok(None),
                    Some("--no-metadata") if !positional_only => out.options.metadata = false,
                    Some("--all-worktrees") if !positional_only => out.options.all_worktrees = true,
                    Some("--worktree-coverage") if !positional_only => {
                        out.options.worktree_coverage = match args.next().as_deref().and_then(|arg| arg.to_str()) {
                            Some("none") => None,
                            Some("ignore-aware") => Some(WorktreeCoverage::IgnoreAware),
                            Some("all") => Some(WorktreeCoverage::All),
                            _ => bail!("--worktree-coverage requires none, ignore-aware or all"),
                        };
                    }
                    Some(flag @ ("--debounce-ms" | "--max-latency-ms" | "--retry-ms" | "--poll-ms"))
                        if !positional_only =>
                    {
                        let value = Duration::from_millis(number(flag, &mut args)?);
                        if Instant::now().checked_add(value).is_none() {
                            bail!("{flag} exceeds the platform's clock range");
                        }
                        if matches!(flag, "--poll-ms" | "--retry-ms") && value.is_zero() {
                            bail!("{flag} must be positive");
                        }
                        match flag {
                            "--debounce-ms" => out.options.metadata_debounce = value,
                            "--max-latency-ms" => out.options.max_latency = value,
                            "--retry-ms" => out.options.retry_interval = value,
                            _ => out.poll_interval = value,
                        }
                    }

                    Some(flag) if !positional_only && flag.starts_with('-') => {
                        bail!("unknown option {flag:?}; use --help for usage");
                    }
                    _ if repository.is_none() => repository = Some(PathBuf::from(arg)),
                    _ => bail!("expected at most one repository path; use --help for usage"),
                }
            }
            if out.options.all_worktrees && !out.options.metadata {
                bail!("--all-worktrees requires metadata monitoring; remove --no-metadata");
            }
            Ok(Some(Self {
                repository: repository.unwrap_or_else(|| PathBuf::from(".")),
                ..out
            }))
        }
    }

    fn number<T: FromStr>(flag: &str, args: &mut impl Iterator<Item = OsString>) -> Result<T> {
        args.next()
            .as_deref()
            .and_then(|arg| arg.to_str())
            .and_then(|arg| arg.parse().ok())
            .ok_or_raise(|| message!("{flag} requires a nonnegative integer within range"))
    }

    pub fn run() -> Result<()> {
        let Some(args) = Args::parse(std::env::args_os().skip(1))? else {
            println!("{USAGE}");
            return Ok(());
        };
        #[cfg(feature = "interrupt")]
        let _signals = gix::interrupt::init_handler_default(1)?.auto_deregister();

        // The temporary repository is dropped here; the monitor retains no caches.
        let monitor = gix::discover(&args.repository)?.monitor(args.options)?;
        println!(
            "Initial selected layout (before service; selected repository only): {:#?}",
            monitor.layout()
        );
        println!("Starting monitoring. Press Ctrl-C to stop.");
        println!("Maximum idle wait: {:?}", args.poll_interval);
        println!("Monitor options: {:#?}", monitor.options());
        repository(monitor, args.poll_interval)
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

    fn repository(mut monitor: RepositoryMonitor, poll_interval: Duration) -> Result<()> {
        let git_dir = monitor.layout().git_dir.clone();
        let started = Instant::now();
        let mut inventory = Inventory::default();
        while !interrupted() {
            let outcome = monitor.service(Instant::now(), || gix::open(&git_dir));
            if inventory.update(monitor.layout(), monitor.other_worktrees()) {
                println!(
                    "[{:.3}s] Discovered inventory after service (selected repository + {} other registered worktrees):",
                    started.elapsed().as_secs_f64(),
                    inventory.other_worktrees.len()
                );
                println!("Selected layout: {:#?}", monitor.layout());
                println!("Other discovered layouts: {:#?}", inventory.other_worktrees);
                println!("Registered but inaccessible checkouts remain listed; discovery does not guarantee coverage.");
            }
            if !outcome.changes.is_empty() || !outcome.other_worktrees.is_empty() {
                println!(
                    "[{:.3}s] Actionable repository updates:",
                    started.elapsed().as_secs_f64()
                );
                if !outcome.changes.is_empty() {
                    println!("Selected worktree invalidations: {:#?}", outcome.changes);
                }
                for worktree in &outcome.other_worktrees {
                    println!(
                        "Other worktree: {} (Git directory: {})",
                        worktree.worktree.display(),
                        worktree.git_dir.display()
                    );
                    match &worktree.kind {
                        WorktreeChange::Added => println!(
                            "Lifecycle: Added (discovered now, including initial enumeration); consumers must build a full baseline."
                        ),
                        WorktreeChange::Moved { previous_worktree } => println!(
                            "Lifecycle: Moved from {}; consumers must rebuild a full baseline.",
                            previous_worktree.display()
                        ),
                        WorktreeChange::Removed => println!("Lifecycle: Removed (registration disappeared)."),
                        WorktreeChange::Changed(changes) => println!("Invalidations: {changes:#?}"),
                    }
                }
            }
            let statistics = &outcome.statistics;
            if statistics.received != 0 || statistics.rescans != 0 || statistics.added != 0 || statistics.removed != 0 {
                println!(
                    "[{:.3}s] Raw watcher statistics (native events/directory watches, not invalidations or worktree lifecycle): {statistics:#?}",
                    started.elapsed().as_secs_f64()
                );
            }
            for error in outcome.errors {
                eprintln!(
                    "[{:.3}s] Monitor error (maintenance will retry): {error:?}",
                    started.elapsed().as_secs_f64()
                );
            }
            thread::sleep(
                monitor
                    .next_timeout(Instant::now())
                    .unwrap_or(poll_interval)
                    .min(poll_interval),
            );
        }
        Ok(())
    }

    fn interrupted() -> bool {
        #[cfg(feature = "interrupt")]
        {
            gix::interrupt::is_triggered()
        }
        #[cfg(not(feature = "interrupt"))]
        {
            false
        }
    }
}
