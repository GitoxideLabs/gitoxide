use std::cmp::Ordering;

use crate::{
    basics::{common_prefix_size, decode_varint, encode_varint, get_be16, get_be64, put_be16, put_be64},
    constants,
    error::Error,
};

/// Variants of values stored in [`RefRecord`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefValue {
    /// Tombstone entry.
    Deletion,
    /// A single object id.
    Val1(Vec<u8>),
    /// A peeled tag with object and target object ids.
    Val2 {
        /// Direct value.
        value: Vec<u8>,
        /// Peeled target value.
        target_value: Vec<u8>,
    },
    /// Symbolic reference.
    Symref(String),
}

/// A reference record (`r` block).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefRecord {
    /// Full refname.
    pub refname: String,
    /// Logical update index.
    pub update_index: u64,
    /// Associated value.
    pub value: RefValue,
}

/// Variants of values stored in [`LogRecord`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogValue {
    /// Tombstone entry.
    Deletion,
    /// Standard reflog update.
    Update {
        /// Previous object id.
        old_hash: Vec<u8>,
        /// New object id.
        new_hash: Vec<u8>,
        /// Committer name.
        name: String,
        /// Committer email.
        email: String,
        /// Commit time (seconds since epoch).
        time: u64,
        /// Timezone offset in minutes.
        tz_offset: i16,
        /// Reflog message.
        message: String,
    },
}

impl LogValue {
    fn update(
        old_hash: Vec<u8>,
        new_hash: Vec<u8>,
        name: String,
        email: String,
        time: u64,
        tz_offset: i16,
        message: String,
    ) -> Self {
        Self::Update {
            old_hash,
            new_hash,
            name,
            email,
            time,
            tz_offset,
            message,
        }
    }
}

/// A reflog record (`g` block).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRecord {
    /// Full refname.
    pub refname: String,
    /// Logical update index.
    pub update_index: u64,
    /// Associated value.
    pub value: LogValue,
}

/// Object index record (`o` block).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjRecord {
    /// Prefix of an object id.
    pub hash_prefix: Vec<u8>,
    /// Absolute offsets of referenced ref blocks.
    pub offsets: Vec<u64>,
}

/// Secondary index record (`i` block).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexRecord {
    /// Last key in the indexed block.
    pub last_key: Vec<u8>,
    /// Offset of the indexed block.
    pub offset: u64,
}

/// Any typed record stored in blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
    /// Reference record.
    Ref(RefRecord),
    /// Reflog record.
    Log(LogRecord),
    /// Object index record.
    Obj(ObjRecord),
    /// Secondary index record.
    Index(IndexRecord),
}

impl Record {
    /// Return block type of this record.
    pub fn block_type(&self) -> u8 {
        match self {
            Record::Ref(_) => constants::BLOCK_TYPE_REF,
            Record::Log(_) => constants::BLOCK_TYPE_LOG,
            Record::Obj(_) => constants::BLOCK_TYPE_OBJ,
            Record::Index(_) => constants::BLOCK_TYPE_INDEX,
        }
    }

    /// Return record value subtype (3-bit `extra`).
    pub fn val_type(&self) -> u8 {
        match self {
            Record::Ref(r) => match r.value {
                RefValue::Deletion => constants::REF_VAL_DELETION,
                RefValue::Val1(_) => constants::REF_VAL_VAL1,
                RefValue::Val2 { .. } => constants::REF_VAL_VAL2,
                RefValue::Symref(_) => constants::REF_VAL_SYMREF,
            },
            Record::Log(l) => match l.value {
                LogValue::Deletion => constants::LOG_VAL_DELETION,
                LogValue::Update { .. } => constants::LOG_VAL_UPDATE,
            },
            Record::Obj(o) => {
                let len = o.offsets.len();
                if (1..8).contains(&len) {
                    len as u8
                } else {
                    0
                }
            }
            Record::Index(_) => 0,
        }
    }

