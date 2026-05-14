use std::ops::ControlFlow;

use bstr::BStr;

use crate::{TreeRef, TreeRefIter, tree, tree::EntryRef};

/// Advance a path lookup by matching the next path component against `tree`.
///
/// `components` must yield the remaining path components to resolve, and `tree` must be the
/// current object to search in.
///
/// The return value indicates how the caller should proceed:
///
/// - [`ControlFlow::Continue`] contains the object id of the matched entry when there are more
///   components left to resolve. Callers should load that object and pass it back into a subsequent
///   invocation.
/// - [`ControlFlow::Break`]`(Some(entry))` contains the matched entry for the final component, and
///   signals that lookup completed successfully.
/// - [`ControlFlow::Break`]`(None)` signals that lookup cannot continue and should stop without a
///   match. This happens if `tree` is not a tree object, if `components` is already exhausted, or if
///   the next component is not present in `tree`.
///
/// Note that this behaviour is tuned to prefer to exhaust the entire chain of `components`, only the
/// last component can yield a [`ControlFlow::Break`].
pub fn next_entry<'a, I, P>(
    components: &mut core::iter::Peekable<I>,
    tree: crate::Data<'a>,
) -> core::ops::ControlFlow<Option<EntryRef<'a>>, gix_hash::ObjectId>
where
    I: Iterator<Item = P>,
    P: PartialEq<BStr>,
{
    if !tree.kind.is_tree() {
        return ControlFlow::Break(None);
    }

    let Some(component) = components.next() else {
        return ControlFlow::Break(None);
    };

    let Some(entry) = TreeRefIter::from_bytes(tree.data, tree.object_hash)
        .filter_map(Result::ok)
        .find(|entry| component.eq(entry.filename))
    else {
        return ControlFlow::Break(None);
    };

    if components.peek().is_none() {
        ControlFlow::Break(Some(entry))
    } else {
        ControlFlow::Continue(entry.oid.to_owned())
    }
}

impl<'a> TreeRefIter<'a> {
    /// Instantiate an iterator from the given tree `data` and `object_hash`.
    pub fn from_bytes(data: &'a [u8], hash_kind: gix_hash::Kind) -> TreeRefIter<'a> {
        TreeRefIter { data, hash_kind }
    }

    /// Follow a sequence of `path` components starting from this instance, and look them up in `odb` one by one using `buffer`
    /// until the last component is looked up and its tree entry is returned.
    ///
    /// # Performance Notes
    ///
    /// Searching tree entries is currently done in sequence, which allows the search to be allocation free. It would be possible
    /// to reuse a vector and use a binary search instead, which might be able to improve performance over all.
    /// However, a benchmark should be created first to have some data and see which trade-off to choose here.
    pub fn lookup_entry<I, P>(
        &self,
        odb: impl crate::Find,
        buffer: &'a mut Vec<u8>,
        path: I,
    ) -> Result<Option<tree::Entry>, crate::find::Error>
    where
        I: IntoIterator<Item = P>,
        P: PartialEq<BStr>,
    {
        buffer.clear();
        buffer.extend_from_slice(self.data);

        let mut iter = path.into_iter().peekable();
        let mut data = crate::Data::new(buffer, crate::Kind::Tree, self.hash_kind);

        loop {
            data = match next_entry(&mut iter, data) {
                ControlFlow::Continue(oid) => {
                    let Some(next_tree) = odb.try_find(&oid, buffer)? else {
                        break Ok(None);
                    };
                    next_tree
                }
                ControlFlow::Break(v) => break Ok(v.map(Into::into)),
            }
        }
    }

    /// Like [`Self::lookup_entry()`], but takes any [`AsRef<Path>`](`std::path::Path`) directly via `relative_path`,
    /// a path relative to this tree.
    /// `odb` and `buffer` are used to lookup intermediate trees.
    ///
    /// # Note
    ///
    /// If any path component contains illformed UTF-8 and thus can't be converted to bytes on platforms which can't do so natively,
    /// the returned component will be empty which makes the lookup fail.
    pub fn lookup_entry_by_path(
        &self,
        odb: impl crate::Find,
        buffer: &'a mut Vec<u8>,
        relative_path: impl AsRef<std::path::Path>,
    ) -> Result<Option<tree::Entry>, crate::find::Error> {
        self.lookup_entry(
            odb,
            buffer,
            relative_path
                .as_ref()
                .components()
                .map(|c| c.as_os_str().as_encoded_bytes()),
        )
    }
}

impl<'a> TreeRef<'a> {
    /// Deserialize a Tree from `data`, assuming `object_hash` to determine how the object ids are encoded in this particular tree.
    pub fn from_bytes(data: &'a [u8], hash_kind: gix_hash::Kind) -> Result<TreeRef<'a>, crate::decode::Error> {
        decode::tree(data, hash_kind.len_in_bytes())
    }

    /// Find an entry named `name` knowing if the entry is a directory or not, using a binary search.
    ///
    /// Note that it's impossible to binary search by name alone as the sort order is special.
    pub fn bisect_entry(&self, name: &BStr, is_dir: bool) -> Option<EntryRef<'a>> {
        static NULL_HASH: gix_hash::ObjectId = gix_hash::Kind::shortest().null();

        let search = EntryRef {
            mode: if is_dir {
                tree::EntryKind::Tree
            } else {
                tree::EntryKind::Blob
            }
            .into(),
            filename: name,
            oid: &NULL_HASH,
        };
        self.entries
            .binary_search_by(|e| e.cmp(&search))
            .ok()
            .map(|idx| self.entries[idx])
    }

