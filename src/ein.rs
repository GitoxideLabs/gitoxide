#![deny(unsafe_code)]

use gix::Result;

fn main() -> Result<()> {
    gitoxide::porcelain::main()
}

#[cfg(not(feature = "pretty-cli"))]
compile_error!("Please set 'pretty-cli' feature flag");
