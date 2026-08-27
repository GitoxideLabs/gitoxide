use std::{
    collections::{BTreeMap, BTreeSet, HashMap, hash_map::Entry},
    io::Read as _,
    ops::Range,
    path::Path,
};

use bstr::BString;
use gix_hash::{Kind, ObjectId, oid};

use crate::{
    Error, Header, Limits, LogRecord, LogRecordRef, LogValue, RefRecord, RefRecordRef, RefValue, Version,
    format::varint, types::malformed,
};

/// A fully validated immutable reftable.
#[derive(Debug, Clone)]
pub struct Table {
    header: Header,
    refs: Vec<RefRecord>,
    logs: Vec<LogRecord>,
    ref_blocks: Vec<SearchBlock>,
    log_blocks: Vec<SearchBlock>,
    object_blocks: Vec<SearchBlock>,
    object_id_len: Option<usize>,
    object_index: Vec<ObjectLookup>,
    file_size: usize,
}

impl Table {
    /// Parse and validate one immutable table from `data`.
    pub fn from_bytes(data: &[u8], limits: Limits) -> Result<Self, Error> {
        Parser::new(data, limits)?.parse()
    }

    /// Read, parse, and validate one immutable table from `path`.
    pub fn read(path: impl AsRef<Path>, limits: Limits) -> Result<Self, Error> {
        let path = path.as_ref();
        let file = std::fs::File::open(path).map_err(|source| Error::Io {
            path: path.to_owned(),
            source,
        })?;
        if file
            .metadata()
            .map_err(|source| Error::Io {
                path: path.to_owned(),
                source,
            })?
            .len()
            > limits.max_file_size as u64
        {
            return Err(Error::Parse {
                path: path.to_owned(),
                source: Box::new(Error::Limit("file size")),
            });
        }
        let read_limit = limits.max_file_size.checked_add(1).ok_or_else(|| Error::Parse {
            path: path.to_owned(),
            source: Box::new(Error::Limit("file size")),
        })?;
        let mut data = Vec::new();
        file.take(read_limit as u64)
            .read_to_end(&mut data)
            .map_err(|source| Error::Io {
                path: path.to_owned(),
                source,
            })?;
        Table::from_bytes(&data, limits).map_err(|source| Error::Parse {
            path: path.to_owned(),
            source: Box::new(source),
        })
    }

    /// Return the validated table header.
    pub fn header(&self) -> Header {
        self.header
    }

    /// Return the immutable table's encoded size in bytes.
    pub fn file_size(&self) -> usize {
        self.file_size
    }

    /// Iterate references in bytewise name order.
    pub fn refs(&self) -> impl ExactSizeIterator<Item = &RefRecord> {
        self.refs.iter()
    }

    /// Iterate borrowed reference-record views in bytewise name order.
    pub fn ref_views(&self) -> impl ExactSizeIterator<Item = RefRecordRef<'_>> {
        self.refs.iter().map(RefRecord::to_ref)
    }

    /// Iterate reference logs by name and descending update index.
    pub fn logs(&self) -> impl ExactSizeIterator<Item = &LogRecord> {
        self.logs.iter()
    }

    /// Iterate borrowed reference-log views by name and descending update index.
    pub fn log_views(&self) -> impl ExactSizeIterator<Item = LogRecordRef<'_>> {
        self.logs.iter().map(LogRecord::to_ref)
    }

    /// Find an exact reference name using the table's validated block index and binary search.
    pub fn find_ref(&self, name: &[u8]) -> Option<&RefRecord> {
        let index = self.lower_bound_ref(name);
        self.refs.get(index).filter(|record| record.name.as_slice() == name)
    }

    /// Iterate references whose names begin with `prefix` using one indexed seek followed by a range scan.
    pub fn refs_with_prefix<'a>(&'a self, prefix: &'a [u8]) -> impl Iterator<Item = &'a RefRecord> + 'a {
        self.refs[self.lower_bound_ref(prefix)..]
            .iter()
            .take_while(move |record| record.name.starts_with(prefix))
    }

    /// Iterate all log records for `name`, newest first, beginning at the indexed containing block.
    pub fn logs_for<'a>(&'a self, name: &'a [u8]) -> impl Iterator<Item = &'a LogRecord> + 'a {
        self.logs[self.lower_bound_log(name)..]
            .iter()
            .take_while(move |record| record.ref_name.as_slice() == name)
    }

    /// Iterate references that contain `object_id` as their direct or peeled value.
    ///
    /// When the table contains an object index, only the referenced ref blocks are scanned.
    /// Tables without an object index fall back to a complete reference scan as required by the format.
    /// An object ID using a different hash kind cannot occur in this table and produces no records.
    pub fn refs_for_object<'a>(&'a self, object_id: &'a oid) -> impl Iterator<Item = &'a RefRecord> + 'a {
        let candidates = if object_id.kind() != self.header.object_hash {
            RefCandidates::Empty
        } else {
            match self.object_id_len {
                None => RefCandidates::All(self.refs.iter()),
                Some(prefix_len) => {
                    let key = &object_id.as_bytes()[..prefix_len];
                    let index = self.lower_bound_object(key);
                    match self.object_index.get(index).filter(|entry| entry.key.as_slice() == key) {
                        Some(entry) if entry.ref_ranges.is_empty() => RefCandidates::All(self.refs.iter()),
                        Some(entry) => RefCandidates::indexed(&self.refs, &entry.ref_ranges),
                        None => RefCandidates::Empty,
                    }
                }
            }
        };
        candidates.filter(move |record| record_contains_object(record, object_id))
    }

    fn lower_bound_ref(&self, name: &[u8]) -> usize {
        let block_index = self
            .ref_blocks
            .partition_point(|block| block.last_key.as_slice() < name);
        let Some(block) = self.ref_blocks.get(block_index) else {
            return self.refs.len();
        };
        block.records.start + self.refs[block.records.clone()].partition_point(|record| record.name.as_slice() < name)
    }

    fn lower_bound_log(&self, name: &[u8]) -> usize {
        let mut first_key = Vec::with_capacity(name.len() + 9);
        first_key.extend_from_slice(name);
        first_key.extend_from_slice(&[0; 9]);
        let block_index = self
            .log_blocks
            .partition_point(|block| block.last_key.as_slice() < first_key.as_slice());
        let Some(block) = self.log_blocks.get(block_index) else {
            return self.logs.len();
        };
        block.records.start
            + self.logs[block.records.clone()].partition_point(|record| record.ref_name.as_slice() < name)
    }

    fn lower_bound_object(&self, key: &[u8]) -> usize {
        let block_index = self
            .object_blocks
            .partition_point(|block| block.last_key.as_slice() < key);
        let Some(block) = self.object_blocks.get(block_index) else {
            return self.object_index.len();
        };
        block.records.start
            + self.object_index[block.records.clone()].partition_point(|record| record.key.as_slice() < key)
    }
}

