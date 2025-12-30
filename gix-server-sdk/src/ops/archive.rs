use std::convert::Infallible;

use bstr::BString;
use gix_hash::ObjectId;
use gix_object::Find;
use gix_odb::HeaderExt;

use crate::error::{Result, SdkError};
use crate::RepoHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    Tar,
    TarGz { compression_level: Option<u8> },
    Zip { compression_level: Option<u8> },
}

pub trait StreamWriter: std::io::Write + Send {}
impl<T: std::io::Write + Send> StreamWriter for T {}

pub trait SeekableStreamWriter: std::io::Write + std::io::Seek + Send {}
impl<T: std::io::Write + std::io::Seek + Send> SeekableStreamWriter for T {}

impl From<ArchiveFormat> for gix_archive::Format {
    fn from(format: ArchiveFormat) -> Self {
        match format {
            ArchiveFormat::Tar => gix_archive::Format::Tar,
            ArchiveFormat::TarGz { compression_level } => {
                gix_archive::Format::TarGz { compression_level }
            }
            ArchiveFormat::Zip { compression_level } => {
                gix_archive::Format::Zip { compression_level }
            }
        }
    }
}

pub fn create_archive<W: StreamWriter>(
    repo: &RepoHandle,
    tree_id: ObjectId,
    format: ArchiveFormat,
    prefix: Option<BString>,
    writer: W,
) -> Result<()> {
    let local = repo.to_local();

    let header = local
        .objects
        .header(&tree_id)
        .map_err(|_| SdkError::ObjectNotFound(tree_id))?;

    if header.kind() != gix_object::Kind::Tree {
        return Err(SdkError::InvalidObjectType {
            expected: "tree".to_string(),
            actual: header.kind().to_string(),
        });
    }

    let pipeline = gix_filter::Pipeline::new(Default::default(), Default::default());
    let objects = local.objects.clone().into_arc().map_err(|e| SdkError::Git(Box::new(e)))?;

    let mut stream = gix_worktree_stream::from_tree(
        tree_id,
        objects,
        pipeline,
        |_, _, _| -> std::result::Result<(), Infallible> { Ok(()) },
    );

    let opts = gix_archive::Options {
        format: format.into(),
        tree_prefix: prefix,
        modification_time: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|t| t.as_secs() as i64)
            .unwrap_or_default(),
    };

    match format {
        ArchiveFormat::Zip { .. } => {
            return Err(SdkError::Operation(
                "Zip format requires a seekable writer, use create_archive_seekable instead".to_string(),
            ));
        }
        _ => {
            gix_archive::write_stream(
                &mut stream,
                gix_worktree_stream::Stream::next_entry,
                writer,
                opts,
            )
            .map_err(|e| SdkError::Git(Box::new(e)))?;
        }
    }

    Ok(())
}

pub fn create_archive_seekable<W: SeekableStreamWriter>(
    repo: &RepoHandle,
    tree_id: ObjectId,
    format: ArchiveFormat,
    prefix: Option<BString>,
    writer: W,
) -> Result<()> {
    let local = repo.to_local();

    let header = local
        .objects
        .header(&tree_id)
        .map_err(|_| SdkError::ObjectNotFound(tree_id))?;

    if header.kind() != gix_object::Kind::Tree {
        return Err(SdkError::InvalidObjectType {
            expected: "tree".to_string(),
            actual: header.kind().to_string(),
        });
    }

    let pipeline = gix_filter::Pipeline::new(Default::default(), Default::default());
    let objects = local.objects.clone().into_arc().map_err(|e| SdkError::Git(Box::new(e)))?;

    let mut stream = gix_worktree_stream::from_tree(
        tree_id,
        objects,
        pipeline,
        |_, _, _| -> std::result::Result<(), Infallible> { Ok(()) },
    );

    let opts = gix_archive::Options {
        format: format.into(),
        tree_prefix: prefix,
        modification_time: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|t| t.as_secs() as i64)
            .unwrap_or_default(),
    };

    gix_archive::write_stream_seek(
        &mut stream,
        gix_worktree_stream::Stream::next_entry,
        writer,
        opts,
    )
    .map_err(|e| SdkError::Git(Box::new(e)))?;

    Ok(())
}

pub fn create_archive_from_commit<W: StreamWriter>(
    repo: &RepoHandle,
    commit_id: ObjectId,
    format: ArchiveFormat,
    prefix: Option<BString>,
    writer: W,
) -> Result<()> {
    let local = repo.to_local();
    let mut buf = Vec::new();

    let commit_data = local
        .objects
        .try_find(&commit_id, &mut buf)
        .map_err(|e| SdkError::Git(e))?
        .ok_or_else(|| SdkError::ObjectNotFound(commit_id))?;

    if commit_data.kind != gix_object::Kind::Commit {
        return Err(SdkError::InvalidObjectType {
            expected: "commit".to_string(),
            actual: commit_data.kind.to_string(),
        });
    }

    let commit = gix_object::CommitRef::from_bytes(&buf)?;
    let tree_id = commit.tree();

    create_archive(repo, tree_id, format, prefix, writer)
}

pub fn create_archive_from_commit_seekable<W: SeekableStreamWriter>(
    repo: &RepoHandle,
    commit_id: ObjectId,
    format: ArchiveFormat,
    prefix: Option<BString>,
    writer: W,
) -> Result<()> {
    let local = repo.to_local();
    let mut buf = Vec::new();

    let commit_data = local
        .objects
        .try_find(&commit_id, &mut buf)
        .map_err(|e| SdkError::Git(e))?
        .ok_or_else(|| SdkError::ObjectNotFound(commit_id))?;

    if commit_data.kind != gix_object::Kind::Commit {
        return Err(SdkError::InvalidObjectType {
            expected: "commit".to_string(),
            actual: commit_data.kind.to_string(),
        });
    }

    let commit = gix_object::CommitRef::from_bytes(&buf)?;
    let tree_id = commit.tree();

    create_archive_seekable(repo, tree_id, format, prefix, writer)
}
