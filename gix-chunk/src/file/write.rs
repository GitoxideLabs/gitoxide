use crate::file::{index::Entry, Index};

mod write_chunk {
    use std::collections::VecDeque;

    use crate::file::index;

    /// A [`Write`][std::io::Write] implementation that validates chunk sizes while allowing the user to know
    /// which chunk is to be written next.
    pub struct Chunk<W> {
        chunks_to_write: VecDeque<index::Entry>,
        inner: W,
        next_chunk: Option<index::Entry>,
        written_bytes: usize,
    }

    impl<W> Chunk<W>
    where
        W: std::io::Write,
    {
        pub(crate) fn new(out: W, chunks: VecDeque<index::Entry>) -> Chunk<W>
        where
            W: std::io::Write,
        {
            Chunk {
                chunks_to_write: chunks,
                inner: out,
                next_chunk: None,
                written_bytes: 0,
            }
        }
    }

    impl<W> std::io::Write for Chunk<W>
    where
        W: std::io::Write,
    {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let written = self.inner.write(buf)?;
            self.written_bytes += written;
            Ok(written)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.inner.flush()
        }
    }

    impl<W> Chunk<W> {
        /// Return the inner writer - should only be called once there is no more chunk to write.
        pub fn into_inner(self) -> W {
            self.inner
        }
        /// Return the next chunk-id to write, if there is one.
        pub fn next_chunk(&mut self) -> Option<crate::Id> {
            if let Some(entry) = self.next_chunk.take() {
                assert_eq!(
                    entry.offset.end,
                    self.written_bytes as u64,
                    "BUG: expected to write {} bytes, but only wrote {} for chunk {:?}",
                    entry.offset.end,
                    self.written_bytes,
                    (&entry.kind)
                );
            }
            self.written_bytes = 0;
            self.next_chunk = self.chunks_to_write.pop_front();
            self.next_chunk.as_ref().map(|e| e.kind)
        }
    }
}
pub use write_chunk::Chunk;

/// Writing
impl Index {
    /// Create a new index whose sole purpose is to be receiving chunks using [`plan_chunk()`][Index::plan_chunk()] and to be written to
    /// an output using [`into_write()`][Index::into_write()]
    pub fn for_writing() -> Self {
        Index {
            will_write: true,
            chunks: Vec::new(),
        }
    }
    /// Plan to write a new chunk as part of the index when [`into_write()`][Index::into_write()] is called.
    pub fn plan_chunk(&mut self, chunk: crate::Id, exact_size_on_disk: u64) {
        assert!(self.will_write, "BUG: create the index with `for_writing()`");
        assert!(
            !self.chunks.iter().any(|e| e.kind == chunk),
            "BUG: must not add chunk of same kind twice: {:?}",
            (&chunk)
        );
        self.chunks.push(Entry {
            kind: chunk,
            offset: 0..exact_size_on_disk,
        });
    }

    /// Return the total size of all planned chunks thus far.
    pub fn planned_storage_size(&self) -> u64 {
        assert!(self.will_write, "BUG: create the index with `for_writing()`");
        self.chunks.iter().map(|e| e.offset.end).sum()
    }

    /// Return the amount of chunks we currently know.
    pub fn num_chunks(&self) -> usize {
        self.chunks.len()
    }

    /// After [planning all chunks][Index::plan_chunk()] call this method with the destination to write the chunks to.
    /// Use the [Chunk] writer to write each chunk in order.
    /// `current_offset` is the byte position at which `out` will continue writing.
    pub fn into_write<W>(self, mut out: W, current_offset: usize) -> std::io::Result<Chunk<W>>
    where
        W: std::io::Write,
    {
        assert!(
            self.will_write,
            "BUG: create the index with `for_writing()`, cannot write decoded indices"
        );
        // First chunk starts past the table of contents
        let mut current_offset = (current_offset + Self::size_for_entries(self.num_chunks())) as u64;

        for entry in &self.chunks {
            out.write_all(&entry.kind)?;
            out.write_all(&current_offset.to_be_bytes())?;

            current_offset += entry.offset.end;
        }

        // sentinel to mark end of chunks
        out.write_all(&0u32.to_be_bytes())?;
        out.write_all(&current_offset.to_be_bytes())?;

        Ok(Chunk::new(out, self.chunks.into()))
    }
}
