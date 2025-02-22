use std::{
    fs,
    io::Read,
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};

use anyhow::{anyhow, Result};
use gix::{
    hash::ObjectId,
    object, objs, odb,
    odb::{loose, pack},
    prelude::Write,
    NestedProgress,
};

#[derive(Default, Clone, Eq, PartialEq, Debug)]
pub enum SafetyCheck {
    SkipFileChecksumVerification,
    SkipFileAndObjectChecksumVerification,
    SkipFileAndObjectChecksumVerificationAndNoAbortOnDecodeError,
    #[default]
    All,
}

impl SafetyCheck {
    pub fn variants() -> &'static [&'static str] {
        &[
            "all",
            "skip-file-checksum",
            "skip-file-and-object-checksum",
            "skip-file-and-object-checksum-and-no-abort-on-decode",
        ]
    }
}

impl std::str::FromStr for SafetyCheck {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "skip-file-checksum" => SafetyCheck::SkipFileChecksumVerification,
            "skip-file-and-object-checksum" => SafetyCheck::SkipFileAndObjectChecksumVerification,
            "skip-file-and-object-checksum-and-no-abort-on-decode" => {
                SafetyCheck::SkipFileAndObjectChecksumVerificationAndNoAbortOnDecodeError
            }
            "all" => SafetyCheck::All,
            _ => return Err(format!("Unknown value for safety check: '{s}'")),
        })
    }
}

