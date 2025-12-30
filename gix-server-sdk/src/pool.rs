use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;

use crate::config::SdkConfig;
use crate::error::SdkError;
use crate::types::PoolStats;

pub struct PooledRepo {
    pub repo: Arc<gix::ThreadSafeRepository>,
    pub last_accessed: Instant,
    pub path: PathBuf,
}

pub struct RepoHandle {
    inner: Arc<gix::ThreadSafeRepository>,
}

impl RepoHandle {
    pub fn to_local(&self) -> gix::Repository {
        self.inner.to_thread_local()
    }
}

pub struct RepoPool {
    repos: DashMap<PathBuf, PooledRepo>,
    config: SdkConfig,
    open_count: AtomicUsize,
    hit_count: AtomicUsize,
}

impl RepoPool {
    pub fn new(config: SdkConfig) -> Self {
        Self {
            repos: DashMap::new(),
            config,
            open_count: AtomicUsize::new(0),
            hit_count: AtomicUsize::new(0),
        }
    }

    pub fn get(&self, path: impl AsRef<Path>) -> Result<RepoHandle, SdkError> {
        let path = path.as_ref().to_path_buf();

        if let Some(mut entry) = self.repos.get_mut(&path) {
            entry.last_accessed = Instant::now();
            self.hit_count.fetch_add(1, Ordering::Relaxed);
            return Ok(RepoHandle {
                inner: Arc::clone(&entry.repo),
            });
        }

        let repo = self.open_repo(&path)?;
        let repo = Arc::new(repo);

        let pooled = PooledRepo {
            repo: Arc::clone(&repo),
            last_accessed: Instant::now(),
            path: path.clone(),
        };

        self.repos.insert(path, pooled);
        self.open_count.fetch_add(1, Ordering::Relaxed);

        Ok(RepoHandle { inner: repo })
    }

    pub fn evict_idle(&self) {
        let now = Instant::now();
        let timeout = self.config.idle_timeout;

        self.repos.retain(|_, pooled| {
            now.duration_since(pooled.last_accessed) < timeout
        });
    }

    pub fn stats(&self) -> PoolStats {
        let cached_count = self.repos.len();
        let open_count = self.open_count.load(Ordering::Relaxed);
        let hit_count = self.hit_count.load(Ordering::Relaxed);

        let total_requests = open_count + hit_count;
        let hit_rate = if total_requests > 0 {
            hit_count as f64 / total_requests as f64
        } else {
            0.0
        };

        PoolStats {
            cached_count,
            open_count,
            hit_count,
            hit_rate,
        }
    }

    fn open_repo(&self, path: &Path) -> Result<gix::ThreadSafeRepository, SdkError> {
        if !path.exists() {
            return Err(SdkError::RepoNotFound(path.to_path_buf()));
        }

        let mut opts = gix::open::Options::default();
        opts.permissions.config.git_binary = false;
        opts.permissions.env.git_prefix = gix::sec::Permission::Deny;

        let repo = gix::open_opts(path, opts)
            .map_err(|e| SdkError::Git(Box::new(e)))?;

        let mut local = repo.clone();
        local.object_cache_size(self.config.object_cache_bytes);

        Ok(local.into_sync())
    }
}
