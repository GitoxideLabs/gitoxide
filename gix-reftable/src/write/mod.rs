use std::{collections::BTreeMap, io::Write as _};

use gix_hash::{Kind, ObjectId};

use crate::{Error, LogRecord, LogValue, RefRecord, RefValue, Version, format::varint, types::Header};

const MAX_BLOCK_SIZE: usize = 0x00ff_ffff;

/// Controls deterministic encoding of one immutable table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteOptions {
    /// The format version to produce.
    pub version: Version,
    /// The object hash used by all input records.
    pub object_hash: Kind,
    /// Target size for ref, object, and index blocks.
    pub block_size: u32,
    /// Number of records between uncompressed restart keys.
    pub restart_interval: u16,
    /// Whether non-log blocks are padded to `block_size` boundaries.
    pub align_blocks: bool,
    /// Whether to emit the optional object-to-reference-block map.
    pub include_object_index: bool,
    /// An explicit inclusive update-index range, primarily for compacted tables.
    ///
    /// When absent, the smallest and largest input record indexes are used.
    /// Reference records must fall inside an explicit range. Historical log
    /// records may retain older keys when copied or tombstoned by a transaction.
    pub update_index_range: Option<(u64, u64)>,
}

#[cfg(any(feature = "sha1", feature = "sha256"))]
impl Default for WriteOptions {
    fn default() -> Self {
        WriteOptions::for_hash(Kind::shortest())
    }
}

impl WriteOptions {
    pub(crate) fn for_hash(object_hash: Kind) -> Self {
        WriteOptions {
            version: if object_hash.len_in_bytes() == 20 {
                Version::V1
            } else {
                Version::V2
            },
            object_hash,
            block_size: 4096,
            restart_interval: 16,
            align_blocks: true,
            include_object_index: true,
            update_index_range: None,
        }
    }
}

/// A deterministic immutable-table writer.
#[derive(Debug, Clone, Copy)]
pub struct Writer {
    options: WriteOptions,
}

#[cfg(any(feature = "sha1", feature = "sha256"))]
impl Default for Writer {
    fn default() -> Self {
        Writer::new(WriteOptions::default())
    }
}

impl Writer {
    /// Create a writer with `options`.
    pub fn new(options: WriteOptions) -> Self {
        Writer { options }
    }

