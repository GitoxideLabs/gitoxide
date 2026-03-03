use std::path::Path;

use crate::{
    basics::HashId,
    basics::{get_be32, get_be64},
    block::{footer_size, header_size, Block, BlockIter},
    blocksource::BlockSource,
    constants,
    error::Error,
    record::Record,
};

const FORMAT_ID_SHA1: u32 = 0x7368_6131;
const FORMAT_ID_SHA256: u32 = 0x7332_3536;

/// Metadata for a section inside a table.
#[derive(Debug, Clone, Copy, Default)]
pub struct TableOffsets {
    /// True if the section is present.
    pub is_present: bool,
    /// Section offset in bytes.
    pub offset: u64,
    /// Optional index offset in bytes.
    pub index_offset: u64,
}

/// A single reftable file.
#[derive(Debug, Clone)]
pub struct Table {
    /// Name of this table.
    pub name: String,
    /// Underlying block source.
    pub source: BlockSource,
    /// Data size excluding footer.
    pub size: u64,
    /// Hash used by this table.
    pub hash_id: HashId,
    /// Reftable format version.
    pub version: u8,
    /// Configured block size (0 for unaligned).
    pub block_size: u32,
    /// Minimum update index encoded in this table.
    pub min_update_index: u64,
    /// Maximum update index encoded in this table.
    pub max_update_index: u64,
    /// Object-id abbreviation length in `o` section.
    pub object_id_len: u8,
    /// Offsets for refs.
    pub ref_offsets: TableOffsets,
    /// Offsets for object index.
    pub obj_offsets: TableOffsets,
    /// Offsets for logs.
    pub log_offsets: TableOffsets,
}

impl Table {
    /// Open a table from a file.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let name = path
            .file_name()
            .map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned());
        let source = BlockSource::from_file(path)?;
        Self::from_block_source(name, source)
    }

    /// Build a table from a block source.
    pub fn from_block_source(name: String, source: BlockSource) -> Result<Self, Error> {
        let file_size = source.size();
        let max_header_size = header_size(2)? + 1;
        if file_size < max_header_size as u64 {
            return Err(Error::Malformed("reftable too small"));
        }

        let header = source.read(0, max_header_size as u32)?.to_vec();
        if header.len() < max_header_size || &header[..4] != b"REFT" {
            return Err(Error::Malformed("missing REFT header"));
        }
        let version = header[4];
        if version != 1 && version != 2 {
            return Err(Error::Malformed("unsupported reftable version"));
        }

        let footer_len = footer_size(version)? as u64;
        if file_size < footer_len {
            return Err(Error::Malformed("reftable too small for footer"));
        }
        let size = file_size - footer_len;

        let footer = source.read(size, footer_len as u32)?.to_vec();
        if footer.len() != footer_len as usize {
            return Err(Error::Truncated);
        }

        Self::parse(name, source, &header, &footer, version, size)
    }

    fn parse(
        name: String,
        source: BlockSource,
        header: &[u8],
        footer: &[u8],
        version: u8,
        size: u64,
    ) -> Result<Self, Error> {
        let mut pos = 0usize;
        if &footer[..4] != b"REFT" {
            return Err(Error::Malformed("footer magic mismatch"));
        }
        pos += 4;

        let version_header_len = header_size(version)?;
        if footer.len() < version_header_len || header.len() < version_header_len + 1 {
            return Err(Error::Truncated);
        }
        if footer[..version_header_len] != header[..version_header_len] {
            return Err(Error::Malformed("header/footer prefix mismatch"));
        }

        pos += 1; // version
        let mut be24 = [0u8; 4];
        be24[1..].copy_from_slice(&footer[pos..pos + 3]);
        let block_size = get_be32(&be24);
        pos += 3;

        let mut be64 = [0u8; 8];
        be64.copy_from_slice(&footer[pos..pos + 8]);
        let min_update_index = get_be64(&be64);
        pos += 8;
        be64.copy_from_slice(&footer[pos..pos + 8]);
        let max_update_index = get_be64(&be64);
        pos += 8;

        let hash_id = if version == 1 {
            HashId::Sha1
        } else {
            let mut be32 = [0u8; 4];
            be32.copy_from_slice(&footer[pos..pos + 4]);
            pos += 4;
            match get_be32(&be32) {
                FORMAT_ID_SHA1 => HashId::Sha1,
                FORMAT_ID_SHA256 => HashId::Sha256,
                _ => return Err(Error::Malformed("unknown hash format id")),
            }
        };

        be64.copy_from_slice(&footer[pos..pos + 8]);
        let ref_index_offset = get_be64(&be64);
        pos += 8;

        be64.copy_from_slice(&footer[pos..pos + 8]);
        let mut obj_offset_field = get_be64(&be64);
        pos += 8;
        let object_id_len = (obj_offset_field & ((1 << 5) - 1)) as u8;
        obj_offset_field >>= 5;

        be64.copy_from_slice(&footer[pos..pos + 8]);
        let obj_index_offset = get_be64(&be64);
        pos += 8;

        be64.copy_from_slice(&footer[pos..pos + 8]);
        let log_offset = get_be64(&be64);
        pos += 8;

        be64.copy_from_slice(&footer[pos..pos + 8]);
        let log_index_offset = get_be64(&be64);
        pos += 8;

        let crc_expected = crc32fast::hash(&footer[..pos]);
        let mut be32 = [0u8; 4];
        be32.copy_from_slice(&footer[pos..pos + 4]);
        let crc_actual = get_be32(&be32);
        if crc_expected != crc_actual {
            return Err(Error::ChecksumMismatch);
        }

        let first_block_typ = header[version_header_len];
        let ref_offsets = TableOffsets {
            is_present: first_block_typ == constants::BLOCK_TYPE_REF,
            offset: 0,
            index_offset: ref_index_offset,
        };
        let obj_offsets = TableOffsets {
            is_present: obj_offset_field > 0,
            offset: obj_offset_field,
            index_offset: obj_index_offset,
        };
        if obj_offsets.is_present && object_id_len == 0 {
            return Err(Error::Malformed("object section present without object_id_len"));
        }
        let log_offsets = TableOffsets {
            is_present: first_block_typ == constants::BLOCK_TYPE_LOG || log_offset > 0,
            offset: log_offset,
            index_offset: log_index_offset,
        };

        Ok(Self {
            name,
            source,
            size,
            hash_id,
            version,
            block_size,
            min_update_index,
            max_update_index,
            object_id_len,
            ref_offsets,
            obj_offsets,
            log_offsets,
        })
    }

    /// Return the offset metadata for a given record block type.
    pub fn offsets_for(&self, typ: u8) -> Result<TableOffsets, Error> {
        match typ {
            constants::BLOCK_TYPE_REF => Ok(self.ref_offsets),
            constants::BLOCK_TYPE_LOG => Ok(self.log_offsets),
            constants::BLOCK_TYPE_OBJ => Ok(self.obj_offsets),
            _ => Err(Error::Malformed("unsupported table section type")),
        }
    }

    /// Decode one block at `offset`.
    pub fn init_block(&self, offset: u64, want_type: u8) -> Result<Option<Block>, Error> {
        if offset >= self.size {
            return Ok(None);
        }
        let header_off = if offset == 0 {
            header_size(self.version)? as u32
        } else {
            0
        };
        Block::init(
            &self.source,
            offset,
            header_off,
            self.block_size,
            self.hash_id.size(),
            want_type,
        )
    }

    /// Create an iterator for records of type `typ`.
    pub fn iter(&self, typ: u8) -> Result<TableIter<'_>, Error> {
        TableIter::new(self, typ)
    }
}

