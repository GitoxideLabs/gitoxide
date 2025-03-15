use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail};
use gix::{worktree::archive, NestedProgress, Progress};

pub struct Options {
    pub format: Option<archive::Format>,
    pub files: Vec<(String, String)>,
    pub prefix: Option<String>,
    pub add_paths: Vec<PathBuf>,
}

pub fn stream(
    repo: gix::Repository,
    destination_path: &Path,
    rev_spec: Option<&str>,
    mut progress: impl NestedProgress,
    Options {
        format,
        prefix,
        add_paths,
        files,
    }: Options,
) -> anyhow::Result<()> {
    let format = format.map_or_else(|| format_from_ext(destination_path), Ok)?;
    let object = repo.rev_parse_single(rev_spec.unwrap_or("HEAD"))?.object()?;
    let (modification_date, tree) = fetch_rev_info(object)?;

    let start = std::time::Instant::now();
    let (mut stream, index) = repo.worktree_stream(tree)?;
    if !add_paths.is_empty() {
        let root = gix::path::realpath(
            repo.workdir()
                .ok_or_else(|| anyhow!("Adding files requires a worktree directory that contains them"))?,
        )?;
        for path in add_paths {
            stream.add_entry_from_path(&root, &gix::path::realpath(&path)?)?;
        }
    }
    for (path, content) in files {
        stream.add_entry(gix::worktree::stream::AdditionalEntry {
            id: gix::hash::Kind::Sha1.null(),
            mode: gix::object::tree::EntryKind::Blob.into(),
            relative_path: path.into(),
            source: gix::worktree::stream::entry::Source::Memory(content.into()),
        });
    }

    let mut entries = progress.add_child("entries");
    entries.init(Some(index.entries().len()), gix::progress::count("entries"));
    let mut bytes = progress.add_child("written");
    bytes.init(None, gix::progress::bytes());

    let mut file = gix::progress::Write {
        inner: std::io::BufWriter::with_capacity(128 * 1024, std::fs::File::create(destination_path)?),
        progress: &mut bytes,
    };
    repo.worktree_archive(
        stream,
        &mut file,
        &mut entries,
        &gix::interrupt::IS_INTERRUPTED,
        gix::worktree::archive::Options {
            format,
            tree_prefix: prefix.map(gix::bstr::BString::from),
            modification_time: modification_date.unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or_default() as gix::date::SecondsSinceUnixEpoch
            }),
        },
    )?;

    entries.show_throughput(start);
    bytes.show_throughput(start);

    Ok(())
}

fn fetch_rev_info(
    object: gix::Object<'_>,
) -> anyhow::Result<(Option<gix::date::SecondsSinceUnixEpoch>, gix::ObjectId)> {
    Ok(match object.kind {
        gix::object::Kind::Commit => {
            let commit = object.into_commit();
            (Some(commit.committer()?.time.seconds), commit.tree_id()?.detach())
        }
        gix::object::Kind::Tree => (None, object.id),
        gix::object::Kind::Tag => fetch_rev_info(object.peel_to_kind(gix::object::Kind::Commit)?)?,
        gix::object::Kind::Blob => bail!("Cannot derive commit or tree from blob at {}", object.id),
    })
}

fn format_from_ext(path: &Path) -> anyhow::Result<archive::Format> {
    Ok(match path.extension().and_then(std::ffi::OsStr::to_str) {
        None => bail!("Cannot derive archive format from a file without extension"),
        Some("tar") => archive::Format::Tar,
        Some("gz") => archive::Format::TarGz {
            compression_level: None,
        },
        Some("zip") => archive::Format::Zip {
            compression_level: None,
        },
        Some("stream") => archive::Format::InternalTransientNonPersistable,
        Some(ext) => bail!("Format for extension '{ext}' is unsupported"),
    })
}