    /// Encode a complete immutable table into memory.
    pub fn write(&self, refs: &[RefRecord], logs: &[LogRecord]) -> Result<Vec<u8>, Error> {
        self.validate_options()?;
        let derived_range = update_range(refs, logs);
        let (min_update_index, max_update_index) = self.options.update_index_range.unwrap_or(derived_range);
        if min_update_index > max_update_index
            || refs
                .iter()
                .any(|record| record.update_index < min_update_index || record.update_index > max_update_index)
            || logs.iter().any(|record| record.update_index > max_update_index)
        {
            return Err(Error::InvalidInput(
                "explicit update-index range must be ordered, contain every reference, and contain no log above its maximum",
            ));
        }
        let header = Header {
            version: self.options.version,
            block_size: if self.options.align_blocks {
                self.options.block_size
            } else {
                0
            },
            min_update_index,
            max_update_index,
            object_hash: self.options.object_hash,
        };
        let header_bytes = encode_header(header)?;

        let mut ref_records = refs
            .iter()
            .map(|record| encode_ref_record(record, header))
            .collect::<Result<Vec<_>, _>>()?;
        ref_records.sort_by(|a, b| a.key.cmp(&b.key));
        ensure_unique(&ref_records, "reference names must be unique")?;

        let mut log_records = logs
            .iter()
            .map(|record| encode_log_record(record, header.object_hash))
            .collect::<Result<Vec<_>, _>>()?;
        log_records.sort_by(|a, b| a.key.cmp(&b.key));
        ensure_unique(&log_records, "reference-log keys must be unique")?;

        let mut out = Vec::new();
        let mut ref_blocks = Vec::new();
        if ref_records.is_empty() {
            out.extend_from_slice(&header_bytes);
        } else {
            ref_blocks = self.emit_plain_blocks(
                &mut out,
                b'r',
                &ref_records,
                Some(&header_bytes),
                true,
                self.options.block_size as usize,
            )?;
        }

        let ref_index_position = self.emit_index(&mut out, &ref_blocks, self.options.align_blocks)?;

        let (obj_position, obj_id_len, obj_index_position) =
            if self.options.include_object_index && !ref_blocks.is_empty() {
                let (object_records, abbreviation_len) = object_records(
                    &ref_records,
                    &ref_blocks,
                    header.object_hash,
                    self.options.block_size as usize,
                )?;
                if object_records.is_empty() {
                    (0, 0, 0)
                } else {
                    let first = out.len() as u64;
                    let blocks = self.emit_plain_blocks(
                        &mut out,
                        b'o',
                        &object_records,
                        None,
                        true,
                        self.options.block_size as usize,
                    )?;
                    let root = self.emit_index(&mut out, &blocks, self.options.align_blocks)?;
                    (first, abbreviation_len, root)
                }
            } else {
                (0, 0, 0)
            };

        let log_blocks = self.emit_log_blocks(&mut out, &log_records)?;
        let log_position = log_blocks.first().map_or(0, |block| block.position);
        let log_index_position = self.emit_index(&mut out, &log_blocks, false)?;

        let footer = encode_footer(
            header,
            ref_index_position,
            obj_position,
            obj_id_len,
            obj_index_position,
            log_position,
            log_index_position,
        )?;
        out.extend_from_slice(&footer);
        Ok(out)
    }

    fn validate_options(&self) -> Result<(), Error> {
        let block_size = self.options.block_size as usize;
        if block_size == 0 || block_size > MAX_BLOCK_SIZE {
            return Err(Error::InvalidInput("block size must be in 1..=0xffffff"));
        }
        if self.options.restart_interval == 0 {
            return Err(Error::InvalidInput("restart interval must be nonzero"));
        }
        if self.options.version == Version::V1 && self.options.object_hash.len_in_bytes() != 20 {
            return Err(Error::InvalidInput("version 1 tables require SHA-1"));
        }
        Ok(())
    }

    fn emit_plain_blocks(
        &self,
        out: &mut Vec<u8>,
        block_type: u8,
        records: &[RawRecord],
        first_prefix: Option<&[u8]>,
        align: bool,
        target_size: usize,
    ) -> Result<Vec<BlockInfo>, Error> {
        let mut result = Vec::new();
        let mut record_idx = 0;
        while record_idx < records.len() {
            let prefix = if record_idx == 0 {
                first_prefix.unwrap_or_default()
            } else {
                &[]
            };
            let position = if prefix.is_empty() { out.len() as u64 } else { 0 };
            let (block, end_idx) = build_plain_block(
                block_type,
                prefix,
                records,
                record_idx,
                target_size,
                self.options.restart_interval as usize,
            )?;
            if matches!(block_type, b'r' | b'o') && block.len() > self.options.block_size as usize {
                return Err(Error::InvalidInput(
                    "a data record cannot fit in the configured block size",
                ));
            }
            let last_key = records[end_idx - 1].key.clone();
            out.extend_from_slice(&block);
            result.push(BlockInfo {
                position,
                last_key,
                record_range: record_idx..end_idx,
            });
            record_idx = end_idx;
            if align && self.options.align_blocks {
                pad_to_alignment(out, self.options.block_size as usize);
            }
        }
        Ok(result)
    }

