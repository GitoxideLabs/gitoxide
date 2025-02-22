use crate::OutputFormat;

pub struct Options {
    pub format: OutputFormat,
    pub bare: bool,
    pub handshake_info: bool,
    pub no_tags: bool,
    pub shallow: gix::remote::fetch::Shallow,
    pub ref_name: Option<gix::refs::PartialName>,
}

pub const PROGRESS_RANGE: std::ops::RangeInclusive<u8> = 1..=3;

pub(crate) mod function {
    use std::{borrow::Cow, ffi::OsStr};

    use anyhow::{bail, Context};
    use gix::{bstr::BString, remote::fetch::Status, NestedProgress};

    use super::Options;
    use crate::{repository::fetch::function::print_updates, OutputFormat};

    pub fn clone<P>(
        url: impl AsRef<OsStr>,
        directory: Option<impl Into<std::path::PathBuf>>,
        overrides: Vec<BString>,
        mut progress: P,
        mut out: impl std::io::Write,
        mut err: impl std::io::Write,
        Options {
            format,
            handshake_info,
            bare,
            no_tags,
            ref_name,
            shallow,
        }: Options,
    ) -> anyhow::Result<()>
    where
        P: NestedProgress,
        P::SubProgress: 'static,
    {
        if format != OutputFormat::Human {
            bail!("JSON output isn't yet supported for fetching.");
        }

        let url: gix::Url = url.as_ref().try_into()?;
        let directory = directory.map_or_else(
            || {
                let path = gix::path::from_bstr(Cow::Borrowed(url.path.as_ref()));
                if !bare && path.extension() == Some(OsStr::new("git")) {
                    path.file_stem().map(Into::into)
                } else {
                    path.file_name().map(Into::into)
                }
                .context("Filename extraction failed - path too short")
            },
            |dir| Ok(dir.into()),
        )?;
        let mut prepare = gix::clone::PrepareFetch::new(
            url,
            directory,
            if bare {
                gix::create::Kind::Bare
            } else {
                gix::create::Kind::WithWorktree
            },
            gix::create::Options::default(),
            {
                let mut opts = gix::open::Options::default().config_overrides(overrides);
                opts.permissions.config.git_binary = true;
                opts
            },
        )?;
        if no_tags {
            prepare = prepare.configure_remote(|r| Ok(r.with_fetch_tags(gix::remote::fetch::Tags::None)));
        }
        let (mut checkout, fetch_outcome) = prepare
            .with_shallow(shallow)
            .with_ref_name(ref_name.as_ref())?
            .fetch_then_checkout(&mut progress, &gix::interrupt::IS_INTERRUPTED)?;

        let (repo, outcome) = if bare {
            (checkout.persist(), None)
        } else {
            let (repo, outcome) = checkout.main_worktree(progress, &gix::interrupt::IS_INTERRUPTED)?;
            (repo, Some(outcome))
        };

        if handshake_info {
            writeln!(out, "Handshake Information")?;
            writeln!(out, "\t{:?}", fetch_outcome.handshake)?;
        }

        match fetch_outcome.status {
            Status::NoPackReceived { dry_run, .. } => {
                assert!(!dry_run, "dry-run unsupported");
                writeln!(err, "The cloned repository appears to be empty")?;
            }
            Status::Change {
                update_refs, negotiate, ..
            } => {
                let remote = repo
                    .find_default_remote(gix::remote::Direction::Fetch)
                    .expect("one origin remote")?;
                let ref_specs = remote.refspecs(gix::remote::Direction::Fetch);
                print_updates(
                    &repo,
                    &negotiate,
                    update_refs,
                    ref_specs,
                    fetch_outcome.ref_map,
                    &mut out,
                    &mut err,
                )?;
            }
        }

        if let Some(gix::worktree::state::checkout::Outcome { collisions, errors, .. }) = outcome {
            if !(collisions.is_empty() && errors.is_empty()) {
                let mut messages = Vec::new();
                if !errors.is_empty() {
                    messages.push(format!("kept going through {} errors(s)", errors.len()));
                    for record in errors {
                        writeln!(err, "{}: {}", record.path, record.error).ok();
                    }
                }
                if !collisions.is_empty() {
                    messages.push(format!("encountered {} collision(s)", collisions.len()));
                    for col in collisions {
                        writeln!(err, "{}: collision ({:?})", col.path, col.error_kind).ok();
                    }
                }
                bail!(
                    "One or more errors occurred - checkout is incomplete: {}",
                    messages.join(", ")
                );
            }
        }
        Ok(())
    }
}
