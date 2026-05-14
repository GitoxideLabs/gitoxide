use std::{ffi::OsString, path::PathBuf};

use crate::OutputFormat;

pub struct Context {
    pub limit: Option<usize>,
    pub spec: OsString,
    pub format: OutputFormat,
    pub text: Format,
    pub long_hashes: bool,
}

pub enum Format {
    Text,
    Svg { path: PathBuf },
}
pub const PROGRESS_RANGE: std::ops::RangeInclusive<u8> = 0..=2;

pub(crate) mod function {
    use crate::repository::HexId;
    use crate::{OutputFormat, repository::revision::list::Format};
    use anyhow::{Context, bail};
    use gix::odb::store::RefreshMode;
    use gix::{Progress, hashtable::HashMap, revision::walk::Sorting};
    use layout::{
        backends::svg::SVGWriter,
        core::{base::Orientation, geometry::Point, style::StyleAttr},
        std_shapes::shapes::{Arrow, Element, ShapeKind},
    };

    pub fn list(
        mut repo: gix::Repository,
        mut progress: impl Progress,
        mut out: impl std::io::Write,
        super::Context {
            spec,
            format,
            text,
            limit,
            long_hashes,
        }: super::Context,
    ) -> anyhow::Result<()> {
        if format != OutputFormat::Human {
            bail!("Only human output is currently supported");
        }
        repo.object_cache_size_if_unset(4 * 1024 * 1024);
        repo.objects.refresh = RefreshMode::Never;

        let spec = gix::path::os_str_into_bstr(&spec)?;
        let id = repo
            .rev_parse_single(spec)
            .context("Only single revisions are currently supported")?;
        let commits = id
            .object()?
            .peel_to_kind(gix::object::Kind::Commit)
            .context("Need committish as starting point")?
            .id()
            .ancestors()
            .sorting(Sorting::ByCommitTime(Default::default()))
            .all()?;

        let mut vg = match text {
            Format::Svg { path } => (
                layout::topo::layout::VisualGraph::new(Orientation::TopToBottom),
                path,
                HashMap::default(),
            )
                .into(),
            Format::Text => None,
        };
        progress.init(None, gix::progress::count("commits"));
        progress.set_name("traverse".into());

        let start = std::time::Instant::now();
        for commit in commits {
            if gix::interrupt::is_triggered() {
                bail!("interrupted by user");
            }
            let commit = commit?;
            match vg.as_mut() {
                Some((vg, _path, map)) => {
                    let source = match map.get(&commit.id) {
                        Some(handle) => *handle,
                        None => {
                            let handle = vg.add_node(new_node(commit.id()));
                            map.insert(commit.id, handle);
                            handle
                        }
                    };

                    for parent_id in commit.parent_ids() {
                        let dest = match map.get(parent_id.as_ref()) {
                            Some(handle) => *handle,
                            None => {
                                let dest = vg.add_node(new_node(parent_id));
                                map.insert(parent_id.detach(), dest);
                                dest
                            }
                        };
                        let arrow = Arrow::simple("");
                        vg.add_edge(arrow, source, dest);
                    }
                }
                None => {
                    writeln!(
                        out,
                        "{} {} {}",
                        HexId::new(commit.id(), long_hashes),
                        commit.commit_time.expect("traversal with date"),
                        commit.parent_ids.len()
                    )?;
                }
            }
            progress.inc();
            if limit.is_some_and(|limit| limit == progress.step()) {
                break;
            }
        }

        progress.show_throughput(start);
        if let Some((mut vg, path, _)) = vg {
            let start = std::time::Instant::now();
            progress.set_name("layout graph".into());
            progress.info(format!("writing {}…", path.display()));
            let mut svg = SVGWriter::new();
            vg.do_it(false, false, false, &mut svg);
            std::fs::write(&path, svg.finalize().as_bytes())?;
            open::that(path)?;
            progress.show_throughput(start);
        }
        return Ok(());

        fn new_node(id: gix::Id<'_>) -> Element {
            let pt = Point::new(100., 30.);
            let name = id.shorten_or_id().to_string();
            let shape = ShapeKind::new_box(name.as_str());
            let style = StyleAttr::simple();
            Element::create(shape, style, Orientation::LeftToRight, pt)
        }
    }
}
