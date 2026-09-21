use std::{
    collections::{BTreeSet, VecDeque},
    time::{Duration, Instant},
};

struct Entry {
    sequence: u64,
    observed: Instant,
    path: Vec<u8>,
}

/// A bounded multi-client history. Tokens identify observations, never cached status results.
pub(crate) struct Journal {
    instance: String,
    generation: u64,
    sequence: u64,
    floor: u64,
    entries: VecDeque<Entry>,
    bytes: usize,
    max_entries: usize,
    max_bytes: usize,
    max_age: Duration,
}

impl Journal {
    pub fn new(max_entries: usize, max_bytes: usize, max_age: Duration) -> std::io::Result<Self> {
        let mut nonce = [0; 16];
        getrandom::fill(&mut nonce).map_err(std::io::Error::other)?;
        Ok(Self {
            instance: format!("{:032x}", u128::from_ne_bytes(nonce)),
            generation: 0,
            sequence: 0,
            floor: 0,
            entries: VecDeque::new(),
            bytes: 0,
            max_entries,
            max_bytes,
            max_age,
        })
    }

    pub fn reset(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.sequence = 0;
        self.floor = 0;
        self.entries.clear();
        self.bytes = 0;
    }

    fn prefix(&self) -> String {
        format!("gix:{}-{}:", self.instance, self.generation)
    }

    fn token(&self) -> String {
        format!("{}{}", self.prefix(), self.sequence)
    }

    pub fn record(&mut self, path: Vec<u8>, now: Instant) {
        self.sequence = match self.sequence.checked_add(1) {
            Some(sequence) => sequence,
            None => {
                self.reset();
                1
            }
        };
        let cost = path.capacity().saturating_add(std::mem::size_of::<Entry>());
        if cost > self.max_bytes || self.max_entries == 0 {
            self.entries.clear();
            self.bytes = 0;
            self.floor = self.sequence;
            return;
        }
        self.bytes += cost;
        self.entries.push_back(Entry {
            sequence: self.sequence,
            observed: now,
            path,
        });
        self.expire(now);
    }

    pub fn expire(&mut self, now: Instant) {
        while self.entries.front().is_some_and(|entry| {
            self.entries.len() > self.max_entries
                || self.bytes > self.max_bytes
                || now.saturating_duration_since(entry.observed) > self.max_age
        }) {
            if let Some(entry) = self.entries.pop_front() {
                self.bytes -= entry.path.capacity() + std::mem::size_of::<Entry>();
                self.floor = entry.sequence;
            }
        }
    }

