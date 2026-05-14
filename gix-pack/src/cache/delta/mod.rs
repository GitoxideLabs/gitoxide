/// Returned when using various methods on a [`Tree`]
#[derive(thiserror::Error, Debug)]
#[allow(missing_docs)]
pub enum Error {
    #[error(
        "Pack offsets must only increment. The previous pack offset was {last_pack_offset}, the current one is {pack_offset}"
    )]
    InvariantIncreasingPackOffset {
        /// The last seen pack offset
        last_pack_offset: crate::data::Offset,
        /// The invariant violating offset
        pack_offset: crate::data::Offset,
    },
}

///
pub mod traverse;

///
pub mod from_offsets;

/// Tree datastructure
// kept in separate module to encapsulate unsafety (it has field invariants)
mod tree;

pub use tree::{Item, Tree};