    fn emit_log_blocks(&self, out: &mut Vec<u8>, records: &[RawRecord]) -> Result<Vec<BlockInfo>, Error> {
        let mut result = Vec::new();
        let mut record_idx = 0;
        let target = (self.options.block_size as usize).saturating_mul(2).min(MAX_BLOCK_SIZE);
        while record_idx < records.len() {
            let physical_position = out.len() as u64;
            let header_prefix = if result.is_empty() && out.len() == self.options.version.header_len() {
                out.clone()
            } else {
                Vec::new()
            };
            let position = physical_position.saturating_sub(header_prefix.len() as u64);
            let (inflated, end_idx) = build_plain_block(
                b'g',
                &header_prefix,
                records,
                record_idx,
                target,
                self.options.restart_interval as usize,
            )?;
            let mut compressor = gix_zlib::stream::deflate::Write::new(Vec::new(), gix_zlib::Compression::DEFAULT);
            compressor
                .write_all(&inflated[header_prefix.len() + 4..])
                .map_err(|source| Error::Compression { source })?;
            compressor.flush().map_err(|source| Error::Compression { source })?;
            let compressed = compressor.into_inner();
            out.push(b'g');
            put_u24(inflated.len(), out)?;
            out.extend_from_slice(&compressed);
            result.push(BlockInfo {
                position,
                last_key: records[end_idx - 1].key.clone(),
                record_range: record_idx..end_idx,
            });
            record_idx = end_idx;
        }
        Ok(result)
    }

    fn emit_index(&self, out: &mut Vec<u8>, leaf_blocks: &[BlockInfo], align: bool) -> Result<u64, Error> {
        if leaf_blocks.len() <= 1 {
            return Ok(0);
        }
        let mut level = leaf_blocks
            .iter()
            .map(|block| RawRecord {
                key: block.last_key.clone(),
                value_type: 0,
                value: encoded_varint(block.position),
                object_ids: Vec::new(),
            })
            .collect::<Vec<_>>();
        loop {
            let blocks = self.emit_plain_blocks(out, b'i', &level, None, align, self.options.block_size as usize)?;
            if blocks.len() == 1 {
                return Ok(blocks[0].position);
            }
            if blocks.len() >= level.len() {
                return Err(Error::InvalidInput(
                    "index keys cannot be reduced within the format maximum",
                ));
            }
            level = blocks
                .iter()
                .map(|block| RawRecord {
                    key: block.last_key.clone(),
                    value_type: 0,
                    value: encoded_varint(block.position),
                    object_ids: Vec::new(),
                })
                .collect();
        }
    }
}

#[derive(Debug)]
struct RawRecord {
    key: Vec<u8>,
    value_type: u8,
    value: Vec<u8>,
    object_ids: Vec<ObjectId>,
}

#[derive(Debug)]
struct BlockInfo {
    position: u64,
    last_key: Vec<u8>,
    record_range: std::ops::Range<usize>,
}

fn update_range(refs: &[RefRecord], logs: &[LogRecord]) -> (u64, u64) {
    refs.iter()
        .map(|record| record.update_index)
        .chain(logs.iter().map(|record| record.update_index))
        .fold(None, |range, value| match range {
            None => Some((value, value)),
            Some((min, max)) => Some((min.min(value), max.max(value))),
        })
        .unwrap_or((0, 0))
}

fn encode_ref_record(record: &RefRecord, header: Header) -> Result<RawRecord, Error> {
    if record.name.is_empty() || record.name.contains(&0) {
        return Err(Error::InvalidInput(
            "reference names must be nonempty and contain no NUL",
        ));
    }
    let delta = record
        .update_index
        .checked_sub(header.min_update_index)
        .ok_or(Error::InvalidInput("reference update index is below the table minimum"))?;
    let mut value = encoded_varint(delta);
    let (value_type, object_ids) = match &record.value {
        RefValue::Deletion => (0, Vec::new()),
        RefValue::Direct(object_id) => {
            ensure_hash(*object_id, header.object_hash)?;
            value.extend_from_slice(object_id.as_slice());
            (1, vec![*object_id])
        }
        RefValue::Peeled {
            target: target_id,
            peeled: peeled_id,
        } => {
            ensure_hash(*target_id, header.object_hash)?;
            ensure_hash(*peeled_id, header.object_hash)?;
            value.extend_from_slice(target_id.as_slice());
            value.extend_from_slice(peeled_id.as_slice());
            (2, vec![*target_id, *peeled_id])
        }
        RefValue::Symbolic(target) => {
            if target.is_empty() || target.contains(&0) {
                return Err(Error::InvalidInput(
                    "symbolic targets must be nonempty and contain no NUL",
                ));
            }
            varint::encode(target.len() as u64, &mut value);
            value.extend_from_slice(target);
            (3, Vec::new())
        }
    };
    Ok(RawRecord {
        key: record.name.to_vec(),
        value_type,
        value,
        object_ids,
    })
}

