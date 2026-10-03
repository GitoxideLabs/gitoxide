pub(super) mod function {
    use gix::Result;
    pub fn env() -> Result<()> {
        for (name, value) in std::env::vars_os() {
            println!("{}={}", repr(&name), repr(&value));
        }
        Ok(())
    }

    #[allow(clippy::unnecessary_debug_formatting, reason = "preserve non-UTF-8 bytes")]
    fn repr(text: &std::ffi::OsStr) -> String {
        text.to_str()
            .filter(|s| !s.chars().any(|c| c == '"' || c == '\n'))
            .map_or_else(|| format!("{text:?}"), ToOwned::to_owned)
    }
}
