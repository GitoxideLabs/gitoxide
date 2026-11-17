//! Compare declarations while keeping implementation text out of the rendered diff.

use std::path::Path;

use gix::diff::blob::{Algorithm, Diff, InternedInput};
use gix::error::{Result, ResultExt, message};

mod parse;
mod renames;
mod words;
use parse::Item;

/// Languages with a structural outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// Rust declarations, including modules, implementations, traits, and fields.
    Rust,
    /// TOML tables and keys, with values hidden.
    Toml,
    /// Markdown heading hierarchies, with section content hidden.
    Markdown,
}

impl Language {
    /// Determine the language from the original file's extension.
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "rs" => Some(Self::Rust),
            "toml" => Some(Self::Toml),
            "md" | "markdown" => Some(Self::Markdown),
            _ => None,
        }
    }
}

/// How a declaration or word participates in a change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// Unchanged context.
    Context,
    /// An added declaration or word.
    Added,
    /// A removed declaration or word.
    Removed,
    /// A modified declaration, including changes to its hidden implementation.
    Modified,
}

impl Change {
    /// The patch-style prefix for this change.
    pub fn marker(self) -> char {
        match self {
            Self::Context => ' ',
            Self::Added => '+',
            Self::Removed => '-',
            Self::Modified => '~',
        }
    }
}

/// A visible fragment whose styling is left to the caller.
#[derive(Debug, Clone)]
pub struct Fragment {
    /// How this fragment changed.
    pub change: Change,
    /// Sanitized source text, with delimiters around changed words.
    pub text: String,
}

/// One visible structural diff row.
#[derive(Debug, Clone)]
pub struct Line {
    /// How the owning declaration changed.
    pub change: Change,
    /// Its visible source fragments, without terminal escape sequences.
    pub fragments: Vec<Fragment>,
}

/// Settings which can be changed without parsing the inputs again.
#[derive(Debug, Default, Clone)]
pub struct Options {
    /// Unchanged siblings on each side, per nesting depth; repeat the last value. Empty means one.
    pub context: Vec<usize>,
    /// Pair unique similar additions and removals as renamed declarations within each container.
    pub find_renames: bool,
    /// Highlight changed words within paired visible declarations.
    pub word_diff: bool,
    /// Exclude Rust test and benchmark declarations, including scopes disabled when `test` is false.
    pub production_only: bool,
}

/// Parsed outlines whose hidden implementation text participates only in comparison.
/// Each input is stored once, shared by the declarations referring to it.
pub struct PreparedDiff {
    before: Vec<Item>,
    after: Vec<Item>,
}

impl PreparedDiff {
    /// Parse both inputs using `language`. Malformed inputs return an error rather than exposing their bodies.
    pub fn new(before: &str, after: &str, language: Language) -> Result<Self> {
        let before = parse::parse(before, language).or_raise(|| message("Could not outline the old input"))?;
        let after = parse::parse(after, language).or_raise(|| message("Could not outline the new input"))?;
        Ok(Self { before, after })
    }

    /// Whether either input contains declarations included by the current settings.
    pub fn has_code(&self, options: &Options) -> bool {
        self.before
            .iter()
            .chain(&self.after)
            .any(|item| !options.production_only || !item.test_only)
    }

    /// Whether the included declarations or their hidden implementations differ.
    pub fn has_changes(&self, options: &Options) -> bool {
        !same_items(&self.before, &self.after, options)
    }

    /// Render the cached outlines with the current settings. Identical outlines emit no rows.
    pub fn render(&self, options: &Options) -> Vec<Line> {
        if !self.has_changes(options) {
            return Vec::new();
        }
        let mut lines = Vec::new();
        render(&self.before, &self.after, 0, options, &mut lines);
        lines
    }
}

fn same_items(before: &[Item], after: &[Item], options: &Options) -> bool {
    let mut before = before.iter().filter(|item| !options.production_only || !item.test_only);
    let mut after = after.iter().filter(|item| !options.production_only || !item.test_only);
    loop {
        match (before.next(), after.next()) {
            (None, None) => return true,
            (Some(before), Some(after)) if unchanged(before, after, options) => {}
            _ => return false,
        }
    }
}

fn unchanged(before: &Item, after: &Item, options: &Options) -> bool {
    if !options.production_only || !(before.contains_tests || after.contains_tests) {
        return before.content == after.content;
    }
    before.content.same_shell(&after.content) && same_items(&before.children, &after.children, options)
}

type Pair<'a> = (Option<&'a Item>, Option<&'a Item>);

