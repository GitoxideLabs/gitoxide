use crate::OutputFormat;

pub struct Options {
    pub format: OutputFormat,
    pub explain: bool,
    pub cat_file: bool,
    pub tree_mode: TreeMode,
    pub blob_format: BlobFormat,
    pub show_reference: bool,
}

pub enum TreeMode {
    Raw,
    Pretty,
}

#[derive(Copy, Clone)]
pub enum BlobFormat {
    Git,
    Worktree,
    Diff,
    DiffOrGit,
}

pub(crate) mod function {
    #[cfg(feature = "serde")]
    use gix::error::bail;
    use gix::{Result, error::ResultExt};
    use std::ffi::OsString;

    use super::Options;
    use crate::{
        OutputFormat,
        repository::{cat::display_object, revision, revision::resolve::BlobFormat},
    };

    pub fn resolve(
        mut repo: gix::Repository,
        specs: Vec<OsString>,
        mut out: impl std::io::Write,
        Options {
            format,
            explain,
            cat_file,
            tree_mode,
            blob_format,
            show_reference,
        }: Options,
    ) -> Result<()> {
        repo.object_cache_size_if_unset(1024 * 1024);
        let mut cache = (!matches!(blob_format, BlobFormat::Git))
            .then(|| {
                repo.diff_resource_cache(
                    match blob_format {
                        BlobFormat::Git => {
                            unreachable!("checked before")
                        }
                        BlobFormat::Worktree | BlobFormat::Diff => {
                            gix::diff::blob::pipeline::Mode::ToWorktreeAndBinaryToText
                        }
                        BlobFormat::DiffOrGit => gix::diff::blob::pipeline::Mode::ToGitUnlessBinaryToTextIsPresent,
                    },
                    Default::default(),
                )
            })
            .transpose()?;

        match format {
            OutputFormat::Human => {
                for spec in specs {
                    if explain {
                        return revision::explain(spec, out);
                    }
                    let spec = gix::path::os_str_into_bstr(&spec)?;
                    let spec = repo.rev_parse(spec)?;
                    if cat_file {
                        return display_object(&repo, spec, tree_mode, cache.as_mut().map(|c| (blob_format, c)), out);
                    }
                    if let Some(r) = spec.first_reference().filter(|_| show_reference) {
                        writeln!(out, "{}", r.name).or_error()?;
                    }
                    if let Some(r) = spec.second_reference().filter(|_| show_reference) {
                        writeln!(out, "{}", r.name).or_error()?;
                    }
                    writeln!(out, "{spec}", spec = spec.detach()).or_error()?;
                }
            }
            #[cfg(feature = "serde")]
            OutputFormat::Json => {
                if explain {
                    bail!(gix::error::unsupported("Explanations are only for human consumption"))
                }
                serde_json::to_writer_pretty(
                    &mut out,
                    &specs
                        .into_iter()
                        .map(|spec| {
                            gix::path::os_str_into_bstr(&spec)
                                .and_then(|spec| repo.rev_parse(spec))
                                .map(gix::revision::Spec::detach)
                        })
                        .collect::<std::result::Result<Vec<_>, _>>()?,
                )
                .or_error()?;
            }
        }
        Ok(())
    }
}
