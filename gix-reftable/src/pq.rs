use std::{cmp::Ordering, collections::BinaryHeap};

use crate::{error::Error, record::Record};

/// Entry in merged-table priority queues.
#[derive(Debug, Clone)]
pub struct PqEntry {
    /// Sub-iterator index.
    pub index: usize,
    /// Current record at this iterator head.
    pub record: Record,
}

impl PqEntry {
    fn try_cmp(&self, other: &Self) -> Result<Ordering, Error> {
        let key_cmp = self.record.cmp_key(&other.record)?;
        Ok(match key_cmp {
            Ordering::Less => Ordering::Greater,
            Ordering::Greater => Ordering::Less,
            Ordering::Equal => self.index.cmp(&other.index),
        })
    }
}

impl PartialEq for PqEntry {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.record == other.record
    }
}

impl Eq for PqEntry {}

impl PartialOrd for PqEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PqEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.try_cmp(other).unwrap_or(Ordering::Equal)
    }
}

/// Priority queue for merged iteration.
#[derive(Default, Debug, Clone)]
pub struct MergedIterPQueue {
    heap: BinaryHeap<PqEntry>,
}

impl MergedIterPQueue {
    /// Add an entry.
    pub fn push(&mut self, entry: PqEntry) {
        self.heap.push(entry);
    }

    /// Pop top entry.
    pub fn pop(&mut self) -> Option<PqEntry> {
        self.heap.pop()
    }

    /// Peek top entry.
    pub fn peek(&self) -> Option<&PqEntry> {
        self.heap.peek()
    }

    /// True if empty.
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.heap.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{RefRecord, RefValue};

    fn ref_record(name: &str, index: usize) -> PqEntry {
        PqEntry {
            index,
            record: Record::Ref(RefRecord {
                refname: name.into(),
                update_index: index as u64,
                value: RefValue::Deletion,
            }),
        }
    }

    #[test]
    fn ordering_matches_reftable_semantics() {
        let mut pq = MergedIterPQueue::default();
        pq.push(ref_record("refs/heads/b", 0));
        pq.push(ref_record("refs/heads/a", 1));
        pq.push(ref_record("refs/heads/a", 0));

        let first = pq.pop().expect("first");
        let second = pq.pop().expect("second");
        let third = pq.pop().expect("third");

        // key order first, then prefer larger subtable index for equal keys.
        assert_eq!(first.index, 1);
        assert_eq!(second.index, 0);
        assert_eq!(third.record.key(), b"refs/heads/b".to_vec());
    }
}
