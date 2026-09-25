use gix_error::Result;
use gix_error::{ErrorExt, ExnMessageResult, Message, corruption};

use gix_object::bstr::{BString, ByteSlice};

use crate::store_impl::{packed, packed::decode};

/// packed-refs specific functionality
impl packed::Buffer {
    /// Return an iterator of references stored in this packed refs buffer, ordered by reference name.
    /// Header failures include [metadata](gix_error::Error::metadata()) `input` (bytes), the first line without its
    /// newline.
    ///
    /// # Note
    ///
    /// There is no namespace support in packed iterators. It can be emulated using `iter_prefixed(…)`.
    pub fn iter(&self) -> Result<packed::Iter<'_>> {
        packed::Iter::new(self.as_ref(), self.object_hash)
    }

    /// Return an iterator yielding only references matching the given prefix, ordered by reference name.
    /// Header failures include [metadata](gix_error::Error::metadata()) `input` (bytes), the first selected line without
    /// its newline.
    pub fn iter_prefixed(&self, prefix: BString) -> Result<packed::Iter<'_>> {
        let first_record_with_prefix = self.binary_search_by(prefix.as_bstr()).unwrap_or_else(|(_, pos)| pos);
        Ok(packed::Iter::new_with_prefix(
            &self.as_ref()[first_record_with_prefix..],
            self.object_hash,
            Some(prefix),
        )?)
    }
}

impl<'a> Iterator for packed::Iter<'a> {
    type Item = Result<packed::Reference<'a>>;

    /// Decode failures include [metadata](gix_error::Error::metadata()) `line` (one-based line number within this
    /// iterator's input) and `input` (line bytes).
    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor.is_empty() {
            return None;
        }

        let start = self.cursor;
        match decode::reference(&mut self.cursor, self.object_hash) {
            Ok(reference) => {
                self.current_line += 1 + usize::from(reference.object.is_some());
                if let Some(ref prefix) = self.prefix
                    && !reference.name.as_bstr().starts_with_str(prefix)
                {
                    self.cursor = &[];
                    return None;
                }
                Some(Ok(reference))
            }
            Err(err) => {
                self.cursor = start;
                let (failed_line, next_cursor) = self
                    .cursor
                    .find_byte(b'\n')
                    .map_or((self.cursor, &[][..]), |pos| self.cursor.split_at(pos + 1));
                self.cursor = next_cursor;
                let line_number = self.current_line;
                self.current_line += 1;

                Some(Err(err
                    .raise(
                        Message::new("Invalid packed reference")
                            .with("input", failed_line.strip_suffix(b"\n").unwrap_or(failed_line))
                            .with("line", line_number),
                    )
                    .into()))
            }
        }
    }
}

impl<'a> packed::Iter<'a> {
    /// Return a new iterator after successfully parsing the possibly existing first line of the given `packed` refs buffer,
    /// parsing object ids as `object_hash`.
    /// Header failures include [metadata](gix_error::Error::metadata()) `input` (bytes), the first line without its
    /// newline.
    pub fn new(packed: &'a [u8], object_hash: gix_hash::Kind) -> Result<Self> {
        Ok(Self::new_with_prefix(packed, object_hash, None)?)
    }

    /// Returns an iterator whose references will only match `prefix`.
    ///
    /// It assumes that the underlying `packed` buffer is indeed sorted and parses object ids as `object_hash`.
    /// Header failures include [metadata](gix_error::Error::metadata()) `input` (bytes), the first line without its
    /// newline.
    pub(in crate::store_impl::packed) fn new_with_prefix(
        packed: &'a [u8],
        object_hash: gix_hash::Kind,
        prefix: Option<BString>,
    ) -> ExnMessageResult<Self> {
        if packed.is_empty() {
            Ok(packed::Iter {
                cursor: packed,
                object_hash,
                prefix,
                current_line: 1,
            })
        } else if packed[0] == b'#' {
            let mut input = packed;
            decode::header(&mut input).map_err(|()| {
                corruption("Invalid packed reference header")
                    .with("input", packed.lines().next().unwrap_or(packed))
                    .raise()
            })?;
            let refs = input;
            Ok(packed::Iter {
                cursor: refs,
                object_hash,
                prefix,
                current_line: 2,
            })
        } else {
            Ok(packed::Iter {
                cursor: packed,
                object_hash,
                prefix,
                current_line: 1,
            })
        }
    }
}
