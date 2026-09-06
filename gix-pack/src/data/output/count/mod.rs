use gix_hash::ObjectId;

use crate::data::output::Count;

/// Specifies how the pack location was handled during counting
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PackLocation {
    /// We did not lookup this object
    NotLookedUp,
    /// The object was looked up and there may be a location in a pack, along with entry information
    LookedUp(Option<crate::data::entry::Location>),
}

impl PackLocation {
    /// Directly go through to `LookedUp` variant, panic otherwise
    pub fn is_none(&self) -> bool {
        match self {
            PackLocation::LookedUp(opt) => opt.is_none(),
            PackLocation::NotLookedUp => unreachable!("must have been resolved"),
        }
    }
    /// Directly go through to `LookedUp` variant, panic otherwise
    pub fn as_ref(&self) -> Option<&crate::data::entry::Location> {
        match self {
            PackLocation::LookedUp(opt) => opt.as_ref(),
            PackLocation::NotLookedUp => unreachable!("must have been resolved"),
        }
    }
}

impl Count {
    /// Create a new instance from the given `oid` and its corresponding location.
    pub fn from_data(oid: impl Into<ObjectId>, location: Option<crate::data::entry::Location>) -> Self {
        Count {
            id: oid.into(),
            entry_pack_location: PackLocation::LookedUp(location),
        }
    }
}

#[path = "objects/mod.rs"]
mod objects_impl;
pub use objects_impl::{objects, objects_unthreaded};

///
pub mod objects {
    pub use super::objects_impl::{ObjectExpansion, Options, Outcome};
}
