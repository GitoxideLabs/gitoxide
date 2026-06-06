use std::io::Write;

use crate::{
    basics::{put_be24, put_be32, put_be64, HashId},
    block::{footer_size, header_size},
    blocksource::BlockSource,
    constants,
    error::Error,
    record::{encode_key, IndexRecord, LogRecord, LogValue, Record, RefRecord},
    table::Table,
};

const FORMAT_ID_SHA1: u32 = 0x7368_6131;
const FORMAT_ID_SHA256: u32 = 0x7332_3536;

/// Options controlling writing behavior.
#[derive(Debug, Clone)]
pub struct WriteOptions {
    /// If true, do not pad non-log blocks to `block_size`.
    pub unpadded: bool,
    /// Desired block size for non-log blocks.
    pub block_size: u32,
    /// If true, skip object reverse index generation.
    pub skip_index_objects: bool,
    /// Restart key interval.
    pub restart_interval: u16,
    /// Hash function used in written tables.
    pub hash_id: HashId,
    /// If false, log messages are normalized to one line ending in `\n`.
    pub exact_log_message: bool,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            unpadded: false,
            block_size: 4096,
            skip_index_objects: true,
            restart_interval: 16,
            hash_id: HashId::Sha1,
            exact_log_message: false,
        }
    }
}

#[derive(Debug, Clone)]
struct SectionResult {
    bytes: Vec<u8>,
    index_offset: u64,
}

/// Writer for single reftable files.
#[derive(Debug, Clone)]
pub struct Writer {
    opts: WriteOptions,
    min_update_index: Option<u64>,
    max_update_index: Option<u64>,
    refs: Vec<RefRecord>,
    logs: Vec<LogRecord>,
}

impl Writer {
    /// Create a new writer.
    pub fn new(opts: WriteOptions) -> Self {
        Self {
            opts,
            min_update_index: None,
            max_update_index: None,
            refs: Vec::new(),
            logs: Vec::new(),
        }
    }

    /// Set update-index limits used by records in this table.
    pub fn set_limits(&mut self, min: u64, max: u64) -> Result<(), Error> {
        if min > max {
            return Err(Error::Api("min_update_index must be <= max_update_index"));
        }
        if !self.refs.is_empty() || !self.logs.is_empty() {
            return Err(Error::Api("set_limits must be called before adding records"));
        }
        self.min_update_index = Some(min);
        self.max_update_index = Some(max);
        Ok(())
    }

    /// Add one ref record.
    pub fn add_ref(&mut self, rec: RefRecord) -> Result<(), Error> {
        self.check_limits(rec.update_index)?;
        if !self.logs.is_empty() {
            return Err(Error::Api("cannot add ref after logs"));
        }
        self.refs.push(rec);
        Ok(())
    }

    /// Add one log record.
    pub fn add_log(&mut self, mut rec: LogRecord) -> Result<(), Error> {
        self.check_limits(rec.update_index)?;
        if !self.opts.exact_log_message {
            normalize_log_message(&mut rec)?;
        }
        self.logs.push(rec);
        Ok(())
    }

    fn check_limits(&self, update_index: u64) -> Result<(), Error> {
        let (min, max) = self
            .limits()
            .ok_or(Error::Api("set_limits must be called before adding records"))?;
        if update_index < min || update_index > max {
            return Err(Error::Api("record update index outside set limits"));
        }
        Ok(())
    }

    fn limits(&self) -> Option<(u64, u64)> {
        Some((self.min_update_index?, self.max_update_index?))
    }

