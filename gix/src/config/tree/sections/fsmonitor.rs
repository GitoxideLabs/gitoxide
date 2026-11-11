//! Git filesystem-monitor configuration.
//!
//! The daemon tuning keys are defined in Git's `builtin/fsmonitor--daemon.c`,
//! even though `git-config(1)` only documents `allowRemote` and `socketDir`.

use crate::{
    config,
    config::tree::{FsMonitor, Key, Section, keys},
};

impl FsMonitor {
    /// The `fsmonitor.allowRemote` key, allowing Git to monitor network-mounted repositories.
    /// Controls worktree eligibility for native daemon commands; permission does not guarantee notifications.
    pub const ALLOW_REMOTE: keys::Boolean =
        keys::Boolean::new_boolean("allowRemote", &config::Tree::FS_MONITOR).with_default(b"false");
    /// The `fsmonitor.socketDir` key, selecting a Unix-domain socket directory on macOS and Linux.
    /// Git requires a native filesystem and only uses this with the built-in daemon.
    pub const SOCKET_DIR: keys::Path = keys::Path::new_path("socketDir", &config::Tree::FS_MONITOR);
    /// The `fsmonitor.ipcThreads` key, specifying the IPC worker thread count.
    pub const IPC_THREADS: keys::UnsignedInteger =
        keys::UnsignedInteger::new_unsigned_integer("ipcThreads", &config::Tree::FS_MONITOR).with_default(b"8");
    /// The `fsmonitor.startTimeout` key, specifying the detached startup wait in seconds.
    /// Zero checks readiness without waiting.
    pub const START_TIMEOUT: keys::UnsignedInteger =
        keys::UnsignedInteger::new_unsigned_integer("startTimeout", &config::Tree::FS_MONITOR).with_default(b"60");
    /// The `fsmonitor.announceStartup` key, enabling daemon startup announcements.
    pub const ANNOUNCE_STARTUP: keys::Boolean =
        keys::Boolean::new_boolean("announceStartup", &config::Tree::FS_MONITOR).with_default(b"false");
}

impl Section for FsMonitor {
    fn name(&self) -> &str {
        "fsmonitor"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[
            &Self::ALLOW_REMOTE,
            &Self::SOCKET_DIR,
            &Self::IPC_THREADS,
            &Self::START_TIMEOUT,
            &Self::ANNOUNCE_STARTUP,
        ]
    }
}
