use gix_reftable::{
    pq::{MergedIterPQueue, PqEntry},
    record::{Record, RefRecord, RefValue},
};

fn entry(name: &str, idx: usize) -> PqEntry {
    PqEntry {
        index: idx,
        record: Record::Ref(RefRecord {
            refname: name.into(),
            update_index: idx as u64,
            value: RefValue::Deletion,
        }),
    }
}

// Upstream mapping: test_reftable_pq__record + merged_iter_pqueue_top
#[test]
fn pq_record_order() {
    let mut pq = MergedIterPQueue::default();
    pq.push(entry("refs/heads/b", 0));
    pq.push(entry("refs/heads/a", 0));
    pq.push(entry("refs/heads/a", 1));

    assert_eq!(pq.pop().unwrap().index, 1);
    assert_eq!(pq.pop().unwrap().record.key(), b"refs/heads/a".to_vec());
    assert_eq!(pq.pop().unwrap().record.key(), b"refs/heads/b".to_vec());
}
