use std::convert::TryFrom;

use crate::{tree, Blob, BlobRef, Commit, CommitRef, Object, ObjectRef, Tag, TagRef, Tree, TreeRef};

impl TryFrom<TagRef<'_>> for Tag {
    type Error = crate::decode::Error;

    fn try_from(other: TagRef<'_>) -> Result<Tag, Self::Error> {
        let TagRef {
            target,
            name,
            target_kind,
            message,
            tagger,
            pgp_signature,
        } = other;
        let tagger = tagger
            .map(|raw| {
                gix_actor::SignatureRef::from_bytes::<crate::decode::ParseError>(raw.as_ref())
                    .map_err(|err| crate::decode::Error::with_err(err, raw.as_ref()))
            })
            .transpose()?
            .map(Into::into);
        Ok(Tag {
            target: gix_hash::ObjectId::from_hex(target).expect("prior parser validation"),
            name: name.to_owned(),
            target_kind,
            message: message.to_owned(),
            tagger,
            pgp_signature: pgp_signature.map(ToOwned::to_owned),
        })
    }
}

impl TryFrom<CommitRef<'_>> for Commit {
    type Error = crate::decode::Error;

    fn try_from(other: CommitRef<'_>) -> Result<Commit, Self::Error> {
        let CommitRef {
            tree,
            parents,
            author: author_raw,
            committer: committer_raw,
            encoding,
            message,
            extra_headers,
        } = other;
        let author = gix_actor::SignatureRef::from_bytes::<crate::decode::ParseError>(author_raw.as_ref())
            .map_err(|err| crate::decode::Error::with_err(err, author_raw.as_ref()))?;
        let committer = gix_actor::SignatureRef::from_bytes::<crate::decode::ParseError>(committer_raw.as_ref())
            .map_err(|err| crate::decode::Error::with_err(err, committer_raw.as_ref()))?;
        Ok(Commit {
            tree: gix_hash::ObjectId::from_hex(tree).expect("prior parser validation"),
            parents: parents
                .iter()
                .map(|parent| gix_hash::ObjectId::from_hex(parent).expect("prior parser validation"))
                .collect(),
            author: author.into(),
            committer: committer.into(),
            encoding: encoding.map(ToOwned::to_owned),
            message: message.to_owned(),
            extra_headers: extra_headers
                .into_iter()
                .map(|(k, v)| (k.into(), v.into_owned()))
                .collect(),
        })
    }
}

impl<'a> From<BlobRef<'a>> for Blob {
    fn from(v: BlobRef<'a>) -> Self {
        Blob {
            data: v.data.to_owned(),
        }
    }
}

impl From<TreeRef<'_>> for Tree {
    fn from(other: TreeRef<'_>) -> Tree {
        let TreeRef { entries } = other;
        Tree {
            entries: entries.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<tree::EntryRef<'_>> for tree::Entry {
    fn from(other: tree::EntryRef<'_>) -> tree::Entry {
        let tree::EntryRef { mode, filename, oid } = other;
        tree::Entry {
            mode,
            filename: filename.to_owned(),
            oid: oid.into(),
        }
    }
}

impl<'a> From<&'a tree::Entry> for tree::EntryRef<'a> {
    fn from(other: &'a tree::Entry) -> tree::EntryRef<'a> {
        let tree::Entry { mode, filename, oid } = other;
        tree::EntryRef {
            mode: *mode,
            filename: filename.as_ref(),
            oid,
        }
    }
}

impl TryFrom<ObjectRef<'_>> for Object {
    type Error = crate::decode::Error;

    fn try_from(v: ObjectRef<'_>) -> Result<Self, Self::Error> {
        Ok(match v {
            ObjectRef::Tree(v) => Object::Tree(v.into()),
            ObjectRef::Blob(v) => Object::Blob(v.into()),
            ObjectRef::Commit(v) => Object::Commit(Commit::try_from(v)?),
            ObjectRef::Tag(v) => Object::Tag(Tag::try_from(v)?),
        })
    }
}

impl From<Tag> for Object {
    fn from(v: Tag) -> Self {
        Object::Tag(v)
    }
}

impl From<Commit> for Object {
    fn from(v: Commit) -> Self {
        Object::Commit(v)
    }
}

impl From<Tree> for Object {
    fn from(v: Tree) -> Self {
        Object::Tree(v)
    }
}

impl From<Blob> for Object {
    fn from(v: Blob) -> Self {
        Object::Blob(v)
    }
}

impl TryFrom<Object> for Tag {
    type Error = Object;

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        Ok(match value {
            Object::Tag(v) => v,
            _ => return Err(value),
        })
    }
}

impl TryFrom<Object> for Commit {
    type Error = Object;

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        Ok(match value {
            Object::Commit(v) => v,
            _ => return Err(value),
        })
    }
}

impl TryFrom<Object> for Tree {
    type Error = Object;

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        Ok(match value {
            Object::Tree(v) => v,
            _ => return Err(value),
        })
    }
}

impl TryFrom<Object> for Blob {
    type Error = Object;

    fn try_from(value: Object) -> Result<Self, Self::Error> {
        Ok(match value {
            Object::Blob(v) => v,
            _ => return Err(value),
        })
    }
}

impl<'a> From<TagRef<'a>> for ObjectRef<'a> {
    fn from(v: TagRef<'a>) -> Self {
        ObjectRef::Tag(v)
    }
}

impl<'a> From<CommitRef<'a>> for ObjectRef<'a> {
    fn from(v: CommitRef<'a>) -> Self {
        ObjectRef::Commit(v)
    }
}

impl<'a> From<TreeRef<'a>> for ObjectRef<'a> {
    fn from(v: TreeRef<'a>) -> Self {
        ObjectRef::Tree(v)
    }
}

impl<'a> From<BlobRef<'a>> for ObjectRef<'a> {
    fn from(v: BlobRef<'a>) -> Self {
        ObjectRef::Blob(v)
    }
}

impl<'a> TryFrom<ObjectRef<'a>> for TagRef<'a> {
    type Error = ObjectRef<'a>;

    fn try_from(value: ObjectRef<'a>) -> Result<Self, Self::Error> {
        Ok(match value {
            ObjectRef::Tag(v) => v,
            _ => return Err(value),
        })
    }
}

impl<'a> TryFrom<ObjectRef<'a>> for CommitRef<'a> {
    type Error = ObjectRef<'a>;

    fn try_from(value: ObjectRef<'a>) -> Result<Self, Self::Error> {
        Ok(match value {
            ObjectRef::Commit(v) => v,
            _ => return Err(value),
        })
    }
}

impl<'a> TryFrom<ObjectRef<'a>> for TreeRef<'a> {
    type Error = ObjectRef<'a>;

    fn try_from(value: ObjectRef<'a>) -> Result<Self, Self::Error> {
        Ok(match value {
            ObjectRef::Tree(v) => v,
            _ => return Err(value),
        })
    }
}

impl<'a> TryFrom<ObjectRef<'a>> for BlobRef<'a> {
    type Error = ObjectRef<'a>;

    fn try_from(value: ObjectRef<'a>) -> Result<Self, Self::Error> {
        Ok(match value {
            ObjectRef::Blob(v) => v,
            _ => return Err(value),
        })
    }
}
