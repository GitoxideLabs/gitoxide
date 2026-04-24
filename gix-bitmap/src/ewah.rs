///
pub mod decode {
    /// The error returned by [`decode()`](super::decode()).
    pub type Error = gix_error::Exn<gix_error::ValidationError>;
}

/// Decode `data` as EWAH bitmap.
pub fn decode(data: &[u8]) -> Result<(Vec, &[u8]), decode::Error> {
    use crate::decode;
    use gix_error::{message, OptionExt};

    let (num_bits, data) = decode::u32(data).ok_or_raise(|| message("eof reading amount of bits").into())?;
    let (len, data) = decode::u32(data).ok_or_raise(|| message("eof reading chunk length").into())?;
    let len = len as usize;

    // NOTE: git does this by copying all bytes first, and then it will change the endianness in a separate loop.
    //       Maybe it's faster, but we can't do it without unsafe. Let's leave it to the optimizer and maybe
    //       one day somebody will find out that it's worth it to use unsafe here.
    let (mut bits, data) = data
        .split_at_checked(len * std::mem::size_of::<u64>())
        .ok_or_raise(|| message("eof while reading bit data").into())?;
    let mut buf = std::vec::Vec::<u64>::with_capacity(len);
    for _ in 0..len {
        let (bit_num, rest) = bits.split_at(std::mem::size_of::<u64>());
        bits = rest;
        buf.push(u64::from_be_bytes(bit_num.try_into().unwrap()));
    }

    let (rlw, data) = decode::u32(data).ok_or_raise(|| message("eof while reading run length width").into())?;

    Ok((
        Vec {
            num_bits,
            bits: buf,
            rlw: rlw.into(),
        },
        data,
    ))
}

mod access {
    use super::Vec;

    impl Vec {
        /// Create a bitmap from a sequence of bit values.
        ///
        /// The resulting bitmap uses a literal-only EWAH representation.
        ///
        /// Returns `None` if `bits.len()` exceeds `u32::MAX`.
        pub fn from_bits(bits: &[bool]) -> Option<Self> {
            let literal_words: std::vec::Vec<u64> = bits
                .chunks(64)
                .map(|chunk| {
                    chunk.iter().enumerate().fold(
                        0u64,
                        |word, (idx, bit)| {
                            if *bit {
                                word | (1u64 << idx)
                            } else {
                                word
                            }
                        },
                    )
                })
                .collect();
            let num_bits = bits.len().try_into().ok()?;

            Some(Vec {
                num_bits,
                bits: std::iter::once((literal_words.len() as u64) << (1 + RLW_RUNNING_BITS))
                    .chain(literal_words)
                    .collect(),
                rlw: 0,
            })
        }

        /// Write the bitmap as EWAH bytes to `out`.
        ///
        /// These bytes can be parsed again with [`decode()`](super::decode()).
        pub fn write_to(&self, out: &mut impl std::io::Write) -> std::io::Result<()> {
            let len: u32 = self.bits.len().try_into().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "bit word count exceeds u32::MAX")
            })?;
            let rlw: u32 = self.rlw.try_into().map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "run length word offset exceeds u32::MAX",
                )
            })?;

            out.write_all(&self.num_bits.to_be_bytes())?;
            out.write_all(&len.to_be_bytes())?;
            for word in &self.bits {
                out.write_all(&word.to_be_bytes())?;
            }
            out.write_all(&rlw.to_be_bytes())
        }

        /// Call `f(index)` for each bit that is true, given the index of the bit that identifies it uniquely within the bit array.
        /// If `f` returns `None` the iteration will be stopped and `None` is returned.
        ///
        /// The index is sequential like in any other vector.
        pub fn for_each_set_bit(&self, mut f: impl FnMut(usize) -> Option<()>) -> Option<()> {
            let num_bits = self.num_bits();
            let mut index = 0usize;
            let mut iter = self.bits.iter();
            while let Some(word) = iter.next() {
                if rlw_runbit_is_set(word) {
                    let len = usize::try_from(rlw_running_len_bits(word)).ok()?;
                    let end = index.checked_add(len)?;
                    if end > num_bits {
                        return None;
                    }
                    for _ in 0..len {
                        f(index)?;
                        index += 1;
                    }
                } else {
                    let len = usize::try_from(rlw_running_len_bits(word)).ok()?;
                    let end = index.checked_add(len)?;
                    if end > num_bits {
                        return None;
                    }
                    index = end;
                }

                for _ in 0..rlw_literal_words(word) {
                    let word = iter.next()?;
                    let remaining = num_bits.checked_sub(index)?;
                    if remaining == 0 {
                        return None;
                    }
                    let bits_in_word = remaining.min(64);
                    if bits_in_word < 64 && (word >> bits_in_word) != 0 {
                        return None;
                    }
                    for bit_index in 0..bits_in_word {
                        if word & (1 << bit_index) != 0 {
                            f(index)?;
                        }
                        index += 1;
                    }
                }
            }
            Some(())
        }

        /// The amount of bits we are currently holding.
        pub fn num_bits(&self) -> usize {
            self.num_bits.try_into().expect("we are not on 16 bit systems")
        }
    }

    #[inline]
    fn rlw_running_len_bits(w: &u64) -> u64 {
        rlw_running_len(w) * 64
    }

    #[inline]
    fn rlw_running_len(w: &u64) -> u64 {
        (w >> 1) & RLW_LARGEST_RUNNING_COUNT
    }

    #[inline]
    fn rlw_literal_words(w: &u64) -> u64 {
        w >> (1 + RLW_RUNNING_BITS)
    }

    #[inline]
    fn rlw_runbit_is_set(w: &u64) -> bool {
        w & 1 == 1
    }

    const RLW_RUNNING_BITS: u64 = 4 * 8;
    const RLW_LARGEST_RUNNING_COUNT: u64 = (1 << RLW_RUNNING_BITS) - 1;
}

/// A growable collection of u64 that are seen as stream of individual bits.
#[allow(dead_code)]
#[derive(Clone)]
pub struct Vec {
    num_bits: u32,
    bits: std::vec::Vec<u64>,
    /// RLW is an offset into the `bits` buffer, so `1` translates into &bits\[1] essentially.
    rlw: u64,
}