    /// Returns true if this is a tombstone/deletion record.
    pub fn is_deletion(&self) -> bool {
        matches!(
            self,
            Record::Ref(RefRecord {
                value: RefValue::Deletion,
                ..
            }) | Record::Log(LogRecord {
                value: LogValue::Deletion,
                ..
            })
        )
    }

    /// Produce sort key bytes.
    pub fn key(&self) -> Vec<u8> {
        match self {
            Record::Ref(r) => r.refname.as_bytes().to_vec(),
            Record::Log(l) => {
                let mut out = Vec::with_capacity(l.refname.len() + 1 + 8);
                out.extend_from_slice(l.refname.as_bytes());
                out.push(0);
                let mut ts = [0u8; 8];
                put_be64(&mut ts, u64::MAX - l.update_index);
                out.extend_from_slice(&ts);
                out
            }
            Record::Obj(o) => o.hash_prefix.clone(),
            Record::Index(i) => i.last_key.clone(),
        }
    }

    /// Encode record value bytes.
    pub fn encode(&self, hash_size: usize) -> Result<Vec<u8>, Error> {
        match self {
            Record::Ref(r) => encode_ref_record(r, hash_size),
            Record::Log(l) => encode_log_record(l, hash_size),
            Record::Obj(o) => encode_obj_record(o),
            Record::Index(i) => encode_index_record(i),
        }
    }

    /// Decode a record from type, key, val-type and value payload.
    pub fn decode(block_type: u8, key: &[u8], val_type: u8, payload: &[u8], hash_size: usize) -> Result<Self, Error> {
        let (record, consumed) = Self::decode_consuming(block_type, key, val_type, payload, hash_size)?;
        if consumed != payload.len() {
            return Err(Error::Malformed("unexpected trailing bytes in record"));
        }
        Ok(record)
    }

    /// Decode a record and return consumed payload bytes.
    pub fn decode_consuming(
        block_type: u8,
        key: &[u8],
        val_type: u8,
        payload: &[u8],
        hash_size: usize,
    ) -> Result<(Self, usize), Error> {
        match block_type {
            constants::BLOCK_TYPE_REF => {
                let (record, consumed) = decode_ref_record(key, val_type, payload, hash_size)?;
                Ok((Record::Ref(record), consumed))
            }
            constants::BLOCK_TYPE_LOG => {
                let (record, consumed) = decode_log_record(key, val_type, payload, hash_size)?;
                Ok((Record::Log(record), consumed))
            }
            constants::BLOCK_TYPE_OBJ => {
                let (record, consumed) = decode_obj_record(key, val_type, payload)?;
                Ok((Record::Obj(record), consumed))
            }
            constants::BLOCK_TYPE_INDEX => {
                let (record, consumed) = decode_index_record(key, payload)?;
                Ok((Record::Index(record), consumed))
            }
            _ => Err(Error::Malformed("unknown block type")),
        }
    }

    /// Compare records of the same variant by key-order.
    pub fn cmp_key(&self, other: &Self) -> Result<Ordering, Error> {
        match (self, other) {
            (Record::Ref(a), Record::Ref(b)) => Ok(a.refname.cmp(&b.refname)),
            (Record::Log(a), Record::Log(b)) => {
                let by_name = a.refname.cmp(&b.refname);
                if by_name != Ordering::Equal {
                    return Ok(by_name);
                }
                Ok(b.update_index.cmp(&a.update_index))
            }
            (Record::Obj(a), Record::Obj(b)) => {
                let common = a.hash_prefix.len().max(b.hash_prefix.len());
                for idx in 0..common {
                    let av = a.hash_prefix.get(idx).copied().unwrap_or(0);
                    let bv = b.hash_prefix.get(idx).copied().unwrap_or(0);
                    if av != bv {
                        return Ok(av.cmp(&bv));
                    }
                }
                Ok(a.hash_prefix.len().cmp(&b.hash_prefix.len()))
            }
            (Record::Index(a), Record::Index(b)) => Ok(a.last_key.cmp(&b.last_key)),
            _ => Err(Error::Malformed("cannot compare different record types")),
        }
    }
}

