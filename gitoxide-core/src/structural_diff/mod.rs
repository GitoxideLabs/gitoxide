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
mod renames;
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
    /// Unchanged siblings on each side, per nesting depth. Repeat the last value; empty means one.
    pub context: Vec<usize>,
    /// Match unique similar additions and deletions as renames within each container.
    pub find_renames: bool,
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
    render(&before_items, &after_items, 0, &options, &mut body)?;
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

type Pair<'a> = (Option<&'a Item>, Option<&'a Item>);

fn align<'a>(before: &'a [Item], after: &'a [Item], find_renames: bool) -> Vec<Pair<'a>> {
    let mut input = InternedInput::default();
    input.update_before(before.iter().map(|item| (item.kind, item.name.as_str())));
    input.update_after(after.iter().map(|item| (item.kind, item.name.as_str())));
    let mut diff = Diff::compute(Algorithm::Histogram, &input);
    diff.postprocess_no_heuristic(&input);
    let mut pairs = Vec::new();
    let (mut left, mut right) = (0, 0);
    for hunk in diff.hunks() {
        while left < hunk.before.start as usize {
            pairs.push((Some(&before[left]), Some(&after[right])));
            left += 1;
            right += 1;
        }
        for item in &before[hunk.before.start as usize..hunk.before.end as usize] {
            pairs.push((Some(item), None));
        }
        for item in &after[hunk.after.start as usize..hunk.after.end as usize] {
            pairs.push((None, Some(item)));
        }
        left = hunk.before.end as usize;
        right = hunk.after.end as usize;
    }
    while left < before.len() {
        pairs.push((Some(&before[left]), Some(&after[right])));
        left += 1;
        right += 1;
    }
    if find_renames {
        let removed: Vec<_> = pairs
            .iter()
            .enumerate()
            .filter_map(|(index, pair)| match pair {
                (Some(item), None) => Some((index, *item)),
                _ => None,
            })
            .collect();
        let added: Vec<_> = pairs
            .iter()
            .enumerate()
            .filter_map(|(index, pair)| match pair {
                (None, Some(item)) => Some((index, *item)),
                _ => None,
            })
            .collect();
        for (left, right) in renames::candidates(
            &removed.iter().map(|(_, item)| *item).collect::<Vec<_>>(),
            &added.iter().map(|(_, item)| *item).collect::<Vec<_>>(),
        ) {
            pairs[removed[left].0] = (None, None);
            pairs[added[right].0].0 = Some(removed[left].1);
        }
        pairs.retain(|(before, after)| before.is_some() || after.is_some());
    }
    pairs
}

fn render(before: &[Item], after: &[Item], depth: usize, options: &Options, out: &mut impl Write) -> Result<()> {
    let pairs = align(before, after, options.find_renames);
    let context = options
        .context
        .get(depth)
        .or_else(|| options.context.last())
        .copied()
        .unwrap_or(1);
    let mut visible = vec![false; pairs.len()];
    for (index, pair) in pairs.iter().enumerate() {
        if !matches!(pair, (Some(before), Some(after)) if before.content == after.content) {
            let end = index.saturating_add(context).saturating_add(1).min(pairs.len());
            visible[index.saturating_sub(context)..end].fill(true);
        }
    }
    let mut skipping = false;
    for ((before, after), visible) in pairs.into_iter().zip(visible) {
        if !visible {
            if !skipping && let Some(item) = after.or(before) {
                omitted(item, options, out)?;
            }
            skipping = true;
            continue;
        }
        skipping = false;
        match (before, after) {
            (Some(before), Some(after)) => paired(before, after, depth, options, out)?,
            (Some(before), None) => entire(before, '-', options, out)?,
            (None, Some(after)) => entire(after, '+', options, out)?,
            (None, None) => unreachable!("each aligned entry has at least one side"),
        }
    }
    Ok(())
}

fn paired(before: &Item, after: &Item, depth: usize, options: &Options, out: &mut impl Write) -> Result<()> {
    if before.name != after.name {
        let indent: String = after.header.chars().take_while(|ch| matches!(ch, ' ' | '\t')).collect();
        lines(
            &format!("{indent}[renamed: {} -> {}]", before.name, after.name),
            '~',
            options,
            out,
        )?;
    }
    if before.header != after.header {
        lines(&before.header, '-', options, out)?;
        lines(&after.header, '+', options, out)?;
    } else {
        let changed = before.content != after.content;
        lines(&after.header, if changed { '~' } else { ' ' }, options, out)?;
    }
    if before.content == after.content {
        if let Some(child) = after.children.first() {
            omitted(child, options, out)?;
        }
    } else {
        render(&before.children, &after.children, depth + 1, options, out)?;
    }
    if before.footer != after.footer {
        lines(&before.footer, '-', options, out)?;
        lines(&after.footer, '+', options, out)?;
    } else {
        lines(&after.footer, ' ', options, out)?;
    }
    Ok(())
}

fn omitted(item: &Item, options: &Options, out: &mut impl Write) -> Result<()> {
    let indent: String = item.header.chars().take_while(|ch| matches!(ch, ' ' | '\t')).collect();
    lines(&format!("{indent}…"), ' ', options, out)
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
