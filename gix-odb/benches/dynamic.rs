use std::{
    collections::BTreeSet,
    hint::black_box,
    sync::{
        Arc, Barrier,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group};
use gix_object::Find as _;

#[path = "../tests/tools/odb.rs"]
mod odb_fixture;

use odb_fixture::{Database, OdbFixture, Pack};

fn populated_fixture(with_multi_index: bool) -> OdbFixture {
    let mut fixture = OdbFixture::from_script().expect("the shared ODB fixture is available");
    for pack in Pack::ALL {
        fixture
            .install_pack(Database::Primary, pack)
            .expect("fixture packs can be installed");
    }
    if with_multi_index {
        fixture
            .write_multi_index(Database::Primary, &Pack::ALL)
            .expect("the fixture MIDX can be written");
    }
    fixture
}

fn open(fixture: &OdbFixture, slots: gix_odb::store::init::Slots) -> gix_odb::Handle {
    gix_odb::at_opts(
        fixture.objects_dir(Database::Primary),
        Vec::new(),
        gix_odb::store::init::Options {
            slots,
            object_hash: fixture.manifest.object_hash,
            ..Default::default()
        },
    )
    .expect("the fixture ODB opens")
}

#[derive(Clone, Copy)]
enum RefreshPolicy {
    Strict,
    Never,
}

impl RefreshPolicy {
    const STEADY: [Self; 2] = [Self::Strict, Self::Never];

    fn apply(self, handle: &mut gix_odb::Handle) {
        handle.refresh = match self {
            RefreshPolicy::Strict => gix_odb::store::RefreshMode::AfterAllIndicesLoaded,
            RefreshPolicy::Never => gix_odb::store::RefreshMode::Never,
        };
    }

    fn name(self) -> &'static str {
        match self {
            RefreshPolicy::Strict => "strict",
            RefreshPolicy::Never => "never",
        }
    }

    fn refreshes_per_miss(self) -> usize {
        match self {
            RefreshPolicy::Strict => 1,
            RefreshPolicy::Never => 0,
        }
    }
}

