#![forbid(unsafe_code)]

use std::{env, ffi::OsStr, fs, io::Write, path::PathBuf};

use gix_error::{OptionExt, Result, ResultExt, message};

fn main() -> Result<()> {
    let tables = PathBuf::from("src/tables");
    println!("cargo:rerun-if-changed={}", tables.display());
    let out_dir = PathBuf::from(
        env::var_os("OUT_DIR").ok_or_raise(|| message("Cargo must set OUT_DIR for compressed encoding tables"))?,
    );
    let mut entries = fs::read_dir(&tables)
        .or_raise(|| message!("Could not list encoding tables in '{}'", tables.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .or_raise(|| message!("Could not read encoding table entries in '{}'", tables.display()))?;
    entries.sort_by_key(fs::DirEntry::file_name);

    for entry in entries {
        let input_path = entry.path();
        if input_path.extension() != Some(OsStr::new("bin")) {
            continue;
        }
        println!("cargo:rerun-if-changed={}", input_path.display());
        let raw =
            fs::read(&input_path).or_raise(|| message!("Could not read encoding table '{}'", input_path.display()))?;
        let size = u32::try_from(raw.len())
            .or_raise(|| message!("Encoding table '{}' exceeds the u32 size limit", input_path.display()))?;
        let mut encoded = Vec::new();
        encoded.extend_from_slice(&size.to_le_bytes());
        let mut compressor = gix_zlib::stream::deflate::Write::new(encoded, gix_zlib::Compression::BEST);
        compressor
            .write_all(&raw)
            .or_raise(|| message!("Could not compress encoding table '{}'", input_path.display()))?;
        // This writer finalizes the zlib stream on flush, not on drop or into_inner().
        compressor.flush().or_raise(|| {
            message!(
                "Could not finalize compressed encoding table '{}'",
                input_path.display()
            )
        })?;
        let output_path = out_dir.join(entry.file_name());
        fs::write(&output_path, compressor.into_inner())
            .or_raise(|| message!("Could not write compressed encoding table '{}'", output_path.display()))?;
    }
    Ok(())
}
