/// Either an object id that the remote has or the matched remote ref itself.
#[derive(Debug, Clone)]
#[cfg(any(feature = "blocking-client", feature = "async-client"))]
pub enum Source {
    /// An object id, as the matched ref-spec was an object id itself.
    ObjectId(gix_hash::ObjectId),
    /// The remote reference that matched the ref-specs name.
    Ref(crate::handshake::Ref),
}

#[cfg(any(feature = "blocking-client", feature = "async-client"))]
impl Source {
    /// Return either the direct object id we refer to or the direct target that a reference refers to.
    /// The latter may be a direct or a symbolic reference.
    /// If unborn, `None` is returned.
    pub fn as_id(&self) -> Option<&gix_hash::oid> {
        match self {
            Source::ObjectId(id) => Some(id),
            Source::Ref(r) => r.unpack().1,
        }
    }

    /// Return the target that this symbolic ref is pointing to, or `None` if it is no symbolic ref.
    pub fn as_target(&self) -> Option<&bstr::BStr> {
        match self {
            Source::ObjectId(_) => None,
            Source::Ref(r) => match r {
                crate::handshake::Ref::Peeled { .. } | crate::handshake::Ref::Direct { .. } => None,
                crate::handshake::Ref::Symbolic { target, .. } | crate::handshake::Ref::Unborn { target, .. } => {
                    Some(target.as_ref())
                }
            },
        }
    }

    /// Returns the peeled id of this instance, that is the object that can't be de-referenced anymore.
    pub fn peeled_id(&self) -> Option<&gix_hash::oid> {
        match self {
            Source::ObjectId(id) => Some(id),
            Source::Ref(r) => {
                let (_name, target, peeled) = r.unpack();
                peeled.or(target)
            }
        }
    }

    /// Return ourselves as the full name of the reference we represent, or `None` if this source isn't a reference but an object.
    pub fn as_name(&self) -> Option<&bstr::BStr> {
        match self {
            Source::ObjectId(_) => None,
            Source::Ref(r) => match r {
                crate::handshake::Ref::Unborn { full_ref_name, .. }
                | crate::handshake::Ref::Symbolic { full_ref_name, .. }
                | crate::handshake::Ref::Direct { full_ref_name, .. }
                | crate::handshake::Ref::Peeled { full_ref_name, .. } => Some(full_ref_name.as_ref()),
            },
        }
    }
}

/// An index into various lists of refspecs that have been used in a [Mapping] of remote references to local ones.
#[cfg(any(feature = "blocking-client", feature = "async-client"))]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum SpecIndex {
    /// An index into the _refspecs of the remote_ that triggered a fetch operation.
    /// These refspecs are explicit and visible to the user.
    ExplicitInRemote(usize),
    /// An index into the list of [extra refspecs][crate::remote::fetch::RefMap::extra_refspecs] that are implicit
    /// to a particular fetch operation.
    Implicit(usize),
}

#[cfg(any(feature = "blocking-client", feature = "async-client"))]
impl SpecIndex {
    /// Depending on our index variant, get the index either from `refspecs` or from `extra_refspecs` for `Implicit` variants.
    pub fn get<'a>(
        self,
        refspecs: &'a [gix_refspec::RefSpec],
        extra_refspecs: &'a [gix_refspec::RefSpec],
    ) -> Option<&'a gix_refspec::RefSpec> {
        match self {
            SpecIndex::ExplicitInRemote(idx) => refspecs.get(idx),
            SpecIndex::Implicit(idx) => extra_refspecs.get(idx),
        }
    }

    /// If this is an `Implicit` variant, return its index.
    pub fn implicit_index(self) -> Option<usize> {
        match self {
            SpecIndex::Implicit(idx) => Some(idx),
            SpecIndex::ExplicitInRemote(_) => None,
        }
    }
}

/// A mapping between a single remote reference and its advertised objects to a local destination which may or may not exist.
#[derive(Debug, Clone)]
#[cfg(any(feature = "blocking-client", feature = "async-client"))]
pub struct Mapping {
    /// The reference on the remote side, along with information about the objects they point to as advertised by the server.
    pub remote: Source,
    /// The local tracking reference to update after fetching the object visible via `remote`.
    pub local: Option<bstr::BString>,
    /// The index into the fetch ref-specs used to produce the mapping, allowing it to be recovered.
    pub spec_index: SpecIndex,
}
