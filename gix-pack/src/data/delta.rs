use std::io::Write;

/// Returned when failing to apply deltas.
#[derive(thiserror::Error, Debug)]
#[allow(missing_docs)]
pub enum ApplyError {
    #[error("Corrupt delta data: {message}")]
    Corrupt { message: &'static str },
    #[error("Encountered unsupported command code: 0")]
    UnsupportedCommandCode,
    #[error("Delta copy from base: byte slices must match")]
    DeltaCopyBaseSliceMismatch,
    #[error("Delta copy data: byte slices must match")]
    DeltaCopyDataSliceMismatch,
}

/// Returned when failing to encode deltas.
#[derive(thiserror::Error, Debug)]
#[allow(missing_docs)]
pub enum EncodeError {
    #[error("Failed to write bytes: {0}")]
    IOError(std::io::Error),
    #[error("Too large offset in Copy instruction, should <= 0xffffffff, got {0}")]
    TooLargeOffset(usize),
    #[error("Too large size in Copy instruction, should <= 0x00ffffff, got {0}")]
    TooLargeSize(usize),
    #[error("Too large data in Add instruction, length should <= 127, got {0}")]
    TooLargeData(usize),
}

/// Given the decompressed pack delta `d`, decode a size in bytes (either the base object size or the result object size)
/// Equivalent to [this canonical git function](https://github.com/git/git/blob/311531c9de557d25ac087c1637818bd2aad6eb3a/delta.h#L89)
pub(crate) fn decode_header_size(d: &[u8]) -> Result<(u64, usize), ApplyError> {
    let mut shift = 0;
    let mut size = 0u64;
    let mut consumed = 0;
    for cmd in d.iter() {
        if shift >= u64::BITS {
            return Err(ApplyError::Corrupt {
                message: "delta header size uses more bits than fit into u64",
            });
        }
        consumed += 1;
        size |= (u64::from(*cmd) & 0x7f) << shift;
        shift += 7;
        if *cmd & 0x80 == 0 {
            return Ok((size, consumed));
        }
    }
    Err(ApplyError::Corrupt {
        message: "delta header size is truncated",
    })
}

pub(crate) fn apply(base: &[u8], mut target: &mut [u8], data: &[u8]) -> Result<(), ApplyError> {
    fn next_byte(data: &[u8], i: &mut usize) -> Result<u8, ApplyError> {
        let byte = *data.get(*i).ok_or(ApplyError::Corrupt {
            message: "delta copy instruction is truncated",
        })?;
        *i += 1;
        Ok(byte)
    }

    let mut i = 0;
    while let Some(cmd) = data.get(i) {
        i += 1;
        match cmd {
            // Copy
            cmd if cmd & 0b1000_0000 != 0 => {
                let (mut ofs, mut size): (u32, u32) = (0, 0);
                if cmd & 0b0000_0001 != 0 {
                    ofs = u32::from(next_byte(data, &mut i)?);
                }
                if cmd & 0b0000_0010 != 0 {
                    ofs |= u32::from(next_byte(data, &mut i)?) << 8;
                }
                if cmd & 0b0000_0100 != 0 {
                    ofs |= u32::from(next_byte(data, &mut i)?) << 16;
                }
                if cmd & 0b0000_1000 != 0 {
                    ofs |= u32::from(next_byte(data, &mut i)?) << 24;
                }
                if cmd & 0b0001_0000 != 0 {
                    size = u32::from(next_byte(data, &mut i)?);
                }
                if cmd & 0b0010_0000 != 0 {
                    size |= u32::from(next_byte(data, &mut i)?) << 8;
                }
                if cmd & 0b0100_0000 != 0 {
                    size |= u32::from(next_byte(data, &mut i)?) << 16;
                }
                if size == 0 {
                    size = 0x10000; // 65536
                }
                let ofs = ofs as usize;
                let end = ofs.checked_add(size as usize).ok_or(ApplyError::Corrupt {
                    message: "delta copy range overflows",
                })?;
                std::io::Write::write_all(
                    &mut target,
                    base.get(ofs..end).ok_or(ApplyError::Corrupt {
                        message: "delta copy range exceeds base object size",
                    })?,
                )
                .map_err(|_e| ApplyError::DeltaCopyBaseSliceMismatch)?;
            }
            0 => {
                return Err(ApplyError::Corrupt {
                    message: "delta command 0 is reserved and invalid",
                })
            }
            size => {
                let end = i.checked_add(*size as usize).ok_or(ApplyError::Corrupt {
                    message: "delta insert range overflows",
                })?;
                std::io::Write::write_all(
                    &mut target,
                    data.get(i..end).ok_or(ApplyError::Corrupt {
                        message: "delta insert data is truncated",
                    })?,
                )
                .map_err(|_e| ApplyError::DeltaCopyDataSliceMismatch)?;
                i = end;
            }
        }
    }
    debug_assert_eq!(
        i,
        data.len(),
        "delta instructions were not consumed completely, should be impossible"
    );
    if !target.is_empty() {
        return Err(ApplyError::Corrupt {
            message: "delta instructions produced fewer bytes than promised",
        });
    }

    Ok(())
}

/// Delta instruction
#[derive(Debug)]
pub enum Instruction<'a> {
    /// Copy data from source
    Copy {
        /// Start position to copy
        offset: usize,
        /// Data length in bytes
        size: usize,
    },
    /// Insert bytes embedded in instruction
    Add {
        /// Data to add
        data: &'a [u8],
    },
}

