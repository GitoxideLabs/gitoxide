//! Source-shaped structural diffs. Hidden implementation text participates in comparison only.

use std::{
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};

use gix::{
    Result,
    error::{ResultExt, bail, message},
};
use gix_imara_diff::{Algorithm, Diff, InternedInput};
use owo_colors::OwoColorize;

mod parse;
use parse::Item;

/// Languages with a structural outline.
#[derive(Debug, Clone, Copy)]
pub enum Language {
    Rust,
    Toml,
    Markdown,
}

impl FromStr for Language {
    type Err = gix::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "rust" | "rs" => Ok(Self::Rust),
            "toml" => Ok(Self::Toml),
            "markdown" | "md" => Ok(Self::Markdown),
            _ => bail!(message!("Unsupported structural diff language: {value}")),
        }
    }
}

/// Configure file identification and terminal rendering.
#[derive(Debug, Default)]
pub struct Options {
    /// Override language detection from the original path or input extensions.
    pub language: Option<Language>,
    /// Original Git path, when the inputs have temporary names.
    pub path: Option<PathBuf>,
    /// Emit ANSI colors with `owo-colors`.
    pub color: bool,
}

/// Compare two files without exposing implementation bodies. Differences are successful output.
pub fn diff(before: &Path, after: &Path, options: Options, mut out: impl Write) -> Result<()> {
    let language = options
        .language
        .or_else(|| {
            options
                .path
                .as_deref()
                .into_iter()
                .chain([after, before])
                .find_map(|path| match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
                    "rs" => Some(Language::Rust),
                    "toml" => Some(Language::Toml),
                    "md" | "markdown" => Some(Language::Markdown),
                    _ => None,
                })
        })
        .ok_or_else(|| {
            message("Cannot determine structural diff language; use --language rust, toml or markdown")
                .validation_error()
        })?;
    let before_text = read(before)?;
    let after_text = read(after)?;
    let before_items =
        parse::parse(&before_text, language).or_raise(|| message!("Could not outline {}", before.display()))?;
    let after_items =
        parse::parse(&after_text, language).or_raise(|| message!("Could not outline {}", after.display()))?;
    if before_items.len() == after_items.len()
        && before_items
            .iter()
            .zip(&after_items)
            .all(|(before, after)| before.content == after.content)
    {
        return Ok(());
    }
    let mut body = Vec::new();
    render(&before_items, &after_items, &options, &mut body)?;
    if body.is_empty() {
        return Ok(());
    }
    if let Some(path) = &options.path {
        writeln!(out, "--- a/{}", terminal_text(&path.to_string_lossy())).or_error()?;
        writeln!(out, "+++ b/{}", terminal_text(&path.to_string_lossy())).or_error()?;
    } else {
        writeln!(out, "--- {}", terminal_text(&before.to_string_lossy())).or_error()?;
        writeln!(out, "+++ {}", terminal_text(&after.to_string_lossy())).or_error()?;
    }
    out.write_all(&body).or_error()
}

fn read(path: &Path) -> Result<String> {
    if path == Path::new("/dev/null") {
        return Ok(String::new());
    }
    std::fs::read_to_string(path).or_raise(|| message!("Could not read {}", path.display()))
}

fn render(before: &[Item], after: &[Item], options: &Options, out: &mut impl Write) -> Result<()> {
    let mut input = InternedInput::default();
    input.update_before(before.iter().map(|item| (item.kind, item.name.as_str())));
    input.update_after(after.iter().map(|item| (item.kind, item.name.as_str())));
    let mut diff = Diff::compute(Algorithm::Histogram, &input);
    diff.postprocess_no_heuristic(&input);
    let (mut left, mut right) = (0, 0);
    for hunk in diff.hunks() {
        while left < hunk.before.start as usize {
            paired(&before[left], &after[right], options, out)?;
            left += 1;
            right += 1;
        }
        for item in &before[hunk.before.start as usize..hunk.before.end as usize] {
            entire(item, '-', options, out)?;
        }
        for item in &after[hunk.after.start as usize..hunk.after.end as usize] {
            entire(item, '+', options, out)?;
        }
        left = hunk.before.end as usize;
        right = hunk.after.end as usize;
    }
    while left < before.len() {
        paired(&before[left], &after[right], options, out)?;
        left += 1;
        right += 1;
    }
    Ok(())
}

fn paired(before: &Item, after: &Item, options: &Options, out: &mut impl Write) -> Result<()> {
    if before.header != after.header {
        lines(&before.header, '-', options, out)?;
        lines(&after.header, '+', options, out)?;
    } else {
        let changed = before.content != after.content;
        lines(&after.header, if changed { '~' } else { ' ' }, options, out)?;
    }
    render(&before.children, &after.children, options, out)?;
    if before.footer != after.footer {
        lines(&before.footer, '-', options, out)?;
        lines(&after.footer, '+', options, out)?;
    } else {
        lines(&after.footer, ' ', options, out)?;
    }
    Ok(())
}

fn entire(item: &Item, marker: char, options: &Options, out: &mut impl Write) -> Result<()> {
    lines(&item.header, marker, options, out)?;
    for child in &item.children {
        entire(child, marker, options, out)?;
    }
    lines(&item.footer, marker, options, out)
}

fn lines(text: &str, marker: char, options: &Options, out: &mut impl Write) -> Result<()> {
    for line in text.lines() {
        let line = format!("{marker}{}", terminal_text(line));
        if options.color {
            match marker {
                '+' => writeln!(out, "{}", line.green()),
                '-' => writeln!(out, "{}", line.red()),
                '~' => writeln!(out, "{}", line.yellow()),
                _ => writeln!(out, "{line}"),
            }
            .or_error()?;
        } else {
            writeln!(out, "{line}").or_error()?;
        }
    }
    Ok(())
}

fn terminal_text(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        if ch.is_control() && ch != '\t' {
            out.extend(ch.escape_default());
        } else {
            out.push(ch);
        }
    }
    out
}