#[derive(Debug, Clone)]
struct SearchBlock {
    last_key: Vec<u8>,
    records: Range<usize>,
}

#[derive(Debug, Clone)]
struct ObjectLookup {
    key: Vec<u8>,
    ref_ranges: Vec<Range<usize>>,
}

struct ExpectedObject {
    object_id: ObjectId,
    positions: Vec<u64>,
}

enum RefCandidates<'a> {
    All(std::slice::Iter<'a, RefRecord>),
    Indexed {
        refs: &'a [RefRecord],
        ranges: &'a [Range<usize>],
        range_index: usize,
        record_index: usize,
    },
    Empty,
}

impl<'a> RefCandidates<'a> {
    fn indexed(refs: &'a [RefRecord], ranges: &'a [Range<usize>]) -> Self {
        RefCandidates::Indexed {
            refs,
            ranges,
            range_index: 0,
            record_index: ranges.first().map_or(0, |range| range.start),
        }
    }
}

impl<'a> Iterator for RefCandidates<'a> {
    type Item = &'a RefRecord;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            RefCandidates::All(records) => records.next(),
            RefCandidates::Indexed {
                refs,
                ranges,
                range_index,
                record_index,
            } => loop {
                let range = ranges.get(*range_index)?;
                if *record_index < range.end {
                    let record = refs.get(*record_index);
                    *record_index += 1;
                    return record;
                }
                *range_index += 1;
                *record_index = ranges.get(*range_index).map_or(0, |range| range.start);
            },
            RefCandidates::Empty => None,
        }
    }
}

fn record_contains_object(record: &RefRecord, object_id: &oid) -> bool {
    match &record.value {
        RefValue::Direct(target_id) => target_id.as_ref() == object_id,
        RefValue::Peeled {
            target: target_id,
            peeled: peeled_id,
        } => target_id.as_ref() == object_id || peeled_id.as_ref() == object_id,
        RefValue::Deletion | RefValue::Symbolic(_) => false,
    }
}

#[derive(Debug, Clone, Copy)]
struct Footer {
    start: usize,
    ref_index_position: u64,
    obj_position: u64,
    obj_id_len: u8,
    obj_index_position: u64,
    log_position: u64,
    log_index_position: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Ref,
    Object,
    Log,
    Index,
}

#[derive(Debug)]
struct Block {
    position: u64,
    kind: BlockKind,
    last_key: Vec<u8>,
    index_entries: Vec<(Vec<u8>, u64)>,
    records: Range<usize>,
}

#[derive(Debug)]
struct ObjectRecord {
    key: Vec<u8>,
    positions: Vec<u64>,
}

struct DecodedRecords {
    last_key: Vec<u8>,
    index_entries: Vec<(Vec<u8>, u64)>,
}

struct Parser<'a> {
    data: &'a [u8],
    limits: Limits,
    header: Header,
    footer: Footer,
    records_seen: usize,
    refs: Vec<RefRecord>,
    logs: Vec<LogRecord>,
    objects: Vec<ObjectRecord>,
    blocks: Vec<Block>,
    last_log_key: Vec<u8>,
}

impl<'a> Parser<'a> {
    fn new(data: &'a [u8], limits: Limits) -> Result<Self, Error> {
        if data.len() > limits.max_file_size {
            return Err(Error::Limit("file size"));
        }
        let header = parse_header(data, 0)?;
        let footer = parse_footer(data, header)?;
        Ok(Parser {
            data,
            limits,
            header,
            footer,
            records_seen: 0,
            refs: Vec::new(),
            logs: Vec::new(),
            objects: Vec::new(),
            blocks: Vec::new(),
            last_log_key: Vec::new(),
        })
    }

    fn parse(mut self) -> Result<Table, Error> {
        let mut position = self.header.version.header_len();
        let mut saw_ref = false;
        while position < self.footer.start {
            match self.data[position] {
                0 => {
                    position += 1;
                }
                b'r' => {
                    let logical_position = if !saw_ref && position == self.header.version.header_len() {
                        0
                    } else {
                        position
                    };
                    let end = self.parse_plain_block(position, logical_position, BlockKind::Ref)?;
                    saw_ref = true;
                    position = end;
                }
                b'o' => {
                    position = self.parse_plain_block(position, position, BlockKind::Object)?;
                }
                b'i' => {
                    position = self.parse_plain_block(position, position, BlockKind::Index)?;
                }
                b'g' => {
                    let header_prefix = if self.blocks.is_empty() && position == self.header.version.header_len() {
                        self.header.version.header_len()
                    } else {
                        0
                    };
                    position = self.parse_log_block(position, header_prefix)?;
                }
                _ => return Err(malformed(position, "unknown block type or nonzero padding")),
            }
        }
        self.validate_layout()?;
        let ref_blocks = self.search_blocks(self.footer.ref_index_position, BlockKind::Ref)?;
        let log_blocks = self.search_blocks(self.footer.log_index_position, BlockKind::Log)?;
        let object_blocks = self.search_blocks(self.footer.obj_index_position, BlockKind::Object)?;
        let object_index = self.object_lookup()?;
        Ok(Table {
            header: self.header,
            refs: self.refs,
            logs: self.logs,
            ref_blocks,
            log_blocks,
            object_blocks,
            object_id_len: (self.footer.obj_id_len != 0).then_some(self.footer.obj_id_len as usize),
            object_index,
            file_size: self.data.len(),
        })
    }

