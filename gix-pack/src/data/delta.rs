///
pub mod apply {
    /// Returned when failing to apply deltas.
    #[derive(thiserror::Error, Debug)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Corrupt delta data: {message}")]
        Corrupt { message: &'static str },
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
pub(crate) fn decode_header_size(d: &[u8]) -> Result<(u64, usize), apply::Error> {
    let mut shift = 0;
    let mut size = 0u64;
    let mut consumed = 0;
    for cmd in d.iter() {
        if shift >= u64::BITS {
            return Err(apply::Error::Corrupt {
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
    Err(apply::Error::Corrupt {
        message: "delta header size is truncated",
    })
}

pub(crate) fn apply(base: &[u8], mut target: &mut [u8], data: &[u8]) -> Result<(), apply::Error> {
    fn next_byte(data: &[u8], i: &mut usize) -> Result<u8, apply::Error> {
        let byte = *data.get(*i).ok_or(apply::Error::Corrupt {
            message: "delta copy instruction is truncated",
        })?;
        *i += 1;
        Ok(byte)
    }

    let mut i = 0;
    while let Some(cmd) = data.get(i) {
        i += 1;
        match cmd {
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
                let end = ofs.checked_add(size as usize).ok_or(apply::Error::Corrupt {
                    message: "delta copy range overflows",
                })?;
                std::io::Write::write(
                    &mut target,
                    base.get(ofs..end).ok_or(apply::Error::Corrupt {
                        message: "delta copy range exceeds base object size",
                    })?,
                )
                .map_err(|_e| apply::Error::DeltaCopyBaseSliceMismatch)?;
            }
            0 => {
                return Err(apply::Error::Corrupt {
                    message: "delta command 0 is reserved and invalid",
                });
            }
            size => {
                let end = i.checked_add(*size as usize).ok_or(apply::Error::Corrupt {
                    message: "delta insert range overflows",
                })?;
                std::io::Write::write(
                    &mut target,
                    data.get(i..end).ok_or(apply::Error::Corrupt {
                        message: "delta insert data is truncated",
                    })?,
                )
                .map_err(|_e| apply::Error::DeltaCopyDataSliceMismatch)?;
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
        return Err(apply::Error::Corrupt {
            message: "delta instructions produced fewer bytes than promised",
        });
    }

    Ok(())
}
