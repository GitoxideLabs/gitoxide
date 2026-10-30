//! Rough reproduction of the per-object compression benchmark in
//! <https://github.com/GitoxideLabs/gitoxide/pull/3127>.
//!
//! Run only the documented features in separate invocations: enabling `gix-zlib/sirocco` directly
//! or through other workspace packages can otherwise select Sirocco for both configurations.
//!
//! ```sh
//! cargo run --release -p gix --no-default-features --features sha1 --example gix-zlib-bench
//! cargo run --release -p gix --no-default-features --features sha1,zlib-sirocco --example gix-zlib-bench
//! ```
//!
//! Set `GIX_ZLIB_BENCH_REPOSITORY` to a repository, Git directory, or directory inside a worktree.
//! Otherwise discovery starts in the current directory. The repository is only read, including its
//! alternates and unreachable objects, without Git-environment overrides or replacement objects.
//!
//! Unique object IDs are sorted and every tenth ID is selected, starting with the first. An optional
//! positive `sample-every` argument changes this stride (`1` selects all objects). The selected payloads
//! are decompressed and delta-resolved into memory before timing, and reused across all measurements.
//! Large repositories can require several GiB of RAM; increase the stride for a smaller sample.
//! A second positive argument selects the parallel worker count; by default all available logical
//! cores are used. For example, append `-- 10 4` to benchmark every tenth object with four workers.
//!
//! Each payload gets a fresh `deflate::Write<Vec<u8>>`, one `write_all()`, and a finalizing `flush()`.
//! Construction, compression, allocation, and destruction are timed on one thread. Report the best
//! of three complete passes at levels 1 and 6, decimal MB/s of input and compressed/input size percent.
//! This excludes loose-object headers, pack headers and delta encoding, filesystem I/O, and corpus
//! loading. It does not reproduce the whole-command measurements or guarantee the PR's exact sample.
//!
//! Decompression uses identical zlib-rs streams at levels 1 and 6 in both configurations, prepared and
//! round-trip checked outside timing. Each pass reuses one `Inflate` and a preallocated output buffer;
//! resetting, decoding, and checking stream completion and byte counts are timed, not allocation or
//! full payload comparisons. Report decoded and encoded decimal MB/s, best of three passes. Encoded
//! input digests allow checking that both backends decoded exactly the same bytes.
//!
//! Parallel passes use the same corpus and report aggregate bytes divided by wall-clock time,
//! including thread startup and joining. Objects are distributed round-robin among workers; each
//! decoder has its own `Inflate` and output buffer, allocated before timing. Native threads are used
//! even without the `gix/parallel` feature. The serial passes remain a baseline.