    /// Finalize and return table bytes.
    pub fn finish(mut self) -> Result<Vec<u8>, Error> {
        let (min_update_index, max_update_index) = self
            .limits()
            .ok_or(Error::Api("set_limits must be called before finish"))?;

        self.refs.sort_by(|a, b| a.refname.cmp(&b.refname));
        self.logs.sort_by(|a, b| {
            let by_name = a.refname.cmp(&b.refname);
            if by_name == std::cmp::Ordering::Equal {
                b.update_index.cmp(&a.update_index)
            } else {
                by_name
            }
        });

        let version = match self.opts.hash_id {
            HashId::Sha1 => 1,
            HashId::Sha256 => 2,
        };
        let header_len = header_size(version)?;

        let mut ref_records = Vec::with_capacity(self.refs.len());
        for rec in &self.refs {
            let mut rec = rec.clone();
            rec.update_index = rec
                .update_index
                .checked_sub(min_update_index)
                .ok_or(Error::Api("ref update index must be >= min_update_index"))?;
            ref_records.push(Record::Ref(rec));
        }
        let log_records = self.logs.into_iter().map(Record::Log).collect::<Vec<_>>();

        let first_section = if !ref_records.is_empty() {
            Some(constants::BLOCK_TYPE_REF)
        } else if !log_records.is_empty() {
            Some(constants::BLOCK_TYPE_LOG)
        } else {
            None
        };

        let mut file = if first_section.is_some() {
            Vec::new()
        } else {
            vec![0u8; header_len]
        };

        let mut ref_index_offset = 0u64;
        let mut log_offset = 0u64;
        let mut log_index_offset = 0u64;

        if let Some(first) = first_section {
            if first == constants::BLOCK_TYPE_REF {
                let ref_section = write_section(&ref_records, constants::BLOCK_TYPE_REF, 0, header_len, &self.opts)?;
                file.extend_from_slice(&ref_section.bytes);
                ref_index_offset = ref_section.index_offset;

                if !log_records.is_empty() {
                    log_offset = file.len() as u64;
                    let log_section =
                        write_section(&log_records, constants::BLOCK_TYPE_LOG, log_offset, 0, &self.opts)?;
                    file.extend_from_slice(&log_section.bytes);
                    log_index_offset = log_section.index_offset;
                }
            } else {
                log_offset = 0;
                let log_section = write_section(&log_records, constants::BLOCK_TYPE_LOG, 0, header_len, &self.opts)?;
                file.extend_from_slice(&log_section.bytes);
                log_index_offset = log_section.index_offset;
            }
        }

        let header = encode_header(
            version,
            self.opts.block_size,
            min_update_index,
            max_update_index,
            self.opts.hash_id,
        )?;
        if file.len() < header_len {
            file.resize(header_len, 0);
        }
        file[..header_len].copy_from_slice(&header);

        let footer = encode_footer(version, &header, ref_index_offset, 0, 0, log_offset, log_index_offset)?;
        file.extend_from_slice(&footer);
        Ok(file)
    }

    /// Finalize directly into a [`Table`] instance.
    pub fn finish_into_table(self, name: &str) -> Result<Table, Error> {
        let bytes = self.finish()?;
        Table::from_block_source(name.into(), BlockSource::from_bytes(bytes))
    }
}

fn normalize_log_message(log: &mut LogRecord) -> Result<(), Error> {
    if let LogValue::Update { message, .. } = &mut log.value {
        if message.is_empty() {
            return Ok(());
        }
        if message.trim_end_matches('\n').contains('\n') {
            return Err(Error::Api(
                "log message must be a single line unless exact_log_message is set",
            ));
        }
        if !message.ends_with('\n') {
            message.push('\n');
        }
    }
    Ok(())
}

fn encode_header(
    version: u8,
    block_size: u32,
    min_update_index: u64,
    max_update_index: u64,
    hash_id: HashId,
) -> Result<Vec<u8>, Error> {
    let header_len = header_size(version)?;
    let mut out = vec![0u8; header_len];
    out[..4].copy_from_slice(b"REFT");
    out[4] = version;

    let mut be24 = [0u8; 3];
    put_be24(&mut be24, block_size);
    out[5..8].copy_from_slice(&be24);

    let mut be64 = [0u8; 8];
    put_be64(&mut be64, min_update_index);
    out[8..16].copy_from_slice(&be64);
    put_be64(&mut be64, max_update_index);
    out[16..24].copy_from_slice(&be64);

    if version == 2 {
        let mut be32 = [0u8; 4];
        put_be32(
            &mut be32,
            match hash_id {
                HashId::Sha1 => FORMAT_ID_SHA1,
                HashId::Sha256 => FORMAT_ID_SHA256,
            },
        );
        out[24..28].copy_from_slice(&be32);
    }

    Ok(out)
}

