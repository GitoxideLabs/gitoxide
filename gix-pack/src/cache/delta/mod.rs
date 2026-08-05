/// Returned when using various methods on a [`Tree`]
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Error {
    InvariantIncreasingPackOffset {
        /// The last seen pack offset
        last_pack_offset: crate::data::Offset,
        /// The invariant violating offset
        pack_offset: crate::data::Offset,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvariantIncreasingPackOffset {
                last_pack_offset,
                pack_offset,
            } => write!(
                f,
                "Pack offsets must only increment. The previous pack offset was {last_pack_offset}, the current one is {pack_offset}"
            ),
        }
    }
}

impl std::error::Error for Error {}

///
pub mod traverse;

///
pub mod from_offsets;

/// Tree datastructure
// kept in separate module to encapsulate unsafety (it has field invariants)
mod tree;

pub use tree::{Item, Tree};