impl Instruction<'_> {
    /// Encode instruction to bytes.
    pub fn encode(self, mut writer: impl Write) -> Result<(), EncodeError> {
        match self {
            Self::Copy { offset, mut size } => {
                let mut header = 0x80u8;
                let mut buf = [0u8; 7];
                let mut n = 0;

                if size == 0x10000 {
                    size = 0;
                } else if size > 0x00ffffff {
                    return Err(EncodeError::TooLargeSize(size));
                }
                if offset > 0xffffffff {
                    return Err(EncodeError::TooLargeOffset(offset));
                }

                for i in 0..4 {
                    let byte = (offset >> (i * 8)) as u8;
                    if byte != 0 {
                        header |= 1 << i;
                        buf[n] = byte;
                        n += 1;
                    }
                }
                for i in 0..3 {
                    let byte = (size >> (i * 8)) as u8;
                    if byte != 0 {
                        header |= 1 << (4 + i);
                        buf[n] = byte;
                        n += 1;
                    }
                }

                writer.write_all(&[header]).map_err(EncodeError::IOError)?;
                writer.write_all(&buf[..n]).map_err(EncodeError::IOError)?;
                Ok(())
            }
            Self::Add { data } => {
                if data.len() > 127 {
                    return Err(EncodeError::TooLargeData(data.len()));
                }

                let header = data.len() as u8;
                writer.write_all(&[header]).map_err(EncodeError::IOError)?;
                writer.write_all(data).map_err(EncodeError::IOError)?;
                Ok(())
            }
        }
    }
}

/// Calculate delta instructions from `source` to `target`.
pub fn compute_delta<'a, 'b>(source: &'a [u8], target: &'b [u8]) -> Vec<Instruction<'a>>
where
    'b: 'a,
{
    // TODO: more efficient
    // TODO: more configurable
    let mut common_prefix_len: usize = 0;
    for (s, t) in source.iter().zip(target) {
        if s == t {
            common_prefix_len += 1;
        } else {
            break;
        }
    }

    let mut insts = Vec::new();
    if common_prefix_len > 0 {
        insts.push(Instruction::Copy {
            offset: 0,
            size: common_prefix_len,
        });
    }
    for chunk in target[common_prefix_len..].chunks(127) {
        insts.push(Instruction::Add { data: chunk });
    }
    insts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply_delta<'a>(source: &'a [u8], delta: &Vec<Instruction<'a>>) -> Vec<u8> {
        let mut buf = Vec::new();
        for inst in delta {
            match inst {
                Instruction::Add { data } => buf.extend_from_slice(data),
                Instruction::Copy { offset, size } => buf.extend_from_slice(&source[*offset..*offset + *size]),
            }
        }
        buf
    }

    #[test]
    fn make_it_right() {
        let source = "hello, world".as_bytes();
        let target = "hello, gitoxide".as_bytes();
        let delta = compute_delta(source, target);
        let restored = apply_delta(source, &delta);
        assert_eq!(target, restored);

        let mut delta_data = Vec::new();
        for inst in delta {
            inst.encode(&mut delta_data).unwrap();
        }

        let mut restored_target = vec![0u8; target.len()];
        apply(source, &mut restored_target, &delta_data).unwrap();
        assert_eq!(target, restored_target);
    }
}
