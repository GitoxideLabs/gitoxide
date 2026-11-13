pub(super) struct Monitor<S> {
    pub(super) watcher: S,
    state: super::state::State,
}

impl<S> std::ops::Deref for Monitor<S> {
    type Target = super::state::State;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}
impl<S> std::ops::DerefMut for Monitor<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

#[maybe_async]
impl<S: EventSource> Monitor<S> {
    pub(super) async fn new(layout: Layout, watcher: S, options: Options) -> Result<Self> {
        let mut monitor = Self {
            watcher,
            state: super::state::State::new(layout, options)?,
        };
        monitor.register().await?;
        monitor.healthy = true;
        Ok(monitor)
    }

    pub(super) async fn register(&mut self) -> Result<()> {
        prepare_cookies(&self.watcher, &self.state.layout)
            .await
            .or_raise(|| message("preparing the native synchronization directory"))?;
        self.watcher
            .register(&self.state.layout.watches())
            .await
            .or_raise(|| message("registering complete worktree coverage"))
    }

    pub(super) async fn repair(&mut self, now: Instant) {
        let result = self.register().await;
        match result {
            Ok(()) => {
                self.state.healthy = true;
                self.state.retry = None;
            }
            Err(_err) => {
                gix_trace::warn!(error = %_err, "filesystem monitoring will retry");
                self.state.healthy = false;
                self.state.retry = Some(now + self.state.options.retry_interval);
                self.state.journal.reset();
            }
        }
    }

    pub(super) async fn ingest(&mut self, batch: Batch, now: Instant) {
        let check_roots = batch.loss.is_some() || batch.error.is_some();
        self.state.ingest(batch, now);
        if check_roots {
            // Consume the batch first; cancellation of the root check leaves conservative recovery state.
            let healthy = self.state.healthy;
            let retry = self.state.retry;
            self.state.healthy = false;
            self.state.retry.get_or_insert(now + self.state.options.retry_interval);
            match roots_lost(&self.watcher, &self.state.layout).await {
                Ok(lost) => {
                    self.state.root_lost |= lost;
                    self.state.healthy = healthy;
                    self.state.retry = retry;
                }
                Err(_error) => {
                    gix_trace::warn!(error = %_error, "filesystem monitor root check failed");
                    self.state.journal.reset();
                }
            }
        }
    }

    pub(super) async fn maintenance(&mut self, now: Instant) -> bool {
        let batch = self.watcher.drain(Budget::default());
        let more = batch.more;
        self.ingest(batch, now).await;
        self.state.journal.expire(now);
        if !self.state.root_lost && self.state.retry.is_some_and(|retry| now >= retry) {
            self.repair(now).await;
        }
        more
    }

    pub(super) async fn response(&mut self, requested: &[u8], now: Instant) -> Vec<u8> {
        self.maintenance(now).await;
        let deadline = Instant::now() + self.state.options.fence_timeout;
        let mut complete = false;
        if self.state.healthy && !self.state.root_lost {
            let directory = self.state.layout.cookie_dir.clone();
            match self.watcher.synchronize_at(&directory, deadline).await {
                Ok(fence) => loop {
                    let batch = self.watcher.drain(Budget::default());
                    let generation = batch.generation;
                    let sequence = batch.sequence;
                    self.ingest(batch, now).await;
                    if generation != fence.generation || !self.state.healthy || self.state.root_lost {
                        break;
                    }
                    if sequence >= fence.sequence {
                        complete = true;
                        break;
                    }
                    if Instant::now() >= deadline {
                        break;
                    }
                },
                Err(gix_notify::SynchronizeError::Unsupported) => {
                    if self.watcher.supports_cookie_synchronization() {
                        match self.synchronize_cookie(now, deadline).await {
                            Ok(synchronized) => complete = synchronized,
                            Err(_error) => {
                                gix_trace::warn!(error = %_error, "filesystem cookie synchronization requires a full response");
                                self.state.healthy = false;
                                self.state.retry.get_or_insert(now + self.state.options.retry_interval);
                            }
                        }
                    }
                }
                Err(_error) => {
                    gix_trace::warn!(error = %_error, "filesystem synchronization requires a full response");
                    self.state.healthy = false;
                    self.state.retry.get_or_insert(now + self.state.options.retry_interval);
                }
            }
        }
        if !complete {
            self.state.journal.reset();
        }
        self.state.journal.response(requested, complete, now)
    }

    pub(super) async fn synchronize_cookie(&mut self, now: Instant, deadline: Instant) -> io::Result<bool> {
        let cookie = create_cookie(&self.watcher, &self.state.layout.cookie_dir).await?;
        let generation = self.state.generation;
        let mut seen = false;
        loop {
            let batch = self.watcher.drain(Budget::default());
            let lost = batch.loss.is_some() || batch.error.is_some() || Some(batch.generation) != generation;
            let more = batch.more;
            seen |= batch.events.iter().any(|event| event.paths.contains(&cookie.path));
            // Ingest the entire batch and any queued remainder before acknowledging the cookie.
            // A marker near a drain budget boundary must not hide later paths in its native batch.
            self.ingest(batch, now).await;
            if lost || !self.state.healthy || self.state.root_lost {
                return Ok(false);
            }
            if seen && !more {
                return Ok(true);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(false);
            }
            if !more {
                wait_for_cookie(&self.watcher, remaining, self.state.options.idle_interval).await;
            }
        }
    }

    pub(super) async fn close(self) -> Result {
        self.watcher.close().await
    }
}
