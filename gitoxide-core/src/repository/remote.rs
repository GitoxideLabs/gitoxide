use gix::{
    Result,
    error::{OptionExt, ResultExt, bail, message},
};

/// Print the effective URL or URLs of the selected remote.
///
/// Without an explicit remote, selection follows the fetch or push configuration for the current branch according to `direction`.
pub fn url(
    repo: gix::Repository,
    name: Option<&str>,
    direction: gix::remote::Direction,
    all: bool,
    mut out: impl std::io::Write,
) -> Result<()> {
    let remote = match (name, direction) {
        (Some(name), _) => repo.find_fetch_remote(Some(name.into()))?,
        (None, gix::remote::Direction::Fetch) => repo.find_fetch_remote(None)?,
        (None, gix::remote::Direction::Push) => repo
            .head()?
            .into_remote(gix::remote::Direction::Push)
            .or_else(|| repo.find_default_remote(gix::remote::Direction::Push))
            .transpose()?
            .ok_or_raise(|| gix::error::not_found("Could not determine a remote for pushing"))?,
    };
    if all {
        let mut urls = remote.urls(direction).peekable();
        if urls.peek().is_none() {
            bail!("The remote has no {} URL".not_found(), direction.as_str());
        }
        for url in urls {
            out.write_all(&url.to_bstring()).or_error()?;
            out.write_all(b"\n").or_error()?;
        }
    } else {
        let url = remote
            .url(direction)
            .ok_or_raise(|| message!("The remote has no {} URL", direction.as_str()).not_found())?;
        out.write_all(&url.to_bstring()).or_error()?;
        out.write_all(b"\n").or_error()?;
    }
    Ok(())
}

#[cfg(any(feature = "blocking-client", feature = "async-client"))]
mod refs_impl {
    use gix::{
        Result,
        error::{OptionExt, ResultExt, bail, unsupported, validation},
        protocol::handshake,
        refspec::{RefSpec, match_group::validate::Fix},
        remote::fetch::refmap::Source,
    };

    use super::by_name_or_url;
    use crate::OutputFormat;

    pub mod refs {
        use gix::bstr::BString;

        use crate::OutputFormat;

        pub const PROGRESS_RANGE: std::ops::RangeInclusive<u8> = 1..=2;

        pub enum Kind {
            Remote,
            Tracking {
                ref_specs: Vec<BString>,
                show_unmapped_remote_refs: bool,
            },
        }

        pub struct Options {
            pub format: OutputFormat,
            pub name_or_url: Option<String>,
            pub handshake_info: bool,
        }

        pub(crate) use super::{print, print_ref};
    }

    #[gix::protocol::bisync::bisync]
    pub async fn refs_fn(
        repo: gix::Repository,
        kind: refs::Kind,
        mut progress: impl gix::Progress,
        mut out: impl std::io::Write,
        err: impl std::io::Write,
        refs::Options {
            format,
            name_or_url,
            handshake_info,
        }: refs::Options,
    ) -> Result<()> {
        let mut remote = by_name_or_url(&repo, name_or_url.as_deref())?;
        let show_unmapped = if let refs::Kind::Tracking {
            ref_specs,
            show_unmapped_remote_refs,
        } = &kind
        {
            if format != OutputFormat::Human {
                bail!(unsupported("JSON output isn't yet supported for listing ref-mappings."));
            }
            if !ref_specs.is_empty() {
                remote.replace_refspecs(ref_specs.iter(), gix::remote::Direction::Fetch)?;
                remote = remote.with_fetch_tags(gix::remote::fetch::Tags::None);
            }
            *show_unmapped_remote_refs
        } else {
            false
        };
        progress.info(format!(
            "Connecting to {:?}",
            remote
                .url(gix::remote::Direction::Fetch)
                .ok_or_raise(|| gix::error::not_found("Remote didn't have a URL to connect to"))?
                .to_bstring()
        ));
        let (map, handshake) = remote
            .connect(gix::remote::Direction::Fetch)
            .await?
            .ref_map(
                &mut progress,
                gix::remote::ref_map::Options {
                    prefix_from_spec_as_filter_on_remote: !matches!(kind, refs::Kind::Remote),
                    ..Default::default()
                },
            )
            .await?;

        if handshake_info {
            writeln!(out, "Handshake Information").or_error()?;
            writeln!(out, "\t{handshake:?}").or_error()?;
        }
        match kind {
            refs::Kind::Tracking { .. } => print_refmap(
                &repo,
                remote.refspecs(gix::remote::Direction::Fetch),
                map,
                show_unmapped,
                out,
                err,
            ),
            refs::Kind::Remote => {
                match format {
                    OutputFormat::Human => drop(print(out, &map.remote_refs)),
                    #[cfg(feature = "serde")]
                    OutputFormat::Json => serde_json::to_writer_pretty(
                        out,
                        &map.remote_refs.into_iter().map(JsonRef::from).collect::<Vec<_>>(),
                    )
                    .or_error()?,
                }
                Ok(())
            }
        }
    }

