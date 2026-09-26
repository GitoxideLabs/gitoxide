use gix_error::Result;
use std::{path::Path, sync::atomic::AtomicBool};

use gix_error::{Class, ClassificationMarker, ErrorExt, ResultExt};
use gix_features::progress::Progress;

/// Returns the `index` at which the following `index + 1` value is not an increment over the value at `index`.
pub fn fan(data: &[u32]) -> Option<usize> {
    data.windows(2)
        .enumerate()
        .find_map(|(win_index, v)| (v[0] > v[1]).then_some(win_index))
}

/// Calculate the hash of the given kind by trying to read the file from disk at `data_path` or falling back on the mapped content in `data`.
/// `Ok(expected)` is returned if the hash matches, otherwise the error is classified as corruption.
pub fn checksum_on_disk_or_mmap(
    data_path: &Path,
    data: &[u8],
    expected: gix_hash::ObjectId,
    object_hash: gix_hash::Kind,
    progress: &mut dyn Progress,
    should_interrupt: &AtomicBool,
) -> Result<gix_hash::ObjectId> {
    let data_len_without_trailer = data.len() - object_hash.len_in_bytes();
    let actual = match gix_hash::bytes_of_file(
        data_path,
        data_len_without_trailer as u64,
        object_hash,
        progress,
        should_interrupt,
    ) {
        Ok(id) => id,
        Err(err) => match err.downcast_any_ref::<std::io::Error>().map(std::io::Error::kind) {
            Some(std::io::ErrorKind::Interrupted) => {
                return Err(ClassificationMarker::with_source(Class::Retryable, err).raise());
            }
            Some(_) => {
                let start = std::time::Instant::now();
                let mut hasher = gix_hash::hasher(object_hash);
                hasher.update(&data[..data_len_without_trailer]);
                progress.inc_by(data_len_without_trailer);
                progress.show_throughput(start);
                hasher.try_finalize()?
            }
            None => return Err(err),
        },
    };

    actual
        .verify(&expected)
        .or_raise(|| gix_error::corruption("Failed to verify pack file checksum"))?;
    Ok(actual)
}
