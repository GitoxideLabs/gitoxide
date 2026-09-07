#![forbid(unsafe_code)]

use anyhow::Result;

fn main() -> Result<()> {
    gix_tix::command::parse().run()
}