/// Encode a key using the same prefix-compression format as Git reftable.
///
/// Returns `(encoded, restart)` where `restart` is true when prefix length is zero.
pub fn encode_key(prev_key: &[u8], key: &[u8], extra: u8) -> Result<(Vec<u8>, bool), Error> {
    if extra > 7 {
        return Err(Error::Malformed("extra must fit in 3 bits"));
    }
    let prefix_len = common_prefix_size(prev_key, key);
    let suffix_len = key.len() - prefix_len;

    let mut out = Vec::with_capacity(16 + suffix_len);
    let mut buf = [0u8; 10];

    let n = encode_varint(prefix_len as u64, &mut buf);
    out.extend_from_slice(&buf[..n]);

    let n = encode_varint(((suffix_len as u64) << 3) | extra as u64, &mut buf);
    out.extend_from_slice(&buf[..n]);

    out.extend_from_slice(&key[prefix_len..]);
    Ok((out, prefix_len == 0))
}

/// Decode key length fields from an encoded key/value record.
pub fn decode_key_len(input: &[u8]) -> Result<(usize, usize, u8, usize), Error> {
    let (prefix_len, mut consumed) = decode_varint(input)?;
    let (suffix_and_extra, n2) = decode_varint(&input[consumed..])?;
    consumed += n2;

    let extra = (suffix_and_extra & 0x7) as u8;
    let suffix_len = (suffix_and_extra >> 3) as usize;
    Ok((prefix_len as usize, suffix_len, extra, consumed))
}

/// Decode key bytes into `last_key`, returning `(consumed, extra)`.
pub fn decode_key(last_key: &mut Vec<u8>, input: &[u8]) -> Result<(usize, u8), Error> {
    let (prefix_len, suffix_len, extra, mut consumed) = decode_key_len(input)?;
    if prefix_len > last_key.len() {
        return Err(Error::Malformed("prefix length exceeds previous key"));
    }
    if input.len().saturating_sub(consumed) < suffix_len {
        return Err(Error::Truncated);
    }

    last_key.truncate(prefix_len);
    last_key.extend_from_slice(&input[consumed..consumed + suffix_len]);
    consumed += suffix_len;

    Ok((consumed, extra))
}

fn encode_string(value: &str, out: &mut Vec<u8>) {
    let mut buf = [0u8; 10];
    let n = encode_varint(value.len() as u64, &mut buf);
    out.extend_from_slice(&buf[..n]);
    out.extend_from_slice(value.as_bytes());
}

fn decode_string(input: &[u8], cursor: &mut usize) -> Result<String, Error> {
    let (len, consumed) = decode_varint(&input[*cursor..])?;
    *cursor += consumed;
    let len = len as usize;
    if input.len().saturating_sub(*cursor) < len {
        return Err(Error::Truncated);
    }
    let bytes = &input[*cursor..*cursor + len];
    *cursor += len;
    String::from_utf8(bytes.to_vec()).map_err(|_| Error::Malformed("invalid utf-8 string"))
}

fn encode_ref_record(record: &RefRecord, hash_size: usize) -> Result<Vec<u8>, Error> {
    let mut out = Vec::with_capacity(64);
    let mut varint_buf = [0u8; 10];
    let n = encode_varint(record.update_index, &mut varint_buf);
    out.extend_from_slice(&varint_buf[..n]);

    match &record.value {
        RefValue::Deletion => {}
        RefValue::Val1(value) => {
            if value.len() != hash_size {
                return Err(Error::Malformed("ref val1 hash has wrong size"));
            }
            out.extend_from_slice(value);
        }
        RefValue::Val2 { value, target_value } => {
            if value.len() != hash_size || target_value.len() != hash_size {
                return Err(Error::Malformed("ref val2 hash has wrong size"));
            }
            out.extend_from_slice(value);
            out.extend_from_slice(target_value);
        }
        RefValue::Symref(target) => encode_string(target, &mut out),
    }
    Ok(out)
}

