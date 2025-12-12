use gix_worktree_stream::{Entry, Stream};

use crate::{Error, Format, Options};

#[cfg(feature = "zip")]
use std::io::Write;

/// Write all stream entries in `stream` as provided by `next_entry(stream)` to `out` configured according to `opts` which
/// also includes the streaming format.
///
/// ### Performance
///
/// * The caller should be sure `out` is fast enough. If in doubt, wrap in [`std::io::BufWriter`].
/// * Further, big files aren't suitable for archival into `tar` archives as they require the size of the stream to be known
///   prior to writing the header of each entry.
#[cfg_attr(not(feature = "tar"), allow(unused_mut, unused_variables))]
pub fn write_stream<NextFn>(
    stream: &mut Stream,
    mut next_entry: NextFn,
    out: impl std::io::Write,
    opts: Options,
) -> Result<(), Error>
where
    NextFn: FnMut(&mut Stream) -> Result<Option<Entry<'_>>, gix_worktree_stream::entry::Error>,
{
    if opts.format == Format::InternalTransientNonPersistable {
        return Err(Error::InternalFormatMustNotPersist);
    }
    #[cfg(any(feature = "tar", feature = "tar_gz"))]
    {
        enum State<W: std::io::Write> {
            #[cfg(feature = "tar")]
            Tar((tar::Builder<W>, Vec<u8>)),
            #[cfg(feature = "tar_gz")]
            TarGz((tar::Builder<flate2::write::GzEncoder<W>>, Vec<u8>)),
        }

        impl<W: std::io::Write> State<W> {
            pub fn new(format: Format, mtime: gix_date::SecondsSinceUnixEpoch, out: W) -> Result<Self, Error> {
                match format {
                    Format::InternalTransientNonPersistable => unreachable!("handled earlier"),
                    Format::Zip { .. } => Err(Error::ZipWithoutSeek),
                    Format::Tar => {
                        #[cfg(feature = "tar")]
                        {
                            Ok(State::Tar((
                                {
                                    let mut ar = tar::Builder::new(out);
                                    ar.mode(tar::HeaderMode::Deterministic);
                                    ar
                                },
                                Vec::with_capacity(64 * 1024),
                            )))
                        }
                        #[cfg(not(feature = "tar"))]
                        {
                            Err(Error::SupportNotCompiledIn { wanted: Format::Tar })
                        }
                    }
                    Format::TarGz { compression_level } => {
                        #[cfg(feature = "tar_gz")]
                        {
                            Ok(State::TarGz((
                                {
                                    let gz = flate2::GzBuilder::new().mtime(mtime as u32).write(
                                        out,
                                        match compression_level {
                                            None => flate2::Compression::default(),
                                            Some(level) => flate2::Compression::new(u32::from(level)),
                                        },
                                    );
                                    let mut ar = tar::Builder::new(gz);
                                    ar.mode(tar::HeaderMode::Deterministic);
                                    ar
                                },
                                Vec::with_capacity(64 * 1024),
                            )))
                        }
                        #[cfg(not(feature = "tar_gz"))]
                        {
                            Err(Error::SupportNotCompiledIn {
                                wanted: Format::TarGz {
                                    compression_level: None,
                                },
                            })
                        }
                    }
                }
            }
        }

        let mut state = State::new(opts.format, opts.modification_time, out)?;
        while let Some(entry) = next_entry(stream)? {
            match &mut state {
                #[cfg(feature = "tar")]
                State::Tar((ar, buf)) => {
                    append_tar_entry(ar, buf, entry, opts.modification_time, &opts)?;
                }
                #[cfg(feature = "tar_gz")]
                State::TarGz((ar, buf)) => {
                    append_tar_entry(ar, buf, entry, opts.modification_time, &opts)?;
                }
            }
        }

        match state {
            #[cfg(feature = "tar")]
            State::Tar((mut ar, _)) => {
                ar.finish()?;
            }
            #[cfg(feature = "tar_gz")]
            State::TarGz((ar, _)) => {
                ar.into_inner()?.finish()?;
            }
        }
    }
    Ok(())
}

/// Like [`write_stream()`], but requires [`std::io::Seek`] for `out`.
///
/// Note that `zip` is able to stream big files, which our `tar` implementation is not able to do, which makes it the
/// only suitable container to support huge files from `git-lfs` without consuming excessive amounts of memory.
#[cfg_attr(not(feature = "zip"), allow(unused_mut, unused_variables))]
pub fn write_stream_seek<NextFn>(
    stream: &mut Stream,
    mut next_entry: NextFn,
    out: impl std::io::Write + std::io::Seek,
    opts: Options,
) -> Result<(), Error>
where
    NextFn: FnMut(&mut Stream) -> Result<Option<Entry<'_>>, gix_worktree_stream::entry::Error>,
{
    let compression_level = match opts.format {
        Format::Zip { compression_level } => compression_level.map(i64::from),
        _other => return write_stream(stream, next_entry, out, opts),
    };

    #[cfg(feature = "zip")]
    {
        let mut ar = rawzip::ZipArchiveWriter::new(out);
        let mut buf = Vec::new();
        let mtime = rawzip::time::UtcDateTime::from_unix(opts.modification_time);
        while let Some(entry) = next_entry(stream)? {
            append_zip_entry(
                &mut ar,
                entry,
                &mut buf,
                mtime,
                compression_level,
                opts.tree_prefix.as_ref(),
            )?;
        }
        ar.finish().map_err(std::io::Error::other)?;
    }

    Ok(())
}