    /// Create an instance of the empty tree.
    ///
    /// It's particularly useful as static part of a program.
    pub const fn empty() -> TreeRef<'static> {
        TreeRef { entries: Vec::new() }
    }
}

impl<'a> TreeRefIter<'a> {
    /// Consume self and return all parsed entries.
    pub fn entries(self) -> Result<Vec<EntryRef<'a>>, crate::decode::Error> {
        self.collect()
    }

    /// Return the offset in bytes that our data advanced from `buf`, the original buffer
    /// to the beginning of the data of the tree.
    ///
    /// Then the tree-iteration can be resumed at the entry that would otherwise be returned next.
    pub fn offset_to_next_entry(&self, buf: &[u8]) -> usize {
        let before = (*buf).as_ptr();
        let after = (*self.data).as_ptr();

        debug_assert!(
            before <= after,
            "`TreeRefIter::offset_to_next_entry(): {after:?} <= {before:?}) violated"
        );
        (after as usize - before as usize) / std::mem::size_of::<u8>()
    }
}

impl<'a> Iterator for TreeRefIter<'a> {
    type Item = Result<EntryRef<'a>, crate::decode::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.is_empty() {
            return None;
        }
        match decode::fast_entry(self.data, self.hash_kind.len_in_bytes()) {
            Some((data_left, entry)) => {
                self.data = data_left;
                Some(Ok(entry))
            }
            None => {
                self.data = &[];
                Some(Err(crate::decode::Error))
            }
        }
    }
}

impl<'a> TryFrom<&'a [u8]> for tree::EntryMode {
    type Error = &'a [u8];

    fn try_from(mode: &'a [u8]) -> Result<Self, Self::Error> {
        tree::EntryMode::from_bytes(mode).ok_or(mode)
    }
}

mod decode {
    use bstr::ByteSlice;

    use crate::{TreeRef, tree, tree::EntryRef};

    pub fn fast_entry(i: &[u8], hash_len: usize) -> Option<(&[u8], EntryRef<'_>)> {
        let (mode, i) = tree::EntryMode::extract_from_bytes(i)?;
        let (filename, i) = i.split_at(i.find_byte(0)?);
        let i = &i[1..];
        let (oid, i) = match i.len() {
            len if len < hash_len => return None,
            _ => i.split_at(hash_len),
        };
        Some((
            i,
            EntryRef {
                mode,
                filename: filename.as_bstr(),
                oid: gix_hash::oid::try_from_bytes(oid)
                    .unwrap_or_else(|_| panic!("we counted exactly {hash_len} bytes")),
            },
        ))
    }

    pub fn tree(data: &[u8], hash_len: usize) -> Result<TreeRef<'_>, crate::decode::Error> {
        let mut i = data;

        // Calculate an estimate of the amount of entries to reduce
        // the amount of allocations necessary.
        // Note that this assumes that we want speed over fitting Vecs, this is a trade-off.
        const AVERAGE_FILENAME_LEN: usize = 24;
        const AVERAGE_MODE_LEN: usize = 6;
        const ENTRY_DELIMITER_LEN: usize = 2; // space + trailing zero
        const AVERAGE_TREE_ENTRIES: usize = 16 * 2; // prevent overallocation beyond what's meaningful or what could be dangerous
        let average_entry_len = ENTRY_DELIMITER_LEN + hash_len + AVERAGE_MODE_LEN + AVERAGE_FILENAME_LEN;
        let upper_bound = i.len() / average_entry_len;
        let mut out = Vec::with_capacity(upper_bound.min(AVERAGE_TREE_ENTRIES));

        while !i.is_empty() {
            let Some((rest, entry)) = fast_entry(i, hash_len) else {
                return Err(crate::decode::Error);
            };
            i = rest;
            out.push(entry);
        }
        Ok(TreeRef { entries: out })
    }
}