fn encode_footer(
    version: u8,
    header: &[u8],
    ref_index_offset: u64,
    obj_offset_field: u64,
    obj_index_offset: u64,
    log_offset: u64,
    log_index_offset: u64,
) -> Result<Vec<u8>, Error> {
    let footer_len = footer_size(version)?;
    let mut out = vec![0u8; footer_len];
    let header_len = header_size(version)?;
    out[..header_len].copy_from_slice(&header[..header_len]);

    let mut pos = header_len;
    let mut be64 = [0u8; 8];

    for value in [
        ref_index_offset,
        obj_offset_field,
        obj_index_offset,
        log_offset,
        log_index_offset,
    ] {
        put_be64(&mut be64, value);
        out[pos..pos + 8].copy_from_slice(&be64);
        pos += 8;
    }

    let crc = crc32fast::hash(&out[..pos]);
    let mut be32 = [0u8; 4];
    put_be32(&mut be32, crc);
    out[pos..pos + 4].copy_from_slice(&be32);

    Ok(out)
}

fn write_section(
    records: &[Record],
    typ: u8,
    start_offset: u64,
    first_block_header_off: usize,
    opts: &WriteOptions,
) -> Result<SectionResult, Error> {
    if records.is_empty() {
        return Ok(SectionResult {
            bytes: Vec::new(),
            index_offset: 0,
        });
    }

    let mut blocks = Vec::new();
    let mut index_records = Vec::new();

    let mut block = Vec::<u8>::new();
    let mut restarts = Vec::<u32>::new();
    let mut entries = 0usize;
    let mut last_key = Vec::new();

    let mut header_off = first_block_header_off;
    block.resize(header_off + 4, 0);
    block[header_off] = typ;

    let block_limit = opts.block_size as usize;

    let flush_block = |blocks: &mut Vec<Vec<u8>>,
                       index_records: &mut Vec<IndexRecord>,
                       block: &mut Vec<u8>,
                       restarts: &mut Vec<u32>,
                       last_key: &Vec<u8>,
                       header_off: usize|
     -> Result<(), Error> {
        if restarts.is_empty() {
            return Ok(());
        }

        for off in restarts.iter().copied() {
            let mut be24 = [0u8; 3];
            put_be24(&mut be24, off);
            block.extend_from_slice(&be24);
        }
        let restart_count = restarts.len() as u16;
        block.extend_from_slice(&restart_count.to_be_bytes());

        let block_len = block.len();
        let mut be24 = [0u8; 3];
        put_be24(&mut be24, block_len as u32);
        block[header_off + 1..header_off + 4].copy_from_slice(&be24);

        let mut on_disk = if typ == constants::BLOCK_TYPE_LOG {
            let split = header_off + 4;
            let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
            encoder.write_all(&block[split..]).map_err(Error::Io)?;
            let compressed = encoder.finish().map_err(Error::Io)?;

            let mut out = Vec::with_capacity(split + compressed.len());
            out.extend_from_slice(&block[..split]);
            out.extend_from_slice(&compressed);
            out
        } else {
            block.clone()
        };

        if typ != constants::BLOCK_TYPE_LOG && !opts.unpadded && opts.block_size > 0 {
            let target = opts.block_size as usize;
            if on_disk.len() < target {
                on_disk.resize(target, 0);
            }
        }

        let block_offset = start_offset
            .checked_add(blocks.iter().map(|b| b.len() as u64).sum::<u64>())
            .ok_or(Error::VarintOverflow)?;
        index_records.push(IndexRecord {
            last_key: last_key.clone(),
            offset: block_offset,
        });

        blocks.push(on_disk);
        Ok(())
    };

    for rec in records {
        let key = rec.key();
        let val_type = rec.val_type();
        let prev_key = if entries % opts.restart_interval.max(1) as usize == 0 {
            &[][..]
        } else {
            last_key.as_slice()
        };
        let (key_bytes, is_restart) = encode_key(prev_key, &key, val_type)?;
        let payload = rec.encode(opts.hash_id.size())?;

        let entry_len = key_bytes.len() + payload.len();
        let restart_growth = if is_restart { 1 } else { 0 };
        let reserved = 2 + 3 * (restarts.len() + restart_growth);
        let would_len = block.len() + entry_len + reserved;

        let log_limit = (opts.block_size as usize).saturating_mul(2).max(256);
        let limit = if typ == constants::BLOCK_TYPE_LOG {
            log_limit
        } else {
            block_limit
        };

        if entries > 0 && would_len > limit {
            flush_block(
                &mut blocks,
                &mut index_records,
                &mut block,
                &mut restarts,
                &last_key,
                header_off,
            )?;
            block.clear();
            header_off = 0;
            block.resize(4, 0);
            block[0] = typ;
            restarts.clear();
            entries = 0;
            last_key.clear();
        }

        let prev_key = if entries % opts.restart_interval.max(1) as usize == 0 {
            &[][..]
        } else {
            last_key.as_slice()
        };
        let (key_bytes, is_restart) = encode_key(prev_key, &key, val_type)?;
        let payload = rec.encode(opts.hash_id.size())?;
        let entry_len = key_bytes.len() + payload.len();
        let reserved = 2 + 3 * (restarts.len() + usize::from(is_restart));
        let limit = if typ == constants::BLOCK_TYPE_LOG {
            log_limit
        } else {
            block_limit
        };
        if block.len() + entry_len + reserved > limit {
            return Err(Error::Api("record does not fit into configured block size"));
        }

        if is_restart {
            restarts.push(block.len() as u32);
        }
        block.extend_from_slice(&key_bytes);
        block.extend_from_slice(&payload);
        last_key = key;
        entries += 1;
    }

    if entries > 0 {
        flush_block(
            &mut blocks,
            &mut index_records,
            &mut block,
            &mut restarts,
            &last_key,
            header_off,
        )?;
    }

    let mut bytes = Vec::new();
    for b in &blocks {
        bytes.extend_from_slice(b);
    }

    let needs_index = index_records.len() >= 4 || ((opts.unpadded || opts.block_size == 0) && index_records.len() > 1);
    let mut index_offset = 0;
    if needs_index {
        index_offset = start_offset + bytes.len() as u64;
        let idx_records = index_records.into_iter().map(Record::Index).collect::<Vec<_>>();
        let idx_section = write_section(&idx_records, constants::BLOCK_TYPE_INDEX, index_offset, 0, opts)?;
        bytes.extend_from_slice(&idx_section.bytes);
    }

    Ok(SectionResult { bytes, index_offset })
}

