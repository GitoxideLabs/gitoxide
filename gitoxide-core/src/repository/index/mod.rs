use gix::{
    Result,
    error::{ResultExt, bail, message},
};
use std::{ffi::OsString, path::PathBuf};

pub fn from_tree(
    repo: gix::Repository,
    mut spec: OsString,
    index_path: Option<PathBuf>,
    force: bool,
    skip_hash: bool,
) -> Result<()> {
    spec.push("^{tree}");
    let spec = gix::path::os_str_into_bstr(&spec)?;
    let tree = repo.rev_parse_single(spec)?;

    let mut index = repo.index_from_tree(&tree)?;
    let options = gix::index::write::Options {
        skip_hash,
        ..Default::default()
    };

    match index_path {
        Some(index_path) => {
            if index_path.is_file() && !force {
                bail!(gix::error::conflict(format!(
                    "File at \"{}\" already exists, to overwrite use the '-f' flag",
                    index_path.display()
                )));
            }
            index.set_path(index_path);
            index.write(options)?;
        }
        None => {
            let mut out = Vec::with_capacity(512 * 1024);
            index.write_to(&mut out, options)?;
        }
    }

    Ok(())
}

pub fn from_list(
    entries_file: PathBuf,
    index_path: Option<PathBuf>,
    force: bool,
    object_hash: gix::hash::Kind,
    skip_hash: bool,
) -> Result<()> {
    use std::io::BufRead;

    let mut index = gix::index::State::new(object_hash);
    for path in std::io::BufReader::new(std::fs::File::open(entries_file).or_error()?).lines() {
        let path: PathBuf = path.or_error()?.into();
        if !path.is_relative() {
            let err = message!(
                "Input paths need to be relative, but {path} is not.",
                path = path.display()
            )
            .validation();
            bail!(err)
        }
        let path = gix::path::into_bstr(path);
        index.dangerously_push_entry(
            gix::index::entry::Stat::default(),
            gix::hash::ObjectId::empty_blob(object_hash),
            gix::index::entry::Flags::empty(),
            gix::index::entry::Mode::FILE,
            gix::path::to_unix_separators_on_windows(path).as_ref(),
        );
    }
    index.sort_entries();

    let options = gix::index::write::Options {
        skip_hash,
        ..Default::default()
    };
    match index_path {
        Some(index_path) => {
            if index_path.is_file() && !force {
                bail!(
                    "File at \"{path}\" already exists, to overwrite use the '-f' flag".conflict(),
                    path = index_path.display()
                );
            }
            let mut index = gix::index::File::from_state(index, index_path);
            index.write(options)?;
        }
        None => {
            let index = gix::index::File::from_state(index, std::path::PathBuf::new());
            let mut out = Vec::with_capacity(512 * 1024);
            index.write_to(&mut out, options)?;
        }
    }

    Ok(())
}

pub mod entries;
pub use entries::function::entries;
