#![cfg(feature = "server")]

use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use bstr::ByteSlice;
use gix_error::{ErrorExt, Result, TestResult, message};
use gix_fsmonitor::{client, server};
use gix_notify::{Batch, Budget, Event, EventKind, Fence, Loss, PathKind, SynchronizeError, Watch};
use gix_parallel::{Mutable, OwnShared, lock};

#[derive(Clone, Copy, Debug)]
enum Delivery {
    Complete,
    SplitBatch,
    StaleCookies,
    Loss,
    GenerationChange,
    Diagnostic,
    CookieDirectory(EventKind),
}

struct Source {
    worktree: PathBuf,
    cookies: PathBuf,
    delivery: OwnShared<Mutable<Delivery>>,
    supported: bool,
    previous_cookie: Option<PathBuf>,
    pending: VecDeque<Batch>,
    generation: u64,
    sequence: u64,
}

impl Source {
    fn batch(&mut self, events: Vec<Event>) -> Batch {
        self.sequence += events.len() as u64;
        Batch {
            events,
            loss: None,
            error: None,
            generation: self.generation,
            sequence: self.sequence,
            more: false,
        }
    }
}

fn event(path: PathBuf) -> Event {
    Event {
        paths: vec![path],
        kind: EventKind::Create,
        path_kind: PathKind::File,
    }
}

impl server::EventSource for Source {
    fn register(&mut self, watches: &[Watch]) -> Result<()> {
        assert!(
            watches
                .iter()
                .any(|watch| watch.path == self.cookies && watch.recursive),
            "the server registers the prepared cookie directory on its source"
        );
        Ok(())
    }

    fn drain(&mut self, _budget: Budget) -> Batch {
        if let Some(batch) = self.pending.pop_front() {
            return batch;
        }
        let mut entries = std::fs::read_dir(&self.cookies).expect("the fixture retains its cookie directory");
        let Some(cookie) = entries.next() else {
            return self.batch(Vec::new());
        };
        let cookie = cookie.expect("the prepared cookie directory remains readable").path();
        assert!(entries.next().is_none(), "only one query cookie is live at a time");
        if self.previous_cookie.as_ref() == Some(&cookie) {
            return self.batch(Vec::new());
        }
        let previous_cookie = self.previous_cookie.replace(cookie.clone());
        let delivery = *lock(&self.delivery);
        let mut batch = match delivery {
            Delivery::StaleCookies => self.batch(vec![
                event(previous_cookie.expect("a successful baseline precedes stale cookie delivery")),
                event(self.cookies.join("foreign-cookie")),
            ]),
            _ => self.batch(vec![event(cookie)]),
        };
        match delivery {
            Delivery::Complete | Delivery::StaleCookies => {}
            Delivery::SplitBatch => {
                batch.events.push(event(self.worktree.join("same-batch")));
                self.sequence += 1;
                batch.sequence = self.sequence;
                batch.more = true;
                let tail = self.batch(vec![event(self.worktree.join("later-batch"))]);
                self.pending.push_back(tail);
            }
            Delivery::Loss => batch.loss = Some(Loss::Overflow),
            Delivery::GenerationChange => {
                self.generation += 1;
                batch.generation = self.generation;
            }
            Delivery::Diagnostic => batch.error = Some(message("injected native diagnostic").raise()),
            Delivery::CookieDirectory(kind) => {
                batch.events.push(Event {
                    paths: vec![
                        self.cookies.clone(),
                        self.cookies
                            .parent()
                            .expect("the cookie directory has an administrative parent")
                            .to_owned(),
                    ],
                    kind,
                    path_kind: PathKind::Directory,
                });
                self.sequence += 1;
                batch.sequence = self.sequence;
            }
        }
        batch
    }

    fn synchronize_at(
        &mut self,
        _directory: &Path,
        _deadline: Instant,
    ) -> std::result::Result<Fence, SynchronizeError> {
        Err(SynchronizeError::Unsupported)
    }

    fn supports_cookie_synchronization(&self) -> bool {
        self.supported
    }

