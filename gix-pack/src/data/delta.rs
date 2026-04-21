///
pub mod apply {
    /// Returned when failing to apply deltas.
    #[derive(thiserror::Error, Debug)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Encountered unsupported command code: 0")]
        UnsupportedCommandCode,
        #[error("Delta copy from base: byte slices must match")]
        DeltaCopyBaseSliceMismatch,
        #[error("Delta copy data: byte slices must match")]
        DeltaCopyDataSliceMismatch,
    }
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

fn encode_size(mut n: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(8);
    loop {
        let mut byte = (n & 0x7F) as u8;
        n >>= 7;
        if n != 0 {
            byte |= 0x80;
            buf.push(byte);
        } else {
            buf.push(byte);
            break;
        }
    }
    buf
}

pub(crate) fn apply(base: &[u8], mut target: &mut [u8], data: &[u8]) -> Result<(), apply::Error> {
    let mut i = 0;
    while let Some(cmd) = data.get(i) {
        i += 1;
        match cmd {
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
                std::io::Write::write(&mut target, &base[ofs..ofs + size as usize])
                    .map_err(|_e| apply::Error::DeltaCopyBaseSliceMismatch)?;
            }
            0 => return Err(apply::Error::UnsupportedCommandCode),
            size => {
                std::io::Write::write(&mut target, &data[i..i + *size as usize])
                    .map_err(|_e| apply::Error::DeltaCopyDataSliceMismatch)?;
                i += *size as usize;
            }
        }
    }
    assert_eq!(i, data.len());
    assert_eq!(target.len(), 0);

    Ok(())
}

enum Instruction {
    Copy { offset: usize, size: usize },
    Add { data: Vec<u8> },
}

impl Instruction {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Copy { offset, size } => todo!(),
            Self::Add { data } => todo!(),
        }
        todo!()
    }
}

fn compute_delta(source: &[u8], target: &[u8]) -> Vec<Instruction> {
    let mut common_prefix_len = 0;
    for (s, t) in source.iter().zip(target) {
        if s == t {
            common_prefix_len += 1;
        } else {
            break;
        }
    }
    vec![
        Instruction::Copy {
            offset: 0,
            size: common_prefix_len,
        },
        Instruction::Add {
            data: target[common_prefix_len..].into(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_size_works() {
        let cases: Vec<u64> = vec![0x00, 0x01, 0x7f, 0xff, 0x7777, 1795265022, 3_825_123_056_546_413_051];
        for n in cases {
            let encoded = encode_size(n);
            let (restored_n, _) = decode_header_size(&encoded);
            assert_eq!(n, restored_n);
        }
    }

    fn apply_delta(source: &[u8], delta: Vec<Instruction>) -> Vec<u8> {
        let mut buf = Vec::new();
        for inst in delta {
            match inst {
                Instruction::Add { data } => buf.extend_from_slice(&data),
                Instruction::Copy { offset, size } => buf.extend_from_slice(&source[offset..offset + size]),
            }
        }
        buf
    }

    #[test]
    fn make_it_right() {
        let source = "hello, world".as_bytes();
        let target = "hello, gitoxide".as_bytes();
        let delta = compute_delta(source, target);
        let restored = apply_delta(source, delta);
        assert_eq!(target, restored);
    }
}
