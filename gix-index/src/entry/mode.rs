use crate::entry::Mode;

impl Mode {
    /// Return `true` if this is a sparse entry, as it points to a directory which usually isn't what an 'unsparse' index tracks.
    pub fn is_sparse(&self) -> bool {
        *self == Self::DIR
    }

    /// Return `true` if this is a submodule entry.
    pub fn is_submodule(&self) -> bool {
        *self == Self::DIR | Self::SYMLINK
    }

    /// Convert this instance to a tree's entry mode, or return `None` if for some
    /// and unexpected reason the bitflags don't resemble any known entry-mode.
    pub fn to_tree_entry_mode(&self) -> Option<gix_object::tree::EntryMode> {
        gix_object::tree::EntryMode::try_from(self.bits()).ok()
    }

    /// Compares this mode to the file system version ([`std::fs::symlink_metadata`])
    /// and returns the change needed to update this mode to match the file.
    ///
    /// * if `has_symlinks` is false symlink entries will simply check if there
    ///   is a normal file on disk
    /// * if `executable_bit` is false the executable bit will not be compared
    ///   `Change::ExecutableBit` will never be generated
    ///
    /// If there is a type change then we will use whatever information is
    /// present on the FS. Specifically if `has_symlinks` is false we will
    /// never generate `Change::TypeChange { new_mode: Mode::SYMLINK }`. and
    /// if `executable_bit` is false we will never generate `Change::TypeChange
    /// { new_mode: Mode::FILE_EXECUTABLE }` (all files are assumed to be not
    /// executable). That means that unstaging and staging files can be a lossy
    /// operation on such file systems.
    ///
    /// If a directory replaced a normal file/symlink we assume that the
    /// directory is a submodule. Normal (non-submodule) directories would
    /// cause a file to be deleted from the index and should be handled before
    /// calling this function.
    ///
    /// If the stat information belongs to something other than a normal file/
    /// directory (like a socket) we just return an identity change (non-files
    /// can not be committed to git).
    pub fn change_to_match_fs(
        self,
        stat: &crate::fs::Metadata,
        has_symlinks: bool,
        executable_bit: bool,
    ) -> Option<Change> {
        match self {
            Mode::FILE if !stat.is_file() => (),
            Mode::SYMLINK if has_symlinks && !stat.is_symlink() => (),
            Mode::SYMLINK if !has_symlinks && !stat.is_file() => (),
            Mode::COMMIT | Mode::DIR if !stat.is_dir() => (),
            Mode::FILE if executable_bit && stat.is_executable() => return Some(Change::ExecutableBit),
            Mode::FILE_EXECUTABLE if executable_bit && !stat.is_executable() => return Some(Change::ExecutableBit),
            _ => return None,
        }
        let new_mode = if stat.is_dir() {
            Mode::COMMIT
        } else if executable_bit && stat.is_executable() {
            Mode::FILE_EXECUTABLE
        } else if has_symlinks && stat.is_symlink() {
            Mode::SYMLINK
        } else {
            Mode::FILE
        };
        Some(Change::Type { new_mode })
    }
}

impl From<gix_object::tree::EntryMode> for Mode {
    fn from(value: gix_object::tree::EntryMode) -> Self {
        Self::from_bits_truncate(u32::from(value.0))
    }
}

/// A change of a [`Mode`].
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Change {
    /// The type of mode changed, like symlink => file.
    Type {
        /// The mode representing the new index type.
        new_mode: Mode,
    },
    /// The executable permission of this file has changed.
    ExecutableBit,
}

impl Change {
    /// Applies this change to `mode` and returns the changed one.
    pub fn apply(self, mode: Mode) -> Mode {
        match self {
            Change::Type { new_mode } => new_mode,
            Change::ExecutableBit => match mode {
                Mode::FILE => Mode::FILE_EXECUTABLE,
                Mode::FILE_EXECUTABLE => Mode::FILE,
                _ => unreachable!("invalid mode change: can't flip executable bit of {mode:?}"),
            },
        }
    }
}
