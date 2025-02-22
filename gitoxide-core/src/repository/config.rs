use anyhow::{bail, Result};
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
    let repo = gix::open_opts(
        repo.git_dir(),
        repo.open_options().clone().lossy_config(false).cli_overrides(overrides),
    )?;
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

struct Filter {
    name: String,
    subsection: Option<BString>,
}

impl Filter {
    fn new(input: BString) -> Self {
        match input.try_as_key() {
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
        (meta.level != 0)
            .then(|| format!(", include level {}", meta.level))
            .unwrap_or_default(),
        (meta.trust != gix::sec::Trust::Full)
            .then_some(", untrusted")
            .unwrap_or_default()
    )
}