    fn set_waker(&mut self, _wake: Box<dyn Fn() + Send + Sync + 'static>) {}
}

struct Fixture {
    server: server::Handle,
    _directory: gix_testtools::tempfile::TempDir,
    cookies: PathBuf,
    delivery: OwnShared<Mutable<Delivery>>,
}

impl Fixture {
    fn new(supported: bool) -> TestResult<Self> {
        let directory = gix_testtools::tempfile::tempdir()?;
        let worktree = directory.path().canonicalize()?.join("worktree");
        let git_dir = worktree.join(".git");
        std::fs::create_dir_all(&git_dir)?;
        let cookies = git_dir.join("fsmonitor--daemon/cookies");
        let delivery = OwnShared::new(Mutable::new(Delivery::Complete));
        let source_delivery = OwnShared::clone(&delivery);
        let server = server::spawn_with_source(
            server::Context {
                worktree,
                common_dir: git_dir.clone(),
                endpoint: git_dir.join("fsmonitor--daemon.ipc"),
                git_dir,
                precompose_unicode: false,
                ignore_case: false,
            },
            Default::default(),
            move |context| {
                Ok(Source {
                    worktree: context.worktree.clone(),
                    cookies: context.git_dir.join("fsmonitor--daemon/cookies"),
                    delivery: source_delivery,
                    supported,
                    previous_cookie: None,
                    pending: VecDeque::new(),
                    generation: 1,
                    sequence: 0,
                })
            },
        )?;
        Ok(Self {
            server,
            _directory: directory,
            cookies,
            delivery,
        })
    }

    fn query(&self, token: &[u8]) -> TestResult<Vec<u8>> {
        Ok(client::daemon::query(
            self.server.endpoint(),
            token.as_bstr(),
            || panic!("the fixture already owns a running server"),
            &client::daemon::Options {
                socket_timeout: Some(Duration::from_secs(5)),
                ..Default::default()
            },
        )?)
    }

    fn assert_cookies_removed(&self) -> TestResult {
        assert!(
            std::fs::read_dir(&self.cookies)?.next().is_none(),
            "finished queries remove their cookie even when synchronization fails"
        );
        Ok(())
    }
}

fn token(response: &[u8]) -> &[u8] {
    response
        .split(|byte| *byte == 0)
        .next()
        .expect("responses contain a token")
}

fn paths(response: &[u8]) -> &[u8] {
    response
        .splitn(2, |byte| *byte == 0)
        .nth(1)
        .expect("responses separate their token from candidate paths")
}

#[test]
fn cookie_queries_ingest_the_complete_batch_and_its_queued_tail() -> TestResult {
    let fixture = Fixture::new(true)?;
    let initial = fixture.query(b"")?;
    assert_eq!(
        paths(&initial),
        b"/\0",
        "a new client first establishes a full baseline"
    );
    *lock(&fixture.delivery) = Delivery::CookieDirectory(EventKind::Modify);
    let warmed = fixture.query(token(&initial))?;
    assert_eq!(
        paths(&warmed),
        b"",
        "cookie directory metadata changes preserve coverage and yield empty increments"
    );

    *lock(&fixture.delivery) = Delivery::SplitBatch;
    let changed = fixture.query(token(&warmed))?;
    assert_eq!(
        paths(&changed),
        b"later-batch\0same-batch\0",
        "paths after the marker in its batch and later queued batches reach the current response"
    );
    *lock(&fixture.delivery) = Delivery::Complete;
    let quiet = fixture.query(token(&changed))?;
    assert_eq!(paths(&quiet), b"", "administrative cookie events never reach clients");
    fixture.assert_cookies_removed()
}

#[test]
fn stale_cookie_events_cannot_complete_a_new_query() -> TestResult {
    let fixture = Fixture::new(true)?;
    let initial = fixture.query(b"")?;
    *lock(&fixture.delivery) = Delivery::StaleCookies;
    let incomplete = fixture.query(token(&initial))?;
    assert_eq!(
        paths(&incomplete),
        b"/\0",
        "old and foreign cookie notifications cannot replace observation of this query's marker"
    );
    fixture.assert_cookies_removed()
}

#[test]
fn cookie_observation_does_not_override_loss_or_diagnostics() -> TestResult {
    for delivery in [
        Delivery::Loss,
        Delivery::GenerationChange,
        Delivery::Diagnostic,
        Delivery::CookieDirectory(EventKind::Remove),
    ] {
        let fixture = Fixture::new(true)?;
        let initial = fixture.query(b"")?;
        *lock(&fixture.delivery) = delivery;
        let invalidated = fixture.query(token(&initial))?;
        assert_eq!(
            paths(&invalidated),
            b"/\0",
            "{delivery:?} invalidates the baseline even when its batch contains the correct cookie"
        );
        fixture.assert_cookies_removed()?;
    }
    Ok(())
}

#[test]
fn unsupported_sources_must_opt_into_cookie_synchronization() -> TestResult {
    let fixture = Fixture::new(false)?;
    let initial = fixture.query(b"")?;
    let repeated = fixture.query(token(&initial))?;
    assert_eq!(
        paths(&repeated),
        b"/\0",
        "an unsupported source without cookie ordering guarantees keeps requesting full verification"
    );
    fixture.assert_cookies_removed()
}
