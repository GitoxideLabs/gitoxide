#![deny(unsafe_code)]

fn main() -> anyhow::Result<()> {
    gitoxide::porcelain::main()
}

#[cfg(not(feature = "pretty-cli"))]
compile_error!("Please set 'pretty-cli' feature flag");
