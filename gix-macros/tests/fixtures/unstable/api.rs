#[gix_macros::unstable(reason = "The plumbing interface is still changing.")]
pub mod plumbing {
    pub fn raw() -> usize {
        42
    }
}

gix_macros::unstable_module! {
    reason = "This file-backed module still exposes plumbing.";
    /// File-backed module documentation is preserved.
    pub mod module;
}

#[gix_macros::unstable(reason = "The type representation is still changing.")]
pub struct Raw(pub usize);

#[gix_macros::unstable_fields]
pub struct Handle {
    #[unstable(reason = "The field exposes plumbing state.")]
    pub raw: Raw,
    pub stable: usize,
}

impl Handle {
    pub fn new() -> Self {
        Self {
            raw: Raw(plumbing::raw()),
            stable: 1,
        }
    }

    pub fn stable(&self) -> usize {
        self.raw() + self.individual()
    }

    #[gix_macros::unstable(reason = "This method exposes plumbing state.")]
    pub fn individual(&self) -> usize {
        self.raw.0
    }
}

#[gix_macros::unstable(reason = "These methods expose plumbing state.")]
impl Handle {
    pub fn raw(&self) -> usize {
        self.raw.0
    }
}

#[gix_macros::unstable(reason = "This formatting is still experimental.")]
impl std::fmt::Display for Handle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.raw.0.fmt(f)
    }
}