    pub fn response(&mut self, requested: &[u8], complete: bool, now: Instant) -> Vec<u8> {
        self.expire(now);
        let since = std::str::from_utf8(requested)
            .ok()
            .and_then(|token| token.strip_prefix(&self.prefix()))
            .filter(|sequence| {
                !sequence.is_empty()
                    && sequence.bytes().all(|byte| byte.is_ascii_digit())
                    && (*sequence == "0" || !sequence.starts_with('0'))
            })
            .and_then(|sequence| sequence.parse::<u64>().ok())
            .filter(|sequence| *sequence >= self.floor && *sequence <= self.sequence);
        let mut out = self.token().into_bytes();
        out.push(0);
        if !complete || since.is_none() {
            out.extend_from_slice(b"/\0");
            return out;
        }
        if let Some(since) = since {
            let paths: BTreeSet<_> = self
                .entries
                .iter()
                .filter(|entry| entry.sequence > since)
                .map(|entry| entry.path.as_slice())
                .collect();
            for path in paths {
                out.extend_from_slice(path);
                out.push(0);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(response: &[u8]) -> &[u8] {
        response
            .split(|byte| *byte == 0)
            .next()
            .expect("responses always contain a token")
    }

    #[test]
    fn independent_clients_keep_history_and_receive_deduplicated_paths() -> std::io::Result<()> {
        let now = Instant::now();
        let mut journal = Journal::new(10, 4096, Duration::from_secs(300))?;
        let initial = journal.response(b"", true, now);
        assert!(
            initial.ends_with(b"\0/\0"),
            "an unknown token establishes a full baseline"
        );
        journal.record(b"a".to_vec(), now);
        let first = journal.response(token(&initial), true, now);
        journal.record(b"b/".to_vec(), now);
        journal.record(b"a".to_vec(), now);
        let old = journal.response(token(&initial), true, now);
        let recent = journal.response(token(&first), true, now);
        assert_eq!(
            old.splitn(2, |byte| *byte == 0).nth(1),
            Some(&b"a\0b/\0"[..]),
            "queries do not consume another client's history"
        );
        assert_eq!(
            recent.splitn(2, |byte| *byte == 0).nth(1),
            Some(&b"a\0b/\0"[..]),
            "repeated changes after a token remain visible"
        );
        let empty = journal.response(token(&recent), true, now);
        assert_eq!(
            empty,
            [token(&recent), b"\0"].concat(),
            "a warmed unchanged query is useful and empty"
        );
        Ok(())
    }

    #[test]
    fn expiration_loss_and_foreign_tokens_require_full_responses() -> std::io::Result<()> {
        let now = Instant::now();
        let mut journal = Journal::new(1, 4096, Duration::from_secs(1))?;
        let old = journal.response(b"", true, now);
        journal.record(b"a".to_vec(), now);
        let recent = journal.response(token(&old), true, now);
        journal.record(b"b".to_vec(), now);
        assert!(
            journal.response(token(&old), true, now).ends_with(b"\0/\0"),
            "capacity expires tokens older than retained changes"
        );
        assert!(
            !journal.response(token(&recent), true, now).ends_with(b"\0/\0"),
            "the last evicted sequence is still a valid boundary"
        );
        assert!(
            journal
                .response(token(&recent), true, now + Duration::from_secs(2))
                .ends_with(b"\0/\0"),
            "age bounds retained history"
        );
        journal.reset();
        assert!(
            journal.response(token(&recent), true, now).ends_with(b"\0/\0"),
            "loss invalidates tokens even if their sequence still fits"
        );
        assert!(
            journal.response(b"builtin:foreign:1", true, now).ends_with(b"\0/\0"),
            "Git daemon tokens are opaque foreign baselines"
        );
        let current = journal.response(b"", true, now);
        assert!(
            journal.response(token(&current), false, now).ends_with(b"\0/\0"),
            "an unsupported or failed fence cannot certify an empty answer"
        );
        Ok(())
    }

    #[test]
    fn oversized_path_expires_every_earlier_baseline() -> std::io::Result<()> {
        let now = Instant::now();
        let mut journal = Journal::new(10, 64, Duration::from_secs(300))?;
        let old = journal.response(b"", true, now);
        journal.record(vec![b'x'; 65], now);
        assert!(
            journal.response(token(&old), true, now).ends_with(b"\0/\0"),
            "dropping a single oversized path still records coverage loss"
        );
        assert!(journal.bytes <= 64, "memory remains bounded");
        Ok(())
    }
    #[test]
    fn malformed_same_epoch_tokens_require_a_full_response() -> std::io::Result<()> {
        let now = Instant::now();
        let mut journal = Journal::new(10, 4096, Duration::from_secs(300))?;
        journal.record(b"path".to_vec(), now);
        for malformed in ["", "+1", "01", " 1", "1 ", "-1", "18446744073709551616"] {
            let token = format!("{}{malformed}", journal.prefix());
            assert!(
                journal.response(token.as_bytes(), true, now).ends_with(b"\0/\0"),
                "only canonical sequence numbers from a retained baseline are accepted"
            );
        }
        let fresh = Journal::new(10, 4096, Duration::from_secs(300))?;
        assert_ne!(
            journal.instance, fresh.instance,
            "daemon instances use independent random epochs"
        );
        Ok(())
    }
}