fn align<'a>(before: &'a [Item], after: &'a [Item], options: &Options) -> Vec<Pair<'a>> {
    let before: Vec<_> = before
        .iter()
        .filter(|item| !options.production_only || !item.test_only)
        .collect();
    let after: Vec<_> = after
        .iter()
        .filter(|item| !options.production_only || !item.test_only)
        .collect();
    let mut input = InternedInput::default();
    input.update_before(before.iter().map(|item| (item.kind, item.name.as_str())));
    input.update_after(after.iter().map(|item| (item.kind, item.name.as_str())));
    let mut diff = Diff::compute(Algorithm::Histogram, &input);
    diff.postprocess_no_heuristic(&input);
    let mut pairs = Vec::new();
    let (mut left, mut right) = (0, 0);
    for hunk in diff.hunks() {
        while left < hunk.before.start as usize {
            pairs.push((Some(before[left]), Some(after[right])));
            left += 1;
            right += 1;
        }
        for item in &before[hunk.before.start as usize..hunk.before.end as usize] {
            pairs.push((Some(*item), None));
        }
        for item in &after[hunk.after.start as usize..hunk.after.end as usize] {
            pairs.push((None, Some(*item)));
        }
        left = hunk.before.end as usize;
        right = hunk.after.end as usize;
    }
    while left < before.len() {
        pairs.push((Some(before[left]), Some(after[right])));
        left += 1;
        right += 1;
    }
    if options.find_renames {
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

fn render(before: &[Item], after: &[Item], depth: usize, options: &Options, out: &mut Vec<Line>) {
    let pairs = align(before, after, options);
    let context = options
        .context
        .get(depth)
        .or_else(|| options.context.last())
        .copied()
        .unwrap_or(1);
    let mut visible = vec![false; pairs.len()];
    for (index, pair) in pairs.iter().enumerate() {
        if !matches!(pair, (Some(before), Some(after)) if unchanged(before, after, options)) {
            let end = index.saturating_add(context).saturating_add(1).min(pairs.len());
            visible[index.saturating_sub(context)..end].fill(true);
        }
    }
    let mut skipping = false;
    for ((before, after), visible) in pairs.into_iter().zip(visible) {
        if !visible {
            if !skipping && let Some(item) = after.or(before) {
                omitted(item, out);
            }
            skipping = true;
            continue;
        }
        skipping = false;
        match (before, after) {
            (Some(before), Some(after)) => paired(before, after, depth, options, out),
            (Some(before), None) => entire(before, Change::Removed, options, out),
            (None, Some(after)) => entire(after, Change::Added, options, out),
            (None, None) => unreachable!("each aligned entry has at least one side"),
        }
    }
}

fn paired(before: &Item, after: &Item, depth: usize, options: &Options, out: &mut Vec<Line>) {
    if before.name != after.name {
        let indent: String = after.header.chars().take_while(|ch| matches!(ch, ' ' | '\t')).collect();
        lines(
            &format!("{indent}[renamed: {} -> {}]", before.name, after.name),
            Change::Modified,
            out,
        );
    }
    if before.header != after.header {
        changed(&before.header, &after.header, options, out);
    } else {
        let change = if !unchanged(before, after, options) {
            Change::Modified
        } else {
            Change::Context
        };
        lines(&after.header, change, out);
    }
    if unchanged(before, after, options) {
        if let Some(child) = after
            .children
            .iter()
            .find(|item| !options.production_only || !item.test_only)
        {
            omitted(child, out);
        }
    } else {
        render(&before.children, &after.children, depth + 1, options, out);
    }
    if before.footer != after.footer {
        changed(&before.footer, &after.footer, options, out);
    } else {
        lines(&after.footer, Change::Context, out);
    }
}

fn changed(before: &str, after: &str, options: &Options, out: &mut Vec<Line>) {
    if options.word_diff {
        let [before, after] =
            [before, after].map(|text| text.lines().map(terminal_text).collect::<Vec<_>>().join("\n"));
        out.extend(words::diff(&before, &after));
    } else {
        lines(before, Change::Removed, out);
        lines(after, Change::Added, out);
    }
}

fn omitted(item: &Item, out: &mut Vec<Line>) {
    let indent: String = item.header.chars().take_while(|ch| matches!(ch, ' ' | '\t')).collect();
    lines(&format!("{indent}…"), Change::Context, out);
}

fn entire(item: &Item, change: Change, options: &Options, out: &mut Vec<Line>) {
    lines(&item.header, change, out);
    for child in item
        .children
        .iter()
        .filter(|item| !options.production_only || !item.test_only)
    {
        entire(child, change, options, out);
    }
    lines(&item.footer, change, out);
}

fn lines(text: &str, change: Change, out: &mut Vec<Line>) {
    out.extend(text.lines().map(|line| Line {
        change,
        fragments: vec![Fragment {
            change,
            text: terminal_text(line),
        }],
    }));
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

#[cfg(test)]
mod tests;
