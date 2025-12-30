use std::time::Duration;

const MB: usize = 1024 * 1024;

#[derive(Debug, Clone)]
pub struct SdkConfig {
    pub pool_size: usize,
    pub object_cache_bytes: usize,
    pub idle_timeout: Duration,
    pub max_blob_size: usize,
}

impl Default for SdkConfig {
    fn default() -> Self {
        Self {
            pool_size: 100,
            object_cache_bytes: 16 * MB,
            idle_timeout: Duration::from_secs(300),
            max_blob_size: 100 * MB,
        }
    }
}

impl SdkConfig {
    pub fn builder() -> SdkConfigBuilder {
        SdkConfigBuilder::default()
    }
}

#[derive(Debug, Clone)]
pub struct SdkConfigBuilder {
    pool_size: usize,
    object_cache_bytes: usize,
    idle_timeout: Duration,
    max_blob_size: usize,
}

impl Default for SdkConfigBuilder {
    fn default() -> Self {
        let defaults = SdkConfig::default();
        Self {
            pool_size: defaults.pool_size,
            object_cache_bytes: defaults.object_cache_bytes,
            idle_timeout: defaults.idle_timeout,
            max_blob_size: defaults.max_blob_size,
        }
    }
}

impl SdkConfigBuilder {
    pub fn pool_size(mut self, size: usize) -> Self {
        self.pool_size = size;
        self
    }

    pub fn object_cache_mb(mut self, mb: usize) -> Self {
        self.object_cache_bytes = mb * MB;
        self
    }

    pub fn idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }

    pub fn max_blob_size_mb(mut self, mb: usize) -> Self {
        self.max_blob_size = mb * MB;
        self
    }

    pub fn build(self) -> SdkConfig {
        SdkConfig {
            pool_size: self.pool_size,
            object_cache_bytes: self.object_cache_bytes,
            idle_timeout: self.idle_timeout,
            max_blob_size: self.max_blob_size,
        }
    }
}
