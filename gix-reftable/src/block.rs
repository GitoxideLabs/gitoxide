use std::cmp::Ordering;

use flate2::{FlushDecompress, Status};

use crate::{
    basics::{get_be16, get_be24},
    blocksource::BlockSource,
    constants,
    error::Error,
    record::{decode_key, decode_key_len, Record},
};

/// Size of the file header for a reftable `version`.
pub fn header_size(version: u8) -> Result<usize, Error> {
    match version {
        1 => Ok(24),
        2 => Ok(28),
        _ => Err(Error::Malformed("unsupported reftable version")),
    }
}

/// Size of the file footer for a reftable `version`.
pub fn footer_size(version: u8) -> Result<usize, Error> {
    match version {
        1 => Ok(68),
        2 => Ok(72),
        _ => Err(Error::Malformed("unsupported reftable version")),
    }
}

/// A decoded reftable block.
#[derive(Debug, Clone)]
pub struct Block {
    /// Offset of a file header in this block (non-zero only for first block).
    pub header_off: u32,
    /// Decoded block bytes.
    pub data: Vec<u8>,
    /// Hash size used for records in this table.
    pub hash_size: usize,
    /// Number of restart points.
    pub restart_count: u16,
    /// Start of restart table (relative to this block start).
    pub restart_off: u32,
    /// Number of bytes consumed in the source file for this block.
    pub full_block_size: u32,
    /// Block type.
    pub block_type: u8,
}

impl Block {
    /// Decode a block at `offset`.
    ///
    /// Returns `Ok(None)` when no block exists at `offset` or the type does not match `want_type`.
    pub fn init(
        source: &BlockSource,
        offset: u64,
        header_off: u32,
        table_block_size: u32,
        hash_size: usize,
        want_type: u8,
    ) -> Result<Option<Self>, Error> {
        let guess_block_size = if table_block_size > 0 {
            table_block_size as usize
        } else {
            constants::DEFAULT_BLOCK_SIZE
        };

        let mut data = source.read(offset, guess_block_size as u32)?.to_vec();
        if data.is_empty() {
            return Ok(None);
        }

        let header_off_usize = header_off as usize;
        if data.len() < header_off_usize + 4 {
            return Err(Error::Truncated);
        }

        let block_type = data[header_off_usize];
        if !is_block_type(block_type) {
            return Err(Error::Malformed("invalid block type"));
        }
        if want_type != constants::BLOCK_TYPE_ANY && want_type != block_type {
            return Ok(None);
        }

        let mut block_size_buf = [0u8; 3];
        block_size_buf.copy_from_slice(&data[header_off_usize + 1..header_off_usize + 4]);
        let block_size = get_be24(&block_size_buf) as usize;
        if block_size < header_off_usize + 4 {
            return Err(Error::Malformed("invalid block size"));
        }

        if block_size > data.len() {
            data = source.read(offset, block_size as u32)?.to_vec();
        }

        let (decoded_data, full_block_size) = if block_type == constants::BLOCK_TYPE_LOG {
            let block_header_skip = header_off_usize + 4;
            if block_size < block_header_skip || data.len() < block_header_skip {
                return Err(Error::Malformed("invalid log block size"));
            }

            let mut uncompressed = vec![0u8; block_size - block_header_skip];
            let mut decompressor = flate2::Decompress::new(true);
            let status = decompressor
                .decompress(&data[block_header_skip..], &mut uncompressed, FlushDecompress::Finish)
                .map_err(|_| Error::Zlib)?;
            if status != Status::StreamEnd || decompressor.total_out() as usize != uncompressed.len() {
                return Err(Error::Zlib);
            }

            let mut out = Vec::with_capacity(block_size);
            out.extend_from_slice(&data[..block_header_skip]);
            out.extend_from_slice(&uncompressed);
            (out, (block_header_skip + decompressor.total_in() as usize) as u32)
        } else {
            if data.len() < block_size {
                return Err(Error::Truncated);
            }
            let mut full_block_size = if table_block_size == 0 {
                block_size as u32
            } else {
                table_block_size
            };
            if block_size < full_block_size as usize && block_size < data.len() && data[block_size] != 0 {
                full_block_size = block_size as u32;
            }
            (data, full_block_size)
        };

        if decoded_data.len() < block_size {
            return Err(Error::Truncated);
        }
        if block_size < 2 {
            return Err(Error::Malformed("block too small"));
        }

        let mut restart_count_buf = [0u8; 2];
        restart_count_buf.copy_from_slice(&decoded_data[block_size - 2..block_size]);
        let restart_count = get_be16(&restart_count_buf);
        let restart_off = block_size
            .checked_sub(2 + 3 * restart_count as usize)
            .ok_or(Error::Malformed("invalid restart table"))? as u32;

        Ok(Some(Self {
            header_off,
            data: decoded_data,
            hash_size,
            restart_count,
            restart_off,
            full_block_size,
            block_type,
        }))
    }