use std::{
    hint::black_box,
    io::Write as _,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use gix::{
    error::{ErrorExt, Result, ResultExt, bail, message},
    hash::ObjectId,
    zlib::{Compression, Inflate, Status, stream::deflate},
};

const REPOSITORY_ENV: &str = "GIX_ZLIB_BENCH_REPOSITORY";
const SAMPLE_EVERY: usize = 10;
const RUNS: usize = 3;

struct Corpus {
    objects: Vec<gix::ObjectDetached>,
    available_objects: usize,
    input_bytes: u64,
    fingerprint: ObjectId,
}

struct Measurement {
    elapsed: Duration,
    compressed_bytes: u64,
}

struct CompressedObject {
    object_id: ObjectId,
    compressed: Vec<u8>,
    decompressed_size: usize,
}

struct CompressedCorpus {
    objects: Vec<CompressedObject>,
    compressed_bytes: u64,
    fingerprint: ObjectId,
}

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let sample_every = match args.next() {
        Some(arg) if arg == "--help" => {
            println!("Usage: gix-zlib-bench [sample-every={SAMPLE_EVERY}] [threads=all]");
            println!("{REPOSITORY_ENV}: input repository; otherwise discover from the current directory.");
            println!(
                "Read-only, serial and parallel compression and decompression at levels 1 and 6; best of {RUNS} runs."
            );
            return Ok(());
        }
        Some(arg) => arg
            .to_string_lossy()
            .parse::<usize>()
            .or_raise(|| message("sample-every must be a positive integer"))?,
        None => SAMPLE_EVERY,
    };
    let threads = match args.next() {
        Some(arg) => arg
            .to_string_lossy()
            .parse::<usize>()
            .or_raise(|| message("threads must be a positive integer"))?,
        None => std::thread::available_parallelism()
            .or_raise(|| message("failed to determine available threads; supply an explicit thread count"))?
            .get(),
    };
    if sample_every == 0 || threads == 0 || args.next().is_some() {
        bail!(message(
            "expected positive sample-every and threads arguments; use --help for usage"
        ));
    }
    if cfg!(debug_assertions) {
        eprintln!("Warning: use --release for comparable codec throughput.");
    }
    let path = match std::env::var_os(REPOSITORY_ENV) {
        Some(path) => PathBuf::from(path),
        None => std::env::current_dir().or_raise(|| message("failed to determine the current directory"))?,
    };
    let setup_start = Instant::now();
    let mut repo = open_repository(&path)?;
    repo.object_cache_size(None);
    println!("repository: {}", repo.git_dir().display());
    println!(
        "gix feature zlib-sirocco: {}; platform: {}-{}",
        cfg!(feature = "zlib-sirocco"),
        std::env::consts::ARCH,
        std::env::consts::OS
    );
    eprintln!("Loading the object sample (stride {sample_every}) into memory before timing...");
    let corpus = load_corpus(&repo, sample_every)?;
    drop(repo);
    println!(
        "sample: {} / {} unique objects, {} input bytes ({:.2} MB), stride {sample_every}",
        corpus.objects.len(),
        corpus.available_objects,
        corpus.input_bytes,
        corpus.input_bytes as f64 / 1_000_000.0
    );
    println!("sample ID digest: {}", corpus.fingerprint);
    println!(
        "setup: {:.3} s (excluded); serial and {threads} parallel workers, best of {RUNS} passes per level",
        setup_start.elapsed().as_secs_f64()
    );
    println!("compression:");
    println!("mode      threads  level  best seconds    aggregate input MB/s    compressed bytes    compressed/input");
    for level in [Compression::BEST_SPEED, Compression::DEFAULT] {
        for (mode, workers) in [("serial", 1), ("parallel", threads)] {
            let mut best = Measurement {
                elapsed: Duration::MAX,
                compressed_bytes: 0,
            };
            for run in 1..=RUNS {
                let measurement = if mode == "serial" {
                    measure_compression(&corpus.objects, level)?
                } else {
                    measure_parallel_compression(&corpus.objects, level, workers)?
                };
                eprintln!(
                    "{mode} compress ({workers} threads) level {} run {run}: {:.6} s, {} compressed bytes",
                    level.level(),
                    measurement.elapsed.as_secs_f64(),
                    measurement.compressed_bytes
                );
                if measurement.elapsed < best.elapsed {
                    best = measurement;
                }
            }
            println!(
                "{mode:<8}  {workers:>7}  {:>5}  {:>12.6}  {:>22.2}  {:>18}  {:>17.2}%",
                level.level(),
                best.elapsed.as_secs_f64(),
                corpus.input_bytes as f64 / 1_000_000.0 / best.elapsed.as_secs_f64(),
                best.compressed_bytes,
                best.compressed_bytes as f64 * 100.0 / corpus.input_bytes as f64
            );
        }
    }
    println!("decompression (identical zlib-rs inputs in both configurations):");
    println!("mode      threads  source level  best seconds    aggregate decoded MB/s    aggregate encoded MB/s");
    for level in [Compression::BEST_SPEED, Compression::DEFAULT] {
        let prepare_start = Instant::now();
        let compressed = prepare_decompression(&corpus, level)?;
        println!(
            "zlib-rs level {} input: {} bytes, digest {}, prepared and verified in {:.3} s (excluded)",
            level.level(),
            compressed.compressed_bytes,
            compressed.fingerprint,
            prepare_start.elapsed().as_secs_f64()
        );
        for (mode, workers) in [("serial", 1), ("parallel", threads)] {
            let mut best = Duration::MAX;
            for run in 1..=RUNS {
                let elapsed = if mode == "serial" {
                    measure_decompression(&compressed)?
                } else {
                    measure_parallel_decompression(&compressed, workers)?.elapsed
                };
                eprintln!(
                    "{mode} decompress ({workers} threads) source level {} run {run}: {:.6} s",
                    level.level(),
                    elapsed.as_secs_f64()
                );
                best = best.min(elapsed);
            }
            println!(
                "{mode:<8}  {workers:>7}  {:>12}  {:>12.6}  {:>24.2}  {:>24.2}",
                level.level(),
                best.as_secs_f64(),
                corpus.input_bytes as f64 / 1_000_000.0 / best.as_secs_f64(),
                compressed.compressed_bytes as f64 / 1_000_000.0 / best.as_secs_f64()
            );
        }
    }
    Ok(())
}