fn bench_open(c: &mut Criterion) {
    let mut group = c.benchmark_group("dynamic/open");
    for (name, slots) in [
        (
            "scan",
            gix_odb::store::init::Slots::AsNeededByDiskState {
                multiplier: 1.1,
                minimum: 32,
            },
        ),
        ("given", gix_odb::store::init::Slots::Given(8)),
    ] {
        group.bench_function(name, |b| {
            b.iter_batched(
                || populated_fixture(false),
                |fixture| black_box(open(&fixture, slots)),
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("dynamic/lookup");
    for with_multi_index in [false, true] {
        let layout = if with_multi_index { "midx" } else { "indices" };
        let fixture = populated_fixture(with_multi_index);
        let id = fixture.manifest.pack(Pack::C).object_ids[0];
        let handle = open(&fixture, gix_odb::store::init::Slots::Given(8));
        let mut buffer = Vec::new();
        handle
            .try_find(&id, &mut buffer)
            .expect("lookup succeeds")
            .expect("fixture object exists");

        group.throughput(Throughput::Elements(1));
        group.bench_function(BenchmarkId::new("warm-hit", layout), |b| {
            b.iter(|| {
                black_box(
                    handle
                        .try_find(black_box(&id), &mut buffer)
                        .expect("lookup succeeds")
                        .expect("fixture object exists")
                        .data
                        .len(),
                )
            });
        });

        group.bench_function(BenchmarkId::new("cold-hit", layout), |b| {
            b.iter_batched(
                || open(&fixture, gix_odb::store::init::Slots::Given(8)),
                |cold| {
                    let mut buffer = Vec::new();
                    black_box(
                        cold.try_find(&id, &mut buffer)
                            .expect("lookup succeeds")
                            .expect("fixture object exists")
                            .data
                            .len(),
                    )
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_missing(c: &mut Criterion) {
    let mut group = c.benchmark_group("dynamic/missing");
    let fixture = populated_fixture(true);
    let missing = fixture.manifest.missing_id();
    for policy in RefreshPolicy::STEADY {
        let mut handle = open(&fixture, gix_odb::store::init::Slots::Given(8));
        handle.packed_object_count().expect("all indices load");
        handle
            .try_find(&missing, &mut Vec::new())
            .expect("the initial missing lookup establishes freshness");
        policy.apply(&mut handle);
        let mut buffer = Vec::new();
        let refreshes = handle.store_ref().metrics().num_refreshes;
        assert!(
            handle
                .try_find(&missing, &mut buffer)
                .expect("policy probe succeeds")
                .is_none(),
            "the policy probe uses a missing object"
        );
        assert_eq!(
            handle.store_ref().metrics().num_refreshes - refreshes,
            policy.refreshes_per_miss(),
            "the benchmark mode must exercise its intended refresh path"
        );
        group.throughput(Throughput::Elements(1));
        group.bench_function(policy.name(), |b| {
            b.iter(|| {
                black_box(
                    handle
                        .try_find(black_box(&missing), &mut buffer)
                        .expect("missing lookup succeeds")
                        .is_none(),
                )
            });
        });
    }
    group.finish();
}

fn bench_prefix(c: &mut Criterion) {
    let mut group = c.benchmark_group("dynamic/disambiguate-prefix");
    let fixture = populated_fixture(true);
    let ids = fixture.manifest.object_ids().collect::<Vec<_>>();
    group.throughput(Throughput::Elements(ids.len() as u64));

    for policy in RefreshPolicy::STEADY {
        let mut handle = open(&fixture, gix_odb::store::init::Slots::Given(8));
        handle.packed_object_count().expect("all indices load");
        handle
            .try_find(&fixture.manifest.missing_id(), &mut Vec::new())
            .expect("the initial missing lookup establishes freshness");
        policy.apply(&mut handle);
        let refreshes = handle.store_ref().metrics().num_refreshes;
        handle
            .disambiguate_prefix(
                gix_odb::store::prefix::disambiguate::Candidate::new(ids[0], 4)
                    .expect("four hex characters form a prefix"),
            )
            .expect("policy probe succeeds");
        assert_eq!(
            handle.store_ref().metrics().num_refreshes - refreshes,
            policy.refreshes_per_miss(),
            "the benchmark mode must exercise its intended refresh path"
        );
        group.bench_function(policy.name(), |b| {
            b.iter(|| {
                for id in &ids {
                    black_box(
                        handle
                            .disambiguate_prefix(
                                gix_odb::store::prefix::disambiguate::Candidate::new(*id, 4)
                                    .expect("four hex characters form a prefix"),
                            )
                            .expect("disambiguation succeeds"),
                    );
                }
            });
        });
    }
    group.finish();
}

fn setup_post_publication(policy: RefreshPolicy) -> (OdbFixture, gix_odb::Handle, gix_hash::ObjectId) {
    let mut fixture = OdbFixture::from_script().expect("the shared ODB fixture is available");
    fixture
        .install_pack(Database::Primary, Pack::A)
        .expect("the initial pack can be installed");
    let mut handle = open(&fixture, gix_odb::store::init::Slots::Given(8));
    handle.packed_object_count().expect("the initial index loads");
    policy.apply(&mut handle);
    fixture
        .install_pack(Database::Primary, Pack::B)
        .expect("the new pack can be published");
    let id = fixture.manifest.pack(Pack::B).object_ids[0];
    (fixture, handle, id)
}

fn bench_post_publication(c: &mut Criterion) {
    let mut group = c.benchmark_group("dynamic/post-publication");
    group.throughput(Throughput::Elements(1));

    for policy in [RefreshPolicy::Strict] {
        {
            let (_fixture, handle, id) = setup_post_publication(policy);
            let refreshes = handle.store_ref().metrics().num_refreshes;
            let mut buffer = Vec::new();
            assert!(
                handle
                    .try_find(&id, &mut buffer)
                    .expect("published-object probe succeeds")
                    .is_some(),
                "an allowed refresh discovers the published object"
            );
            assert_eq!(
                handle.store_ref().metrics().num_refreshes - refreshes,
                1,
                "post-publication lookup performs exactly one refresh"
            );
        }

        group.bench_function(policy.name(), |b| {
            b.iter_batched(
                || setup_post_publication(policy),
                |(_fixture, handle, id)| {
                    let mut buffer = Vec::new();
                    black_box(
                        handle
                            .try_find(&id, &mut buffer)
                            .expect("published-object lookup succeeds")
                            .expect("the published object is discovered")
                            .data
                            .len(),
                    )
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_slot_pressure(c: &mut Criterion) {
    let mut group = c.benchmark_group("dynamic/slot-pressure");
    group.throughput(Throughput::Elements(1));

    for with_multi_index in [false, true] {
        let fixture = populated_fixture(with_multi_index);
        let layout = if with_multi_index { "midx" } else { "indices" };
        let slot_counts = if with_multi_index { &[1][..] } else { &[1, 3][..] };
        for &slots in slot_counts {
            for policy in [RefreshPolicy::Strict, RefreshPolicy::Never] {
                let mut handle = open(&fixture, gix_odb::store::init::Slots::Given(slots));
                policy.apply(&mut handle);
                let id = fixture.manifest.pack(Pack::C).object_ids[0];
                let mut buffer = Vec::new();
                let _ = black_box(handle.try_find(&id, &mut buffer).map(|object| object.is_some()));
                assert!(
                    handle.store_ref().metrics().num_refreshes >= 1,
                    "the setup query initializes the store"
                );

                group.bench_function(BenchmarkId::new(format!("{layout}/{}", policy.name()), slots), |b| {
                    b.iter(|| {
                        black_box(
                            handle
                                .try_find(black_box(&id), &mut buffer)
                                .map(|object| object.is_some()),
                        )
                    });
                });
            }
        }
    }
    group.finish();
}

fn bench_concurrent(c: &mut Criterion) {
    let fixture = populated_fixture(true);
    let ids = Arc::new(fixture.manifest.object_ids().collect::<Vec<_>>());
    let missing = fixture.manifest.missing_id();
    let base = open(&fixture, gix_odb::store::init::Slots::Given(8));
    base.packed_object_count().expect("all indices load");
    let cores = std::thread::available_parallelism().map_or(1, usize::from);
    let worker_counts = [1, cores, cores.saturating_mul(2)].into_iter().collect::<BTreeSet<_>>();

    let mut group = c.benchmark_group("dynamic/concurrent");
    for workers in worker_counts {
        for workload in [
            Workload::Hit,
            Workload::Missing(RefreshPolicy::Strict),
            Workload::Missing(RefreshPolicy::Never),
        ] {
            let refreshes = base.store_ref().metrics().num_refreshes;
            run_workers(base.clone(), Arc::clone(&ids), missing, workers, 1, workload);
            let expected = match workload {
                Workload::Hit => 0,
                Workload::Missing(policy) => policy.refreshes_per_miss(),
            };
            assert_eq!(
                base.store_ref().metrics().num_refreshes - refreshes,
                expected,
                "the concurrent benchmark mode must exercise its intended refresh path"
            );
            group.throughput(Throughput::Elements(1));
            group.bench_with_input(BenchmarkId::new(workload.name(), workers), &workers, |b, &workers| {
                b.iter_custom(|iterations| {
                    run_workers(base.clone(), Arc::clone(&ids), missing, workers, iterations, workload)
                });
            });
        }
    }
    group.finish();
}

#[derive(Clone, Copy)]
enum Workload {
    Hit,
    Missing(RefreshPolicy),
}

impl Workload {
    fn name(self) -> &'static str {
        match self {
            Workload::Hit => "hit",
            Workload::Missing(policy) => policy.name(),
        }
    }
}

fn run_workers(
    base: gix_odb::Handle,
    ids: Arc<Vec<gix_hash::ObjectId>>,
    missing: gix_hash::ObjectId,
    workers: usize,
    iterations: u64,
    workload: Workload,
) -> Duration {
    std::thread::scope(|scope| {
        let ready = Arc::new(Barrier::new(workers + 1));
        let start = Arc::new(AtomicBool::new(false));
        let mut handles = Vec::with_capacity(workers);
        for worker in 0..workers {
            let mut handle = base.clone();
            if let Workload::Missing(policy) = workload {
                policy.apply(&mut handle);
            }
            let ready = Arc::clone(&ready);
            let start = Arc::clone(&start);
            let ids = Arc::clone(&ids);
            let worker_iterations =
                iterations / workers as u64 + u64::from((worker as u64) < iterations % workers as u64);
            handles.push(scope.spawn(move || {
                let mut buffer = Vec::new();
                ready.wait();
                while !start.load(Ordering::Acquire) {
                    std::hint::spin_loop();
                }
                for iteration in 0..worker_iterations {
                    match workload {
                        Workload::Hit => {
                            let id = ids[(iteration as usize + worker) % ids.len()];
                            black_box(
                                handle
                                    .try_find(&id, &mut buffer)
                                    .expect("lookup succeeds")
                                    .expect("fixture object exists")
                                    .data
                                    .len(),
                            );
                        }
                        Workload::Missing(_) => {
                            black_box(
                                handle
                                    .try_find(&missing, &mut buffer)
                                    .expect("missing lookup succeeds")
                                    .is_none(),
                            );
                        }
                    }
                }
            }));
        }
        ready.wait();
        let before = Instant::now();
        start.store(true, Ordering::Release);
        for handle in handles {
            handle.join().expect("benchmark worker does not panic");
        }
        before.elapsed()
    })
}

criterion_group!(
    benches,
    bench_open,
    bench_lookup,
    bench_missing,
    bench_prefix,
    bench_post_publication,
    bench_slot_pressure,
    bench_concurrent
);

fn main() {
    benches();
    Criterion::default().configure_from_args().final_summary();
}