/// Iterator for all records of one section in a single table.
pub struct TableIter<'a> {
    table: &'a Table,
    typ: u8,
    start_off: u64,
    block_off: u64,
    block_iter: Option<BlockIter>,
    finished: bool,
}

impl<'a> TableIter<'a> {
    fn new(table: &'a Table, typ: u8) -> Result<Self, Error> {
        let offsets = table.offsets_for(typ)?;
        if !offsets.is_present {
            return Ok(Self {
                table,
                typ,
                start_off: 0,
                block_off: 0,
                block_iter: None,
                finished: true,
            });
        }

        let mut iter = Self {
            table,
            typ,
            start_off: offsets.offset,
            block_off: offsets.offset,
            block_iter: None,
            finished: false,
        };
        iter.seek_to(offsets.offset, typ)?;
        Ok(iter)
    }

    fn seek_to(&mut self, off: u64, typ: u8) -> Result<(), Error> {
        let Some(block) = self.table.init_block(off, typ)? else {
            self.finished = true;
            self.block_iter = None;
            return Ok(());
        };

        self.block_off = off;
        self.block_iter = Some(BlockIter::new(block));
        self.finished = false;
        Ok(())
    }

    fn next_block(&mut self) -> Result<(), Error> {
        let Some(current) = self.block_iter.as_ref() else {
            self.finished = true;
            return Ok(());
        };

        let next_off = self
            .block_off
            .checked_add(current.block().full_block_size as u64)
            .ok_or(Error::Malformed("block offset overflow"))?;
        self.seek_to(next_off, self.typ)
    }

