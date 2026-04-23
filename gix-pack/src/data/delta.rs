use std::io::Write;

/// Returned when failing to apply deltas.
#[derive(thiserror::Error, Debug)]
#[allow(missing_docs)]
pub enum ApplyError {
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
    #[error("Failed to write bytes")]
    IOError,
    #[error("Too large size in Copy instruction, should <= 0x00ffffff")]
    TooLargeSize,
    #[error("Too large data in Add instruction, length should <= 127")]
    TooLargeData,
}

/// Given the decompressed pack delta `d`, decode a size in bytes (either the base object size or the result object size)
/// Equivalent to [this canonical git function](https://github.com/git/git/blob/311531c9de557d25ac087c1637818bd2aad6eb3a/delta.h#L89)
pub(crate) fn decode_header_size(d: &[u8]) -> (u64, usize) {
    let mut i = 0;
    let mut size = 0u64;
    let mut consumed = 0;
    for cmd in d.iter() {
        consumed += 1;
        size |= (u64::from(*cmd) & 0x7f) << i;
        i += 7;
        if *cmd & 0x80 == 0 {
            break;
        }
    }
    (size, consumed)
}

pub(crate) fn apply(base: &[u8], mut target: &mut [u8], data: &[u8]) -> Result<(), ApplyError> {
    let mut i = 0;
    while let Some(cmd) = data.get(i) {
        eprintln!("index: {i}, cmd: {cmd}");
        i += 1;
        match cmd {
            // Copy
            cmd if cmd & 0b1000_0000 != 0 => {
                let (mut ofs, mut size): (u32, u32) = (0, 0);
                if cmd & 0b0000_0001 != 0 {
                    ofs = u32::from(data[i]);
                    i += 1;
                }
                if cmd & 0b0000_0010 != 0 {
                    ofs |= u32::from(data[i]) << 8;
                    i += 1;
                }
                if cmd & 0b0000_0100 != 0 {
                    ofs |= u32::from(data[i]) << 16;
                    i += 1;
                }
                if cmd & 0b0000_1000 != 0 {
                    ofs |= u32::from(data[i]) << 24;
                    i += 1;
                }
                if cmd & 0b0001_0000 != 0 {
                    size = u32::from(data[i]);
                    i += 1;
                }
                if cmd & 0b0010_0000 != 0 {
                    size |= u32::from(data[i]) << 8;
                    i += 1;
                }
                if cmd & 0b0100_0000 != 0 {
                    size |= u32::from(data[i]) << 16;
                    i += 1;
                }
                if size == 0 {
                    size = 0x10000; // 65536
                }
                let ofs = ofs as usize;
                Write::write(&mut target, &base[ofs..ofs + size as usize])
                    .map_err(|_e| ApplyError::DeltaCopyBaseSliceMismatch)?;
            }
            // Reserved
            0 => return Err(ApplyError::UnsupportedCommandCode),
            // Add
            size => {
                Write::write(&mut target, &data[i..i + *size as usize])
                    .map_err(|_e| ApplyError::DeltaCopyDataSliceMismatch)?;
                i += *size as usize;
            }
        }
    }
    assert_eq!(i, data.len());
    assert_eq!(target.len(), 0);

    Ok(())
}

/// Delta instruction
#[derive(Debug)]
pub enum Instruction<'a> {
    /// Copy data from source
    Copy {
        /// Start position to copy
        offset: u32,
        /// Data length in bytes
        size: u32,
    },
    /// Insert bytes embedded in instruction
    Add {
        /// Data to add
        data: &'a [u8], // TODO: use borrow here
    },
}

impl<'a> Instruction<'a> {
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
                    return Err(EncodeError::TooLargeSize);
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

                writer.write_all(&[header]).map_err(|_| EncodeError::IOError)?;
                writer.write_all(&buf[..n]).map_err(|_| EncodeError::IOError)?;
                Ok(())
            }
            Self::Add { data } => {
                if data.len() > 127 {
                    return Err(EncodeError::TooLargeData);
                }

                let header = data.len() as u8;
                writer.write(&[header]).map_err(|_| EncodeError::IOError)?;
                writer.write(data).map_err(|_| EncodeError::IOError)?;
                Ok(())
            }
        }
    }
}

/// Calcuate delta instructions from `source` to `target`.
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
    insts.push(Instruction::Copy {
        offset: 0,
        size: common_prefix_len as u32,
    });
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
                Instruction::Add { data } => buf.extend_from_slice(&data),
                Instruction::Copy { offset, size } => {
                    buf.extend_from_slice(&source[(*offset as usize)..(*offset as usize + *size as usize)])
                }
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
