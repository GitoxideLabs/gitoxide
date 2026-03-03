/// Reftable block type used for log records.
pub const BLOCK_TYPE_LOG: u8 = b'g';
/// Reftable block type used for index records.
pub const BLOCK_TYPE_INDEX: u8 = b'i';
/// Reftable block type used for ref records.
pub const BLOCK_TYPE_REF: u8 = b'r';
/// Reftable block type used for object-index records.
pub const BLOCK_TYPE_OBJ: u8 = b'o';
/// Wildcard block type.
pub const BLOCK_TYPE_ANY: u8 = 0;

/// Ref record value type for tombstones.
pub const REF_VAL_DELETION: u8 = 0;
/// Ref record value type for direct object ids.
pub const REF_VAL_VAL1: u8 = 1;
/// Ref record value type for direct+peeled object ids.
pub const REF_VAL_VAL2: u8 = 2;
/// Ref record value type for symbolic refs.
pub const REF_VAL_SYMREF: u8 = 3;

/// Log record value type for tombstones.
pub const LOG_VAL_DELETION: u8 = 0;
/// Log record value type for updates.
pub const LOG_VAL_UPDATE: u8 = 1;

/// Default reftable block size.
pub const DEFAULT_BLOCK_SIZE: usize = 4096;
/// Maximum reftable block size.
pub const MAX_BLOCK_SIZE: usize = 16_777_215;
/// Maximum restart interval per block.
pub const MAX_RESTART_INTERVAL: usize = u16::MAX as usize;
