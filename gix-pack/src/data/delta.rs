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

pub(crate) fn apply<W: std::io::Write + ?Sized>(base: &[u8], target: &mut W, data: &[u8]) -> Result<(), apply::Error> {
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
                target
                    .write_all(&base[ofs..ofs + size as usize])
                    .map_err(|_e| apply::Error::DeltaCopyBaseSliceMismatch)?;
            }
            0 => return Err(apply::Error::UnsupportedCommandCode),
            size => {
                target
                    .write_all(&data[i..i + *size as usize])
                    .map_err(|_e| apply::Error::DeltaCopyDataSliceMismatch)?;
                i += *size as usize;
            }
        }
    }
    debug_assert_eq!(i, data.len());

    Ok(())
}