#[cfg(test)]
mod tests {
    use crate::{
        constants,
        record::{LogValue, RefValue},
    };

    use super::*;

    #[test]
    fn write_and_read_roundtrip() {
        let mut writer = Writer::new(WriteOptions {
            block_size: 256,
            ..Default::default()
        });
        writer.set_limits(1, 10).expect("limits");
        writer
            .add_ref(RefRecord {
                refname: "HEAD".into(),
                update_index: 2,
                value: RefValue::Symref("refs/heads/main".into()),
            })
            .expect("add ref");
        writer
            .add_ref(RefRecord {
                refname: "refs/heads/main".into(),
                update_index: 2,
                value: RefValue::Val1(vec![1; 20]),
            })
            .expect("add ref");
        writer
            .add_log(LogRecord {
                refname: "refs/heads/main".into(),
                update_index: 2,
                value: LogValue::Update {
                    old_hash: vec![0; 20],
                    new_hash: vec![1; 20],
                    name: "a".into(),
                    email: "a@example.com".into(),
                    time: 0,
                    tz_offset: 0,
                    message: "msg".into(),
                },
            })
            .expect("add log");

        let table = writer.finish_into_table("mem").expect("table");

        let mut refs = table.iter(constants::BLOCK_TYPE_REF).expect("ref iter");
        let mut count = 0;
        while refs.next_record().expect("next").is_some() {
            count += 1;
        }
        assert_eq!(count, 2);

        let mut logs = table.iter(constants::BLOCK_TYPE_LOG).expect("log iter");
        assert!(logs.next_record().expect("next").is_some());
    }

    #[test]
    fn limits_are_enforced() {
        let mut writer = Writer::new(WriteOptions::default());
        writer.set_limits(5, 5).expect("limits");
        let err = writer
            .add_ref(RefRecord {
                refname: "refs/heads/main".into(),
                update_index: 4,
                value: RefValue::Deletion,
            })
            .expect_err("must fail");
        assert!(matches!(err, Error::Api(_)));
    }

    #[test]
    fn index_is_written_for_many_blocks() {
        let mut writer = Writer::new(WriteOptions {
            block_size: 96,
            ..Default::default()
        });
        writer.set_limits(1, 10).expect("limits");
        for idx in 0..32 {
            writer
                .add_ref(RefRecord {
                    refname: format!("refs/heads/{idx:02}"),
                    update_index: 2,
                    value: RefValue::Val1(vec![idx as u8; 20]),
                })
                .expect("add");
        }

        let table = writer.finish_into_table("many").expect("table");
        assert!(table.ref_offsets.index_offset > 0);
    }
}
