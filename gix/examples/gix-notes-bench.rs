//! Benchmark creating, packing, and reading one Git note per commit reachable from `HEAD`.
//!
//! Run from the workspace root, selecting a repository and an unused notes ref:
//!
//! ```sh
//! cargo run --release -p gix --no-default-features --features sha1,notes \
//!   --example gix-notes-bench -- /path/to/repository all refs/notes/gix-notes-bench
//! ```
//!
//! Append `--no-compression` after the notes ref to disable pack compression (zlib level 0 instead of 6).
//!
//! Replace `all` with a positive limit for a smaller run. Traverse `HEAD` and its ancestors, stopping
//! when the limit is reached. A commit-graph file is optional and used as a traversal cache if available.
//! Each note contains the annotated commit's hexadecimal ID followed by a newline.
//! One retained notes state stages all edits before serializing
//! the final trees and a single notes commit into memory. Packing includes compression, index creation,
//! integrity verification, and syncing both files. Reading verifies every note using a fresh ODB and
//! notes state with a warm OS file cache. The destination ref is published only after verification.
//!
//! To see these notes in `git log` or `tix`, export `GIT_NOTES_DISPLAY_REF` before running either
//! command in the target repository (use your chosen destination ref if it differs from the example):
//!
//! ```sh
//! export GIT_NOTES_DISPLAY_REF=refs/notes/gix-notes-bench
//! ```

