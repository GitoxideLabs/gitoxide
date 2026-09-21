use std::{collections::VecDeque, mem::size_of, panic::AssertUnwindSafe};

use gix_error::{ErrorExt, message};
use gix_features::threading::{Mutable, OwnShared, lock};

use crate::{Batch, Budget, Error, Event, Loss, Options};

type Wake = OwnShared<dyn Fn() + Send + Sync>;

struct Entry {
    event: Event,
    bytes: usize,
    sequence: u64,
}

pub(crate) struct State {
    options: Options,
    events: VecDeque<Entry>,
    bytes: usize,
    loss: Option<(Loss, u64)>,
    error: Option<Error>,
    generation: u64,
    published: u64,
    consumed: u64,
    pub(crate) registration: u64,
    pub(crate) needs_restart: bool,
    pub(crate) waker: Option<Wake>,
}

impl State {
    pub(crate) fn new(options: Options) -> Self {
        Self {
            options,
            events: VecDeque::new(),
            bytes: 0,
            loss: None,
            error: None,
            generation: 0,
            published: 0,
            consumed: 0,
            registration: 0,
            needs_restart: false,
            waker: None,
        }
    }

    pub(crate) fn invalidate(&mut self, reason: Loss) {
        self.events.clear();
        self.bytes = 0;
        self.generation = self.generation.wrapping_add(1);
        self.published = self.published.wrapping_add(1);
        self.loss = Some((reason, self.published));
    }

    fn publish(&mut self, events: Vec<Event>) {
        let bytes = events
            .iter()
            .fold(0usize, |sum, event| sum.saturating_add(event_bytes(event)));
        if events.len() > self.options.max_events.saturating_sub(self.events.len())
            || bytes > self.options.max_bytes.saturating_sub(self.bytes)
        {
            self.invalidate(Loss::Overflow);
            return;
        }
        self.bytes += bytes;
        for event in events {
            self.published = self.published.wrapping_add(1);
            self.events.push_back(Entry {
                bytes: event_bytes(&event),
                event,
                sequence: self.published,
            });
        }
    }

    pub(crate) fn drain(&mut self, budget: Budget) -> Batch {
        let loss = self.loss.take().map(|(loss, sequence)| {
            self.consumed = sequence;
            loss
        });
        let mut events = Vec::new();
        let mut bytes = 0usize;
        while events.len() < budget.max_events {
            let Some(next) = self.events.front() else {
                break;
            };
            if !events.is_empty() && next.bytes > budget.max_bytes.saturating_sub(bytes) {
                break;
            }
            let Some(entry) = self.events.pop_front() else {
                break;
            };
            bytes = bytes.saturating_add(entry.bytes);
            self.bytes -= entry.bytes;
            self.consumed = entry.sequence;
            events.push(entry.event);
        }
        Batch {
            events,
            loss,
            errors: self.error.take().into_iter().collect(),
            generation: self.generation,
            sequence: self.consumed,
            more: !self.events.is_empty(),
        }
    }
}

fn event_bytes(event: &Event) -> usize {
    size_of::<Entry>()
        .saturating_add(event.paths.capacity().saturating_mul(size_of::<std::path::PathBuf>()))
        .saturating_add(
            event
                .paths
                .iter()
                .fold(0usize, |sum, path| sum.saturating_add(path.capacity())),
        )
}

/// One native registration, which publishes complete native batches under a single queue lock.
pub(crate) struct Sender {
    state: OwnShared<Mutable<State>>,
    registration: u64,
}

impl Sender {
    pub(crate) fn new(state: OwnShared<Mutable<State>>, registration: u64) -> Self {
        Self { state, registration }
    }

    pub(crate) fn publish(&self, events: Vec<Event>, loss: Option<Loss>, error: Option<Error>) {
        {
            let mut state = lock(&self.state);
            if state.registration != self.registration {
                return;
            }
            if let Some(loss) = loss {
                state.invalidate(loss);
            }
            if let Some(error) = error {
                state.error = Some(error);
                state.needs_restart = true;
            }
            state.publish(events);
        }
        wake(&self.state);
    }
}

impl Drop for Sender {
    fn drop(&mut self) {
        {
            let mut state = lock(&self.state);
            if state.registration != self.registration {
                return;
            }
            state.invalidate(Loss::BackendStopped);
            state.needs_restart = true;
            if state.error.is_none() {
                state.error = Some(message("filesystem notification backend stopped").raise());
            }
        }
        wake(&self.state);
    }
}

pub(crate) fn wake(state: &OwnShared<Mutable<State>>) {
    let waker = lock(state).waker.clone();
    if let Some(waker) = waker
        && std::panic::catch_unwind(AssertUnwindSafe(|| waker())).is_err()
    {
        let mut state = lock(state);
        state.waker = None;
        state.error = Some(message("filesystem notification wake callback panicked and was disabled").raise());
    }
}

#[cfg(test)]
mod tests;
