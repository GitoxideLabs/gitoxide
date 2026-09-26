use crate::{
    Repository, Result,
    config::{cache::util::ApplyLeniency, tree::Pack},
};
use gix_error::ResultExt;

pub fn index_threads(repo: &Repository) -> Result<Option<usize>> {
    Pack::THREADS
        .try_into_usize(
            repo.config
                .resolved
                .integer_filter(Pack::THREADS, &mut repo.filter_config_section()),
        )
        .with_leniency(repo.options.lenient_config)
        .or_raise(|| gix_error::corruption("The configured pack thread count is invalid"))
}

pub fn pack_index_version(repo: &Repository) -> Result<gix_pack::index::Version> {
    Ok(Pack::INDEX_VERSION
        .try_into_index_version(repo.config.resolved.integer(Pack::INDEX_VERSION))
        .with_leniency(repo.options.lenient_config)
        .or_raise(|| gix_error::corruption("The configured pack index version is invalid"))?
        .unwrap_or(gix_pack::index::Version::V2))
}
