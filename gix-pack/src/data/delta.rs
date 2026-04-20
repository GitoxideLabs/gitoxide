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
        #[error("Delta output size mismatch: expected {expected} bytes, got {actual}")]
        OutputSizeMismatch { expected: usize, actual: usize },
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

pub(crate) fn apply<W: std::io::Write + ?Sized>(
    base: &[u8],
    target: &mut W,
    data: &[u8],
    expected_size: usize,
) -> Result<(), apply::Error> {
    let mut i = 0;
    let mut produced = 0usize;
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
                let size = size as usize;
                target
                    .write_all(&base[ofs..ofs + size])
                    .map_err(|_e| apply::Error::DeltaCopyBaseSliceMismatch)?;
                produced += size;
            }
            0 => return Err(apply::Error::UnsupportedCommandCode),
            size => {
                let size = *size as usize;
                target
                    .write_all(&data[i..i + size])
                    .map_err(|_e| apply::Error::DeltaCopyDataSliceMismatch)?;
                produced += size;
                i += size;
            }
        }
    }
    debug_assert_eq!(i, data.len());

    if produced != expected_size {
        return Err(apply::Error::OutputSizeMismatch {
            expected: expected_size,
            actual: produced,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::apply;

    #[test]
    fn rejects_outputs_shorter_than_declared() {
        let mut out = Vec::new();
        let err = super::apply(b"hello", &mut out, &[0x90, 0x05], 6).expect_err("malformed deltas must fail");
        assert!(matches!(
            err,
            apply::Error::OutputSizeMismatch { expected: 6, actual: 5 }
        ));
    }
}
