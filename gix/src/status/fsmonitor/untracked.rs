use std::{path::Path, sync::atomic::Ordering};

#[cfg(windows)]
use crate::error::ResultExt;
use crate::error::{ErrorExt, bail, message};
use crate::{
    Repository, Result,
    bstr::{BStr, BString, ByteSlice},
};
use gix_dir::{
    entry::Kind,
    walk::{CachedListing, EmissionMode},
};
use gix_index::extension::UntrackedCache;

impl super::Refresh {
    pub(in crate::status) fn cached_listing<'index>(
        &mut self,
        repo: &Repository,
        index: &'index gix_index::State,
        lookup: Option<&gix_index::AccelerateLookup<'index>>,
        options: Option<crate::dirwalk::Options>,
        has_pathspec: bool,
    ) -> Result<Option<CachedListing>> {
        let _span = gix_trace::detail!("gix::status::fsmonitor::cached_listing");
        let Some(cache) = &mut self.untracked else {
            return Ok(None);
        };
        if !self.use_untracked {
            return Ok(None);
        }
        let Some(options) = options else { return Ok(None) };
        let options = gix_dir::walk::Options::from(options);
        let flags = if options.emit_untracked == EmissionMode::CollapseDirectory {
            6
        } else {
            0
        };
        if cache.dir_flags() != flags
            || has_pathspec
            || options.recurse_repositories
            || options.emit_ignored.is_some()
            || options.emit_tracked
            || options.emit_pruned
            || options.emit_empty_directories
            || options.emit_collapsed.is_some()
            || options.for_deletion.is_some()
            || options.symlinks_to_directories_are_ignored_like_directories
        {
            gix_trace::debug!("untracked cache options differ from Git's cached walk");
            return Ok(None);
        }
        // An uncertain tracked entry must not create a phantom child in a cached listing.
        for (entry, valid) in index.entries().iter().zip(&self.valid) {
            if !valid.load(Ordering::Relaxed)
                || entry
                    .flags
                    .intersects(gix_index::entry::Flags::SKIP_WORKTREE | gix_index::entry::Flags::ASSUME_VALID)
            {
                *self.changed.get_mut() |= cache.invalidate_path(entry.path(index));
            }
        }
        let listing = listing(repo, index, cache, lookup);
        match listing {
            Ok(Some(listing)) => {
                gix_trace::debug!(
                    cached_directories = listing.directories.len(),
                    "prepared untracked directory cache"
                );
                Ok(Some(listing))
            }
            Ok(None) => {
                gix_trace::debug!("precious ignore patterns differ from Git's cached rules; scanning directories");
                Ok(None)
            }
            Err(_err) => {
                gix_trace::debug!(reason = %_err, "untracked cache validation failed; scanning directories");
                *self.changed.get_mut() |= cache.invalidate_all();
                Ok(None)
            }
        }
    }
}