impl From<SafetyCheck> for pack::index::traverse::SafetyCheck {
    fn from(v: SafetyCheck) -> Self {
        use pack::index::traverse::SafetyCheck::*;
        match v {
            SafetyCheck::All => All,
            SafetyCheck::SkipFileChecksumVerification => SkipFileChecksumVerification,
            SafetyCheck::SkipFileAndObjectChecksumVerification => SkipFileAndObjectChecksumVerification,
            SafetyCheck::SkipFileAndObjectChecksumVerificationAndNoAbortOnDecodeError => {
                SkipFileAndObjectChecksumVerificationAndNoAbortOnDecodeError
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("An IO error occurred while writing an object")]
    Io(#[from] std::io::Error),
    #[error("An object could not be written to the database")]
    OdbWrite(#[from] loose::write::Error),
    #[error("Failed to write {kind} object {id}")]
    Write {
        source: Box<dyn std::error::Error + Send + Sync>,
        kind: object::Kind,
        id: ObjectId,
    },
    #[error("Object didn't verify after right after writing it")]
    Verify(#[from] objs::data::verify::Error),
    #[error("{kind} object {expected} wasn't re-encoded without change - new hash is {actual}")]
    ObjectEncodeMismatch {
        kind: object::Kind,
        actual: ObjectId,
        expected: ObjectId,
    },
    #[error("The recently written file for loose object {id} could not be found")]
    WrittenFileMissing { id: ObjectId },
    #[error("The recently written file for loose object {id} cold not be read")]
    WrittenFileCorrupt { source: loose::find::Error, id: ObjectId },
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone)]
enum OutputWriter {
    Loose(loose::Store),
    Sink(odb::Sink),
}

impl gix::objs::Write for OutputWriter {
    fn write_buf(&self, kind: object::Kind, from: &[u8]) -> Result<ObjectId, gix::objs::write::Error> {
        match self {
            OutputWriter::Loose(db) => db.write_buf(kind, from),
            OutputWriter::Sink(db) => db.write_buf(kind, from),
        }
    }

    fn write_stream(
        &self,
        kind: object::Kind,
        size: u64,
        from: &mut dyn Read,
    ) -> Result<ObjectId, gix::objs::write::Error> {
        match self {
            OutputWriter::Loose(db) => db.write_stream(kind, size, from),
            OutputWriter::Sink(db) => db.write_stream(kind, size, from),
        }
    }
}

impl OutputWriter {
    fn new(path: Option<impl AsRef<Path>>, compress: bool, object_hash: gix::hash::Kind) -> Self {
        match path {
            Some(path) => OutputWriter::Loose(loose::Store::at(path.as_ref(), object_hash)),
            None => OutputWriter::Sink(odb::sink(object_hash).compress(compress)),
        }
    }
}

#[derive(Default)]
pub struct Context {
    pub thread_limit: Option<usize>,
    pub delete_pack: bool,
    pub sink_compress: bool,
    pub verify: bool,
    pub should_interrupt: Arc<AtomicBool>,
    pub object_hash: gix::hash::Kind,
}

pub fn pack_or_pack_index(
    pack_path: impl AsRef<Path>,
    object_path: Option<impl AsRef<Path>>,
    check: SafetyCheck,
    mut progress: impl NestedProgress + 'static,
    Context {
        thread_limit,
        delete_pack,
        sink_compress,
        verify,
        should_interrupt,
        object_hash,
    }: Context,
) -> Result<()> {
    use anyhow::Context;

    let path = pack_path.as_ref();
    let bundle = pack::Bundle::at(path, object_hash).with_context(|| {
        format!(
            "Could not find .idx or .pack file from given file at '{}'",
            path.display()
        )
    })?;

    if !object_path.as_ref().is_none_or(|p| p.as_ref().is_dir()) {
        return Err(anyhow!(
            "The object directory at '{}' is inaccessible",
            object_path
                .expect("path present if no directory on disk")
                .as_ref()
                .display()
        ));
    }

    let algorithm = object_path.as_ref().map_or_else(
        || {
            if sink_compress {
                pack::index::traverse::Algorithm::Lookup
            } else {
                pack::index::traverse::Algorithm::DeltaTreeLookup
            }
        },
        |_| pack::index::traverse::Algorithm::Lookup,
    );

    let pack::index::traverse::Outcome { .. } = bundle
        .index
        .traverse(
            &bundle.pack,
            &mut progress,
            &should_interrupt,
            {
                let object_path = object_path.map(|p| p.as_ref().to_owned());
                let out = OutputWriter::new(object_path.clone(), sink_compress, object_hash);
                let loose_odb = verify
                    .then(|| object_path.as_ref().map(|path| loose::Store::at(path, object_hash)))
                    .flatten();
                let mut read_buf = Vec::new();
                move |object_kind, buf, index_entry, progress| {
                    let written_id = out.write_buf(object_kind, buf).map_err(|err| Error::Write {
                        source: err,
                        kind: object_kind,
                        id: index_entry.oid,
                    })?;
                    if written_id != index_entry.oid {
                        if let object::Kind::Tree = object_kind {
                            progress.info(format!(
                                "The tree in pack named {} was written as {} due to modes 100664 and 100640 rewritten as 100644.",
                                index_entry.oid, written_id
                            ));
                        } else {
                            return Err(Error::ObjectEncodeMismatch {
                                kind: object_kind,
                                actual: index_entry.oid,
                                expected: written_id,
                            });
                        }
                    }
                    if let Some(verifier) = loose_odb.as_ref() {
                        let obj = verifier
                            .try_find(&written_id, &mut read_buf)
                            .map_err(|err| Error::WrittenFileCorrupt {
                                source: err,
                                id: written_id,
                            })?
                            .ok_or(Error::WrittenFileMissing { id: written_id })?;
                        obj.verify_checksum(&written_id)?;
                    }
                    Ok(())
                }
            },
            pack::index::traverse::Options {
                traversal: algorithm,
                thread_limit,
                check: check.into(),
                make_pack_lookup_cache: pack::cache::lru::StaticLinkedList::<64>::default,
            },
        )
        .with_context(|| "Failed to explode the entire pack - some loose objects may have been created nonetheless")?;

    let (index_path, data_path) = (bundle.index.path().to_owned(), bundle.pack.path().to_owned());
    drop(bundle);

    if delete_pack {
        fs::remove_file(&index_path)
            .and_then(|_| fs::remove_file(&data_path))
            .with_context(|| {
                format!(
                    "Failed to delete pack index file at '{} or data file at '{}'",
                    index_path.display(),
                    data_path.display()
                )
            })?;
        progress.info(format!(
            "Removed '{}' and '{}'",
            index_path.display(),
            data_path.display()
        ));
    }
    Ok(())
}