fn open_repository(path: &Path) -> Result<gix::Repository> {
    gix::discover_opts(
        path,
        Default::default(),
        gix::open::Options::isolated().config_overrides(["core.useReplaceRefs=false"]),
    )
    .map(gix::Repository::without_freelist)
    .or_raise(|| message!("failed to discover the input repository at {}", path.display()))
}

fn sample_object_ids(mut object_ids: Vec<ObjectId>, sample_every: usize) -> (usize, Vec<ObjectId>) {
    object_ids.sort_unstable();
    object_ids.dedup();
    (object_ids.len(), object_ids.into_iter().step_by(sample_every).collect())
}

fn load_corpus(repo: &gix::Repository, sample_every: usize) -> Result<Corpus> {
    if sample_every == 0 {
        bail!(message("sample-every must be positive"));
    }
    let object_ids = repo
        .objects
        .iter()
        .or_raise(|| message("failed to enumerate the object database"))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .or_raise(|| message("failed to enumerate an object ID"))?;
    let (available_objects, sample_ids) = sample_object_ids(object_ids, sample_every);
    if sample_ids.is_empty() {
        bail!(message("the input repository contains no objects"));
    }
    let mut objects = Vec::with_capacity(sample_ids.len());
    let mut input_bytes = 0;
    let mut fingerprint = gix::hash::hasher(repo.object_hash());
    for object_id in sample_ids {
        fingerprint.update(object_id.as_bytes());
        let object = repo
            .find_object(object_id)
            .or_raise(|| message!("failed to load benchmark object {object_id}"))?
            .detach();
        input_bytes += object.data.len() as u64;
        objects.push(object);
    }
    if input_bytes == 0 {
        bail!(message(
            "the sample contains no payload bytes; try a smaller sample-every value"
        ));
    }
    Ok(Corpus {
        objects,
        available_objects,
        input_bytes,
        fingerprint: fingerprint.try_finalize()?,
    })
}

fn measure_compression(objects: &[gix::ObjectDetached], level: Compression) -> Result<Measurement> {
    let start = Instant::now();
    let mut compressed_bytes = 0;
    for object in objects {
        compressed_bytes += compress_object(object, level)?;
    }
    Ok(Measurement {
        elapsed: start.elapsed(),
        compressed_bytes,
    })
}

fn compress_object(object: &gix::ObjectDetached, level: Compression) -> Result<u64> {
    let mut writer = deflate::Write::new(Vec::new(), level);
    writer
        .write_all(black_box(&object.data))
        .or_raise(|| message!("failed to compress benchmark object {}", object.id))?;
    writer
        .flush()
        .or_raise(|| message!("failed to finish benchmark object {}", object.id))?;
    Ok(black_box(writer.into_inner()).len() as u64)
}

fn measure_parallel_compression(
    objects: &[gix::ObjectDetached],
    level: Compression,
    threads: usize,
) -> Result<Measurement> {
    measure_parallel(objects, vec![(); threads], |object, _| compress_object(object, level))
}

fn measure_parallel_decompression(corpus: &CompressedCorpus, threads: usize) -> Result<Measurement> {
    let states = (0..threads)
        .map(|worker| {
            let size = corpus
                .objects
                .iter()
                .skip(worker)
                .step_by(threads)
                .map(|object| object.decompressed_size)
                .max()
                .unwrap_or(0)
                + 1;
            (Inflate::default(), vec![0; size])
        })
        .collect();
    measure_parallel(&corpus.objects, states, |object, (inflate, output)| {
        let written = decode_into(object, inflate, output)?;
        black_box(&output[..written]);
        Ok(object.compressed.len() as u64)
    })
}

