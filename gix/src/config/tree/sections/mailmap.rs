use crate::config::{
    Tree,
    tree::{Key, Mailmap, Section, keys},
};

impl Mailmap {
    /// The `mailmap.blob` key
    pub const BLOB: keys::String = keys::String::new_string("blob", &Tree::MAILMAP);
    /// The `mailmap.file` key
    pub const FILE: keys::Path = keys::Path::new_path("file", &Tree::MAILMAP);
}

impl Section for Mailmap {
    fn name(&self) -> &str {
        "mailmap"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::BLOB, &Self::FILE]
    }
}