fn decode_ref_record(key: &[u8], val_type: u8, payload: &[u8], hash_size: usize) -> Result<(RefRecord, usize), Error> {
    let (update_index, mut cursor) = decode_varint(payload)?;
    let refname = String::from_utf8(key.to_vec()).map_err(|_| Error::Malformed("invalid refname utf-8"))?;

    let value = match val_type {
        constants::REF_VAL_DELETION => RefValue::Deletion,
        constants::REF_VAL_VAL1 => {
            if payload.len().saturating_sub(cursor) < hash_size {
                return Err(Error::Truncated);
            }
            let v = payload[cursor..cursor + hash_size].to_vec();
            cursor += hash_size;
            RefValue::Val1(v)
        }
        constants::REF_VAL_VAL2 => {
            if payload.len().saturating_sub(cursor) < hash_size * 2 {
                return Err(Error::Truncated);
            }
            let value = payload[cursor..cursor + hash_size].to_vec();
            cursor += hash_size;
            let target_value = payload[cursor..cursor + hash_size].to_vec();
            cursor += hash_size;
            RefValue::Val2 { value, target_value }
        }
        constants::REF_VAL_SYMREF => RefValue::Symref(decode_string(payload, &mut cursor)?),
        _ => return Err(Error::Malformed("unknown ref value type")),
    };

    Ok((
        RefRecord {
            refname,
            update_index,
            value,
        },
        cursor,
    ))
}

fn encode_log_record(record: &LogRecord, hash_size: usize) -> Result<Vec<u8>, Error> {
    match &record.value {
        LogValue::Deletion => Ok(Vec::new()),
        LogValue::Update {
            old_hash,
            new_hash,
            name,
            email,
            time,
            tz_offset,
            message,
        } => {
            if old_hash.len() != hash_size || new_hash.len() != hash_size {
                return Err(Error::Malformed("log hash has wrong size"));
            }
            let mut out = Vec::with_capacity(2 * hash_size + 64);
            out.extend_from_slice(old_hash);
            out.extend_from_slice(new_hash);
            encode_string(name, &mut out);
            encode_string(email, &mut out);

            let mut varint_buf = [0u8; 10];
            let n = encode_varint(*time, &mut varint_buf);
            out.extend_from_slice(&varint_buf[..n]);

            let mut be_tz = [0u8; 2];
            put_be16(&mut be_tz, *tz_offset as u16);
            out.extend_from_slice(&be_tz);

            encode_string(message, &mut out);
            Ok(out)
        }
    }
}

fn decode_log_record(key: &[u8], val_type: u8, payload: &[u8], hash_size: usize) -> Result<(LogRecord, usize), Error> {
    if key.len() <= 9 || key[key.len() - 9] != 0 {
        return Err(Error::Malformed("invalid log key"));
    }

    let refname =
        String::from_utf8(key[..key.len() - 9].to_vec()).map_err(|_| Error::Malformed("invalid log refname utf-8"))?;
    let mut rev_ts = [0u8; 8];
    rev_ts.copy_from_slice(&key[key.len() - 8..]);
    let update_index = u64::MAX - get_be64(&rev_ts);

    let (value, consumed) = match val_type {
        constants::LOG_VAL_DELETION => (LogValue::Deletion, 0),
        constants::LOG_VAL_UPDATE => {
            let mut cursor = 0;
            if payload.len() < 2 * hash_size {
                return Err(Error::Truncated);
            }

            let old_hash = payload[cursor..cursor + hash_size].to_vec();
            cursor += hash_size;
            let new_hash = payload[cursor..cursor + hash_size].to_vec();
            cursor += hash_size;

            let name = decode_string(payload, &mut cursor)?;
            let email = decode_string(payload, &mut cursor)?;
            let (time, consumed) = decode_varint(&payload[cursor..])?;
            cursor += consumed;

            if payload.len().saturating_sub(cursor) < 2 {
                return Err(Error::Truncated);
            }
            let mut tz = [0u8; 2];
            tz.copy_from_slice(&payload[cursor..cursor + 2]);
            cursor += 2;
            let tz_offset = get_be16(&tz) as i16;

            let message = decode_string(payload, &mut cursor)?;
            (
                LogValue::update(old_hash, new_hash, name, email, time, tz_offset, message),
                cursor,
            )
        }
        _ => return Err(Error::Malformed("unknown log value type")),
    };

    Ok((
        LogRecord {
            refname,
            update_index,
            value,
        },
        consumed,
    ))
}

