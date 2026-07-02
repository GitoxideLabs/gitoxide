use anyhow::{Context, Result, bail};
use gix::{bstr::BString, config::AsKey};

use crate::OutputFormat;

pub fn list(
    repo: gix::Repository,
    filters: Vec<BString>,
    overrides: Vec<BString>,
    format: OutputFormat,
    mut out: impl std::io::Write,
) -> Result<()> {
    if format != OutputFormat::Human {
        bail!("Only human output format is supported at the moment");
    }
    let repo = gix::open_opts(repo.git_dir(), repo.open_options().clone().cli_overrides(overrides))?;
    let config = repo.config_snapshot();
    if let Some(frontmatter) = config.frontmatter() {
        for event in frontmatter {
            event.write_to(&mut out)?;
        }
    }
    let filters: Vec<_> = filters.into_iter().map(Filter::new).collect();
    let mut last_meta = None;
    let mut it = config.sections_and_postmatter().peekable();
    while let Some((section, matter)) = it.next() {
        if !filters.is_empty() && !filters.iter().any(|filter| filter.matches_section(section)) {
            continue;
        }

        let meta = section.meta();
        if last_meta != Some(meta) {
            write_meta(meta, &mut out)?;
        }
        last_meta = Some(meta);

        section.write_to(&mut out)?;
        for event in matter {
            event.write_to(&mut out)?;
        }
        if it
            .peek()
            .is_some_and(|(next_section, _)| next_section.header().name() != section.header().name())
        {
            writeln!(&mut out)?;
        }
    }
    Ok(())
}

/// Format the git configuration file at `in_file`, or the repository-local configuration if `in_file`
/// is `None`, writing the result back in place, to `out_file`, or to `out` (stdout) respectively.
pub fn fmt(
    repo: Option<gix::Repository>,
    in_file: Option<std::path::PathBuf>,
    out_file: Option<std::path::PathBuf>,
    in_place: bool,
    mut out: impl std::io::Write,
) -> Result<()> {
    if in_place && out_file.is_some() {
        bail!("Cannot combine --in-place with an explicit output file");
    }
    let source = match in_file {
        Some(path) => path,
        None => repo
            .context("Formatting the repository-local configuration requires being in a repository")?
            .common_dir()
            .join("config"),
    };
    let input = std::fs::read(&source)
        .with_context(|| format!("Could not read configuration file at '{}'", source.display()))?;
    let formatted = gix::config::parse::format::normalize(&input, &Default::default())?;
    let destination = if in_place { Some(source) } else { out_file };
    match destination {
        Some(path) => std::fs::write(&path, &formatted)
            .with_context(|| format!("Could not write formatted configuration to '{}'", path.display()))?,
        None => out.write_all(&formatted)?,
    }
    Ok(())
}

struct Filter {
    name: String,
    subsection: Option<BString>,
}

impl Filter {
    fn new(input: BString) -> Self {
        match (&input).try_as_key() {
            Some(key) => Filter {
                name: key.section_name.into(),
                subsection: key.subsection_name.map(ToOwned::to_owned),
            },
            None => Filter {
                name: input.to_string(),
                subsection: None,
            },
        }
    }

    fn matches_section(&self, section: &gix::config::file::Section<'_>) -> bool {
        let ignore_case = gix::glob::wildmatch::Mode::IGNORE_CASE;

        if !gix::glob::wildmatch(self.name.as_bytes().into(), section.header().name(), ignore_case) {
            return false;
        }
        match (self.subsection.as_deref(), section.header().subsection_name()) {
            (Some(filter), Some(name)) => {
                if !gix::glob::wildmatch(filter.as_slice().into(), name, ignore_case) {
                    return false;
                }
            }
            (None, _) => {}
            (Some(_), None) => return false,
        }
        true
    }
}

fn write_meta(meta: &gix::config::file::Metadata, out: &mut impl std::io::Write) -> std::io::Result<()> {
    writeln!(
        out,
        "# From '{}' ({:?}{}{})",
        meta.path
            .as_deref()
            .map_or_else(|| "memory".into(), |p| p.display().to_string()),
        meta.source,
        if meta.level != 0 {
            format!(", include level {}", meta.level)
        } else {
            Default::default()
        },
        if meta.trust != gix::sec::Trust::Full {
            ", untrusted"
        } else {
            Default::default()
        }
    )
}
