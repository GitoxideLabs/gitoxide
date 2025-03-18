pub(super) mod function {
    pub fn env() -> anyhow::Result<()> {
        for (name, value) in std::env::vars_os() {
            println!("{}={}", repr(&name), repr(&value));
        }
        Ok(())
    }

    fn repr(text: &std::ffi::OsStr) -> String {
        text.to_str()
            .filter(|s| !s.chars().any(|c| c == '"' || c == '\n'))
            .map_or_else(|| format!("{text:?}"), ToOwned::to_owned)
    }
}