fn encode_log_record(record: &LogRecord, hash: Kind) -> Result<RawRecord, Error> {
    if record.ref_name.is_empty() || record.ref_name.contains(&0) {
        return Err(Error::InvalidInput(
            "log reference names must be nonempty and contain no NUL",
        ));
    }
    let mut key = record.ref_name.to_vec();
    key.push(0);
    key.extend_from_slice(&(u64::MAX - record.update_index).to_be_bytes());
    let (value_type, value) = match &record.value {
        LogValue::Deletion => (0, Vec::new()),
        LogValue::Placeholder => {
            let mut value = Vec::new();
            value.extend_from_slice(hash.null().as_slice());
            value.extend_from_slice(hash.null().as_slice());
            put_varbytes(&[], &mut value);
            put_varbytes(&[], &mut value);
            varint::encode(0, &mut value);
            value.extend_from_slice(&0i16.to_be_bytes());
            put_log_message(&[], &mut value);
            (1, value)
        }
        LogValue::Update {
            old_id,
            new_id,
            name,
            email,
            time,
            tz_offset,
            message,
        } => {
            ensure_hash(*old_id, hash)?;
            ensure_hash(*new_id, hash)?;
            if message.ends_with(b"\n") {
                return Err(Error::InvalidInput(
                    "log messages must omit their on-disk terminating newline",
                ));
            }
            if old_id.is_null()
                && new_id.is_null()
                && name.is_empty()
                && email.is_empty()
                && *time == 0
                && *tz_offset == 0
                && message.is_empty()
            {
                return Err(Error::InvalidInput(
                    "an all-zero log update is reserved for the placeholder value",
                ));
            }
            let mut value = Vec::new();
            value.extend_from_slice(old_id.as_slice());
            value.extend_from_slice(new_id.as_slice());
            put_varbytes(name, &mut value);
            put_varbytes(email, &mut value);
            varint::encode(*time, &mut value);
            value.extend_from_slice(&tz_offset.to_be_bytes());
            put_log_message(message, &mut value);
            (1, value)
        }
    };
    Ok(RawRecord {
        key,
        value_type,
        value,
        object_ids: Vec::new(),
    })
}

fn ensure_hash(object_id: ObjectId, expected: Kind) -> Result<(), Error> {
    if object_id.kind() != expected {
        return Err(Error::InvalidInput("all object ids must use the table hash"));
    }
    Ok(())
}

fn ensure_unique(records: &[RawRecord], message: &'static str) -> Result<(), Error> {
    if records.windows(2).any(|pair| pair[0].key == pair[1].key) {
        return Err(Error::InvalidInput(message));
    }
    Ok(())
}