    fn parse_plain_block(
        &mut self,
        header_position: usize,
        logical_position: usize,
        kind: BlockKind,
    ) -> Result<usize, Error> {
        let block_len = read_u24(self.data, header_position + 1)?;
        if block_len > self.limits.max_block_size {
            return Err(Error::Limit("block size"));
        }
        let end = logical_position
            .checked_add(block_len)
            .ok_or_else(|| malformed(header_position, "block end overflows"))?;
        if end > self.footer.start || end <= header_position + 4 {
            return Err(malformed(header_position, "block length is outside the table body"));
        }
        if matches!(kind, BlockKind::Ref | BlockKind::Object)
            && self.header.block_size != 0
            && block_len > self.header.block_size as usize
        {
            return Err(malformed(
                header_position,
                "data block exceeds the configured block size",
            ));
        }
        let block = self.data[logical_position..end].to_vec();
        let type_offset = header_position - logical_position;
        let record_start = self.record_count(kind);
        let decoded = self.decode_records(&block, logical_position, type_offset, kind)?;
        let records = record_start..self.record_count(kind);
        self.blocks.push(Block {
            position: logical_position as u64,
            kind,
            last_key: decoded.last_key,
            index_entries: decoded.index_entries,
            records,
        });
        Ok(end)
    }

    fn parse_log_block(&mut self, position: usize, header_prefix: usize) -> Result<usize, Error> {
        let inflated_len = read_u24(self.data, position + 1)?;
        if inflated_len > self.limits.max_block_size {
            return Err(Error::Limit("inflated log block size"));
        }
        if inflated_len <= header_prefix + 6 {
            return Err(malformed(position, "log block is too short"));
        }
        let compressed = self
            .data
            .get(position + 4..self.footer.start)
            .ok_or_else(|| malformed(position, "log block header is truncated"))?;
        let mut inflated = vec![0; inflated_len];
        if header_prefix != 0 {
            inflated[..header_prefix].copy_from_slice(&self.data[..header_prefix]);
        }
        inflated[header_prefix..header_prefix + 4].copy_from_slice(&self.data[position..position + 4]);
        let (status, consumed, written) = gix_zlib::Inflate::default()
            .once(compressed, &mut inflated[header_prefix + 4..])
            .map_err(|_| malformed(position + 4, "invalid zlib log block"))?;
        if status != gix_zlib::Status::StreamEnd || written != inflated_len - header_prefix - 4 {
            return Err(malformed(
                position + 4,
                "zlib stream does not match the advertised block length",
            ));
        }
        let end = position
            .checked_add(4)
            .and_then(|value| value.checked_add(consumed))
            .ok_or_else(|| malformed(position, "compressed log block end overflows"))?;
        if end > self.footer.start {
            return Err(malformed(position, "compressed log block overlaps the footer"));
        }
        let record_start = self.logs.len();
        let decoded = self.decode_records(
            &inflated,
            position.saturating_sub(header_prefix),
            header_prefix,
            BlockKind::Log,
        )?;
        self.blocks.push(Block {
            position: position.saturating_sub(header_prefix) as u64,
            kind: BlockKind::Log,
            last_key: decoded.last_key,
            index_entries: decoded.index_entries,
            records: record_start..self.logs.len(),
        });
        Ok(end)
    }

