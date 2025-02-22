use gix::bstr::BString;

use crate::OutputFormat;

pub struct Options {
    pub format: OutputFormat,
    pub dry_run: bool,
    pub remote: Option<String>,
    /// If non-empty, override all ref-specs otherwise configured in the remote
    pub ref_specs: Vec<BString>,
    pub shallow: gix::remote::fetch::Shallow,
    pub handshake_info: bool,
    pub negotiation_info: bool,
    pub open_negotiation_graph: Option<std::path::PathBuf>,
}

pub const PROGRESS_RANGE: std::ops::RangeInclusive<u8> = 1..=3;

pub(crate) mod function {
    use anyhow::bail;
    use gix::{
        prelude::ObjectIdExt,
        refspec::match_group::validate::Fix,
        remote::fetch::{refs::update::TypeChange, Status},
    };
    use layout::{
        backends::svg::SVGWriter,
        core::{base::Orientation, geometry::Point, style::StyleAttr},
        std_shapes::shapes::{Arrow, Element, ShapeKind},
    };

    use super::Options;
    use crate::OutputFormat;

    pub fn fetch<P>(
        repo: gix::Repository,
        mut progress: P,
        mut out: impl std::io::Write,
        err: impl std::io::Write,
        Options {
            format,
            dry_run,
            remote,
            handshake_info,
            negotiation_info,
            open_negotiation_graph,
            shallow,
            ref_specs,
        }: Options,
    ) -> anyhow::Result<()>
    where
        P: gix::NestedProgress,
        P::SubProgress: 'static,
    {
        if format != OutputFormat::Human {
            bail!("JSON output isn't yet supported for fetching.");
        }

        let mut remote = crate::repository::remote::by_name_or_url(&repo, remote.as_deref())?;
        if !ref_specs.is_empty() {
            remote.replace_refspecs(ref_specs.iter(), gix::remote::Direction::Fetch)?;
            remote = remote.with_fetch_tags(gix::remote::fetch::Tags::None);
        }
        let res: gix::remote::fetch::Outcome = remote
            .connect(gix::remote::Direction::Fetch)?
            .prepare_fetch(&mut progress, Default::default())?
            .with_dry_run(dry_run)
            .with_shallow(shallow)
            .receive(&mut progress, &gix::interrupt::IS_INTERRUPTED)?;

        if handshake_info {
            writeln!(out, "Handshake Information")?;
            writeln!(out, "\t{:?}", res.handshake)?;
        }

        let ref_specs = remote.refspecs(gix::remote::Direction::Fetch);
        match res.status {
            Status::NoPackReceived {
                update_refs,
                negotiate,
                dry_run: _,
            } => {
                let negotiate_default = Default::default();
                print_updates(
                    &repo,
                    negotiate.as_ref().unwrap_or(&negotiate_default),
                    update_refs,
                    ref_specs,
                    res.ref_map,
                    &mut out,
                    err,
                )?;
                if negotiation_info {
                    print_negotiate_info(&mut out, negotiate.as_ref())?;
                }
                if let Some((negotiate, path)) =
                    open_negotiation_graph.and_then(|path| negotiate.as_ref().map(|n| (n, path)))
                {
                    render_graph(&repo, &negotiate.graph, &path, progress)?;
                }
                Ok::<_, anyhow::Error>(())
            }
            Status::Change {
                update_refs,
                write_pack_bundle,
                negotiate,
            } => {
                print_updates(&repo, &negotiate, update_refs, ref_specs, res.ref_map, &mut out, err)?;
                if let Some(data_path) = write_pack_bundle.data_path {
                    writeln!(out, "pack  file: \"{}\"", data_path.display()).ok();
                }
                if let Some(index_path) = write_pack_bundle.index_path {
                    writeln!(out, "index file: \"{}\"", index_path.display()).ok();
                }
                if negotiation_info {
                    print_negotiate_info(&mut out, Some(&negotiate))?;
                }
                if let Some(path) = open_negotiation_graph {
                    render_graph(&repo, &negotiate.graph, &path, progress)?;
                }
                Ok(())
            }
        }?;
        if dry_run {
            writeln!(out, "DRY-RUN: No ref was updated and no pack was received.").ok();
        }
        Ok(())
    }

    fn render_graph(
        repo: &gix::Repository,
        graph: &gix::negotiate::IdMap,
        path: &std::path::Path,
        mut progress: impl gix::Progress,
    ) -> anyhow::Result<()> {
        progress.init(Some(graph.len()), gix::progress::count("commits"));
        progress.set_name("building graph".into());

        let mut map = gix::hashtable::HashMap::default();
        let mut vg = layout::topo::layout::VisualGraph::new(Orientation::TopToBottom);

        for (id, commit) in graph.iter().inspect(|_| progress.inc()) {
            let source = match map.get(id) {
                Some(handle) => *handle,
                None => {
                    let handle = vg.add_node(new_node(id.attach(repo), commit.data.flags));
                    map.insert(*id, handle);
                    handle
                }
            };

            for parent_id in &commit.parents {
                let dest = match map.get(parent_id) {
                    Some(handle) => *handle,
                    None => {
                        let flags = match graph.get(parent_id) {
                            Some(c) => c.data.flags,
                            None => continue,
                        };
                        let dest = vg.add_node(new_node(parent_id.attach(repo), flags));
                        map.insert(*parent_id, dest);
                        dest
                    }
                };
                let arrow = Arrow::simple("");
                vg.add_edge(arrow, source, dest);
            }
        }

        let start = std::time::Instant::now();
        progress.set_name("layout graph".into());
        progress.info(format!("writing {path:?}…"));
        let mut svg = SVGWriter::new();
        vg.do_it(false, false, false, &mut svg);
        std::fs::write(path, svg.finalize().as_bytes())?;
        open::that(path)?;
        progress.show_throughput(start);

        return Ok(());

        fn new_node(id: gix::Id<'_>, flags: gix::negotiate::Flags) -> Element {
            let pt = Point::new(250., 50.);
            let name = format!("{}\n\n{flags:?}", id.shorten_or_id());
            let shape = ShapeKind::new_box(name.as_str());
            let style = StyleAttr::simple();
            Element::create(shape, style, Orientation::LeftToRight, pt)
        }
    }