fn build_plain_block(
    block_type: u8,
    prefix: &[u8],
    records: &[RawRecord],
    start_idx: usize,
    target_size: usize,
    restart_interval: usize,
) -> Result<(Vec<u8>, usize), Error> {
    let mut block = prefix.to_vec();
    block.push(block_type);
    block.extend_from_slice(&[0; 3]);
    let mut restarts = Vec::new();
    let mut previous_key = Vec::new();
    let mut idx = start_idx;
    while idx < records.len() {
        let restart = (idx - start_idx).is_multiple_of(restart_interval);
        let encoded = encode_record(&records[idx], &previous_key, restart)?;
        let prospective_restarts = restarts.len() + usize::from(restart);
        let prospective_len = block
            .len()
            .checked_add(encoded.len())
            .and_then(|len| len.checked_add(prospective_restarts * 3 + 2))
            .ok_or(Error::InvalidInput("block size overflow"))?;
        if idx > start_idx && prospective_len > target_size {
            break;
        }
        if restart {
            if restarts.len() == u16::MAX as usize {
                break;
            }
            restarts.push(block.len());
        }
        block.extend_from_slice(&encoded);
        previous_key.clone_from(&records[idx].key);
        idx += 1;
    }
    if idx == start_idx {
        return Err(Error::InvalidInput("a record cannot fit in a reftable block"));
    }
    for offset in &restarts {
        put_u24(*offset, &mut block)?;
    }
    block.extend_from_slice(&(restarts.len() as u16).to_be_bytes());
    if block.len() > MAX_BLOCK_SIZE {
        return Err(Error::InvalidInput("block exceeds the format maximum"));
    }
    let header_at = prefix.len();
    let length = block.len();
    put_u24_at(length, &mut block[header_at + 1..header_at + 4])?;
    Ok((block, idx))
}

fn encode_record(record: &RawRecord, previous_key: &[u8], restart: bool) -> Result<Vec<u8>, Error> {
    let prefix_len = if restart {
        0
    } else {
        common_prefix(previous_key, &record.key)
    };
    let suffix = &record.key[prefix_len..];
    let suffix_len = u64::try_from(suffix.len()).map_err(|_| Error::InvalidInput("key is too long"))?;
    let typed_len = suffix_len
        .checked_mul(8)
        .and_then(|value| value.checked_add(u64::from(record.value_type)))
        .ok_or(Error::InvalidInput("key is too long"))?;
    let mut out = Vec::new();
    varint::encode(prefix_len as u64, &mut out);
    varint::encode(typed_len, &mut out);
    out.extend_from_slice(suffix);
    out.extend_from_slice(&record.value);
    Ok(out)
}

fn object_records(
    refs: &[RawRecord],
    blocks: &[BlockInfo],
    hash: Kind,
    block_size: usize,
) -> Result<(Vec<RawRecord>, u8), Error> {
    let mut positions_by_object_id = BTreeMap::<ObjectId, Vec<u64>>::new();
    for block in blocks {
        for record in &refs[block.record_range.clone()] {
            for object_id in &record.object_ids {
                positions_by_object_id
                    .entry(*object_id)
                    .or_default()
                    .push(block.position);
            }
        }
    }
    for positions in positions_by_object_id.values_mut() {
        positions.sort_unstable();
        positions.dedup();
    }
    if positions_by_object_id.is_empty() {
        return Ok((Vec::new(), 0));
    }
    let Some(abbreviation_len) = (2..=31.min(hash.len_in_bytes())).find(|len| {
        positions_by_object_id
            .keys()
            .map(|object_id| &object_id.as_slice()[..*len])
            .collect::<Vec<_>>()
            .windows(2)
            .all(|pair| pair[0] != pair[1])
    }) else {
        return Ok((Vec::new(), 0));
    };
    let records = positions_by_object_id
        .into_iter()
        .map(|(object_id, positions)| {
            let mut value = Vec::new();
            let mut value_type = if positions.len() <= 7 {
                positions.len() as u8
            } else {
                varint::encode(positions.len() as u64, &mut value);
                0
            };
            let mut previous = 0;
            for (idx, position) in positions.into_iter().enumerate() {
                let delta = if idx == 0 { position } else { position - previous };
                varint::encode(delta, &mut value);
                previous = position;
            }
            let conservative_record_size = 4 + 2 + abbreviation_len + value.len() + 3 + 2;
            if conservative_record_size > block_size {
                value.clear();
                varint::encode(0, &mut value);
                value_type = 0;
            }
            RawRecord {
                key: object_id.as_slice()[..abbreviation_len].to_vec(),
                value_type,
                value,
                object_ids: Vec::new(),
            }
        })
        .collect();
    Ok((records, abbreviation_len as u8))
}