    fn decode_records(
        &mut self,
        block: &[u8],
        base: usize,
        type_offset: usize,
        kind: BlockKind,
    ) -> Result<DecodedRecords, Error> {
        if block.get(type_offset).copied() != Some(kind.byte()) {
            return Err(malformed(base + type_offset, "block type changed while decoding"));
        }
        let restart_count = read_u16_at_end(block, base)? as usize;
        if restart_count == 0 {
            return Err(malformed(base + block.len() - 2, "restart table is empty"));
        }
        let restart_bytes = restart_count
            .checked_mul(3)
            .ok_or_else(|| malformed(base, "restart table size overflows"))?;
        let restart_start = block
            .len()
            .checked_sub(2 + restart_bytes)
            .ok_or_else(|| malformed(base, "restart table is truncated"))?;
        let records_start = type_offset + 4;
        if restart_start <= records_start {
            return Err(malformed(base + restart_start, "block has no records"));
        }
        let mut restarts = Vec::with_capacity(restart_count);
        for idx in 0..restart_count {
            let offset = read_u24(block, restart_start + idx * 3)?;
            if offset < records_start || offset >= restart_start {
                return Err(malformed(
                    base + restart_start + idx * 3,
                    "restart offset is outside record data",
                ));
            }
            if restarts.last().is_some_and(|previous| *previous >= offset) {
                return Err(malformed(
                    base + restart_start + idx * 3,
                    "restart offsets are not ascending",
                ));
            }
            restarts.push(offset);
        }
        if restarts[0] != records_start {
            return Err(malformed(base + restarts[0], "the first record is not a restart"));
        }

        let mut cursor = Cursor::new(&block[..restart_start], base, records_start);
        let mut previous_key = Vec::new();
        let mut restart_idx = 0;
        let mut local_index_entries = Vec::new();
        while cursor.position < restart_start {
            self.bump_record_count()?;
            let record_offset = cursor.position;
            let at_restart = restarts.get(restart_idx).copied() == Some(record_offset);
            if restarts
                .get(restart_idx)
                .is_some_and(|restart| *restart < record_offset)
            {
                return Err(malformed(base + record_offset, "restart does not point to a record"));
            }
            if at_restart {
                restart_idx += 1;
            }
            let prefix_len = usize::try_from(cursor.varint()?)
                .map_err(|_| malformed(base + record_offset, "key prefix length does not fit in memory"))?;
            let typed_suffix_len = cursor.varint()?;
            let value_type = (typed_suffix_len & 7) as u8;
            let suffix_len = usize::try_from(typed_suffix_len >> 3)
                .map_err(|_| malformed(base + record_offset, "key length does not fit in memory"))?;
            if prefix_len > previous_key.len() {
                return Err(malformed(base + record_offset, "key prefix exceeds the prior key"));
            }
            if (previous_key.is_empty() || at_restart) && prefix_len != 0 {
                return Err(malformed(base + record_offset, "restart key is prefix-compressed"));
            }
            let key_len = prefix_len
                .checked_add(suffix_len)
                .ok_or_else(|| malformed(base + record_offset, "key length overflows"))?;
            if key_len > self.limits.max_value_size {
                return Err(Error::Limit("record key size"));
            }
            let suffix = cursor.bytes(suffix_len)?;
            let mut key = previous_key[..prefix_len].to_vec();
            key.extend_from_slice(suffix);
            if !previous_key.is_empty() && key <= previous_key {
                return Err(malformed(
                    base + record_offset,
                    "record keys are not strictly ascending",
                ));
            }
            match kind {
                BlockKind::Ref => self.decode_ref(&mut cursor, key.clone(), value_type)?,
                BlockKind::Object => self.decode_object(&mut cursor, key.clone(), value_type)?,
                BlockKind::Log => self.decode_log(&mut cursor, key.clone(), value_type)?,
                BlockKind::Index => {
                    if value_type != 0 {
                        return Err(malformed(
                            base + record_offset,
                            "index record has a reserved value type",
                        ));
                    }
                    local_index_entries.push((key.clone(), cursor.varint()?));
                }
            }
            previous_key = key;
        }
        if cursor.position != restart_start || restart_idx != restarts.len() {
            return Err(malformed(
                base + cursor.position,
                "record data does not end at the restart table",
            ));
        }
        Ok(DecodedRecords {
            last_key: previous_key,
            index_entries: local_index_entries,
        })
    }

    fn decode_ref(&mut self, cursor: &mut Cursor<'_>, name: Vec<u8>, value_type: u8) -> Result<(), Error> {
        let delta = cursor.varint()?;
        let update_index = self
            .header
            .min_update_index
            .checked_add(delta)
            .ok_or_else(|| malformed(cursor.absolute(), "reference update index overflows"))?;
        self.ensure_update_index(update_index, cursor.absolute())?;
        let value = match value_type {
            0 => RefValue::Deletion,
            1 => RefValue::Direct(cursor.object_id(self.header.object_hash)?),
            2 => RefValue::Peeled {
                target: cursor.object_id(self.header.object_hash)?,
                peeled: cursor.object_id(self.header.object_hash)?,
            },
            3 => RefValue::Symbolic(BString::from(cursor.varbytes(self.limits.max_value_size)?.to_vec())),
            _ => {
                return Err(malformed(
                    cursor.absolute(),
                    "reference record has a reserved value type",
                ));
            }
        };
        if self
            .refs
            .last()
            .is_some_and(|record| record.name.as_slice() >= name.as_slice())
        {
            return Err(malformed(
                cursor.absolute(),
                "reference names are not globally ascending",
            ));
        }
        self.refs.push(RefRecord {
            name: BString::from(name),
            update_index,
            value,
        });
        Ok(())
    }

    fn decode_log(&mut self, cursor: &mut Cursor<'_>, key: Vec<u8>, value_type: u8) -> Result<(), Error> {
        if key.len() < 10 || key[key.len() - 9] != 0 || key[..key.len() - 9].contains(&0) {
            return Err(malformed(
                cursor.absolute(),
                "log key is not a reference name plus reversed update index",
            ));
        }
        let split = key.len() - 8;
        let reversed = u64::from_be_bytes(
            key[split..]
                .try_into()
                .map_err(|_| malformed(cursor.absolute(), "log update index is truncated"))?,
        );
        let update_index = u64::MAX - reversed;
        if update_index > self.header.max_update_index {
            return Err(malformed(
                cursor.absolute(),
                "log update index exceeds the header maximum",
            ));
        }
        if !self.last_log_key.is_empty() && key <= self.last_log_key {
            return Err(malformed(cursor.absolute(), "log keys are not globally ascending"));
        }
        let value = match value_type {
            0 => LogValue::Deletion,
            1 => {
                let old_id = cursor.object_id(self.header.object_hash)?;
                let new_id = cursor.object_id(self.header.object_hash)?;
                let name = BString::from(cursor.varbytes(self.limits.max_value_size)?.to_vec());
                let email = BString::from(cursor.varbytes(self.limits.max_value_size)?.to_vec());
                let time = cursor.varint()?;
                let tz_offset = i16::from_be_bytes(
                    cursor
                        .bytes(2)?
                        .try_into()
                        .map_err(|_| malformed(cursor.absolute(), "timezone is truncated"))?,
                );
                let mut message = cursor.varbytes(self.limits.max_value_size)?.to_vec();
                if message.ends_with(b"\n") {
                    message.pop();
                }
                if old_id.is_null()
                    && new_id.is_null()
                    && name.is_empty()
                    && email.is_empty()
                    && time == 0
                    && tz_offset == 0
                    && message.is_empty()
                {
                    LogValue::Placeholder
                } else {
                    LogValue::Update {
                        old_id,
                        new_id,
                        name,
                        email,
                        time,
                        tz_offset,
                        message: BString::from(message),
                    }
                }
            }
            _ => return Err(malformed(cursor.absolute(), "log record has a reserved value type")),
        };
        self.logs.push(LogRecord {
            ref_name: BString::from(key[..key.len() - 9].to_vec()),
            update_index,
            value,
        });
        self.last_log_key = key;
        Ok(())
    }

