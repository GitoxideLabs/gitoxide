#![deny(unsafe_code)]

#[cfg(feature = "pretty-cli")]
fn main() -> anyhow::Result<()> {
    gitoxide::plumbing::main()
}

#[cfg(not(feature = "pretty-cli"))]
compile_error!("Please set 'pretty-cli' feature flag");
