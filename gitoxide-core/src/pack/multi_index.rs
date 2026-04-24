use std::{io::BufWriter, path::PathBuf, sync::atomic::AtomicBool};

use anyhow::bail;
use gix::NestedProgress;

use crate::OutputFormat;

pub const PROGRESS_RANGE: std::ops::RangeInclusive<u8> = 1..=3;

pub fn verify(
    multi_index_path: PathBuf,
    mut progress: impl NestedProgress + 'static,
    should_interrupt: &AtomicBool,
) -> anyhow::Result<()> {
    gix::odb::pack::multi_index::File::at(multi_index_path, None)?
        .verify_integrity_fast(&mut progress, should_interrupt)?;
    Ok(())
}

pub fn create(
    index_paths: Vec<PathBuf>,
    output_path: PathBuf,
    mut progress: impl NestedProgress + 'static,
    should_interrupt: &AtomicBool,
    object_hash: gix::hash::Kind,
) -> anyhow::Result<()> {
    let mut out = BufWriter::new(gix::lock::File::acquire_to_update_resource(
        output_path,
        gix::lock::acquire::Fail::Immediately,
        None,
    )?);
    gix::odb::pack::multi_index::write_from_index_paths(
        index_paths,
        &mut out,
        &mut progress,
        should_interrupt,
        gix::odb::pack::multi_index::write::Options { object_hash },
    )?;
    out.into_inner()?.commit()?;
    Ok(())
}

#[cfg(feature = "serde")]
mod info {
    use std::path::PathBuf;

    #[derive(serde::Serialize)]
    pub struct Statistics {
        pub path: PathBuf,
        pub num_objects: u32,
        pub index_names: Vec<PathBuf>,
        pub object_hash: String,
    }
}

#[cfg_attr(not(feature = "serde"), allow(unused_variables))]
pub fn info(
    multi_index_path: PathBuf,
    format: OutputFormat,
    out: impl std::io::Write,
    mut err: impl std::io::Write,
) -> anyhow::Result<()> {
    if format == OutputFormat::Human {
        writeln!(err, "Defaulting to JSON as human format isn't implemented").ok();
    }
    #[cfg(feature = "serde")]
    {
        let file = gix::odb::pack::multi_index::File::at(&multi_index_path, None)?;
        serde_json::to_writer_pretty(
            out,
            &info::Statistics {
                path: multi_index_path,
                num_objects: file.num_objects(),
                index_names: file.index_names().to_vec(),
                object_hash: file.object_hash().to_string(),
            },
        )?;
    }
    Ok(())
}

pub fn entries(multi_index_path: PathBuf, format: OutputFormat, mut out: impl std::io::Write) -> anyhow::Result<()> {
    if format != OutputFormat::Human {
        bail!("Only human format is supported right now");
    }
    let file = gix::odb::pack::multi_index::File::at(multi_index_path, None)?;
    for entry in file.iter() {
        writeln!(out, "{} {} {}", entry.oid, entry.pack_index, entry.pack_offset)?;
    }
    Ok(())
}
