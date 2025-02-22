#[derive(Debug)]
pub struct Options {
    pub format: crate::OutputFormat,
    /// If true, also show attributes
    pub attributes: Option<Attributes>,
    pub statistics: bool,
    pub simple: bool,
    pub recurse_submodules: bool,
}

#[derive(Debug, Copy, Clone)]
pub enum Attributes {
    /// Look at worktree attributes and index as fallback.
    WorktreeAndIndex,
    /// Look at attributes from index files only.
    Index,
}

pub(crate) mod function {
    use std::{
        borrow::Cow,
        collections::BTreeSet,
        io::{BufWriter, Write},
    };

    use gix::index::entry::Stage;
    use gix::{
        bstr::{BStr, BString},
        worktree::IndexPersistedOrInMemory,
        Repository,
    };

    use crate::{
        is_dir_to_mode,
        repository::index::entries::{Attributes, Options},
        OutputFormat,
    };

    pub fn entries(
        repo: gix::Repository,
        pathspecs: Vec<BString>,
        out: impl std::io::Write,
        mut err: impl std::io::Write,
        Options {
            simple,
            format,
            attributes,
            statistics,
            recurse_submodules,
        }: Options,
    ) -> anyhow::Result<()> {
        let mut out = BufWriter::with_capacity(64 * 1024, out);
        let mut all_attrs = statistics.then(BTreeSet::new);

        #[cfg(feature = "serde")]
        if let OutputFormat::Json = format {
            out.write_all(b"[\n")?;
        }

        let stats = print_entries(
            &repo,
            attributes,
            pathspecs.iter(),
            format,
            all_attrs.as_mut(),
            simple,
            "".into(),
            recurse_submodules,
            &mut out,
        )?;

        #[cfg(feature = "serde")]
        if format == OutputFormat::Json {
            out.write_all(b"]\n")?;
            out.flush()?;
            if statistics {
                serde_json::to_writer_pretty(&mut err, &stats)?;
            }
        }
        if format == OutputFormat::Human && statistics {
            out.flush()?;
            writeln!(err, "{stats:#?}")?;
            if let Some(attrs) = all_attrs.filter(|a| !a.is_empty()) {
                writeln!(err, "All encountered attributes:")?;
                for attr in attrs {
                    writeln!(err, "\t{attr}", attr = attr.as_ref())?;
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn print_entries(
        repo: &Repository,
        attributes: Option<Attributes>,
        pathspecs: impl IntoIterator<Item = impl AsRef<BStr>> + Clone,
        format: OutputFormat,
        mut all_attrs: Option<&mut BTreeSet<gix::attrs::Assignment>>,
        simple: bool,
        prefix: &BStr,
        recurse_submodules: bool,
        out: &mut impl std::io::Write,
    ) -> anyhow::Result<Statistics> {
        let _span = gix::trace::coarse!("print_entries()", git_dir = ?repo.git_dir());
        let (mut pathspec, index, mut cache) = init_cache(repo, attributes, pathspecs.clone())?;
        let mut repo_attrs = all_attrs.is_some().then(BTreeSet::default);
        let submodules_by_path = recurse_submodules
            .then(|| {
                repo.submodules()
                    .map(|opt| {
                        opt.map(|submodules| {
                            submodules
                                .map(|sm| sm.path().map(Cow::into_owned).map(move |path| (path, sm)))
                                .collect::<Result<Vec<_>, _>>()
                        })
                    })
                    .transpose()
            })
            .flatten()
            .transpose()?
            .transpose()?;
        let mut stats = Statistics {
            entries: index.entries().len(),
            ..Default::default()
        };
        if let Some(entries) = index.prefixed_entries(pathspec.common_prefix()) {
            stats.entries_after_prune = entries.len();
            let mut entries = entries.iter().peekable();
            while let Some(entry) = entries.next() {
                let mut last_match = None;
                let attrs = cache
                    .as_mut()
                    .and_then(|(attrs, cache)| {
                        // If the user wants to see assigned attributes, we always have to match.
                        attributes.is_some().then(|| {
                            cache.at_entry(entry.path(&index), None).map(|entry| {
                                let is_excluded = entry.is_excluded();
                                stats.excluded += usize::from(is_excluded);
                                let attributes: Vec<_> = {
                                    last_match = Some(entry.matching_attributes(attrs));
                                    attrs.iter().map(|m| m.assignment.to_owned()).collect()
                                };
                                stats.with_attributes += usize::from(!attributes.is_empty());
                                stats.max_attributes_per_path = stats.max_attributes_per_path.max(attributes.len());
                                if let Some(attrs) = repo_attrs.as_mut() {
                                    attributes.iter().for_each(|attr| {
                                        attrs.insert(attr.clone());
                                    });
                                }
                                Attrs {
                                    is_excluded,
                                    attributes,
                                }
                            })
                        })
                    })
                    .transpose()?;

                // Note that we intentionally ignore `_case` so that we act like git does, attribute matching case is determined
                // by the repository, not the pathspec.
                let entry_is_excluded = pathspec
                    .pattern_matching_relative_path(
                        entry.path(&index),
                        Some(false),
                        &mut |rela_path, _case, is_dir, out| {
                            cache
                                .as_mut()
                                .map(|(attrs, cache)| {
                                    match last_match {
                                        // The user wants the attributes for display, so the match happened already.
                                        Some(matched) => {
                                            attrs.copy_into(cache.attributes_collection(), out);
                                            matched
                                        }
                                        // The user doesn't want attributes, so we set the cache position on demand only
                                        None => cache
                                            .at_entry(rela_path, Some(is_dir_to_mode(is_dir)))
                                            .ok()
                                            .map(|platform| platform.matching_attributes(out))
                                            .unwrap_or_default(),
                                    }
                                })
                                .unwrap_or_default()
                        },
                    )
                    .is_none_or(|m| m.is_excluded());

                let entry_is_submodule = entry.mode.is_submodule();
                if entry_is_excluded && (!entry_is_submodule || !recurse_submodules) {
                    continue;
                }
                if let Some(sm) = submodules_by_path
                    .as_ref()
                    .filter(|_| entry_is_submodule)
                    .and_then(|sms_by_path| {
                        let entry_path = entry.path(&index);
                        sms_by_path
                            .iter()
                            .find_map(|(path, sm)| (path == entry_path).then_some(sm))
                            .filter(|sm| sm.git_dir_try_old_form().is_ok_and(|dot_git| dot_git.exists()))
                    })
                {
                    let sm_path = gix::path::to_unix_separators_on_windows(sm.path()?);
                    let sm_repo = sm.open()?.expect("we checked it exists");
                    let mut prefix = prefix.to_owned();
                    prefix.extend_from_slice(sm_path.as_ref());
                    if !sm_path.ends_with(b"/") {
                        prefix.push(b'/');
                    }
                    let sm_stats = print_entries(
                        &sm_repo,
                        attributes,
                        pathspecs.clone(),
                        format,
                        all_attrs.as_deref_mut(),
                        simple,
                        prefix.as_ref(),
                        recurse_submodules,
                        out,
                    )?;
                    stats.submodule.push((sm_path.into_owned(), sm_stats));
                } else {
                    match format {
                        OutputFormat::Human => {
                            if simple {
                                to_human_simple(out, &index, entry, attrs, prefix)
                            } else {
                                to_human(out, &index, entry, attrs, prefix)
                            }?;
                        }
                        #[cfg(feature = "serde")]
                        OutputFormat::Json => to_json(out, &index, entry, attrs, entries.peek().is_none(), prefix)?,
                    }
                }
            }
        }

        stats.cache = cache.map(|c| *c.1.statistics());
        if let Some((attrs, all_attrs)) = repo_attrs.zip(all_attrs) {
            stats
                .attributes
                .extend(attrs.iter().map(|attr| attr.as_ref().to_string()));
            all_attrs.extend(attrs);
        }
        Ok(stats)
    }

    #[allow(clippy::type_complexity)]
    fn init_cache(
        repo: &Repository,
        attributes: Option<Attributes>,
        pathspecs: impl IntoIterator<Item = impl AsRef<BStr>>,
    ) -> anyhow::Result<(
        gix::pathspec::Search,
        IndexPersistedOrInMemory,
        Option<(gix::attrs::search::Outcome, gix::AttributeStack<'_>)>,
    )> {
        let index = repo.index_or_load_from_head()?;
        let pathspec = repo.pathspec(
            true,
            pathspecs,
            false,
            &index,
            gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping.adjust_for_bare(repo.is_bare()),
        )?;
        let cache = attributes
            .or_else(|| {
                pathspec
                    .search()
                    .patterns()
                    .any(|spec| !spec.attributes.is_empty())
                    .then_some(Attributes::Index)
            })
            .map(|attrs| {
                repo.attributes(
                    &index,
                    match attrs {
                        Attributes::WorktreeAndIndex => {
                            gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping
                                .adjust_for_bare(repo.is_bare())
                        }
                        Attributes::Index => gix::worktree::stack::state::attributes::Source::IdMapping,
                    },
                    match attrs {
                        Attributes::WorktreeAndIndex => {
                            gix::worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped
                                .adjust_for_bare(repo.is_bare())
                        }
                        Attributes::Index => gix::worktree::stack::state::ignore::Source::IdMapping,
                    },
                    None,
                )
                .map(|cache| (cache.attribute_matches(), cache))
            })
            .transpose()?;
        Ok((pathspec.into_parts().0, index, cache))
    }

    #[cfg_attr(feature = "serde", derive(serde::Serialize))]
    struct Attrs {
        is_excluded: bool,
        attributes: Vec<gix::attrs::Assignment>,
    }

    #[cfg_attr(feature = "serde", derive(serde::Serialize))]
    #[derive(Default, Debug)]
    struct Statistics {
        #[allow(dead_code)] // Not really dead, but Debug doesn't count for it even though it's crucial.
        pub entries: usize,
        pub entries_after_prune: usize,
        pub excluded: usize,
        pub with_attributes: usize,
        pub max_attributes_per_path: usize,
        pub cache: Option<gix::worktree::stack::Statistics>,
        pub attributes: Vec<String>,
        pub submodule: Vec<(BString, Statistics)>,
    }

    #[cfg(feature = "serde")]
    fn to_json(
        out: &mut impl std::io::Write,
        index: &gix::index::File,
        entry: &gix::index::Entry,
        attrs: Option<Attrs>,
        is_last: bool,
        prefix: &BStr,
    ) -> anyhow::Result<()> {
        use gix::bstr::ByteSlice;
        #[derive(serde::Serialize)]
        struct Entry<'a> {
            stat: &'a gix::index::entry::Stat,
            hex_id: String,
            flags: u32,
            mode: u32,
            path: std::borrow::Cow<'a, str>,
            meta: Option<Attrs>,
        }

        serde_json::to_writer(
            &mut *out,
            &Entry {
                stat: &entry.stat,
                hex_id: entry.id.to_hex().to_string(),
                flags: entry.flags.bits(),
                mode: entry.mode.bits(),
                path: if prefix.is_empty() {
                    entry.path(index).to_str_lossy()
                } else {
                    let mut path = prefix.to_owned();
                    path.extend_from_slice(entry.path(index));
                    path.to_string().into()
                },
                meta: attrs,
            },
        )?;

        if is_last {
            out.write_all(b"\n")?;
        } else {
            out.write_all(b",\n")?;
        }
        Ok(())
    }

    fn to_human_simple(
        out: &mut impl std::io::Write,
        file: &gix::index::File,
        entry: &gix::index::Entry,
        attrs: Option<Attrs>,
        prefix: &BStr,
    ) -> std::io::Result<()> {
        if !prefix.is_empty() {
            out.write_all(prefix)?;
        }
        match attrs {
            Some(attrs) => {
                out.write_all(entry.path(file))?;
                out.write_all(print_attrs(Some(attrs), entry.mode).as_bytes())
            }
            None => out.write_all(entry.path(file)),
        }?;
        out.write_all(b"\n")
    }

    fn to_human(
        out: &mut impl std::io::Write,
        file: &gix::index::File,
        entry: &gix::index::Entry,
        attrs: Option<Attrs>,
        prefix: &BStr,
    ) -> std::io::Result<()> {
        writeln!(
            out,
            "{} {}{:?} {} {}{}{}",
            match entry.flags.stage() {
                Stage::Unconflicted => "       ",
                Stage::Base => "BASE   ",
                Stage::Ours => "OURS   ",
                Stage::Theirs => "THEIRS ",
            },
            if entry.flags.is_empty() {
                "".to_string()
            } else {
                format!("{:?} ", entry.flags)
            },
            entry.mode,
            entry.id,
            prefix,
            entry.path(file),
            print_attrs(attrs, entry.mode)
        )
    }

    fn print_attrs(attrs: Option<Attrs>, mode: gix::index::entry::Mode) -> Cow<'static, str> {
        attrs.map_or(Cow::Borrowed(""), |a| {
            let mut buf = String::new();
            if mode.is_sparse() {
                buf.push_str(" 📁 ");
            } else if mode.is_submodule() {
                buf.push_str(" ➡ ");
            }
            if a.is_excluded {
                buf.push_str(" 🗑️");
            }
            if !a.attributes.is_empty() {
                buf.push_str(" (");
                for assignment in a.attributes {
                    use std::fmt::Write;
                    write!(&mut buf, "{}", assignment.as_ref()).ok();
                    buf.push_str(", ");
                }
                buf.pop();
                buf.pop();
                buf.push(')');
            }
            buf.into()
        })
    }
}
