use crate::{
    config,
    config::tree::{Init, Key, Section, extensions, keys},
};

impl Init {
    /// The `init.templateDir` key.
    pub const TEMPLATE_DIR: keys::Path =
        keys::Path::new_path("templateDir", &config::Tree::INIT).with_environment_override("GIT_TEMPLATE_DIR");
    /// The `init.defaultBranch` key.
    // TODO: review its usage for cases where this key is not set - sometimes it's 'master', sometimes it's 'main'.
    pub const DEFAULT_BRANCH: keys::Any = keys::Any::new("defaultBranch", &config::Tree::INIT)
        .with_deviation("If not set, we use `main` instead of `master`");
    /// The `init.defaultObjectFormat` key.
    pub const DEFAULT_OBJECT_FORMAT: extensions::ObjectFormat =
        extensions::ObjectFormat::new_object_format("defaultObjectFormat", &config::Tree::INIT)
            .with_environment_override("GIT_DEFAULT_HASH")
            .with_deviation("Invalid values are an error, whereas `git` only warns about invalid configuration");
}

impl Section for Init {
    fn name(&self) -> &str {
        "init"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::DEFAULT_BRANCH, &Self::DEFAULT_OBJECT_FORMAT, &Self::TEMPLATE_DIR]
    }
}