fn encode_obj_record(record: &ObjRecord) -> Result<Vec<u8>, Error> {
    let mut out = Vec::with_capacity(32);
    let mut varint_buf = [0u8; 10];

    let offset_len = record.offsets.len();
    if offset_len == 0 || offset_len >= 8 {
        let n = encode_varint(offset_len as u64, &mut varint_buf);
        out.extend_from_slice(&varint_buf[..n]);
    }

    if offset_len == 0 {
        return Ok(out);
    }

    let first = record.offsets[0];
    let n = encode_varint(first, &mut varint_buf);
    out.extend_from_slice(&varint_buf[..n]);

    let mut last = first;
    for &offset in &record.offsets[1..] {
        let delta = offset
            .checked_sub(last)
            .ok_or(Error::Malformed("object offsets must be ascending"))?;
        let n = encode_varint(delta, &mut varint_buf);
        out.extend_from_slice(&varint_buf[..n]);
        last = offset;
    }

    Ok(out)
}

fn decode_obj_record(key: &[u8], val_type: u8, payload: &[u8]) -> Result<(ObjRecord, usize), Error> {
    let mut cursor = 0;

    let count = if val_type == 0 {
        let (count, consumed) = decode_varint(payload)?;
        cursor += consumed;
        count as usize
    } else {
        val_type as usize
    };

    let mut offsets = Vec::with_capacity(count);
    if count > 0 {
        let (first, consumed) = decode_varint(&payload[cursor..])?;
        cursor += consumed;
        offsets.push(first);

        let mut last = first;
        for _ in 1..count {
            let (delta, consumed) = decode_varint(&payload[cursor..])?;
            cursor += consumed;
            let next = last.checked_add(delta).ok_or(Error::VarintOverflow)?;
            offsets.push(next);
            last = next;
        }
    }

    Ok((
        ObjRecord {
            hash_prefix: key.to_vec(),
            offsets,
        },
        cursor,
    ))
}

fn encode_index_record(record: &IndexRecord) -> Result<Vec<u8>, Error> {
    let mut out = Vec::with_capacity(10);
    let mut varint_buf = [0u8; 10];
    let n = encode_varint(record.offset, &mut varint_buf);
    out.extend_from_slice(&varint_buf[..n]);
    Ok(out)
}

fn decode_index_record(key: &[u8], payload: &[u8]) -> Result<(IndexRecord, usize), Error> {
    let (offset, consumed) = decode_varint(payload)?;
    Ok((
        IndexRecord {
            last_key: key.to_vec(),
            offset,
        },
        consumed,
    ))
}

/// Compare reference names for sorting.
pub fn ref_record_compare_name(a: &RefRecord, b: &RefRecord) -> Ordering {
    a.refname.cmp(&b.refname)
}

