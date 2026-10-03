#![deny(unsafe_code)]

use gix::Result;

#[cfg(feature = "pretty-cli")]
fn main() -> Result<()> {
    gitoxide::plumbing::main()
}

#[cfg(not(feature = "pretty-cli"))]
compile_error!("Please set 'pretty-cli' feature flag");