fn listing<'index>(
    repo: &Repository,
    index: &'index gix_index::State,
    cache: &UntrackedCache,
    lookup: Option<&gix_index::AccelerateLookup<'index>>,
) -> Result<Option<CachedListing>> {
    let root = crate::path::realpath_opts(
        repo.workdir().expect("status has a worktree"),
        repo.current_dir(),
        crate::path::realpath::MAX_SYMLINKS,
    )?;
    // Git for Windows resolves worktree aliases through a file handle, including short DOS paths.
    #[cfg(windows)]
    let root = root.canonicalize().or_error()?;
    let system = match std::env::consts::OS {
        "linux" | "android" => "Linux",
        "macos" | "ios" | "tvos" | "watchos" | "visionos" => "Darwin",
        "windows" => "Windows",
        "freebsd" => "FreeBSD",
        "openbsd" => "OpenBSD",
        "netbsd" => "NetBSD",
        "dragonfly" => "DragonFly",
        "solaris" | "illumos" => "SunOS",
        _ => bail!(message("unsupported untracked cache system identity")),
    };
    let mut identifier: BString = "Location ".into();
    let root_bytes = crate::path::to_unix_separators_on_windows(crate::path::into_bstr(root.clone())?);
    #[cfg(windows)]
    let root_bytes: BString = if let Some(unc) = root_bytes.strip_prefix(b"//?/UNC/") {
        [b"//".as_slice(), unc].concat().into()
    } else {
        root_bytes.strip_prefix(b"//?/").unwrap_or(&root_bytes).into()
    };
    identifier.extend_from_slice(&root_bytes);
    identifier.extend_from_slice(b", system ");
    identifier.extend_from_slice(system.as_bytes());
    if cache.identifier().split(|byte| *byte == 0).next() != Some(identifier.as_slice())
        || cache.exclude_filename_per_dir() != ".gitignore"
    {
        bail!(message("untracked cache belongs to another worktree or system"));
    }
    let global = repo.config.excludes_file(&mut |_| {})?;
    let support_precious = repo.config.ignore_pattern_parser()?.support_precious;
    if !validate_ignore(
        &repo.common_dir().join("info/exclude"),
        cache.info_exclude().map(|value| value.id),
        repo.object_hash(),
        support_precious,
    )? || !match global {
        Some(path) => validate_ignore(
            &path,
            cache.excludes_file().map(|value| value.id),
            repo.object_hash(),
            support_precious,
        )?,
        None if cache.excludes_file().is_none() => true,
        None => bail!(message("global ignore rules changed")),
    } {
        return Ok(None);
    }
    let mut out = CachedListing::default();
    let mut pending = vec![(0, BString::default())];
    while let Some((position, path)) = pending.pop() {
        let Some(dir) = cache.directories().get(position) else {
            continue;
        };
        if let Some(lookup) = lookup
            && let Some(entry) = index.entry_closest_to_directory_or_directory_icase(path.as_bstr(), true, lookup)
            && entry.path(index).get(..path.len()) != Some(path.as_slice())
        {
            bail!(message("cached directory spelling differs from the index"));
        }
        if (dir.stat().is_none() || dir.exclude_file_oid().is_some())
            && !validate_ignore(
                &root.join(crate::path::from_bstr(&path)?).join(".gitignore"),
                dir.exclude_file_oid(),
                repo.object_hash(),
                support_precious,
            )?
        {
            return Ok(None);
        }
        for &child in dir.sub_directories() {
            let name = cache.directories()[child].name();
            if !component(name) {
                bail!(message("invalid cached directory name"));
            }
            pending.push((child, join(path.as_bstr(), name)));
        }
        // UNTR identifies directories but cannot distinguish files from symlinks; read_dir obtains their types together.
        if dir.stat().is_none() || dir.check_only() || dir.untracked_entries().iter().any(|name| !name.ends_with(b"/"))
        {
            continue;
        }
        let mut entries = Vec::new();
        for name in dir.untracked_entries().iter().map(|name| name.as_bstr()).chain(
            dir.sub_directories()
                .iter()
                .map(|&child| cache.directories()[child].name()),
        ) {
            let name = name.strip_suffix(b"/").unwrap_or(name).as_bstr();
            if !component(name) {
                bail!(message("invalid cached entry name"));
            }
            entries.push((name.to_owned(), Kind::Directory));
        }
        out.directories.insert(path, entries);
    }
    // UNTR omits tracked names, which still prevent a mixed directory from collapsing.
    let mut previous = b"".as_slice();
    for entry in index.entries() {
        let path = entry.path(index);
        let mut end = path.len();
        let mut kind = if entry.mode.is_submodule() {
            Kind::Directory
        } else if entry.mode == gix_index::entry::Mode::SYMLINK {
            Kind::Symlink
        } else {
            Kind::File
        };
        loop {
            let start = path[..end].rfind_byte(b'/').map_or(0, |position| position + 1);
            let parent = path[..start.saturating_sub(1)].as_bstr();
            if let Some(entries) = out.directories.get_mut(parent) {
                entries.push((path[start..end].into(), kind));
            }
            // Adjacent paths share parents that have already been added and checked.
            if start != 0 && previous.starts_with(&path[..start]) {
                break;
            }
            // More than one index spelling can map to the same canonical directory.
            if let Some(lookup) = lookup
                && let Some(entry) = index.entry_closest_to_directory_or_directory_icase(parent, true, lookup)
                && entry.path(index).get(..parent.len()) != Some(parent.as_ref())
            {
                bail!(message("index directory spellings differ"));
            }
            if start == 0 {
                break;
            }
            end = start - 1;
            kind = Kind::Directory;
        }
        previous = path.as_ref();
    }
    for entries in out.directories.values_mut() {
        entries.sort_unstable();
        entries.dedup_by(|a, b| a.0 == b.0);
    }
    Ok(Some(out))
}

fn component(name: &BStr) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(&b'/') && !name.contains(&b'\\')
}

fn join(parent: &BStr, name: &BStr) -> BString {
    let mut path = parent.to_owned();
    if !path.is_empty() {
        path.push(b'/');
    }
    path.extend_from_slice(name);
    path
}

fn validate_ignore(
    path: &Path,
    expected: Option<gix_hash::ObjectId>,
    hash: gix_hash::Kind,
    support_precious: bool,
) -> Result<bool> {
    let mut bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound && expected.is_none() => return Ok(true),
        Err(err) => return Err(err.raise()),
    };
    let Some(expected) = expected else {
        bail!(message!("Ignore rules changed at '{}'", path.display()));
    };
    if gix_object::compute_hash(hash, gix_object::Kind::Blob, &bytes)? != expected {
        // Git's ignore parser appends a newline before hashing files read from disk.
        if !bytes.is_empty() {
            bytes.push(b'\n');
        }
        if gix_object::compute_hash(hash, gix_object::Kind::Blob, &bytes)? != expected {
            bail!(message!("Ignore rules changed at '{}'", path.display()));
        }
    }
    // Incompatible parser settings skip reuse without invalidating Git's unchanged cache.
    Ok(!support_precious || gix_ignore::parse(&bytes, true).eq(gix_ignore::parse(&bytes, false)))
}