    pub(crate) fn print_refmap(
        repo: &gix::Repository,
        refspecs: &[RefSpec],
        mut map: gix::remote::fetch::RefMap,
        show_unmapped_remotes: bool,
        mut out: impl std::io::Write,
        mut err: impl std::io::Write,
    ) -> Result<()> {
        let mut last_spec_index = gix::remote::fetch::refmap::SpecIndex::ExplicitInRemote(usize::MAX);
        map.mappings.sort_by_key(|m| m.spec_index);
        for mapping in &map.mappings {
            if mapping.spec_index != last_spec_index {
                last_spec_index = mapping.spec_index;
                let spec = mapping
                    .spec_index
                    .get(refspecs, &map.extra_refspecs)
                    .expect("refspecs here are the ones used for mapping");
                spec.to_ref().write_to(&mut out).or_error()?;
                let is_implicit = mapping.spec_index.implicit_index().is_some();
                if is_implicit {
                    write!(&mut out, " (implicit").or_error()?;
                    if spec.to_ref()
                        == gix::remote::fetch::Tags::Included
                            .to_refspec()
                            .expect("always yields refspec")
                    {
                        write!(&mut out, ", due to auto-tag").or_error()?;
                    }
                    write!(&mut out, ")").or_error()?;
                }
                writeln!(out).or_error()?;
            }

            write!(out, "\t").or_error()?;
            let target_id = match &mapping.remote {
                gix::remote::fetch::refmap::Source::ObjectId(id) => {
                    write!(out, "{id}").or_error()?;
                    id
                }
                gix::remote::fetch::refmap::Source::Ref(r) => print_ref(&mut out, r).or_error()?,
            };
            match &mapping.local {
                Some(local) => {
                    write!(out, " -> {local} ").or_error()?;
                    match repo.try_find_reference(local)? {
                        Some(tracking) => {
                            let msg = match tracking.try_id() {
                                Some(id) => {
                                    if id.as_ref() == target_id {
                                        "[up-to-date]"
                                    } else {
                                        "[changed]"
                                    }
                                }
                                None => "[skipped]",
                            };
                            writeln!(out, "{msg}")
                        }
                        None => writeln!(out, "[new]"),
                    }
                }
                None => writeln!(out, " (fetch only)"),
            }
            .or_error()?;
        }
        if !map.fixes.is_empty() {
            writeln!(
                err,
                "The following destination refs were removed as they didn't start with 'ref/'"
            )
            .or_error()?;
            map.fixes.sort_by(|l, r| match (l, r) {
                (
                    Fix::MappingWithPartialDestinationRemoved { spec: l, .. },
                    Fix::MappingWithPartialDestinationRemoved { spec: r, .. },
                ) => l.cmp(r),
            });
            let mut prev_spec = None;
            for fix in &map.fixes {
                match fix {
                    Fix::MappingWithPartialDestinationRemoved { name, spec } => {
                        if prev_spec.is_some_and(|prev_spec| prev_spec != spec) {
                            prev_spec = spec.into();
                            spec.to_ref().write_to(&mut err).or_error()?;
                            writeln!(err).or_error()?;
                        }
                        writeln!(err, "\t{name}").or_error()?;
                    }
                }
            }
        }
        if map.remote_refs.len() - map.mappings.len() != 0 {
            writeln!(
                err,
                "server sent {} tips, {} were filtered due to {} refspec(s).",
                map.remote_refs.len(),
                map.remote_refs.len() - map.mappings.len(),
                refspecs.len()
            )
            .or_error()?;
            if show_unmapped_remotes {
                writeln!(&mut out, "\nFiltered: ").or_error()?;
                for remote_ref in map.remote_refs.iter().filter(|r| {
                    !map.mappings.iter().any(|m| match &m.remote {
                        Source::Ref(other) => other == *r,
                        Source::ObjectId(_) => false,
                    })
                }) {
                    print_ref(&mut out, remote_ref).or_error()?;
                    writeln!(&mut out).or_error()?;
                }
            }
        }
        if refspecs.is_empty() {
            bail!(validation(
                "Without refspecs there is nothing to show here. Add refspecs as arguments or configure them in .git/config."
            ))
        }
        Ok(())
    }

    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub enum JsonRef {
        Peeled {
            path: String,
            tag: String,
            object: String,
        },
        Direct {
            path: String,
            object: String,
        },
        Unborn {
            path: String,
            target: String,
        },
        Symbolic {
            path: String,
            #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
            tag: Option<String>,
            target: String,
            object: String,
        },
    }

