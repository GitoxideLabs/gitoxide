use std::collections::BTreeSet;

use crate::{
    file::{init, init::Options, Metadata},
    File,
};

/// The error returned by [`File::from_paths_metadata()`] and [`File::from_path_no_includes()`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("The configuration file at \"{}\" could not be read", path.display())]
    Io {
        source: std::io::Error,
        path: std::path::PathBuf,
    },
    #[error(transparent)]
    Init(#[from] init::Error),
}

/// Instantiation from one or more paths
impl File<'static> {
    /// Load the single file at `path` with `source` without following include directives.
    ///
    /// Note that the path will be checked for ownership to derive trust.
    pub fn from_path_no_includes(path: std::path::PathBuf, source: crate::Source) -> Result<Self, Error> {
        let trust = match gix_sec::Trust::from_path_ownership(&path) {
            Ok(t) => t,
            Err(err) => return Err(Error::Io { source: err, path }),
        };

        let mut buf = Vec::new();
        match std::io::copy(
            &mut match std::fs::File::open(&path) {
                Ok(f) => f,
                Err(err) => return Err(Error::Io { source: err, path }),
            },
            &mut buf,
        ) {
            Ok(_) => {}
            Err(err) => return Err(Error::Io { source: err, path }),
        }

        Ok(File::from_bytes_owned(
            &mut buf,
            Metadata::from(source).at(path).with(trust),
            Default::default(),
        )?)
    }

    /// Constructs a `git-config` file from the provided metadata, which must include a path to read from or be ignored.
    /// Returns `Ok(None)` if there was not a single input path provided, which is a possibility due to
    /// [`Metadata::path`] being an `Option`.
    /// If an input path doesn't exist, the entire operation will abort. See [`from_paths_metadata_buf()`][Self::from_paths_metadata_buf()]
    /// for a more powerful version of this method.
    pub fn from_paths_metadata(
        path_meta: impl IntoIterator<Item = impl Into<Metadata>>,
        options: Options<'_>,
    ) -> Result<Option<Self>, Error> {
        let mut buf = Vec::with_capacity(512);
        let err_on_nonexisting_paths = true;
        Self::from_paths_metadata_buf(
            &mut path_meta.into_iter().map(Into::into),
            &mut buf,
            err_on_nonexisting_paths,
            options,
        )
    }

    /// Like [`from_paths_metadata()`][Self::from_paths_metadata()], but will use `buf` to temporarily store the config file
    /// contents for parsing instead of allocating an own buffer.
    ///
    /// If `err_on_nonexisting_paths` is false, instead of aborting with error, we will continue to the next path instead.
    pub fn from_paths_metadata_buf(
        path_meta: &mut dyn Iterator<Item = Metadata>,
        buf: &mut Vec<u8>,
        err_on_non_existing_paths: bool,
        options: Options<'_>,
    ) -> Result<Option<Self>, Error> {
        let mut target = None;
        let mut seen = BTreeSet::default();
        for (path, mut meta) in path_meta.filter_map(|mut meta| meta.path.take().map(|p| (p, meta))) {
            if !seen.insert(path.clone()) {
                continue;
            }

            buf.clear();
            match std::io::copy(
                &mut match std::fs::File::open(&path) {
                    Ok(f) => f,
                    Err(err) if !err_on_non_existing_paths && err.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(err) => {
                        let err = Error::Io { source: err, path };
                        if options.ignore_io_errors {
                            gix_features::trace::warn!("ignoring: {err:#?}");
                            continue;
                        } else {
                            return Err(err);
                        }
                    }
                },
                buf,
            ) {
                Ok(_) => {}
                Err(err) => {
                    if options.ignore_io_errors {
                        gix_features::trace::warn!(
                            "ignoring: {:#?}",
                            Error::Io {
                                source: err,
                                path: path.clone()
                            }
                        );
                        buf.clear();
                    } else {
                        return Err(Error::Io { source: err, path });
                    }
                }
            }
            meta.path = Some(path);

            let config = Self::from_bytes_owned(buf, meta, options)?;
            match &mut target {
                None => {
                    target = Some(config);
                }
                Some(target) => {
                    target.append(config);
                }
            }
        }
        Ok(target)
    }
}
