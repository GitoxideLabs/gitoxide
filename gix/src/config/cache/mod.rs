use super::{Cache, Error};

mod incubate;
pub(crate) use incubate::StageOne;

mod init;
pub(crate) use init::load;

impl std::fmt::Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cache").finish_non_exhaustive()
    }
}

pub(crate) mod access;

pub(crate) mod util;

pub(crate) use util::interpolate_context;
