//! Read graph edges without validating unrelated object metadata.

use bstr::ByteSlice;
use gix_error::{Result, ResultExt, bail, message};
use gix_hash::ObjectId;

use crate::{Data, Kind};

/// One object required by an object's graph structure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The referenced object's identifier.
    pub id: ObjectId,
    /// The kind the referenced object must have.
    pub kind: Kind,
}

/// Graph edges of one raw object, in storage order, excluding submodule gitlinks.
///
/// Commit trees precede parents. Identity, tag name, message, and other metadata
/// are not decoded. Missing or malformed edge fields stop the iterator with an
/// error, but validation of the entire object is the caller's responsibility.
pub struct Iter<'a> {
    data: &'a [u8],
    kind: Kind,
    hash: gix_hash::Kind,
    commit_tree: bool,
    done: bool,
}

impl<'a> Data<'a> {
    /// Iterate over graph edges without validating identities or other metadata.
    ///
    /// This is useful for connectivity checks whose structural object validation
    /// is performed separately, possibly under a configurable compatibility policy.
    /// Gitlinks name commits in other repositories and are excluded. No referenced
    /// object is loaded; callers must check its existence and expected kind.
    pub fn references(&self) -> Iter<'a> {
        Iter {
            data: self.data,
            kind: self.kind,
            hash: self.object_hash,
            commit_tree: false,
            done: false,
        }
    }
}

impl Iter<'_> {
    fn next_entry(&mut self) -> Result<Option<Entry>> {
        Ok(match self.kind {
            Kind::Blob => None,
            Kind::Commit => {
                let (field, kind) = if !self.commit_tree {
                    self.commit_tree = true;
                    (b"tree".as_slice(), Kind::Tree)
                } else if self.data.starts_with(b"parent ") {
                    (b"parent".as_slice(), Kind::Commit)
                } else {
                    return Ok(None);
                };
                let hex = crate::parse::header_field(&mut self.data, field, |value| {
                    crate::parse::hex_hash(value, self.hash)
                })?;
                Some(Entry {
                    id: ObjectId::from_hex(hex).or_error()?,
                    kind,
                })
            }
            Kind::Tag => {
                let hex = crate::tag::decode::target(&mut self.data, self.hash)?;
                let kind = crate::tag::decode::kind(&mut self.data)?;
                self.done = true;
                Some(Entry {
                    id: ObjectId::from_hex(hex).or_error()?,
                    kind,
                })
            }
            Kind::Tree => loop {
                if self.data.is_empty() {
                    return Ok(None);
                }
                let Some((mode, object_id, rest)) = tree_edge(self.data, self.hash.len_in_bytes()) else {
                    bail!(message("A tree graph edge cannot be decoded").corrupted());
                };
                self.data = rest;
                let kind = match mode & 0o170000 {
                    0o160000 => continue,
                    0o40000 => Kind::Tree,
                    _ => Kind::Blob,
                };
                break Some(Entry { id: object_id, kind });
            },
        })
    }
}

impl Iterator for Iter<'_> {
    type Item = Result<Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        match self.next_entry() {
            Ok(Some(entry)) => Some(Ok(entry)),
            Ok(None) => {
                self.done = true;
                None
            }
            Err(err) => {
                self.done = true;
                Some(Err(err))
            }
        }
    }
}

impl std::iter::FusedIterator for Iter<'_> {}

// Preserve raw mode bits: nonstandard modes still refer to blobs unless they
// denote a directory or gitlink, just as in Git's connectivity walk. Their
// spelling and validity are left to the caller's structural validation policy.
fn tree_edge(data: &[u8], hash_len: usize) -> Option<(u32, ObjectId, &[u8])> {
    let space = data.find_byte(b' ')?;
    let mode = &data[..space];
    if mode.is_empty() || mode.iter().any(|byte| !(b'0'..=b'7').contains(byte)) {
        return None;
    }
    let mode = mode.iter().try_fold(0u32, |value, byte| {
        value.checked_mul(8)?.checked_add(u32::from(byte - b'0'))
    })?;
    let data = &data[space + 1..];
    let nul = data.find_byte(0)?;
    if nul == 0 {
        return None;
    }
    let data = &data[nul + 1..];
    let object_id = ObjectId::try_from(data.get(..hash_len)?).ok()?;
    Some((mode, object_id, &data[hash_len..]))
}