/// Compare log records by key (`refname`, reverse `update_index`).
pub fn log_record_compare_key(a: &LogRecord, b: &LogRecord) -> Ordering {
    let by_name = a.refname.cmp(&b.refname);
    if by_name != Ordering::Equal {
        return by_name;
    }
    b.update_index.cmp(&a.update_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(seed: u8, hash_size: usize) -> Vec<u8> {
        (0..hash_size).map(|idx| seed.wrapping_add(idx as u8)).collect()
    }

    fn roundtrip(record: Record, hash_size: usize) {
        let key = record.key();
        let payload = record.encode(hash_size).expect("encode");
        let decoded =
            Record::decode(record.block_type(), &key, record.val_type(), &payload, hash_size).expect("decode");
        assert_eq!(record, decoded);
    }

    #[test]
    fn key_roundtrip() {
        let prev = b"refs/heads/master";
        let key = b"refs/tags/v1.0";
        let extra = 6;

        let (encoded, restart) = encode_key(prev, key, extra).expect("encode");
        assert!(!restart);

        let mut decoded = prev.to_vec();
        let (consumed, decoded_extra) = decode_key(&mut decoded, &encoded).expect("decode");
        assert_eq!(consumed, encoded.len());
        assert_eq!(decoded_extra, extra);
        assert_eq!(decoded, key);
    }

    #[test]
    fn ref_record_roundtrip() {
        let hash_size = 20;

        roundtrip(
            Record::Ref(RefRecord {
                refname: "refs/heads/main".into(),
                update_index: 1,
                value: RefValue::Deletion,
            }),
            hash_size,
        );

        roundtrip(
            Record::Ref(RefRecord {
                refname: "refs/heads/main".into(),
                update_index: 2,
                value: RefValue::Val1(hash(1, hash_size)),
            }),
            hash_size,
        );

        roundtrip(
            Record::Ref(RefRecord {
                refname: "refs/tags/v1".into(),
                update_index: 3,
                value: RefValue::Val2 {
                    value: hash(2, hash_size),
                    target_value: hash(3, hash_size),
                },
            }),
            hash_size,
        );

        roundtrip(
            Record::Ref(RefRecord {
                refname: "HEAD".into(),
                update_index: 4,
                value: RefValue::Symref("refs/heads/main".into()),
            }),
            hash_size,
        );
    }

    #[test]
    fn log_record_roundtrip() {
        let hash_size = 20;

        roundtrip(
            Record::Log(LogRecord {
                refname: "refs/heads/main".into(),
                update_index: 5,
                value: LogValue::Deletion,
            }),
            hash_size,
        );

        roundtrip(
            Record::Log(LogRecord {
                refname: "refs/heads/main".into(),
                update_index: 6,
                value: LogValue::Update {
                    old_hash: hash(10, hash_size),
                    new_hash: hash(20, hash_size),
                    name: "alice".into(),
                    email: "alice@example.com".into(),
                    time: 1_577_123_507,
                    tz_offset: 100,
                    message: "test".into(),
                },
            }),
            hash_size,
        );
    }

    #[test]
    fn obj_record_roundtrip() {
        roundtrip(
            Record::Obj(ObjRecord {
                hash_prefix: vec![1, 2, 3, 4, 0],
                offsets: vec![1, 2, 3],
            }),
            20,
        );

        roundtrip(
            Record::Obj(ObjRecord {
                hash_prefix: vec![1, 2, 3, 4, 0],
                offsets: vec![1, 2, 3, 4, 500, 600, 700, 800, 9_000],
            }),
            20,
        );

        roundtrip(
            Record::Obj(ObjRecord {
                hash_prefix: vec![1, 2, 3, 4, 0],
                offsets: vec![],
            }),
            20,
        );
    }

    #[test]
    fn index_record_roundtrip() {
        roundtrip(
            Record::Index(IndexRecord {
                last_key: b"refs/heads/main".to_vec(),
                offset: 42,
            }),
            20,
        );
    }

    #[test]
    fn comparisons_match_expectations() {
        let a = Record::Ref(RefRecord {
            refname: "refs/heads/main".into(),
            update_index: 1,
            value: RefValue::Val1(vec![0; 20]),
        });
        let b = Record::Ref(RefRecord {
            refname: "HEAD".into(),
            update_index: 1,
            value: RefValue::Symref("refs/heads/main".into()),
        });
        assert_eq!(a.cmp_key(&b).expect("same type"), Ordering::Greater);

        let l1 = Record::Log(LogRecord {
            refname: "refs/heads/main".into(),
            update_index: 42,
            value: LogValue::Deletion,
        });
        let l2 = Record::Log(LogRecord {
            refname: "refs/heads/main".into(),
            update_index: 22,
            value: LogValue::Deletion,
        });
        assert_eq!(l1.cmp_key(&l2).expect("same type"), Ordering::Less);
    }
}
