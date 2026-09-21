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

#[cfg(feature = "notify")]
impl crate::Repository {
    /// Return resolved configuration and pattern dependencies without requiring their files to exist.
    pub(crate) fn notification_sources(&self) -> Result<Vec<crate::notify::Source>, crate::notify::Error> {
        use gix_error::{ResultExt, message};

        use crate::notify::{Source, SourceKind};

        let mut sources: Vec<_> = self
            .config
            .source_paths
            .iter()
            .map(|path| Source {
                path: path.clone(),
                kind: SourceKind::Configuration,
            })
            .collect();
        sources.extend([
            Source {
                path: self.common_dir().join("info/exclude"),
                kind: SourceKind::Ignore,
            },
            Source {
                path: self.common_dir().join("info/attributes"),
                kind: SourceKind::Attributes,
            },
        ]);
        self.config
            .excludes_file(&mut |path| {
                sources.push(Source {
                    path: path.to_owned(),
                    kind: SourceKind::Ignore,
                });
            })
            .or_raise(|| message("could not resolve ignore monitoring sources"))?;
        self.config
            .attribute_global_paths(self.options.permissions.attributes, &mut |path| {
                sources.push(Source {
                    path: path.to_owned(),
                    kind: SourceKind::Attributes,
                });
            })
            .or_raise(|| message("could not resolve attribute monitoring sources"))?;
        sources.sort();
        sources.dedup();
        Ok(sources)
    }
}

#[cfg(all(test, feature = "notify"))]
mod tests;
