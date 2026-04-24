use std::{ops::Range, path::PathBuf, sync::atomic::AtomicBool};

pub fn virtual_path(suffix: &str) -> PathBuf {
    PathBuf::from(format!("fuzz-input{suffix}"))
}

pub fn interrupt_flag() -> AtomicBool {
    AtomicBool::new(false)
}

pub fn empty_candidates() -> Range<u32> {
    0..0
}
