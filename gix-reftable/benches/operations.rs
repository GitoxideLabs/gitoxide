use std::{hint::black_box, time::Duration};

use bstr::BString;
use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use gix_hash::{Kind, ObjectId};
use gix_reftable::{
    CompactOptions, Limits, LockOptions, RefRecord, RefValue, SnapshotOptions, Stack, Table, WriteOptions, Writer,
};

fn records(count: usize, update_index: u64) -> Vec<RefRecord> {
    records_from(0, count, update_index)
}

fn records_from(start: usize, end: usize, update_index: u64) -> Vec<RefRecord> {
    (start..end)
        .map(|index| RefRecord {
            name: BString::from(if index % 2 == 0 {
                format!("refs/heads/feature/{index:08}")
            } else {
                format!("refs/tags/release/{index:08}")
            }),
            update_index,
            value: RefValue::Direct(ObjectId::from([((index % 255) + 1) as u8; 20])),
        })
        .collect()
}

fn table(count: usize) -> Table {
    let bytes = Writer::new(WriteOptions {
        object_hash: Kind::Sha1,
        ..WriteOptions::default()
    })
    .write(&records(count, 1), &[])
    .expect("benchmark records form a valid table");
    Table::from_bytes(&bytes, Limits::default()).expect("the benchmark writer emits a readable table")
}

fn table_operations(c: &mut Criterion) {
    for count in [100, 10_000] {
        let table = table(count);
        let lookup = format!("refs/heads/feature/{:08}", count - 2);
        let mut group = c.benchmark_group("reftable/read");
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::new("exact lookup", count), &count, |b, _| {
            b.iter(|| black_box(table.find_ref(black_box(lookup.as_bytes()))));
        });
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::new("full iteration", count), &count, |b, _| {
            b.iter(|| {
                black_box(table.refs().fold(0, |seen, record| {
                    black_box(record);
                    seen + 1
                }))
            });
        });
        group.throughput(Throughput::Elements(count.div_ceil(2) as u64));
        group.bench_with_input(BenchmarkId::new("prefix iteration", count), &count, |b, _| {
            b.iter(|| {
                black_box(table.refs_with_prefix(b"refs/heads/feature/").fold(0, |seen, record| {
                    black_box(record);
                    seen + 1
                }))
            });
        });
        group.finish();
    }
}

fn writes(c: &mut Criterion) {
    let writer = Writer::new(WriteOptions {
        object_hash: Kind::Sha1,
        ..WriteOptions::default()
    });
    let mut group = c.benchmark_group("reftable/write");
    for count in [100, 10_000] {
        let refs = records(count, 1);
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| black_box(writer.write(black_box(&refs), &[]).expect("benchmark input is valid")));
        });
    }
    group.finish();
}

fn stack_with_generations(count: usize) -> (tempfile::TempDir, Stack) {
    let temp = tempfile::tempdir().expect("create benchmark directory");
    let stack = Stack::create(
        temp.path().join("reftable"),
        Kind::Sha1,
        SnapshotOptions::default(),
        Limits::default(),
    )
    .expect("create benchmark stack");
    let generations = 8;
    let per_generation = count.div_ceil(generations);
    for generation in 0..generations {
        let start = generation * per_generation;
        let end = count.min(start + per_generation);
        if start == end {
            break;
        }
        let addition = stack
            .begin_addition(LockOptions {
                timeout: Duration::ZERO,
            })
            .expect("lock benchmark stack");
        let update_index = addition.next_update_index();
        addition
            .commit(&records_from(start, end, update_index), &[])
            .expect("append benchmark generation");
    }
    (temp, stack)
}

fn compaction(c: &mut Criterion) {
    let mut group = c.benchmark_group("reftable/compact eight generations");
    for count in [100, 10_000] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, count| {
            b.iter_batched(
                || stack_with_generations(*count),
                |(temp, stack)| {
                    let outcome = stack
                        .compact(
                            CompactOptions::default(),
                            LockOptions {
                                timeout: Duration::ZERO,
                            },
                        )
                        .expect("compact benchmark stack");
                    (black_box(outcome.snapshot.refs().count()), temp)
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

criterion_group!(benches, table_operations, writes, compaction);
criterion_main!(benches);
