use std::time::Duration;

fn randomize(backoff_ms: usize) -> usize {
    let new_value = (fastrand::usize(750..=1250) * backoff_ms) / 1000;
    if new_value == 0 { backoff_ms } else { new_value }
}

/// A utility to calculate steps for quadratic backoff similar to how it's done in `git`.
pub struct Quadratic<Fn> {
    multiplier: usize,
    max_multiplier: usize,
    exponent: usize,
    transform: Fn,
}

impl Default for Quadratic<fn(usize) -> usize> {
    fn default() -> Self {
        Quadratic {
            multiplier: 1,
            max_multiplier: 1000,
            exponent: 1,
            transform: std::convert::identity,
        }
    }
}

impl Quadratic<fn(usize) -> usize> {
    /// Create a new quadratic backoff iterator that backs off in randomized, ever increasing steps.
    pub fn default_with_random() -> Self {
        Quadratic {
            multiplier: 1,
            max_multiplier: 1000,
            exponent: 1,
            transform: randomize,
        }
    }
}

impl<Transform> Quadratic<Transform>
where
    Transform: Fn(usize) -> usize,
{
    /// Return an iterator that yields `Duration` instances to sleep on until `time` is depleted.
    pub fn until_no_remaining(&mut self, time: Duration) -> impl Iterator<Item = Duration> + '_ {
        let mut elapsed = Duration::default();
        let mut stop_next_iteration = false;
        self.take_while(move |d| {
            if stop_next_iteration {
                false
            } else {
                elapsed += *d;
                if elapsed > time {
                    stop_next_iteration = true;
                }
                true
            }
        })
    }
}

impl<Transform> Iterator for Quadratic<Transform>
where
    Transform: Fn(usize) -> usize,
{
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        let wait = Duration::from_millis((self.transform)(self.multiplier) as u64);

        self.multiplier += 2 * self.exponent + 1;
        if self.multiplier > self.max_multiplier {
            self.multiplier = self.max_multiplier;
        } else {
            self.exponent += 1;
        }
        Some(wait)
    }
}