#[cfg(feature = "zip")]
fn append_zip_entry<W: std::io::Write + std::io::Seek>(
    ar: &mut rawzip::ZipArchiveWriter<W>,
    mut entry: gix_worktree_stream::Entry<'_>,
    buf: &mut Vec<u8>,
    mtime: rawzip::time::UtcDateTime,
    compression_level: Option<i64>,
    tree_prefix: Option<&bstr::BString>,
) -> Result<(), Error> {
    let path = add_prefix(entry.relative_path(), tree_prefix).into_owned();
    let unix_permissions = if entry.mode.is_executable() { 0o755 } else { 0o644 };
    
    match entry.mode.kind() {
        gix_object::tree::EntryKind::Blob | gix_object::tree::EntryKind::BlobExecutable => {
            use bstr::ByteSlice;
            let file_builder = ar
                .new_file(path.to_str().map_err(|_| {
                    Error::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Invalid UTF-8 in file path",
                    ))
                })?)
                .compression_method(rawzip::CompressionMethod::Deflate)
                .last_modified(mtime)
                .unix_permissions(unix_permissions);
            
            let (mut zip_entry, config) = file_builder.start().map_err(std::io::Error::other)?;
            
            // Use flate2 for compression. Level 9 is the maximum compression level for deflate.
            let encoder = flate2::write::DeflateEncoder::new(
                &mut zip_entry,
                match compression_level {
                    None => flate2::Compression::default(),
                    Some(level) => flate2::Compression::new(level.clamp(0, 9) as u32),
                },
            );
            let mut writer = config.wrap(encoder);
            std::io::copy(&mut entry, &mut writer)?;
            let (encoder, descriptor) = writer.finish().map_err(std::io::Error::other)?;
            encoder.finish()?;
            zip_entry.finish(descriptor).map_err(std::io::Error::other)?;
        }
        gix_object::tree::EntryKind::Tree | gix_object::tree::EntryKind::Commit => {
            use bstr::ByteSlice;
            // rawzip requires directory paths to end with '/'
            let mut dir_path = path.to_str().map_err(|_| {
                Error::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid UTF-8 in directory path",
                ))
            })?.to_string();
            if !dir_path.ends_with('/') {
                dir_path.push('/');
            }
            ar.new_dir(&dir_path)
            .last_modified(mtime)
            .unix_permissions(unix_permissions)
            .create()
            .map_err(std::io::Error::other)?;
        }
        gix_object::tree::EntryKind::Link => {
            use bstr::ByteSlice;
            buf.clear();
            std::io::copy(&mut entry, buf)?;
            
            // For symlinks, we need to create a file with symlink permissions
            let symlink_path = path.to_str().map_err(|_| {
                Error::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid UTF-8 in symlink path",
                ))
            })?;
            let target = buf.as_bstr().to_str().map_err(|_| {
                Error::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid UTF-8 in symlink target",
                ))
            })?;
            
            let (mut zip_entry, config) = ar
                .new_file(symlink_path)
                .compression_method(rawzip::CompressionMethod::Store)
                .last_modified(mtime)
                .unix_permissions(0o120644) // Symlink mode
                .start()
                .map_err(std::io::Error::other)?;
            
            let mut writer = config.wrap(&mut zip_entry);
            writer.write_all(target.as_bytes())?;
            let (_, descriptor) = writer.finish().map_err(std::io::Error::other)?;
            zip_entry.finish(descriptor).map_err(std::io::Error::other)?;
        }
    }
    Ok(())
}

#[cfg(any(feature = "tar", feature = "tar_gz"))]
fn append_tar_entry<W: std::io::Write>(
    ar: &mut tar::Builder<W>,
    buf: &mut Vec<u8>,
    mut entry: gix_worktree_stream::Entry<'_>,
    mtime_seconds_since_epoch: i64,
    opts: &Options,
) -> Result<(), Error> {
    let mut header = tar::Header::new_gnu();
    header.set_mtime(mtime_seconds_since_epoch as u64);
    header.set_entry_type(tar_entry_type(entry.mode));
    header.set_mode(if entry.mode.is_executable() { 0o755 } else { 0o644 });
    buf.clear();
    std::io::copy(&mut entry, buf)?;

    let path = gix_path::from_bstr(add_prefix(entry.relative_path(), opts.tree_prefix.as_ref()));
    header.set_size(buf.len() as u64);

    if entry.mode.is_link() {
        use bstr::ByteSlice;
        let target = gix_path::from_bstr(buf.as_bstr());
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        ar.append_link(&mut header, path, target)?;
    } else {
        ar.append_data(&mut header, path, buf.as_slice())?;
    }
    Ok(())
}

#[cfg(any(feature = "tar", feature = "tar_gz"))]
fn tar_entry_type(mode: gix_object::tree::EntryMode) -> tar::EntryType {
    use gix_object::tree::EntryKind;
    use tar::EntryType;
    match mode.kind() {
        EntryKind::Tree | EntryKind::Commit => EntryType::Directory,
        EntryKind::Blob => EntryType::Regular,
        EntryKind::BlobExecutable => EntryType::Regular,
        EntryKind::Link => EntryType::Link,
    }
}

#[cfg(any(feature = "tar", feature = "tar_gz", feature = "zip"))]
fn add_prefix<'a>(relative_path: &'a bstr::BStr, prefix: Option<&bstr::BString>) -> std::borrow::Cow<'a, bstr::BStr> {
    use std::borrow::Cow;
    match prefix {
        None => Cow::Borrowed(relative_path),
        Some(prefix) => {
            use bstr::ByteVec;
            let mut buf = prefix.clone();
            buf.push_str(relative_path);
            Cow::Owned(buf)
        }
    }
}
