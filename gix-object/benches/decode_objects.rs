use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn parse_commit(c: &mut Criterion) {
    c.bench_function("CommitRef(sig)", |b| {
        b.iter(|| {
            black_box(gix_object::CommitRef::from_bytes(
                COMMIT_WITH_MULTI_LINE_HEADERS,
                gix_hash::Kind::Sha1,
            ))
            .unwrap()
        });
    });
    c.bench_function("CommitRefIter(sig)", |b| {
        b.iter(|| {
            black_box(
                gix_object::CommitRefIter::from_bytes(COMMIT_WITH_MULTI_LINE_HEADERS, gix_hash::Kind::Sha1).count(),
            )
        });
    });
}

fn parse_tag(c: &mut Criterion) {
    c.bench_function("TagRef(sig)", |b| {
        b.iter(|| black_box(gix_object::TagRef::from_bytes(TAG_WITH_SIGNATURE, gix_hash::Kind::Sha1)).unwrap());
    });
    c.bench_function("TagRefIter(sig)", |b| {
        b.iter(|| black_box(gix_object::TagRefIter::from_bytes(TAG_WITH_SIGNATURE, gix_hash::Kind::Sha1).count()));
    });
}

fn parse_tree(c: &mut Criterion) {
    let hash_kind = gix_testtools::object_hash();
    c.bench_function("TreeRef()", |b| {
        b.iter(|| black_box(gix_object::TreeRef::from_bytes(TREE, hash_kind)).unwrap());
    });
    c.bench_function("TreeRefIter()", |b| {
        b.iter(|| black_box(gix_object::TreeRefIter::from_bytes(TREE, hash_kind).count()));
    });
}

criterion_group!(benches, parse_commit, parse_tag, parse_tree);
criterion_main!(benches);

const COMMIT_WITH_MULTI_LINE_HEADERS: &[u8] = include_bytes!("../tests/fixtures/commit/two-multiline-headers.txt");
const TAG_WITH_SIGNATURE: &[u8] = include_bytes!("../tests/fixtures/tag/signed.txt");
const TREE: &[u8] = include_bytes!("../tests/fixtures/tree/everything.tree");