    fn print_negotiate_info(
        mut out: impl std::io::Write,
        negotiate: Option<&gix::remote::fetch::outcome::Negotiate>,
    ) -> std::io::Result<()> {
        writeln!(out, "Negotiation Phase Information")?;
        match negotiate {
            Some(negotiate) => {
                writeln!(out, "\t{:?}", negotiate.rounds)?;
                writeln!(out, "\tnum commits traversed in graph: {}", negotiate.graph.len())
            }
            None => writeln!(out, "\tno negotiation performed"),
        }
    }

    pub(crate) fn print_updates(
        repo: &gix::Repository,
        negotiate: &gix::remote::fetch::outcome::Negotiate,
        update_refs: gix::remote::fetch::refs::update::Outcome,
        refspecs: &[gix::refspec::RefSpec],
        mut map: gix::remote::fetch::RefMap,
        mut out: impl std::io::Write,
        mut err: impl std::io::Write,
    ) -> anyhow::Result<()> {
        let mut last_spec_index = gix::remote::fetch::refmap::SpecIndex::ExplicitInRemote(usize::MAX);
        let mut updates = update_refs
            .iter_mapping_updates(&map.mappings, refspecs, &map.extra_refspecs)
            .filter_map(|(update, mapping, spec, edit)| spec.map(|spec| (update, mapping, spec, edit)))
            .collect::<Vec<_>>();
        updates.sort_by_key(|t| t.2);
        let mut skipped_due_to_implicit_tag = None;
        fn consume_skipped_tags(skipped: &mut Option<usize>, out: &mut impl std::io::Write) -> std::io::Result<()> {
            if let Some(skipped) = skipped.take() {
                if skipped != 0 {
                    writeln!(
                        out,
                        "\tskipped {skipped} tags known to the remote without bearing on this commit-graph. Use `gix remote ref-map` to list them."
                    )?;
                }
            }
            Ok(())
        }
        for (update, mapping, spec, edit) in updates {
            if mapping.spec_index != last_spec_index {
                last_spec_index = mapping.spec_index;
                consume_skipped_tags(&mut skipped_due_to_implicit_tag, &mut out)?;
                spec.to_ref().write_to(&mut out)?;
                let is_implicit = mapping.spec_index.implicit_index().is_some();
                if is_implicit {
                    write!(&mut out, " (implicit")?;
                    if spec.to_ref()
                        == gix::remote::fetch::Tags::Included
                            .to_refspec()
                            .expect("always yields refspec")
                    {
                        skipped_due_to_implicit_tag = Some(0);
                        write!(&mut out, ", due to auto-tag")?;
                    }
                    write!(&mut out, ")")?;
                }
                writeln!(out)?;
            }

            if let Some(num_skipped) = skipped_due_to_implicit_tag.as_mut() {
                if matches!(update.mode, gix::remote::fetch::refs::update::Mode::NoChangeNeeded) {
                    *num_skipped += 1;
                    continue;
                }
            }

            write!(out, "\t")?;
            match &mapping.remote {
                gix::remote::fetch::refmap::Source::ObjectId(id) => {
                    write!(out, "{}", id.attach(repo).shorten_or_id())?;
                }
                gix::remote::fetch::refmap::Source::Ref(r) => {
                    crate::repository::remote::refs::print_ref(&mut out, r)?;
                }
            }
            let mode_and_type = update.type_change.map_or_else(
                || format!("{}", update.mode),
                |type_change| {
                    format!(
                        "{} ({})",
                        update.mode,
                        match type_change {
                            TypeChange::DirectToSymbolic => {
                                "direct ref overwrites symbolic"
                            }
                            TypeChange::SymbolicToDirect => {
                                "symbolic ref overwrites direct"
                            }
                        }
                    )
                },
            );
            match edit {
                Some(edit) => {
                    writeln!(out, " -> {} [{mode_and_type}]", edit.name)
                }
                None => writeln!(out, " [{mode_and_type}]"),
            }?;
        }
        consume_skipped_tags(&mut skipped_due_to_implicit_tag, &mut out)?;
        if !map.fixes.is_empty() {
            writeln!(
                err,
                "The following destination refs were removed as they didn't start with 'ref/'"
            )?;
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
                            spec.to_ref().write_to(&mut err)?;
                            writeln!(err)?;
                        }
                        writeln!(err, "\t{name}")?;
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
            )?;
        }
        match negotiate.rounds.len() {
            0 => writeln!(err, "no negotiation was necessary")?,
            1 => {}
            rounds => writeln!(err, "needed {rounds} rounds of pack-negotiation")?,
        }
        Ok(())
    }
}