fn measure_parallel<T: Sync, S: Send>(
    objects: &[T],
    states: Vec<S>,
    consume: impl Fn(&T, &mut S) -> Result<u64> + Sync,
) -> Result<Measurement> {
    let threads = states.len();
    let start = Instant::now();
    let compressed_bytes = std::thread::scope(|scope| -> Result<u64> {
        let mut handles = Vec::with_capacity(threads);
        for (worker, mut state) in states.into_iter().enumerate() {
            let consume = &consume;
            let handle = std::thread::Builder::new()
                .spawn_scoped(scope, move || -> Result<u64> {
                    let mut bytes = 0;
                    // shortcut: static assignment can leave workers idle on skewed corpora; use a work queue if needed.
                    for object in objects.iter().skip(worker).step_by(threads) {
                        bytes += consume(object, &mut state)?;
                    }
                    Ok(bytes)
                })
                .or_raise(|| message("failed to start a benchmark worker"));
            let failed = handle.is_err();
            handles.push(handle);
            if failed {
                break;
            }
        }
        let mut bytes = 0;
        // Join every worker even after failure, so panics cannot escape through the scope.
        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| {
                handle.and_then(|handle| handle.join().map_err(|_| message("benchmark worker panicked").raise()))
            })
            .collect();
        for result in results {
            bytes += result??;
        }
        Ok(bytes)
    })?;
    Ok(Measurement {
        elapsed: start.elapsed(),
        compressed_bytes,
    })
}

/// The standalone compressor always uses zlib-rs, regardless of the selected writer backend.
fn reference_compress(mut input: &[u8], level: Compression) -> Result<Vec<u8>> {
    let mut compressor = deflate::Compress::new(level);
    let mut buffer = [0; 32 * 1024];
    let mut compressed = Vec::new();
    loop {
        let before_in = compressor.total_in();
        let before_out = compressor.total_out();
        let status = compressor.compress(input, &mut buffer, deflate::FlushCompress::Finish)?;
        let consumed = (compressor.total_in() - before_in) as usize;
        let written = (compressor.total_out() - before_out) as usize;
        compressed.extend_from_slice(&buffer[..written]);
        input = &input[consumed..];
        if status == Status::StreamEnd {
            return Ok(compressed);
        }
        if consumed == 0 && written == 0 {
            bail!(message("the reference compressor made no progress"));
        }
    }
}

fn prepare_decompression(corpus: &Corpus, level: Compression) -> Result<CompressedCorpus> {
    let mut objects = Vec::with_capacity(corpus.objects.len());
    let mut compressed_bytes = 0;
    let mut fingerprint = gix::hash::hasher(corpus.fingerprint.kind());
    let mut inflate = Inflate::default();
    let mut output = vec![0; corpus.objects.iter().map(|object| object.data.len()).max().unwrap_or(0) + 1];
    for object in &corpus.objects {
        let compressed = reference_compress(&object.data, level)
            .or_raise(|| message!("failed to prepare decompression input for {}", object.id))?;
        compressed_bytes += compressed.len() as u64;
        fingerprint.update(&(compressed.len() as u64).to_le_bytes());
        fingerprint.update(&compressed);
        let encoded = CompressedObject {
            object_id: object.id,
            compressed,
            decompressed_size: object.data.len(),
        };
        let written = decode_into(&encoded, &mut inflate, &mut output)?;
        if output[..written] != object.data {
            bail!(message!(
                "decompression did not reproduce benchmark object {}",
                object.id
            ));
        }
        objects.push(encoded);
    }
    Ok(CompressedCorpus {
        objects,
        compressed_bytes,
        fingerprint: fingerprint.try_finalize()?,
    })
}

fn decode_into(object: &CompressedObject, inflate: &mut Inflate, output: &mut [u8]) -> Result<usize> {
    inflate.reset();
    // Leave one spare byte so a full payload buffer cannot prevent checking the stream trailer.
    let (status, consumed, written) = inflate
        .once(
            black_box(&object.compressed),
            &mut output[..object.decompressed_size + 1],
        )
        .or_raise(|| message!("failed to decompress benchmark object {}", object.object_id))?;
    if status != Status::StreamEnd || consumed != object.compressed.len() || written != object.decompressed_size {
        bail!(message!(
            "benchmark object {} did not decode completely with the expected byte counts",
            object.object_id
        ));
    }
    Ok(written)
}

fn measure_decompression(corpus: &CompressedCorpus) -> Result<Duration> {
    let mut inflate = Inflate::default();
    let mut output = vec![
        0;
        corpus
            .objects
            .iter()
            .map(|object| object.decompressed_size)
            .max()
            .unwrap_or(0)
            + 1
    ];
    let start = Instant::now();
    for object in &corpus.objects {
        let written = decode_into(object, &mut inflate, &mut output)?;
        black_box(&output[..written]);
    }
    Ok(start.elapsed())
}
