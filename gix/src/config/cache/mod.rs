use super::Cache;

mod incubate;
pub(crate) use incubate::StageOne;

mod init;
pub(crate) use init::{load, source_path};

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
    pub(crate) fn notification_dependencies(&self, attributes: bool) -> crate::Result<Vec<crate::notify::Dependency>> {
        use crate::notify::{Dependency, DependencyKind};

        let mut dependencies: Vec<_> = self
            .config
            .source_paths
            .iter()
            .map(|path| Dependency {
                path: path.clone(),
                kind: DependencyKind::Configuration,
            })
            .collect();
        dependencies.push(Dependency {
            path: self.common_dir().join("info/exclude"),
            kind: DependencyKind::Ignore,
        });
        // Share selection with pattern loading, retaining optional candidates before existence checks.
        self.config.excludes_file(Some(self.current_dir()), &mut |path| {
            dependencies.push(Dependency {
                path: path.to_owned(),
                kind: DependencyKind::Ignore,
            });
        })?;
        if attributes {
            dependencies.push(Dependency {
                path: self.common_dir().join("info/attributes"),
                kind: DependencyKind::Attributes,
            });
            self.config.attribute_global_paths(
                self.options.permissions.attributes,
                Some(self.current_dir()),
                &mut |path| {
                    dependencies.push(Dependency {
                        path: path.to_owned(),
                        kind: DependencyKind::Attributes,
                    });
                },
            )?;
        }
        dependencies.sort();
        dependencies.dedup();
        Ok(dependencies)
    }
}

#[cfg(all(test, feature = "notify"))]
mod tests;