    fn decode_object(&mut self, cursor: &mut Cursor<'_>, key: Vec<u8>, count_bits: u8) -> Result<(), Error> {
        if self.objects.last().is_some_and(|record| record.key >= key) {
            return Err(malformed(cursor.absolute(), "object keys are not globally ascending"));
        }
        let count = if count_bits == 0 {
            usize::try_from(cursor.varint()?)
                .map_err(|_| malformed(cursor.absolute(), "object block position count does not fit in memory"))?
        } else {
            count_bits as usize
        };
        if count > self.limits.max_records {
            return Err(Error::Limit("object block positions"));
        }
        if count > cursor.remaining() {
            return Err(malformed(
                cursor.absolute(),
                "object block position count exceeds the remaining record data",
            ));
        }
        let mut positions = Vec::new();
        positions
            .try_reserve(count)
            .map_err(|_| Error::Limit("object block positions"))?;
        let mut prior = 0u64;
        for idx in 0..count {
            let delta = cursor.varint()?;
            let position = if idx == 0 {
                delta
            } else {
                prior
                    .checked_add(delta)
                    .ok_or_else(|| malformed(cursor.absolute(), "object block position overflows"))?
            };
            if idx > 0 && position <= prior {
                return Err(malformed(cursor.absolute(), "object block positions are not ascending"));
            }
            positions.push(position);
            prior = position;
        }
        self.objects.push(ObjectRecord { key, positions });
        Ok(())
    }

    fn validate_layout(&self) -> Result<(), Error> {
        let ref_blocks = self.positions(BlockKind::Ref);
        let object_blocks = self.positions(BlockKind::Object);
        let log_blocks = self.positions(BlockKind::Log);
        if !ref_blocks.is_empty() && ref_blocks[0] != 0 {
            return Err(malformed(
                ref_blocks[0] as usize,
                "the first ref block does not share the file header",
            ));
        }
        if self.footer.obj_position != object_blocks.first().copied().unwrap_or(0) {
            return Err(malformed(
                self.footer.start,
                "footer object position does not name the first object block",
            ));
        }
        if self.footer.log_position != log_blocks.first().copied().unwrap_or(0) {
            return Err(malformed(
                self.footer.start,
                "footer log position does not name the first log block",
            ));
        }
        if let (Some(last_ref), Some(first_object)) = (ref_blocks.last(), object_blocks.first())
            && last_ref >= first_object
        {
            return Err(malformed(*first_object as usize, "object blocks precede ref blocks"));
        }
        if let (Some(last_ref), Some(first_log)) = (ref_blocks.last(), log_blocks.first())
            && last_ref >= first_log
        {
            return Err(malformed(*first_log as usize, "log blocks precede ref blocks"));
        }
        if let (Some(last_object), Some(first_log)) = (object_blocks.last(), log_blocks.first())
            && last_object >= first_log
        {
            return Err(malformed(*first_log as usize, "log blocks precede object blocks"));
        }
        if object_blocks.is_empty() {
            if self.footer.obj_id_len != 0 || self.footer.obj_index_position != 0 || !self.objects.is_empty() {
                return Err(malformed(
                    self.footer.start,
                    "object footer fields exist without object blocks",
                ));
            }
        } else {
            if !(2..=31).contains(&self.footer.obj_id_len) {
                return Err(malformed(
                    self.footer.start,
                    "object abbreviation length is outside 2..=31",
                ));
            }
            if self.footer.obj_id_len as usize > self.header.object_hash.len_in_bytes() {
                return Err(malformed(
                    self.footer.start,
                    "object abbreviation exceeds the configured object hash width",
                ));
            }
            if self
                .objects
                .iter()
                .any(|record| record.key.len() != self.footer.obj_id_len as usize)
            {
                return Err(malformed(
                    self.footer.obj_position as usize,
                    "object key length differs from the footer",
                ));
            }
            let ref_positions = ref_blocks.iter().copied().collect::<BTreeSet<_>>();
            if self
                .objects
                .iter()
                .flat_map(|record| &record.positions)
                .any(|position| !ref_positions.contains(position))
            {
                return Err(malformed(
                    self.footer.obj_position as usize,
                    "object record points outside the ref section",
                ));
            }
            self.validate_object_records()?;
        }
        if self.header.block_size == 0 && ref_blocks.len() > 1 && self.footer.ref_index_position == 0 {
            return Err(malformed(
                self.footer.start,
                "an unaligned multi-block ref section requires an index",
            ));
        }
        if log_blocks.len() > 1 && self.footer.log_index_position == 0 {
            return Err(malformed(
                self.footer.start,
                "a multi-block log section requires an index",
            ));
        }

        let mut owned_indexes = BTreeSet::new();
        let block_map = self
            .blocks
            .iter()
            .map(|block| (block.position, block))
            .collect::<BTreeMap<_, _>>();
        self.validate_index(
            self.footer.ref_index_position,
            BlockKind::Ref,
            &ref_blocks,
            &mut owned_indexes,
            &block_map,
        )?;
        self.validate_index(
            self.footer.obj_index_position,
            BlockKind::Object,
            &object_blocks,
            &mut owned_indexes,
            &block_map,
        )?;
        self.validate_index(
            self.footer.log_index_position,
            BlockKind::Log,
            &log_blocks,
            &mut owned_indexes,
            &block_map,
        )?;
        let all_indexes = self.positions(BlockKind::Index).into_iter().collect::<BTreeSet<_>>();
        if owned_indexes != all_indexes {
            return Err(malformed(
                self.footer.start,
                "an index block is unreachable from the footer",
            ));
        }
        Ok(())
    }

