use gix::{
    Result,
    error::{ResultExt, bail, unsupported},
};
use std::path::Path;

pub struct Options {
    pub object_hash: gix::hash::Kind,
    pub format: crate::OutputFormat,
}

pub mod information;

fn parse_file(index_path: impl AsRef<Path>, object_hash: gix::hash::Kind) -> Result<gix::index::File> {
    gix::index::File::at(index_path.as_ref(), object_hash, false, Default::default()).or_error()
}

pub mod checkout_exclusive {
    pub struct Options {
        pub index: super::Options,
        /// If true, all files will be written with zero bytes despite having made an ODB lookup.
        pub empty_files: bool,
        pub keep_going: bool,
        /// If set, don't use more than this amount of threads.
        /// Otherwise, usually use as many threads as there are logical cores.
        /// A value of 0 is interpreted as no-limit
        pub thread_limit: Option<usize>,
    }
}

mod checkout;
pub use checkout::checkout_exclusive;

pub fn verify(
    index_path: impl AsRef<Path>,
    mut out: impl std::io::Write,
    Options { object_hash, format }: Options,
) -> Result<()> {
    let file = parse_file(index_path, object_hash)?;
    file.verify_integrity()?;
    file.verify_entries()?;
    file.verify_extensions(false, gix::objs::find::Never)?;
    #[cfg_attr(not(feature = "serde"), allow(irrefutable_let_patterns))]
    if let crate::OutputFormat::Human = format {
        writeln!(out, "OK").ok();
    }
    Ok(())
}

#[cfg_attr(not(feature = "serde"), allow(unused_variables, unused_mut))]
pub fn information(
    index_path: impl AsRef<Path>,
    out: impl std::io::Write,
    mut err: impl std::io::Write,
    information::Options {
        index: Options {
            object_hash,
            mut format,
        },
        extension_details,
    }: information::Options,
) -> Result<()> {
    use crate::OutputFormat::*;
    #[cfg(feature = "serde")]
    if let Human = format {
        writeln!(err, "Defaulting to JSON printing as nothing else will be implemented.").ok();
        format = Json;
    }
    match format {
        Human => {
            bail!(unsupported("Cannot print information using 'human' format."))
        }
        #[cfg(feature = "serde")]
        Json => {
            let info = information::Collection::try_from_file(parse_file(index_path, object_hash)?, extension_details)?;
            serde_json::to_writer_pretty(out, &info).or_error()?;
            Ok(())
        }
    }
}
