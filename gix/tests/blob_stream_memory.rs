#![allow(clippy::result_large_err)]

use std::{
    alloc::{GlobalAlloc, Layout, System},
    io,
    path::Path,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

use gix::odb::find::Header;
use gix_object::Kind;
use gix_testtools::{tempfile, Result};

#[global_allocator]
static ALLOCATOR: MeasuringAllocator = MeasuringAllocator::new();

struct MeasuringAllocator {
    current: AtomicUsize,
    peak: AtomicUsize,
}

impl MeasuringAllocator {
    const fn new() -> Self {
        Self {
            current: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
        }
    }

    fn prefixed_layout(layout: Layout) -> (Layout, usize) {
        let (layout, offset) = Layout::new::<usize>()
            .extend(layout)
            .expect("prefix layout can be extended");
        (layout.pad_to_align(), offset)
    }

    fn note_increase(&self, size: usize) {
        if size == 0 {
            return;
        }
        let current = self.current.fetch_add(size, Ordering::SeqCst) + size;
        let mut observed = self.peak.load(Ordering::SeqCst);
        while current > observed
            && self
                .peak
                .compare_exchange(observed, current, Ordering::SeqCst, Ordering::SeqCst)
                .is_err()
        {
            observed = self.peak.load(Ordering::SeqCst);
        }
    }

    fn note_decrease(&self, size: usize) {
        if size != 0 {
            self.current.fetch_sub(size, Ordering::SeqCst);
        }
    }

    fn measure<T>(&self, f: impl FnOnce() -> T) -> (T, usize) {
        let baseline = self.current.load(Ordering::SeqCst);
        self.peak.store(baseline, Ordering::SeqCst);
        let value = f();
        (value, self.peak.load(Ordering::SeqCst).saturating_sub(baseline))
    }
}

unsafe impl GlobalAlloc for MeasuringAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let requested = layout.size();
        let (layout, offset) = Self::prefixed_layout(layout);
        let raw = unsafe { System.alloc(layout) };
        if !raw.is_null() {
            self.note_increase(requested);
            unsafe { raw.add(offset) }
        } else {
            raw
        }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let requested = layout.size();
        let (layout, offset) = Self::prefixed_layout(layout);
        let raw = unsafe { System.alloc_zeroed(layout) };
        if !raw.is_null() {
            self.note_increase(requested);
            unsafe { raw.add(offset) }
        } else {
            raw
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let requested = layout.size();
        let (layout, offset) = Self::prefixed_layout(layout);
        self.note_decrease(requested);
        unsafe { System.dealloc(ptr.sub(offset), layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let (old_layout, old_offset) = Self::prefixed_layout(layout);
        let (new_layout, new_offset) =
            Self::prefixed_layout(Layout::from_size_align(new_size, layout.align()).expect("valid layout"));
        let raw = unsafe { System.realloc(ptr.sub(old_offset), old_layout, new_layout.size()) };
        if !raw.is_null() {
            match new_size.cmp(&layout.size()) {
                std::cmp::Ordering::Greater => self.note_increase(new_size - layout.size()),
                std::cmp::Ordering::Less => self.note_decrease(layout.size() - new_size),
                std::cmp::Ordering::Equal => {}
            }
            unsafe { raw.add(new_offset) }
        } else {
            raw
        }
    }
}

fn restricted() -> gix::open::Options {
    gix::open::Options::isolated().config_overrides(["user.name=gitoxide", "user.email=gitoxide@localhost"])
}

fn open_repo(path: &Path) -> Result<gix::Repository> {
    Ok(gix::ThreadSafeRepository::open_opts(path, restricted())?.to_thread_local())
}

fn git(dir: &Path, args: &[&str]) -> Result<()> {
    let output = Command::new("git").current_dir(dir).args(args).output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "git {:?} failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        args,
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .into())
}

fn create_packed_delta_repo() -> Result<tempfile::TempDir> {
    let dir = tempfile::tempdir()?;
    git(dir.path(), &["init"])?;
    git(dir.path(), &["config", "user.name", "gitoxide"])?;
    git(dir.path(), &["config", "user.email", "gitoxide@localhost"])?;

    let blob_path = dir.path().join("blob.bin");
    let mut base = vec![b'a'; 16 * 1024 * 1024];
    for chunk in base.chunks_mut(4096) {
        chunk[0] = b'A';
        chunk[1] = b'0';
    }
    std::fs::write(&blob_path, &base)?;
    git(dir.path(), &["add", "blob.bin"])?;
    git(dir.path(), &["commit", "-m", "base"])?;

    let mut changed = base;
    for idx in (0..changed.len()).step_by(4096) {
        changed[idx + 1] = b'1';
    }
    std::fs::write(&blob_path, &changed)?;
    git(dir.path(), &["add", "blob.bin"])?;
    git(dir.path(), &["commit", "-m", "delta"])?;
    git(
        dir.path(),
        &["repack", "-adf", "--window=250", "--depth=50", "--window-memory=1g"],
    )?;
    git(dir.path(), &["prune-packed"])?;
    Ok(dir)
}

fn packed_delta_blob_id(repo: &gix::Repository) -> Result<(gix::ObjectId, u64)> {
    for id in repo.objects.iter()? {
        let id = id?;
        match repo.try_find_header(id)? {
            Some(Header::Packed(header)) if header.kind == Kind::Blob && header.num_deltas > 0 => {
                return Ok((id, header.object_size));
            }
            _ => {}
        }
    }
    Err("expected at least one packed delta blob".into())
}

#[test]
fn streaming_packed_delta_blobs_uses_less_peak_memory_than_eager_lookup() -> Result {
    let repo_dir = create_packed_delta_repo()?;
    let repo = open_repo(repo_dir.path())?;
    let (id, object_size) = packed_delta_blob_id(&repo)?;
    assert!(object_size >= 16 * 1024 * 1024);
    drop(repo);

    let eager_peak = {
        let repo = open_repo(repo_dir.path())?;
        let (_, peak) = ALLOCATOR.measure(|| {
            let blob = repo.find_blob(id).expect("packed delta blob can be decoded eagerly");
            assert_eq!(blob.data.len() as u64, object_size);
        });
        peak
    };

    let streaming_peak = {
        let repo = open_repo(repo_dir.path())?;
        let (_, peak) = ALLOCATOR.measure(|| {
            let mut stream = repo.find_blob_stream(id).expect("packed delta blob can be streamed");
            assert_eq!(stream.size(), object_size);
            io::copy(&mut stream, &mut io::sink()).expect("streaming copy to sink succeeds");
        });
        peak
    };

    assert!(
        streaming_peak < eager_peak,
        "streaming should lower peak allocations, got eager={eager_peak} and stream={streaming_peak}"
    );
    assert!(
        eager_peak.saturating_sub(streaming_peak) >= 8 * 1024 * 1024,
        "expected a meaningful peak-memory reduction, got eager={eager_peak} and stream={streaming_peak}"
    );
    Ok(())
}