    /// Returns the first key in this block.
    pub fn first_key(&self) -> Result<Vec<u8>, Error> {
        let mut key = Vec::new();
        let off = self.header_off as usize + 4;
        let end = self.restart_off as usize;
        if off >= end || end > self.data.len() {
            return Err(Error::Malformed("block has no record payload"));
        }
        let (consumed, _extra) = decode_key(&mut key, &self.data[off..end])?;
        if consumed == 0 || key.is_empty() {
            return Err(Error::Malformed("invalid first key"));
        }
        Ok(key)
    }

    fn restart_offset(&self, idx: usize) -> Result<u32, Error> {
        if idx >= self.restart_count as usize {
            return Err(Error::Malformed("restart index out of bounds"));
        }
        let off = self.restart_off as usize + 3 * idx;
        let mut buf = [0u8; 3];
        buf.copy_from_slice(&self.data[off..off + 3]);
        Ok(get_be24(&buf))
    }
}

/// Iterator over records in a single block.
#[derive(Debug, Clone)]
pub struct BlockIter {
    pub(crate) block: Block,
    next_off: u32,
    last_key: Vec<u8>,
}

impl BlockIter {
    /// Initialize an iterator over `block` at the first record.
    pub fn new(block: Block) -> Self {
        Self {
            next_off: block.header_off + 4,
            block,
            last_key: Vec::new(),
        }
    }

    /// Seek to the first key >= `want`.
    pub fn seek_key(&mut self, want: &[u8]) -> Result<(), Error> {
        let restart_index = self.find_first_restart_greater_than(want)?;
        if restart_index > 0 {
            self.next_off = self.block.restart_offset(restart_index - 1)?;
        } else {
            self.next_off = self.block.header_off + 4;
        }
        self.last_key.clear();

        loop {
            let prev_off = self.next_off;
            let Some(record) = self.next_record()? else {
                self.next_off = prev_off;
                return Ok(());
            };

            let key = record.key();
            if key.as_slice() >= want {
                self.next_off = prev_off;
                self.last_key = key;
                return Ok(());
            }
        }
    }

    /// Decode and return the next record.
    pub fn next_record(&mut self) -> Result<Option<Record>, Error> {
        if self.next_off >= self.block.restart_off {
            return Ok(None);
        }

        let start = self.next_off as usize;
        let end = self.block.restart_off as usize;
        if end > self.block.data.len() || start > end {
            return Err(Error::Malformed("invalid record boundaries"));
        }

        let input = &self.block.data[start..end];
        let (key_bytes_consumed, extra) = decode_key(&mut self.last_key, input)?;
        if self.last_key.is_empty() {
            return Err(Error::Malformed("empty record key"));
        }

        let payload = &input[key_bytes_consumed..];
        let (record, payload_consumed) = Record::decode_consuming(
            self.block.block_type,
            &self.last_key,
            extra,
            payload,
            self.block.hash_size,
        )?;

        self.next_off = self
            .next_off
            .checked_add((key_bytes_consumed + payload_consumed) as u32)
            .ok_or(Error::Malformed("offset overflow"))?;

        Ok(Some(record))
    }

    /// Access the currently iterated block.
    pub fn block(&self) -> &Block {
        &self.block
    }

    fn find_first_restart_greater_than(&self, want: &[u8]) -> Result<usize, Error> {
        let mut low = 0usize;
        let mut high = self.block.restart_count as usize;

        while low < high {
            let mid = low + (high - low) / 2;
            match self.restart_key_cmp(mid, want)? {
                Ordering::Greater => high = mid,
                Ordering::Equal | Ordering::Less => low = mid + 1,
            }
        }

        Ok(low)
    }

    fn restart_key_cmp(&self, idx: usize, want: &[u8]) -> Result<Ordering, Error> {
        let off = self.block.restart_offset(idx)? as usize;
        let restart_off = self.block.restart_off as usize;
        if off >= restart_off {
            return Err(Error::Malformed("restart points outside payload"));
        }
        let in_block = &self.block.data[off..restart_off];

        let (prefix_len, suffix_len, _extra, consumed) = decode_key_len(in_block)?;
        if prefix_len != 0 {
            return Err(Error::Malformed("restart key must have empty prefix"));
        }
        if in_block.len().saturating_sub(consumed) < suffix_len {
            return Err(Error::Truncated);
        }
        let key = &in_block[consumed..consumed + suffix_len];
        Ok(key.cmp(want))
    }
}

fn is_block_type(typ: u8) -> bool {
    matches!(
        typ,
        constants::BLOCK_TYPE_REF | constants::BLOCK_TYPE_LOG | constants::BLOCK_TYPE_OBJ | constants::BLOCK_TYPE_INDEX
    )
}