    fn validate_index(
        &self,
        root: u64,
        leaf_kind: BlockKind,
        expected_leaves: &[u64],
        owned_indexes: &mut BTreeSet<u64>,
        block_map: &BTreeMap<u64, &Block>,
    ) -> Result<(), Error> {
        if root == 0 {
            return Ok(());
        }
        let mut leaves = BTreeSet::new();
        let mut visiting = BTreeSet::new();
        IndexWalk {
            leaf_kind,
            leaves: &mut leaves,
            visiting: &mut visiting,
            owned_indexes,
            block_map,
        }
        .walk(root, 0)?;
        if leaves != expected_leaves.iter().copied().collect() {
            return Err(malformed(root as usize, "index does not cover exactly its data blocks"));
        }
        Ok(())
    }

    fn validate_object_records(&self) -> Result<(), Error> {
        let prefix_len = self.footer.obj_id_len as usize;
        let mut expected = HashMap::<Vec<u8>, ExpectedObject>::new();
        for block in self.blocks.iter().filter(|block| block.kind == BlockKind::Ref) {
            for record in &self.refs[block.records.clone()] {
                let object_ids = match &record.value {
                    RefValue::Direct(object_id) => [Some(object_id.as_ref()), None],
                    RefValue::Peeled { target, peeled } => [Some(target.as_ref()), Some(peeled.as_ref())],
                    RefValue::Deletion | RefValue::Symbolic(_) => [None, None],
                };
                for object_id in object_ids.into_iter().flatten() {
                    let key = object_id.as_bytes()[..prefix_len].to_vec();
                    match expected.entry(key) {
                        Entry::Vacant(entry) => {
                            entry.insert(ExpectedObject {
                                object_id: object_id.to_owned(),
                                positions: vec![block.position],
                            });
                        }
                        Entry::Occupied(mut entry) => {
                            let expected = entry.get_mut();
                            if expected.object_id.as_ref() != object_id {
                                return Err(malformed(
                                    self.footer.obj_position as usize,
                                    "object abbreviation is not unique within the table",
                                ));
                            }
                            if expected.positions.last().copied() != Some(block.position) {
                                expected.positions.push(block.position);
                            }
                        }
                    }
                }
            }
        }
        if expected.len() != self.objects.len()
            || self.objects.iter().any(|record| {
                expected
                    .get(&record.key)
                    .is_none_or(|expected| !record.positions.is_empty() && record.positions != expected.positions)
            })
        {
            return Err(malformed(
                self.footer.obj_position as usize,
                "object records do not exactly map object IDs to containing ref blocks",
            ));
        }
        Ok(())
    }

    fn search_blocks(&self, root: u64, leaf_kind: BlockKind) -> Result<Vec<SearchBlock>, Error> {
        let block_map = self
            .blocks
            .iter()
            .map(|block| (block.position, block))
            .collect::<BTreeMap<_, _>>();
        let positions = if root == 0 {
            self.positions(leaf_kind)
        } else {
            let mut positions = Vec::new();
            Self::collect_index_leaves(root, leaf_kind, 0, &block_map, &mut positions)?;
            positions
        };
        positions
            .into_iter()
            .map(|position| {
                let block = block_map
                    .get(&position)
                    .copied()
                    .ok_or_else(|| malformed(position as usize, "search index points outside known blocks"))?;
                Ok(SearchBlock {
                    last_key: block.last_key.clone(),
                    records: block.records.clone(),
                })
            })
            .collect()
    }

    fn collect_index_leaves(
        position: u64,
        leaf_kind: BlockKind,
        depth: usize,
        block_map: &BTreeMap<u64, &Block>,
        leaves: &mut Vec<u64>,
    ) -> Result<(), Error> {
        if depth > 64 {
            return Err(malformed(position as usize, "search index nesting exceeds 64 levels"));
        }
        let block = block_map
            .get(&position)
            .copied()
            .ok_or_else(|| malformed(position as usize, "search index points outside known blocks"))?;
        for (_, target_position) in &block.index_entries {
            let target = block_map
                .get(target_position)
                .copied()
                .ok_or_else(|| malformed(position as usize, "search index points outside known blocks"))?;
            match target.kind {
                BlockKind::Index => {
                    Self::collect_index_leaves(*target_position, leaf_kind, depth + 1, block_map, leaves)?;
                }
                kind if kind == leaf_kind => leaves.push(*target_position),
                _ => return Err(malformed(position as usize, "search index points into another section")),
            }
        }
        Ok(())
    }

