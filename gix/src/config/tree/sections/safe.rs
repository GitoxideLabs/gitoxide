use crate::{
    config,
    config::tree::{Key, Safe, Section, keys},
};

impl Safe {
    /// The `safe.directory` key
    pub const DIRECTORY: keys::Any = keys::Any::new("directory", &config::Tree::SAFE);
}

impl Safe {
    /// Implements the directory filter to trust only global and system files, for use with `safe.directory`.
    pub fn directory_filter(meta: &gix_config::file::Metadata) -> bool {
        let kind = meta.source.kind();
        kind == gix_config::source::Kind::System || kind == gix_config::source::Kind::Global
    }
}

impl Section for Safe {
    fn name(&self) -> &str {
        "safe"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::DIRECTORY]
    }
}
