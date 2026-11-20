use std::{
    fs, io,
    sync::atomic::{AtomicBool, Ordering},
    time::SystemTime,
};

use gix_error::{Result, ResultExt, bail, message};

use super::Store;

/// Settings for [`Store::prune()`].
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Preserve files modified after this time, even if they are not protected by the caller.
    pub expire: SystemTime,
    /// Count the objects that would be removed without changing the object database.
    pub dry_run: bool,
}

/// Statistics returned by [`Store::prune()`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// Number of removed objects, or candidates for removal in a dry run.
    pub pruned_objects: usize,
    /// Compressed bytes removed, or candidates for removal in a dry run.
    pub pruned_bytes: u64,
}

impl Store {
    /// Remove unprotected loose objects whose modification time is at most `options.expire`.
    ///
    /// `is_protected` must include the complete reachable object graph, including objects
    /// reachable from recent unreachable objects. Compute it before pruning: preserving a
    /// recent object alone does not protect its older dependencies. The caller is responsible
    /// for repository policy, including precious objects, reflogs, indices, and concurrent writers.
    ///
    /// Only regular files in the immediate hexadecimal fanout directories are considered.
    /// Symbolic links, unexpected files, packs, alternates, and temporary files are left alone.
    /// Modification times are read after calling `is_protected`, just before deletion, to honor
    /// concurrent object freshening. This does not make pruning atomic with concurrent writes;
    /// callers should choose an expiry interval appropriate for their repository.
    ///
    /// A failure or interruption may leave some objects pruned. Files that disappear during
    /// iteration are ignored. Empty fanout directories are retained.
    pub fn prune(
        &self,
        mut is_protected: impl FnMut(&gix_hash::oid) -> bool,
        options: Options,
        should_interrupt: &AtomicBool,
    ) -> Result<Outcome> {
        let mut outcome = Outcome::default();
        for first_byte in 0..=255u8 {
            check_interrupt(should_interrupt)?;
            let directory = self.path.join(format!("{first_byte:02x}"));
            let metadata = match fs::symlink_metadata(&directory) {
                Ok(metadata) => metadata,
                Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
                Err(err) => return Err(err).or_raise(|| message!("Could not inspect {}", directory.display())),
            };
            if !metadata.is_dir() {
                continue;
            }
            for entry in
                fs::read_dir(&directory).or_raise(|| message!("Could not enumerate {}", directory.display()))?
            {
                check_interrupt(should_interrupt)?;
                let entry = entry.or_raise(|| message!("Could not enumerate {}", directory.display()))?;
                let filename = entry.file_name();
                let Some(filename) = filename.to_str() else { continue };
                if filename.len() != self.object_hash.len_in_hex() - 2 {
                    continue;
                }
                let mut hex = gix_hash::Kind::hex_buf();
                let prefix = format!("{first_byte:02x}");
                hex[..2].copy_from_slice(prefix.as_bytes());
                hex[2..self.object_hash.len_in_hex()].copy_from_slice(filename.as_bytes());
                let Ok(object_id) = gix_hash::ObjectId::from_hex(&hex[..self.object_hash.len_in_hex()]) else {
                    continue;
                };
                if is_protected(&object_id) {
                    continue;
                }
                check_interrupt(should_interrupt)?;
                let path = entry.path();
                let metadata = match fs::symlink_metadata(&path) {
                    Ok(metadata) => metadata,
                    Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
                    Err(err) => return Err(err).or_raise(|| message!("Could not inspect {}", path.display())),
                };
                if !metadata.is_file()
                    || metadata
                        .modified()
                        .or_raise(|| message!("Could not read modification time of {}", path.display()))?
                        > options.expire
                {
                    continue;
                }
                if !options.dry_run {
                    match fs::remove_file(&path) {
                        Ok(()) => {}
                        Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
                        Err(err) => return Err(err).or_raise(|| message!("Could not prune {}", path.display())),
                    }
                }
                outcome.pruned_objects += 1;
                outcome.pruned_bytes += metadata.len();
            }
        }
        Ok(outcome)
    }
}

fn check_interrupt(should_interrupt: &AtomicBool) -> Result<()> {
    if should_interrupt.load(Ordering::Relaxed) {
        bail!(message("Loose object pruning interrupted").cancelled());
    }
    Ok(())
}