    fn object_lookup(&self) -> Result<Vec<ObjectLookup>, Error> {
        let ref_blocks = self
            .blocks
            .iter()
            .filter(|block| block.kind == BlockKind::Ref)
            .map(|block| (block.position, block.records.clone()))
            .collect::<BTreeMap<_, _>>();
        self.objects
            .iter()
            .map(|record| {
                let ref_ranges = record
                    .positions
                    .iter()
                    .map(|position| {
                        ref_blocks
                            .get(position)
                            .cloned()
                            .ok_or_else(|| malformed(*position as usize, "object record points outside ref blocks"))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ObjectLookup {
                    key: record.key.clone(),
                    ref_ranges,
                })
            })
            .collect()
    }

    fn positions(&self, kind: BlockKind) -> Vec<u64> {
        self.blocks
            .iter()
            .filter(|block| block.kind == kind)
            .map(|block| block.position)
            .collect()
    }

    fn record_count(&self, kind: BlockKind) -> usize {
        match kind {
            BlockKind::Ref => self.refs.len(),
            BlockKind::Object => self.objects.len(),
            BlockKind::Log => self.logs.len(),
            BlockKind::Index => 0,
        }
    }

    fn ensure_update_index(&self, value: u64, offset: usize) -> Result<(), Error> {
        if value < self.header.min_update_index || value > self.header.max_update_index {
            return Err(malformed(offset, "record update index is outside the header range"));
        }
        Ok(())
    }

    fn bump_record_count(&mut self) -> Result<(), Error> {
        self.records_seen = self.records_seen.checked_add(1).ok_or(Error::Limit("record count"))?;
        if self.records_seen > self.limits.max_records {
            return Err(Error::Limit("record count"));
        }
        Ok(())
    }
}

struct IndexWalk<'a, 'blocks> {
    leaf_kind: BlockKind,
    leaves: &'a mut BTreeSet<u64>,
    visiting: &'a mut BTreeSet<u64>,
    owned_indexes: &'a mut BTreeSet<u64>,
    block_map: &'a BTreeMap<u64, &'blocks Block>,
}

impl IndexWalk<'_, '_> {
    fn walk(&mut self, position: u64, depth: usize) -> Result<(), Error> {
        if depth > 64 {
            return Err(malformed(position as usize, "index nesting exceeds 64 levels"));
        }
        let block = self
            .block_map
            .get(&position)
            .copied()
            .ok_or_else(|| malformed(position as usize, "index points outside known blocks"))?;
        if block.kind != BlockKind::Index {
            return Err(malformed(
                position as usize,
                "footer index position does not name an index block",
            ));
        }
        if !self.visiting.insert(position) {
            return Err(malformed(position as usize, "index contains a cycle"));
        }
        if !self.owned_indexes.insert(position) {
            return Err(malformed(position as usize, "index block is shared between sections"));
        }
        for (key, target_position) in &block.index_entries {
            if *target_position >= position {
                return Err(malformed(position as usize, "index does not point backward"));
            }
            let target = self
                .block_map
                .get(target_position)
                .copied()
                .ok_or_else(|| malformed(position as usize, "index points outside known blocks"))?;
            if key != &target.last_key {
                return Err(malformed(
                    position as usize,
                    "index key is not the target block's last key",
                ));
            }
            match target.kind {
                BlockKind::Index => self.walk(*target_position, depth + 1)?,
                kind if kind == self.leaf_kind => {
                    self.leaves.insert(*target_position);
                }
                _ => return Err(malformed(position as usize, "index points into another section")),
            }
        }
        self.visiting.remove(&position);
        Ok(())
    }
}

impl BlockKind {
    fn byte(self) -> u8 {
        match self {
            BlockKind::Ref => b'r',
            BlockKind::Object => b'o',
            BlockKind::Log => b'g',
            BlockKind::Index => b'i',
        }
    }
}

struct Cursor<'a> {
    data: &'a [u8],
    base: usize,
    position: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8], base: usize, position: usize) -> Self {
        Cursor { data, base, position }
    }

    fn absolute(&self) -> usize {
        self.base.saturating_add(self.position)
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.position)
    }

    fn bytes(&mut self, len: usize) -> Result<&'a [u8], Error> {
        let end = self
            .position
            .checked_add(len)
            .ok_or_else(|| malformed(self.absolute(), "record length overflows"))?;
        let value = self
            .data
            .get(self.position..end)
            .ok_or_else(|| malformed(self.absolute(), "record is truncated"))?;
        self.position = end;
        Ok(value)
    }

    fn varint(&mut self) -> Result<u64, Error> {
        let (value, consumed) = varint::decode(
            self.data
                .get(self.position..)
                .ok_or_else(|| malformed(self.absolute(), "varint is truncated"))?,
        )
        .map_err(|_| malformed(self.absolute(), "invalid offset-varint"))?;
        self.position = self
            .position
            .checked_add(consumed)
            .ok_or_else(|| malformed(self.absolute(), "varint position overflows"))?;
        Ok(value)
    }

    fn varbytes(&mut self, limit: usize) -> Result<&'a [u8], Error> {
        let len = usize::try_from(self.varint()?)
            .map_err(|_| malformed(self.absolute(), "variable byte string does not fit in memory"))?;
        if len > limit {
            return Err(Error::Limit("record value size"));
        }
        self.bytes(len)
    }

    fn object_id(&mut self, kind: Kind) -> Result<ObjectId, Error> {
        ObjectId::try_from(self.bytes(kind.len_in_bytes())?)
            .map_err(|_| malformed(self.absolute(), "object id has an unsupported length"))
    }
}

fn parse_header(data: &[u8], offset: usize) -> Result<Header, Error> {
    let prefix = data
        .get(offset..offset + 24)
        .ok_or_else(|| malformed(offset, "header is truncated"))?;
    if &prefix[..4] != b"REFT" {
        return Err(malformed(offset, "header magic is not REFT"));
    }
    let version = match prefix[4] {
        1 => Version::V1,
        2 => Version::V2,
        _ => return Err(malformed(offset + 4, "unsupported reftable version")),
    };
    let full = data
        .get(offset..offset + version.header_len())
        .ok_or_else(|| malformed(offset, "header is truncated"))?;
    let block_size = read_u24(full, 5)? as u32;
    let min_update_index = read_u64(full, 8)?;
    let max_update_index = read_u64(full, 16)?;
    if min_update_index > max_update_index {
        return Err(malformed(offset + 8, "minimum update index exceeds maximum"));
    }
    let object_hash = match version {
        Version::V1 => sha1_kind()?,
        Version::V2 => hash_kind(
            full.get(24..28)
                .ok_or_else(|| malformed(offset + 24, "hash id is truncated"))?,
        )?,
    };
    Ok(Header {
        version,
        block_size,
        min_update_index,
        max_update_index,
        object_hash,
    })
}