    /// Position iterator at the first record whose key is >= `want`.
    pub fn seek_key(&mut self, want: &[u8]) -> Result<(), Error> {
        self.seek_to(self.start_off, self.typ)?;
        if self.finished {
            return Ok(());
        }

        loop {
            let Some(_current) = self.block_iter.as_ref() else {
                return Ok(());
            };

            let mut probe = self.clone_for_probe();
            probe.next_block()?;
            if probe.finished {
                break;
            }

            let Some(probe_block) = probe.block_iter.as_ref() else {
                break;
            };
            let first_key = probe_block.block().first_key()?;
            if first_key.as_slice() > want {
                break;
            }

            self.block_off = probe.block_off;
            self.block_iter = probe.block_iter;
            self.finished = probe.finished;
        }

        if let Some(block_iter) = self.block_iter.as_mut() {
            block_iter.seek_key(want)?;
        }
        Ok(())
    }

    fn clone_for_probe(&self) -> Self {
        Self {
            table: self.table,
            typ: self.typ,
            start_off: self.start_off,
            block_off: self.block_off,
            block_iter: self.block_iter.clone(),
            finished: self.finished,
        }
    }

    /// Return the next record, if any.
    pub fn next_record(&mut self) -> Result<Option<Record>, Error> {
        loop {
            if self.finished {
                return Ok(None);
            }

            if let Some(block_iter) = self.block_iter.as_mut() {
                if let Some(mut rec) = block_iter.next_record()? {
                    if let Record::Ref(ref mut r) = rec {
                        r.update_index = r
                            .update_index
                            .checked_add(self.table.min_update_index)
                            .ok_or(Error::VarintOverflow)?;
                    }
                    return Ok(Some(rec));
                }
            }

            self.next_block()?;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::{constants, record::Record};

    use super::Table;

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("valid time")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("gix-reftable-{stamp}"));
            fs::create_dir_all(&path).expect("temp dir");
            Self { path }
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn run(cwd: &Path, args: &[&str]) {
        let status = Command::new(args[0])
            .args(&args[1..])
            .current_dir(cwd)
            .status()
            .expect("command executed");
        assert!(status.success(), "command failed: {args:?}");
    }

    fn create_reftable_repo() -> Option<(TempDir, PathBuf)> {
        let tmp = TempDir::new();
        let source = tmp.path.join("source");
        let clone = tmp.path.join("clone");
        fs::create_dir_all(&source).expect("source dir");

        run(&source, &["git", "init", "-q"]);
        run(&source, &["git", "config", "user.name", "committer"]);
        run(&source, &["git", "config", "user.email", "committer@example.com"]);
        run(&source, &["git", "config", "commit.gpgSign", "false"]);
        fs::write(source.join("file"), "hello\n").expect("write file");
        run(&source, &["git", "add", "file"]);
        run(&source, &["git", "commit", "-q", "-m", "c1"]);

        let clone_status = Command::new("git")
            .args(["clone", "--ref-format=reftable"])
            .arg(source.to_str().expect("utf-8 path"))
            .arg(clone.to_str().expect("utf-8 path"))
            .current_dir(&tmp.path)
            .status()
            .ok()?;
        if !clone_status.success() {
            return None;
        }

        let list = fs::read_to_string(clone.join(".git/reftable/tables.list")).ok()?;
        let table_name = list.lines().next()?;
        Some((tmp, clone.join(".git/reftable").join(table_name)))
    }

    #[test]
    fn open_table_and_iterate_refs_and_logs() {
        let Some((_tmp, table_path)) = create_reftable_repo() else {
            return;
        };
        let table = Table::open(&table_path).expect("open table");

        let mut refs = table.iter(constants::BLOCK_TYPE_REF).expect("ref iter");
        let mut saw_head = false;
        while let Some(rec) = refs.next_record().expect("next ref") {
            if let Record::Ref(ref_record) = rec {
                if ref_record.refname == "HEAD" {
                    saw_head = true;
                }
            }
        }
        assert!(saw_head, "HEAD must be present in reftable refs");

        let mut logs = table.iter(constants::BLOCK_TYPE_LOG).expect("log iter");
        let mut log_count = 0usize;
        loop {
            match logs.next_record() {
                Ok(Some(_)) => log_count += 1,
                Ok(None) => break,
                Err(err) => panic!("next log #{log_count} failed: {err:?}"),
            }
        }
        assert!(log_count > 0, "expected log records in cloned repository");
    }

    #[test]
    fn seek_by_key_in_ref_section() {
        let Some((_tmp, table_path)) = create_reftable_repo() else {
            return;
        };
        let table = Table::open(&table_path).expect("open table");

        let mut refs = table.iter(constants::BLOCK_TYPE_REF).expect("ref iter");
        refs.seek_key(b"refs/heads/").expect("seek works");
        let rec = refs.next_record().expect("record read").expect("record exists");
        let Record::Ref(rec) = rec else {
            panic!("expected ref record");
        };
        assert!(rec.refname.starts_with("refs/heads/"));
    }
}