    impl From<handshake::Ref> for JsonRef {
        fn from(value: handshake::Ref) -> Self {
            match value {
                handshake::Ref::Unborn { full_ref_name, target } => JsonRef::Unborn {
                    path: full_ref_name.to_string(),
                    target: target.to_string(),
                },
                handshake::Ref::Direct {
                    full_ref_name: path,
                    object,
                } => JsonRef::Direct {
                    path: path.to_string(),
                    object: object.to_string(),
                },
                handshake::Ref::Symbolic {
                    full_ref_name: path,
                    tag,
                    target,
                    object,
                } => JsonRef::Symbolic {
                    path: path.to_string(),
                    tag: tag.map(|t| t.to_string()),
                    target: target.to_string(),
                    object: object.to_string(),
                },
                handshake::Ref::Peeled {
                    full_ref_name: path,
                    tag,
                    object,
                } => JsonRef::Peeled {
                    path: path.to_string(),
                    tag: tag.to_string(),
                    object: object.to_string(),
                },
            }
        }
    }

    pub(crate) fn print_ref(mut out: impl std::io::Write, r: &handshake::Ref) -> std::io::Result<&gix::hash::oid> {
        match r {
            handshake::Ref::Direct {
                full_ref_name: path,
                object,
            } => write!(&mut out, "{object} {path}").map(|_| object.as_ref()),
            handshake::Ref::Peeled {
                full_ref_name: path,
                tag,
                object,
            } => write!(&mut out, "{tag} {path} object:{object}").map(|_| tag.as_ref()),
            handshake::Ref::Symbolic {
                full_ref_name: path,
                tag,
                target,
                object,
            } => match tag {
                Some(tag) => {
                    write!(&mut out, "{tag} {path} symref-target:{target} peeled:{object}").map(|_| tag.as_ref())
                }
                None => write!(&mut out, "{object} {path} symref-target:{target}").map(|_| object.as_ref()),
            },
            handshake::Ref::Unborn { full_ref_name, target } => {
                static NULL: gix::hash::ObjectId = gix::hash::ObjectId::null(gix::hash::Kind::Sha1);
                write!(&mut out, "unborn {full_ref_name} symref-target:{target}").map(|_| NULL.as_ref())
            }
        }
    }

    pub(crate) fn print(mut out: impl std::io::Write, refs: &[handshake::Ref]) -> std::io::Result<()> {
        for r in refs {
            print_ref(&mut out, r)?;
            writeln!(out)?;
        }
        Ok(())
    }
}
#[cfg(any(feature = "blocking-client", feature = "async-client"))]
pub use refs_impl::{JsonRef, refs, refs_fn as refs};

#[cfg(any(feature = "blocking-client", feature = "async-client"))]
pub(crate) fn by_name_or_url<'repo>(
    repo: &'repo gix::Repository,
    name_or_url: Option<&str>,
) -> Result<gix::Remote<'repo>> {
    repo.find_fetch_remote(name_or_url.map(Into::into))
}