use std::{
    fs::File,
    io::Cursor,
    path::PathBuf,
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

use gix::{
    note::plumbing::State,
    objs::{FindExt, Kind, Write},
};
use gix_pack::data::output::{self, bytes::FromEntriesIter};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
type Memory = gix::odb::memory::Proxy<gix::objs::find::Never>;
const NOTES_REF: &str = "refs/notes/gix-notes-bench";

fn main() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let repository = args.next().map_or_else(|| ".".into(), PathBuf::from);
    if repository.as_os_str() == "--help" {
        println!("Usage: gix-notes-bench [repository=.] [limit=all] [ref={NOTES_REF}] [--no-compression]");
        println!("Annotate commits reachable from HEAD; refuse an existing notes ref.");
        println!("--no-compression: disable pack compression (default: zlib level 6).");
        return Ok(());
    }
    let limit = args
        .next()
        .filter(|arg| arg != "all")
        .map(|arg| arg.to_string_lossy().parse::<usize>())
        .transpose()?
        .unwrap_or(usize::MAX);
    let notes_ref = args
        .next()
        .map(|arg| arg.into_string().map_err(|_| "the notes ref must be UTF-8"))
        .transpose()?
        .unwrap_or_else(|| NOTES_REF.into());
    let compression = match args.next() {
        Some(arg) if arg == "--no-compression" => gix::zlib::Compression::NONE,
        Some(_) => return Err("expected --no-compression after the notes ref".into()),
        None => gix::zlib::Compression::DEFAULT,
    };
    if args.next().is_some() || limit == 0 || !notes_ref.starts_with("refs/notes/") {
        return Err("expected a repository, a positive limit, and a ref under refs/notes/".into());
    }

    let setup_start = Instant::now();
    let repo = gix::open(&repository)?;
    if repo.try_find_reference(notes_ref.as_str())?.is_some() {
        return Err(format!("{notes_ref} already exists; choose a new notes ref").into());
    }
    let hash = repo.object_hash();
    let commit_ids = collect_commit_ids(&repo, limit)?;
    if commit_ids.is_empty() {
        return Err("no commits are reachable from HEAD".into());
    }
    let count = commit_ids.len();
    println!("repository: {}", repo.git_dir().display());
    println!("commits: {count}");
    println!("payload: commit ID plus newline ({} bytes/note)", hash.len_in_hex() + 1);
    println!("compression_level: {}", compression.level());
    println!("setup_seconds: {:.6}", setup_start.elapsed().as_secs_f64());

    let write_start = Instant::now();
    let mut objects = Memory::new(gix::objs::find::Never, hash);
    let empty_tree_id = objects.write_buf(Kind::Tree, &[])?;
    let mut state = State::new(empty_tree_id, &objects).map_err(gix::Exn::into_error)?;
    let mut payload = vec![b'\n'; hash.len_in_hex() + 1];
    let mut last_report = Instant::now();
    for (index, &commit_id) in commit_ids.iter().enumerate() {
        let _ = commit_id.hex_to_buf(&mut payload[..hash.len_in_hex()]);
        let note_blob_id = objects.write_buf(Kind::Blob, &payload)?;
        let previous = state
            .edit(commit_id, Some(note_blob_id), &objects)
            .map_err(gix::Exn::into_error)?;
        assert!(previous.is_none(), "each commit must be annotated exactly once");
        if (index + 1).is_multiple_of(50_000) && last_report.elapsed() >= Duration::from_secs(1) {
            eprintln!(
                "created {}/{count} notes in {:.1}s",
                index + 1,
                write_start.elapsed().as_secs_f64()
            );
            last_report = Instant::now();
        }
    }
    // Serialize the retained notes tree once, into the same in-memory object database.
    let root_tree_id = state.write(&objects).map_err(gix::Exn::into_error)?;
    drop(state);
    let signature = gix::actor::Signature {
        name: "gix-notes-bench".into(),
        email: "gix-notes-bench@example.com".into(),
        time: gix::date::Time::now_utc(),
    };
    let notes_commit_id = objects.write(&gix::objs::Commit {
        tree: root_tree_id,
        parents: Default::default(),
        author: signature.clone(),
        committer: signature,
        encoding: None,
        message: format!("Attach benchmark notes to {count} commits\n").into(),
        extra_headers: Vec::new(),
    })?;
    let create_time = write_start.elapsed();
    report("create_in_memory", create_time, count);

    let pack_start = Instant::now();
    let mut storage = objects.take_object_memory().expect("object memory is enabled");
    drop(objects);
    // The bootstrap empty tree is the only superseded object; all other objects are final.
    storage.remove(&empty_tree_id);
    let object_count = u32::try_from(storage.len())?;
    let blob_count = storage.values().filter(|(kind, _)| *kind == Kind::Blob).count();
    assert_eq!(blob_count, count, "each commit has a distinct note blob");
    println!("packed_objects: {object_count}");
    println!(
        "object_bytes: {}",
        storage.values().map(|(_, data)| data.len()).sum::<usize>()
    );
    let entries = storage.drain().map(|(object_id, (kind, data))| {
        output::Entry::from_data(
            &output::Count::from_data(object_id, None),
            &gix::objs::Data {
                kind,
                data: &data,
                object_hash: hash,
            },
            compression,
        )
        .map(|entry| vec![entry])
    });
    let mut encoder = FromEntriesIter::new(
        entries,
        Vec::new(),
        object_count,
        gix_pack::data::Version::default(),
        hash,
    );
    for chunk in encoder.by_ref() {
        chunk?;
    }
    let pack = encoder.into_write();
    drop(storage);
    println!("pack_bytes: {}", pack.len());
    let pack_dir = repo.objects.store().path().join("pack");
    let outcome = gix_pack::Bundle::write_to_directory(
        &mut Cursor::new(pack),
        Some(&pack_dir),
        &mut gix::progress::Discard,
        &AtomicBool::new(false),
        None::<gix::objs::find::Never>,
        hash,
        gix_pack::bundle::write::Options {
            thread_limit: Some(1),
            ..Default::default()
        },
    )?;
    let pack_path = outcome.data_path.as_ref().ok_or("no pack was written")?;
    let index_path = outcome.index_path.as_ref().ok_or("no pack index was written")?;
    // Windows requires write access for `FlushFileBuffers`, which backs `sync_all()`.
    File::options().write(true).open(pack_path)?.sync_all()?;
    File::options().write(true).open(index_path)?.sync_all()?;
    let pack_time = pack_start.elapsed();
    report("pack_and_index", pack_time, count);
    report("write_total", create_time + pack_time, count);
    println!("pack: {}", pack_path.display());
    println!("notes_commit: {notes_commit_id}");

    // Read through a fresh on-disk ODB, with no in-memory objects or parsed notes state.
    // The operating system's file cache is intentionally warm from writing the pack.
    let read_start = Instant::now();
    let disk = gix::odb::at(repo.objects.store().path(), hash)?;
    let mut buffer = Vec::new();
    let persisted_tree_id = disk.find_commit(&notes_commit_id, &mut buffer)?.tree();
    assert_eq!(
        persisted_tree_id, root_tree_id,
        "the packed commit references the final notes tree"
    );
    let mut state = State::new(persisted_tree_id, &disk).map_err(gix::Exn::into_error)?;
    for commit_id in &commit_ids {
        let note_blob_id = state
            .get(commit_id, &disk)
            .map_err(gix::Exn::into_error)?
            .ok_or_else(|| format!("missing note for {commit_id}"))?;
        let actual = disk.find_blob(&note_blob_id, &mut buffer)?;
        let _ = commit_id.hex_to_buf(&mut payload[..hash.len_in_hex()]);
        assert_eq!(actual.data, payload, "note payload for {commit_id}");
    }
    report("read_and_verify", read_start.elapsed(), count);

    // Publish only after the complete pack round-trip succeeds. Never replace an existing ref.
    repo.reference(
        notes_ref.as_str(),
        notes_commit_id,
        gix::refs::transaction::PreviousValue::MustNotExist,
        "gix-notes benchmark",
    )?;
    if let Some(keep_path) = outcome.keep_path {
        std::fs::remove_file(keep_path)?;
    }
    println!("verified_notes: {count}");
    println!("notes_ref: {notes_ref}");
    Ok(())
}

fn collect_commit_ids(repo: &gix::Repository, limit: usize) -> Result<Vec<gix::ObjectId>> {
    let mut commit_ids = Vec::new();
    for commit in repo.head_id()?.ancestors().all()?.take(limit) {
        commit_ids.push(commit?.id);
    }
    // Sorting by the suffix spreads edits over the notes fanout.
    commit_ids.sort_unstable_by(|a, b| a.as_bytes()[8..].cmp(&b.as_bytes()[8..]));
    Ok(commit_ids)
}

fn report(phase: &str, elapsed: Duration, count: usize) {
    println!("{phase}_seconds: {:.6}", elapsed.as_secs_f64());
    println!("{phase}_notes_per_second: {:.0}", count as f64 / elapsed.as_secs_f64());
}
