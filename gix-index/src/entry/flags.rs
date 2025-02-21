use bitflags::bitflags;

use crate::entry::Stage;

bitflags! {
    /// In-memory flags.
    ///
    /// Notably, not all of these will be persisted but can be used to aid all kinds of operations.
    #[derive(Debug, Clone, Copy, Eq, PartialEq)]
    pub struct Flags: u32 {
        // TODO: could we use the pathlen ourselves to save 8 bytes? And how to handle longer paths than that? 0 as sentinel maybe?
        /// The mask to obtain the length of the path associated with this entry, up to 4095 characters without extension.
        const PATH_LEN = 0x0fff;
        /// The mask to apply to obtain the stage number of an entry, encoding three value: 0 = base, 1 = ours, 2 = theirs.
        const STAGE_MASK = (1<<12) | (1<<13);
        /// If set, additional bits need to be written to storage.
        const EXTENDED = 1<<14;
        /// If set, the entry be assumed to match with the version on the working tree, as a way to avoid `lstat()`  checks.
        const ASSUME_VALID = 1 << 15;
        /// Indicates that an entry needs to be updated as it's in-memory representation doesn't match what's on disk.
        const UPDATE = 1 << 16;
        /// Indicates an entry should be removed - this typically happens during writing, by simply skipping over them.
        const REMOVE = 1 << 17;
        /// Indicates that an entry is known to be up-to-date.
        const UPTODATE = 1 << 18;
        /// Only temporarily used by unpack_trees() (in C).
        const ADDED = 1 << 19;

        /// Whether an up-to-date object hash exists for the entry.
        const HASHED = 1 << 20;
        /// Set if the filesystem monitor is valid.
        const FSMONITOR_VALID = 1 << 21;
        /// Remove in work directory
        const WORKTREE_REMOVE = 1 << 22;
        /// Set to indicate the entry exists in multiple stages at once due to conflicts.
        const CONFLICTED = 1 << 23;

        /// Indicates that the entry was already turned into a tree.
        const UNPACKED = 1 << 24;
        /// Only temporarily used by unpack_trees() (in C)
        const NEW_SKIP_WORKTREE = 1 << 25;

        /// temporarily mark paths matched by a path spec
        const PATHSPEC_MATCHED = 1 << 26;

        /// When the index is split, this indicates the entry is up-to-date in the shared portion of the index.
        const UPDATE_IN_BASE = 1 << 27;
        /// Indicates the entry name is present in the base/shared index, and thus doesn't have to be stored in this one.
        const STRIP_NAME = 1 << 28;

        /// Created with `git add --intent-to-add` to mark empty entries that have their counter-part in the worktree, but not
        /// yet in the object database.
        const INTENT_TO_ADD = 1 << 29;
        /// Stored at rest
        const SKIP_WORKTREE = 1 << 30;

        /// For future extension
        const EXTENDED_2 = 1 << 31;
    }
}

impl Flags {
    /// Create a new instance whose stage is set to `stage`.
    pub fn from_stage(stage: Stage) -> Self {
        Flags::from_bits((stage as u32) << 12).expect("stage can only be valid flags")
    }

    /// Return the stage as extracted from the bits of this instance.
    pub fn stage(&self) -> Stage {
        match self.stage_raw() {
            0 => Stage::Unconflicted,
            1 => Stage::Base,
            2 => Stage::Ours,
            3 => Stage::Theirs,
            _ => unreachable!("BUG: Flags::STAGE_MASK is two bits, whose 4 possible values we have covered"),
        }
    }

    /// Return an entry's stage as raw number between 0 and 4.
    /// Possible values are:
    ///
    /// * 0 = no conflict,
    /// * 1 = base,
    /// * 2 = ours,
    /// * 3 = theirs
    pub fn stage_raw(&self) -> u32 {
        (*self & Flags::STAGE_MASK).bits() >> 12
    }

    /// Transform ourselves to a storage representation to keep all flags which are to be persisted,
    /// skipping all extended flags. Note that the caller has to check for the `EXTENDED` bit to be present
    /// and write extended flags as well if so.
    pub fn to_storage(mut self) -> at_rest::Flags {
        at_rest::Flags::from_bits_retain(
            {
                self.remove(Self::PATH_LEN);
                self
            }
            .bits() as u16,
        )
    }
}

impl From<Stage> for Flags {
    fn from(value: Stage) -> Self {
        Flags::from_stage(value)
    }
}

pub(crate) mod at_rest {
    use bitflags::bitflags;

    bitflags! {
        /// Flags how they are serialized to a storage location
        #[derive(Copy, Clone, Debug)]
        pub struct Flags: u16 {
            /// A portion of a the flags that encodes the length of the path that follows.
            const PATH_LEN = 0x0fff;
            const STAGE_MASK = 0x3000;
            /// If set, there is more extended flags past this one
            const EXTENDED = 0x4000;
            /// If set, the entry be assumed to match with the version on the working tree, as a way to avoid `lstat()`  checks.
            const ASSUME_VALID = 0x8000;
        }
    }

    impl Flags {
        pub fn to_memory(self) -> super::Flags {
            super::Flags::from_bits_retain(u32::from(self.bits()))
        }
    }

    bitflags! {
        /// Extended flags - add flags for serialization here and offset them down to u16.
        #[derive(Copy, Clone, Debug, PartialEq)]
        pub struct FlagsExtended: u16 {
            const INTENT_TO_ADD = 1 << (29 - 16);
            const SKIP_WORKTREE = 1 << (30 - 16);
        }
    }

    impl FlagsExtended {
        pub fn from_flags(flags: super::Flags) -> Self {
            Self::from_bits_retain(
                ((flags & (super::Flags::INTENT_TO_ADD | super::Flags::SKIP_WORKTREE)).bits() >> 16) as u16,
            )
        }
        pub fn to_flags(self) -> Option<super::Flags> {
            super::Flags::from_bits(u32::from(self.bits()) << 16)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn flags_extended_conversion() {
            assert_eq!(
                FlagsExtended::all().to_flags(),
                Some(super::super::Flags::INTENT_TO_ADD | super::super::Flags::SKIP_WORKTREE)
            );
            assert_eq!(
                FlagsExtended::from_flags(super::super::Flags::all()),
                FlagsExtended::all()
            );
        }

        #[test]
        fn flags_from_bits_with_conflict() {
            let input = 0b1110_0010_1000_1011;
            assert_eq!(Flags::from_bits_retain(input).bits(), input);
        }
    }
}
