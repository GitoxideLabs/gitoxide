use super::Error;
use crate::{
    Repository,
    config::{cache::util::ApplyLeniency, tree::Pack},
};

pub fn index_threads(repo: &Repository) -> Result<Option<usize>, Error> {
    Ok(repo
        .config
        .resolved
        .integer_filter(Pack::THREADS, &mut repo.filter_config_section())
        .map(|threads| Pack::THREADS.try_into_usize(threads))
        .transpose()
        .with_leniency(repo.options.lenient_config)?)
}

pub fn pack_index_version(repo: &Repository) -> Result<gix_pack::index::Version, Error> {
    Ok(repo
        .config
        .resolved
        .integer(Pack::INDEX_VERSION)
        .map(|value| Pack::INDEX_VERSION.try_into_index_version(value))
        .transpose()
        .with_leniency(repo.options.lenient_config)?
        .unwrap_or(gix_pack::index::Version::V2))
}