fn parse_footer(data: &[u8], header: Header) -> Result<Footer, Error> {
    let footer_len = header.version.footer_len();
    let start = data
        .len()
        .checked_sub(footer_len)
        .ok_or_else(|| malformed(0, "table is shorter than its footer"))?;
    if start < header.version.header_len() {
        return Err(malformed(start, "footer overlaps the file header"));
    }
    let footer_bytes = &data[start..];
    let expected_crc = read_u32(footer_bytes, footer_len - 4)?;
    let actual_crc = gix_features::hash::crc32(&footer_bytes[..footer_len - 4]);
    if actual_crc != expected_crc {
        return Err(malformed(start + footer_len - 4, "footer CRC-32 does not match"));
    }
    let footer_header = parse_header(data, start)?;
    if footer_header != header {
        return Err(malformed(start, "footer header differs from the file header"));
    }
    let fields = start + header.version.header_len();
    let ref_index_position = read_u64(data, fields)?;
    let packed_object = read_u64(data, fields + 8)?;
    let obj_position = packed_object >> 5;
    let obj_id_len = (packed_object & 31) as u8;
    let obj_index_position = read_u64(data, fields + 16)?;
    let log_position = read_u64(data, fields + 24)?;
    let log_index_position = read_u64(data, fields + 32)?;
    for position in [
        ref_index_position,
        obj_position,
        obj_index_position,
        log_position,
        log_index_position,
    ] {
        if position != 0 {
            let position = usize::try_from(position)
                .map_err(|_| malformed(start, "footer section position does not fit in memory"))?;
            if position >= start {
                return Err(malformed(start, "footer section position is outside the table body"));
            }
        }
    }
    Ok(Footer {
        start,
        ref_index_position,
        obj_position,
        obj_id_len,
        obj_index_position,
        log_position,
        log_index_position,
    })
}

fn sha1_kind() -> Result<Kind, Error> {
    #[cfg(feature = "sha1")]
    {
        Ok(Kind::Sha1)
    }
    #[cfg(not(feature = "sha1"))]
    {
        Err(Error::Unsupported("SHA-1 support is not compiled in"))
    }
}

fn hash_kind(id: &[u8]) -> Result<Kind, Error> {
    let _ = id;
    #[cfg(feature = "sha1")]
    if id == b"sha1" {
        return Ok(Kind::Sha1);
    }
    #[cfg(feature = "sha256")]
    if id == b"s256" {
        return Ok(Kind::Sha256);
    }
    Err(Error::Unsupported("table hash id"))
}

fn read_u16_at_end(data: &[u8], base: usize) -> Result<u16, Error> {
    let start = data
        .len()
        .checked_sub(2)
        .ok_or_else(|| malformed(base, "block is shorter than its restart count"))?;
    let bytes = data
        .get(start..)
        .ok_or_else(|| malformed(base + start, "restart count is truncated"))?;
    Ok(u16::from_be_bytes(
        bytes
            .try_into()
            .map_err(|_| malformed(base + start, "restart count is truncated"))?,
    ))
}

fn read_u24(data: &[u8], offset: usize) -> Result<usize, Error> {
    let bytes = data
        .get(offset..offset + 3)
        .ok_or_else(|| malformed(offset, "24-bit integer is truncated"))?;
    Ok((usize::from(bytes[0]) << 16) | (usize::from(bytes[1]) << 8) | usize::from(bytes[2]))
}

fn read_u32(data: &[u8], offset: usize) -> Result<u32, Error> {
    let bytes = data
        .get(offset..offset + 4)
        .ok_or_else(|| malformed(offset, "32-bit integer is truncated"))?;
    Ok(u32::from_be_bytes(
        bytes
            .try_into()
            .map_err(|_| malformed(offset, "32-bit integer is truncated"))?,
    ))
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64, Error> {
    let bytes = data
        .get(offset..offset + 8)
        .ok_or_else(|| malformed(offset, "64-bit integer is truncated"))?;
    Ok(u64::from_be_bytes(
        bytes
            .try_into()
            .map_err(|_| malformed(offset, "64-bit integer is truncated"))?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_position_count_is_bounded_by_remaining_data_before_allocation() {
        let count = 1_000_000usize;
        let mut encoded_count = Vec::new();
        varint::encode(count as u64, &mut encoded_count);
        let mut cursor = Cursor::new(&encoded_count, 0, 0);
        let mut parser = Parser {
            data: &[],
            limits: Limits {
                max_records: count,
                ..Limits::default()
            },
            header: Header {
                version: Version::V2,
                block_size: 4096,
                min_update_index: 0,
                max_update_index: 0,
                object_hash: Kind::shortest(),
            },
            footer: Footer {
                start: 0,
                ref_index_position: 0,
                obj_position: 0,
                obj_id_len: 0,
                obj_index_position: 0,
                log_position: 0,
                log_index_position: 0,
            },
            records_seen: 0,
            refs: Vec::new(),
            logs: Vec::new(),
            objects: Vec::new(),
            blocks: Vec::new(),
            last_log_key: Vec::new(),
        };

        let err = parser
            .decode_object(&mut cursor, vec![1], 0)
            .expect_err("a count without enough encoded positions must be rejected");
        assert!(
            matches!(
                err,
                Error::Malformed {
                    message: "object block position count exceeds the remaining record data",
                    ..
                }
            ),
            "the byte bound is checked before reserving the advertised count: {err:?}"
        );
        assert!(parser.objects.is_empty(), "no partial object record is retained");
    }
}