fn encode_header(header: Header) -> Result<Vec<u8>, Error> {
    let mut out = Vec::with_capacity(header.version.header_len());
    out.extend_from_slice(b"REFT");
    out.push(header.version.byte());
    put_u24(header.block_size as usize, &mut out)?;
    out.extend_from_slice(&header.min_update_index.to_be_bytes());
    out.extend_from_slice(&header.max_update_index.to_be_bytes());
    if header.version == Version::V2 {
        out.extend_from_slice(hash_id(header.object_hash)?);
    }
    Ok(out)
}

fn encode_footer(
    header: Header,
    ref_index_position: u64,
    obj_position: u64,
    obj_id_len: u8,
    obj_index_position: u64,
    log_position: u64,
    log_index_position: u64,
) -> Result<Vec<u8>, Error> {
    if obj_position > (u64::MAX >> 5) || obj_id_len > 31 {
        return Err(Error::InvalidInput("object index footer value is out of range"));
    }
    let mut out = encode_header(header)?;
    out.extend_from_slice(&ref_index_position.to_be_bytes());
    out.extend_from_slice(&((obj_position << 5) | u64::from(obj_id_len)).to_be_bytes());
    out.extend_from_slice(&obj_index_position.to_be_bytes());
    out.extend_from_slice(&log_position.to_be_bytes());
    out.extend_from_slice(&log_index_position.to_be_bytes());
    let crc = gix_features::hash::crc32(&out);
    out.extend_from_slice(&crc.to_be_bytes());
    Ok(out)
}

fn hash_id(hash: Kind) -> Result<&'static [u8; 4], Error> {
    let _ = hash;
    #[cfg(feature = "sha1")]
    if hash == Kind::Sha1 {
        return Ok(b"sha1");
    }
    #[cfg(feature = "sha256")]
    if hash == Kind::Sha256 {
        return Ok(b"s256");
    }
    Err(Error::Unsupported("object hash"))
}

fn put_varbytes(value: &[u8], out: &mut Vec<u8>) {
    varint::encode(value.len() as u64, out);
    out.extend_from_slice(value);
}

fn put_log_message(value: &[u8], out: &mut Vec<u8>) {
    let needs_newline = !value.is_empty() && !value.ends_with(b"\n");
    varint::encode((value.len() + usize::from(needs_newline)) as u64, out);
    out.extend_from_slice(value);
    if needs_newline {
        out.push(b'\n');
    }
}

fn encoded_varint(value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    varint::encode(value, &mut out);
    out
}

fn common_prefix(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b).take_while(|(a, b)| a == b).count()
}

fn put_u24(value: usize, out: &mut Vec<u8>) -> Result<(), Error> {
    if value > MAX_BLOCK_SIZE {
        return Err(Error::InvalidInput("24-bit integer overflow"));
    }
    out.extend_from_slice(&[(value >> 16) as u8, (value >> 8) as u8, value as u8]);
    Ok(())
}

fn put_u24_at(value: usize, out: &mut [u8]) -> Result<(), Error> {
    if value > MAX_BLOCK_SIZE || out.len() != 3 {
        return Err(Error::InvalidInput("24-bit integer overflow"));
    }
    out.copy_from_slice(&[(value >> 16) as u8, (value >> 8) as u8, value as u8]);
    Ok(())
}

fn pad_to_alignment(out: &mut Vec<u8>, block_size: usize) {
    let remainder = out.len() % block_size;
    if remainder != 0 {
        out.resize(out.len() + block_size - remainder, 0);
    }
}
