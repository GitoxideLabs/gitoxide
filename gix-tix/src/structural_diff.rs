use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    path::Path,
};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};
use gix::{
    Result,
    bstr::{BStr, BString, ByteSlice},
    error::{OptionExt, ResultExt, bail, message},
};
use ratatui::{
    Frame,
    layout::{Position, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::{
    FileChange, OBJECT_CACHE_SIZE,
    app::{ChangeKind, Changes, PathChange, TreeDiffTarget},
    command_menu::CommandGroup,
    diff_colors::Palette,
    graph_tiles::Tile,
    ui,
};

pub(crate) mod outline;

use outline::{Change, Language, Options, PreparedDiff, RenderOptions};

pub(crate) struct Diff {
    title: BString,
    files: Vec<File>,
}

struct File {
    path: PathChange,
    headers: Vec<String>,
    language: Option<Language>,
    metadata_changed: bool,
    outline: std::result::Result<PreparedDiff, String>,
}

pub(crate) fn prepare_file(repository_path: &Path, bare: bool, change: &FileChange, path: &PathChange) -> Result<Diff> {
    let mut repository = crate::open_repository(repository_path, bare, false)?;
    repository.object_cache_size(OBJECT_CACHE_SIZE);
    let mut path = path.clone();
    if path.lines.is_none() && !matches!(change, FileChange::Unavailable(_)) {
        let (mut tree, mut worktree) = crate::line_diff_state(&repository)?;
        path.lines = crate::line_counts_for_change(&repository, change, &mut tree, worktree.as_mut())?;
    }
    Ok(Diff {
        title: format!(
            "Structural diff · {} {}",
            path.kind.letter(),
            quoted(path.path.as_bstr(), "")
        )
        .into(),
        files: vec![File::prepare(&repository, change, &path)],
    })
}

pub(crate) fn prepare_commit(
    repository_path: &Path,
    bare: bool,
    target: TreeDiffTarget,
    title: BString,
) -> Result<Diff> {
    let mut repository = crate::open_repository(repository_path, bare, false)?;
    repository.object_cache_size(OBJECT_CACHE_SIZE);
    prepare_commit_with_repository(&repository, target, title)
}

fn prepare_commit_with_repository(
    repository: &gix::Repository,
    target: TreeDiffTarget,
    title: BString,
) -> Result<Diff> {
    let mut changes = crate::load_changes_with_diff_options(
        repository,
        target,
        Some(gix::diff::Options::default().with_rewrites(Some(Default::default()))),
    )?;
    crate::add_line_counts(repository, &mut changes)?;
    Ok(prepare_changes(repository, &changes, title))
}

pub(crate) fn prepare_changes(repository: &gix::Repository, changes: &Changes, title: BString) -> Diff {
    Diff {
        title: format!("Structural diff · {}", title.to_str_lossy()).into(),
        files: changes
            .diffs
            .iter()
            .zip(&changes.paths)
            .map(|(change, path)| File::prepare(repository, change, path))
            .collect(),
    }
}

impl File {
    fn prepare(repository: &gix::Repository, change: &FileChange, path: &PathChange) -> Self {
        let language = [
            Some(path.path.as_bstr()),
            path.source.as_ref().map(|source| source.as_bstr()),
        ]
        .into_iter()
        .flatten()
        .find_map(|path| {
            gix::path::from_bstr(path)
                .ok()
                .and_then(|path| Language::from_path(&path))
        });
        let outline = (|| {
            let language = language
                .ok_or_raise(|| message("Unsupported structural language; supported: Rust, Python, TOML, Markdown"))?;
            let resources = crate::prepare_file_diff_resources(repository, change)?;
            let (old, new) = resources
                .resources()
                .ok_or_raise(|| message("The file's diff resources are unavailable"))?;
            use gix::diff::blob::platform::resource::Data;
            if old.data == Data::Missing && new.data == Data::Missing {
                bail!(message("Neither side of this file exists; refresh the changes"));
            }
            let input = |data| -> Result<&str> {
                match data {
                    Data::Missing => Ok(""),
                    Data::Buffer { buf, .. } => {
                        std::str::from_utf8(buf).or_raise(|| message("Structural inputs must be UTF-8"))
                    }
                    Data::Binary { .. } => bail!(message("Binary or oversized contents have no structural outline")),
                }
            };
            PreparedDiff::new(input(old.data)?, input(new.data)?, language)
        })()
        .map_err(|err: gix::Error| format!("{err:#}"));
        let (_, _, old_mode, new_mode) = crate::diff_sides(path, change);
        let mut headers = file_headers(path, change);
        if let Some(group) = path.group.worktree_label() {
            headers[0].push_str(" · ");
            headers[0].push_str(group);
        }
        Self {
            path: path.clone(),
            headers,
            language,
            metadata_changed: old_mode != new_mode || path.source.is_some(),
            outline,
        }
    }

    fn visible(&self, options: &Options) -> bool {
        if !options.production_only {
            return true;
        }
        if non_production_path(self.path.path.as_bstr())
            && (self.path.kind == ChangeKind::Copied
                || self
                    .path
                    .source
                    .as_ref()
                    .is_none_or(|source| non_production_path(source.as_bstr())))
        {
            return false;
        }
        match &self.outline {
            Ok(outline) => outline.has_code(options) && (self.metadata_changed || outline.has_changes(options)),
            Err(_) => true,
        }
    }
}

// shortcut: custom Cargo target paths are unclassified; read target metadata if those layouts need filtering.
fn non_production_path(path: &BStr) -> bool {
    if path.split(|byte| *byte == b'/').any(|part| {
        matches!(
            part,
            b"test"
                | b"tests"
                | b"bench"
                | b"benches"
                | b"benchmark"
                | b"benchmarks"
                | b"example"
                | b"examples"
                | b"fixture"
                | b"fixtures"
        )
    }) {
        return true;
    }
    let name = path.rsplit(|byte| *byte == b'/').next().unwrap_or_default();
    if let Some(stem) = name.strip_suffix(b".py") {
        return stem == b"conftest" || stem.starts_with(b"test_") || stem.ends_with(b"_test");
    }
    let stem = name.strip_suffix(b".rs").unwrap_or_default();
    matches!(
        stem,
        b"test"
            | b"tests"
            | b"bench"
            | b"benches"
            | b"benchmark"
            | b"benchmarks"
            | b"example"
            | b"examples"
            | b"fixture"
            | b"fixtures"
    ) || stem.starts_with(b"test_")
        || stem.ends_with(b"_test")
        || stem.ends_with(b"_tests")
}

fn file_headers(path: &PathChange, change: &FileChange) -> Vec<String> {
    let (old, new, old_mode, new_mode) = crate::diff_sides(path, change);
    let display = old.or(new).unwrap_or(path.path.as_bstr());
    let mut headers = vec![format!(
        "diff --git {} {}",
        quoted(old.unwrap_or(display), "a/"),
        quoted(new.unwrap_or(display), "b/")
    )];
    if let Some(source) = &path.source {
        let operation = if path.kind == ChangeKind::Copied {
            "copy"
        } else {
            "rename"
        };
        headers.push(format!("{operation} from {}", quoted(source.as_bstr(), "")));
        headers.push(format!("{operation} to {}", quoted(path.path.as_bstr(), "")));
    }
    if old_mode != new_mode {
        if let Some(mode) = old_mode {
            headers.push(format!(
                "{} {}",
                if new.is_none() { "deleted file mode" } else { "old mode" },
                mode.kind().as_octal_str()
            ));
        }
        if let Some(mode) = new_mode {
            headers.push(format!(
                "{} {}",
                if old.is_none() { "new file mode" } else { "new mode" },
                mode.kind().as_octal_str()
            ));
        }
    }
    headers.push(format!(
        "--- {}",
        old.map_or_else(|| "/dev/null".into(), |path| quoted(path, "a/"))
    ));
    headers.push(format!(
        "+++ {}",
        new.map_or_else(|| "/dev/null".into(), |path| quoted(path, "b/"))
    ));
    headers
}

fn quoted(path: &BStr, prefix: &str) -> String {
    let mut path_with_prefix = BString::from(prefix);
    path_with_prefix.extend_from_slice(path);
    gix::quote::ansi_c::quote(path_with_prefix.as_bstr())
        .to_str_lossy()
        .into_owned()
}

#[derive(Default)]
struct FileTree {
    directories: BTreeMap<BString, FileTree>,
    files: BTreeSet<(BString, usize)>,
    added: u64,
    removed: u64,
    without_counts: usize,
}

struct DirectoryStat {
    row: usize,
    columns: Range<usize>,
    counts: Line<'static>,
}

fn line_count_spans(added: u64, removed: u64, colors: &Palette) -> [Span<'static>; 2] {
    [
        Span::styled(format!("  +{added}"), colors.path(ChangeKind::Added)),
        Span::styled(format!(" -{removed}"), colors.path(ChangeKind::Deleted)),
    ]
}

impl FileTree {
    fn add_line_counts(&mut self, counts: Option<(u32, u32)>) {
        if let Some((added, removed)) = counts {
            self.added += u64::from(added);
            self.removed += u64::from(removed);
        } else {
            self.without_counts += 1;
        }
    }

    fn append_line_counts(&self, spans: &mut Vec<Span<'static>>, colors: &Palette) {
        spans.extend(line_count_spans(self.added, self.removed, colors));
        if self.without_counts != 0 {
            spans.push(Span::raw(format!(" · {} without line counts", self.without_counts)));
        }
    }

    fn render(
        &self,
        files: &[File],
        (prefix, compact): (&str, bool),
        lines: &mut Vec<Line<'static>>,
        file_rows: &mut [Option<usize>],
        directories: &mut Vec<DirectoryStat>,
        colors: &Palette,
    ) {
        let branch = |last| match (compact, last) {
            (true, false) => "├",
            (true, true) => "└",
            (false, false) => "├── ",
            (false, true) => "└── ",
        };
        let count = self.directories.len() + self.files.len();
        for (index, (name, tree)) in self.directories.iter().enumerate() {
            let last = index + 1 == count;
            let name = quoted(name.as_bstr(), "");
            let indent = format!("{prefix}{}", branch(last));
            let start = Span::raw(indent.as_str()).width();
            let mut counts = Vec::new();
            tree.append_line_counts(&mut counts, colors);
            directories.push(DirectoryStat {
                row: lines.len(),
                columns: start..start + Span::raw(name.as_str()).width() + 1,
                counts: Line::from(counts),
            });
            lines.push(Line::from(vec![
                Span::styled(indent, colors.metadata),
                Span::styled(format!("{name}/"), colors.metadata),
            ]));
            let continuation = match (compact, last) {
                (true, false) => "│",
                (true, true) => " ",
                (false, false) => "│   ",
                (false, true) => "    ",
            };
            tree.render(
                files,
                (&format!("{prefix}{continuation}"), compact),
                lines,
                file_rows,
                directories,
                colors,
            );
        }
        for (index, (name, file)) in self.files.iter().enumerate() {
            let path = &files[*file].path;
            let last = self.directories.len() + index + 1 == count;
            let mut spans = vec![
                Span::raw(format!("{prefix}{}", branch(last))),
                Span::styled(format!("{} ", path.kind.letter()), colors.path(path.kind)),
                Span::raw(quoted(name.as_bstr(), "")),
            ];
            if path.path.ends_with(b"/") {
                spans.push(Span::raw("/"));
            }
            if let Some(source) = &path.source {
                spans.push(Span::styled(
                    format!(" ← {}", quoted(source.as_bstr(), "")),
                    colors.metadata,
                ));
            }
            if let Some(group) = path.group.worktree_label() {
                spans.push(Span::styled(format!(" · {group}"), colors.metadata));
            }
            file_rows[*file] = Some(lines.len());
            lines.push(Line::from(spans));
        }
    }
}

type DiffRows = Vec<(Line<'static>, Option<Language>)>;

impl Diff {
    fn render(
        &self,
        options: &Options,
        colors: &Palette,
        compact: bool,
    ) -> (DiffRows, Vec<FileLink>, Vec<DirectoryStat>) {
        let format = RenderOptions {
            omit_word_delimiters: colors.colored,
            omit_unchanged_parameters: true,
        };
        let visible: Vec<_> = self
            .files
            .iter()
            .enumerate()
            .filter(|(_, file)| file.visible(options))
            .filter_map(|(index, file)| {
                let outline = file
                    .outline
                    .as_ref()
                    .map(|outline| outline.render_with(options, format));
                if options.hide_modifications {
                    match file.path.kind {
                        ChangeKind::Added | ChangeKind::Deleted | ChangeKind::Renamed => {}
                        ChangeKind::Modified if outline.as_ref().is_ok_and(|lines| !lines.is_empty()) => {}
                        _ => return None,
                    }
                }
                Some((index, outline))
            })
            .collect();
        let mut tree = FileTree::default();
        for &(index, _) in &visible {
            let file = &self.files[index];
            tree.add_line_counts(file.path.lines);
            let components = file
                .path
                .path
                .split(|byte| *byte == b'/')
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>();
            let Some((name, directories)) = components.split_last() else {
                continue;
            };
            let mut node = &mut tree;
            for directory in directories {
                node = node.directories.entry(BString::from(*directory)).or_default();
                node.add_line_counts(file.path.lines);
            }
            node.files.insert((BString::from(*name), index));
        }
        let mut heading = vec![Span::styled(
            format!("Changed files ({})", visible.len()),
            colors.header,
        )];
        tree.append_line_counts(&mut heading, colors);
        let mut lines = vec![Line::from(heading)];
        if visible.is_empty() {
            lines.push(Line::from(if options.hide_modifications {
                "No additions, deletions, or renames."
            } else if options.production_only {
                "No production file changes."
            } else {
                "No file changes."
            }));
        }
        let mut file_rows = vec![None; self.files.len()];
        let mut directories = Vec::new();
        tree.render(
            &self.files,
            ("", compact),
            &mut lines,
            &mut file_rows,
            &mut directories,
            colors,
        );
        let mut rows = lines.into_iter().map(|line| (line, None)).collect::<Vec<_>>();
        let mut file_links = Vec::with_capacity(visible.len());
        for (index, outline) in visible {
            let file = &self.files[index];
            rows.push((Line::default(), None));
            if let Some(row) = file_rows[index] {
                let target = rows.len();
                let line = &mut rows[row].0;
                file_links.push(FileLink {
                    row,
                    columns: line.spans.iter().take(2).map(Span::width).sum::<usize>()..line.width(),
                    label_spans: line.spans.len(),
                    target,
                });
                if let Some((added, removed)) = file.path.lines {
                    line.spans
                        .extend(line_count_spans(added.into(), removed.into(), colors));
                } else {
                    line.spans.push(Span::styled("  n/a", colors.metadata));
                }
            }
            rows.extend(file.headers.iter().map(|header| {
                (
                    Line::styled(
                        header.clone(),
                        if header.starts_with("diff --git") {
                            crate::ui::DIFF_FILE_HEADER
                        } else {
                            colors.metadata
                        },
                    ),
                    None,
                )
            }));
            match outline {
                Ok(rendered) => {
                    if rendered.is_empty() {
                        rows.push((
                            Line::styled("No structural changes.", Style::default().add_modifier(Modifier::DIM)),
                            None,
                        ));
                    }
                    rows.extend(
                        rendered
                            .into_iter()
                            .map(|line| (structural_line(line, colors), file.language)),
                    );
                }
                Err(reason) => rows.push((
                    Line::styled(format!("Outline unavailable: {reason}"), colors.warning),
                    None,
                )),
            }
        }
        file_links.sort_unstable_by_key(|link| link.row);
        (rows, file_links, directories)
    }
}

fn structural_line(line: outline::Line, colors: &Palette) -> Line<'static> {
    let mut spans = Vec::with_capacity(line.fragments.len() + usize::from(!colors.colored));
    if !colors.colored {
        spans.push(Span::raw(line.change.marker().to_string()));
    }
    let mut column = 0;
    for fragment in line.fragments {
        let mut text = String::new();
        for (index, part) in fragment.text.split('\t').enumerate() {
            if index != 0 {
                let spaces = 4 - column % 4;
                text.extend(std::iter::repeat_n(' ', spaces));
                column += spaces;
            }
            text.push_str(part);
            column += Span::raw(part).width();
        }
        let style = if line.change == Change::Modified && matches!(fragment.change, Change::Added | Change::Removed) {
            colors.emphasis(fragment.change)
        } else {
            Style::default()
        };
        spans.push(Span::styled(text, style));
    }
    Line::from(spans).style(colors.row(line.change))
}

struct FileLink {
    row: usize,
    columns: Range<usize>,
    label_spans: usize,
    target: usize,
}

#[derive(Debug)]
pub(crate) struct Settings {
    pub(crate) options: Options,
    pub(crate) colored: bool,
}

#[derive(Clone, Copy)]
enum ViewOption {
    Color,
    Context,
    Production,
    Modifications,
    Renames,
    Words,
}

const VIEW_OPTIONS: [(char, ViewOption); 6] = [
    ('c', ViewOption::Color),
    ('x', ViewOption::Context),
    ('p', ViewOption::Production),
    ('m', ViewOption::Modifications),
    ('r', ViewOption::Renames),
    ('w', ViewOption::Words),
];

impl Default for Settings {
    fn default() -> Self {
        Self {
            options: Options::default(),
            colored: true,
        }
    }
}

pub(crate) struct View<'a> {
    diff: &'a Diff,
    settings: &'a mut Settings,
    lines: DiffRows,
    max_width: usize,
    tree_width: usize,
    file_headers: Vec<(usize, usize)>,
    file_links: Vec<FileLink>,
    directories: Vec<DirectoryStat>,
    pointer: Option<Position>,
    graphics: bool,
    tree_rows: Vec<(usize, usize)>,
    tree_visible: bool,
    tree_scroll: Option<(usize, usize)>,
    prefix: Option<CommandGroup>,
    theme: &'a Palette,
}

impl<'a> View<'a> {
    pub(crate) fn new(diff: &'a Diff, settings: &'a mut Settings, theme: &'a Palette, graphics: bool) -> Self {
        let colors = if settings.colored { theme } else { &Palette::PLAIN };
        let (lines, file_links, directories) = diff.render(&settings.options, colors, graphics);
        let file_headers = file_header_rows(&lines);
        let (tree_width, max_width) = measure_widths(&lines, &file_headers, &directories);
        let tree_rows = file_tree_rows(&file_links);
        Self {
            diff,
            settings,
            lines,
            max_width,
            tree_width,
            file_headers,
            file_links,
            directories,
            pointer: None,
            graphics,
            tree_rows,
            tree_visible: false,
            tree_scroll: None,
            prefix: None,
            theme,
        }
    }

    pub(crate) fn colored(&self) -> bool {
        self.settings.colored
    }

    fn colors(&self) -> &Palette {
        if self.colored() { self.theme } else { &Palette::PLAIN }
    }

    pub(crate) fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub(crate) fn max_width(&self) -> usize {
        self.max_width
    }

    pub(crate) fn sticky_header_index(&self, offset: usize) -> Option<usize> {
        crate::ui::sticky_diff_header(&self.file_headers, offset)
    }

    pub(crate) fn last_file_header(&self) -> usize {
        self.file_headers.last().map_or(0, |&(start, _)| start)
    }

    pub(crate) fn file_target(&self, row: usize, column: usize) -> Option<usize> {
        let index = self.file_links.binary_search_by_key(&row, |link| link.row).ok()?;
        let link = &self.file_links[index];
        link.columns.contains(&column).then_some(link.target)
    }

    pub(crate) fn areas(&self, area: Rect) -> [Rect; 2] {
        let width = u16::try_from(self.tree_width.saturating_add(1))
            .unwrap_or(u16::MAX)
            .min(area.width.saturating_sub(1));
        if !self.tree_visible || width < 2 {
            return [Rect::default(), area];
        }
        [
            Rect::new(area.x, area.y, width, area.height),
            Rect::new(area.x + width, area.y, area.width - width, area.height),
        ]
    }

    fn tree_line_count(&self) -> usize {
        tree_line_count(&self.lines, &self.file_headers)
    }

    fn visible_files(&self, area: Rect, offset: usize) -> Range<usize> {
        let height = usize::from(area.height.saturating_sub(2));
        if height == 0 {
            return 0..0;
        }
        let frozen = usize::from(height > 1 && self.sticky_header_index(offset).is_some());
        let end = offset.saturating_add(height - frozen).min(self.lines.len());
        let index = self
            .file_headers
            .partition_point(|&(start, _)| start <= offset)
            .saturating_sub(1);
        let start = self.file_headers.get(index).map_or(end, |&(start, _)| start);
        start..end
    }

    fn tree_offset(&self, area: Rect, offset: usize) -> usize {
        let page = usize::from(area.height.saturating_sub(2)).max(1);
        let max = self.tree_line_count().saturating_sub(page);
        if let Some((diff_offset, tree_offset)) = self.tree_scroll
            && diff_offset == offset
        {
            return tree_offset.min(max);
        }
        let visible = self.visible_files(area, offset);
        let index = self.tree_rows.partition_point(|&(target, _)| target < visible.start);
        let row = self
            .tree_rows
            .get(index)
            .filter(|&&(target, _)| visible.contains(&target))
            .map_or(0, |&(_, row)| row);
        row.saturating_sub(page / 2).min(max)
    }

    pub(crate) fn scroll_tree(&mut self, mouse: MouseEvent, area: Rect, offset: usize) -> bool {
        let [tree, _] = self.areas(area);
        if !tree.contains(Position::new(mouse.column, mouse.row)) {
            return false;
        }
        let current = self.tree_offset(tree, offset);
        let next = match mouse.kind {
            MouseEventKind::ScrollUp => current.saturating_sub(1),
            MouseEventKind::ScrollDown => current.saturating_add(1),
            _ => return false,
        };
        let max = self
            .tree_line_count()
            .saturating_sub(usize::from(tree.height.saturating_sub(2)).max(1));
        self.tree_scroll = Some((offset, next.min(max)));
        true
    }

    fn tree_cell(
        &self,
        pointer: Position,
        area: Rect,
        offset: usize,
        horizontal_offset: usize,
    ) -> Option<(usize, usize)> {
        let [tree, diff] = self.areas(area);
        let (area, offset, frozen, horizontal_offset) = if tree.contains(pointer) {
            if pointer.x == tree.right() - 1 {
                return None;
            }
            (tree, self.tree_offset(tree, offset), 0, 0)
        } else {
            let frozen = usize::from(diff.height > 3 && self.sticky_header_index(offset).is_some());
            (diff, offset, frozen, horizontal_offset)
        };
        if !area.contains(pointer) || pointer.y <= area.y || pointer.y >= area.bottom().saturating_sub(1) {
            return None;
        }
        let row = offset + usize::from(pointer.y - area.y - 1).checked_sub(frozen)?;
        Some((row, usize::from(pointer.x - area.x) + horizontal_offset))
    }

    pub(crate) fn tree_target(
        &self,
        pointer: Position,
        area: Rect,
        offset: usize,
        horizontal_offset: usize,
    ) -> Option<usize> {
        let (row, column) = self.tree_cell(pointer, area, offset, horizontal_offset)?;
        self.file_target(row, column)
    }

    pub(crate) fn set_pointer(&mut self, pointer: Option<Position>) {
        self.pointer = pointer;
    }

    fn draw_directory_counts(&self, frame: &mut Frame<'_>, area: Rect, offset: usize, horizontal_offset: usize) {
        let Some(pointer) = self.pointer else {
            return;
        };
        let Some((row, column)) = self.tree_cell(pointer, area, offset, horizontal_offset) else {
            return;
        };
        let Ok(index) = self.directories.binary_search_by_key(&row, |dir| dir.row) else {
            return;
        };
        let dir = &self.directories[index];
        if !dir.columns.contains(&column) {
            return;
        }
        let [tree, diff] = self.areas(area);
        let right = if tree.contains(pointer) {
            tree.right() - 1
        } else {
            diff.right()
        };
        let x = usize::from(pointer.x) + dir.columns.end - column;
        if x < usize::from(right) {
            let x = x as u16;
            frame.buffer_mut().set_line(x, pointer.y, &dir.counts, right - x);
        }
    }

    pub(crate) fn key(&mut self, key: KeyEvent) -> bool {
        if key.kind == KeyEventKind::Release {
            return false;
        }
        if let Some(group @ (CommandGroup::View | CommandGroup::Information)) = crate::prefix_input::prefix_group(key) {
            if key.kind == KeyEventKind::Press {
                self.prefix = (self.prefix != Some(group)).then_some(group);
            }
            return true;
        }
        if key.modifiers.is_empty() {
            if key.code == KeyCode::Tab {
                if key.kind == KeyEventKind::Press {
                    self.close_prefix();
                    self.tree_visible = !self.tree_visible;
                    self.tree_scroll = None;
                }
                return key.kind == KeyEventKind::Press;
            }
            if self.prefix == Some(CommandGroup::View)
                && let KeyCode::Char(letter) = key.code
                && let Some((_, option)) = VIEW_OPTIONS.iter().find(|(shortcut, _)| *shortcut == letter)
            {
                if key.kind == KeyEventKind::Press {
                    self.toggle(*option);
                }
                return true;
            }
        }
        let closed = self.close_prefix();
        key.code == KeyCode::Esc && (closed || key.kind == KeyEventKind::Repeat)
    }

    pub(crate) fn close_prefix(&mut self) -> bool {
        self.prefix.take().is_some()
    }

    fn toggle(&mut self, option: ViewOption) {
        let options = &mut self.settings.options;
        match option {
            ViewOption::Color => self.settings.colored = !self.settings.colored,
            ViewOption::Renames => options.find_renames = !options.find_renames,
            ViewOption::Words => options.word_diff = !options.word_diff,
            ViewOption::Production => options.production_only = !options.production_only,
            ViewOption::Modifications => options.hide_modifications = !options.hide_modifications,
            ViewOption::Context => {
                let context = usize::from(options.context.first() == Some(&0));
                options.context.clear();
                options.context.push(context);
            }
        }
        (self.lines, self.file_links, self.directories) =
            self.diff.render(&self.settings.options, self.colors(), self.graphics);
        self.file_headers = file_header_rows(&self.lines);
        self.tree_rows = file_tree_rows(&self.file_links);
        self.tree_scroll = None;
        (self.tree_width, self.max_width) = measure_widths(&self.lines, &self.file_headers, &self.directories);
    }

    fn view_hints(&self) -> Vec<Vec<Span<'static>>> {
        let options = &self.settings.options;
        let mut hints: Vec<_> = VIEW_OPTIONS
            .into_iter()
            .map(|(key, option)| {
                let (label, enabled) = match option {
                    ViewOption::Color if self.colored() => ("color on", true),
                    ViewOption::Color => ("color off", false),
                    ViewOption::Context if options.context.first() == Some(&0) => ("context 0", true),
                    ViewOption::Context => ("context 1", true),
                    ViewOption::Production if options.production_only => ("production only on", true),
                    ViewOption::Production => ("production only off", false),
                    ViewOption::Modifications if options.hide_modifications => ("modifications hidden", true),
                    ViewOption::Modifications => ("modifications shown", false),
                    ViewOption::Renames if options.find_renames => ("renames on", true),
                    ViewOption::Renames => ("renames off", false),
                    ViewOption::Words if options.word_diff => ("words on", true),
                    ViewOption::Words => ("words off", false),
                };
                ui::shortcut(label, key, enabled)
            })
            .collect();
        hints.push(vec![Span::raw("Tab files")]);
        hints
    }

    fn draw_status(&self, frame: &mut Frame<'_>, area: Rect, offset: usize) {
        if area.is_empty() {
            return;
        }
        let footer = Rect::new(area.x, area.bottom() - 1, area.width, 1);
        let mut spans = vec![Span::raw(format!("#{}/{}", offset + 1, self.lines.len()))];
        let mut anchor = None;
        for (group, label) in [(CommandGroup::View, "view"), (CommandGroup::Information, "? help")] {
            spans.push(Span::raw(" · "));
            let mut label = ui::shortcut(label, group.prefix(), true);
            if self.prefix == Some(group) {
                anchor = Some(spans.iter().map(Span::width).sum());
                ui::emphasize_prefix(&mut label);
            }
            spans.extend(label);
        }
        spans.push(Span::raw(" · "));
        spans.extend(ui::shortcut("q back", 'q', true));
        let monochrome = |spans: &mut [Span<'_>]| {
            if !self.colored() {
                for span in spans {
                    span.style.fg = None;
                    span.style.bg = None;
                }
            }
        };
        monochrome(&mut spans);
        frame.render_widget(Clear, footer);
        frame.render_widget(Paragraph::new(Line::from(spans)), footer);
        if let Some(anchor) = anchor {
            let mut hints = if self.prefix == Some(CommandGroup::View) {
                self.view_hints()
            } else {
                [
                    "↑↓/jk scroll",
                    "wheel scroll",
                    "PgUp/PgDn or Ctrl+b/f page",
                    "Ctrl+u/d half page",
                    "Home/End or g/G ends",
                    "h/l pan",
                    "Tab files",
                    "click filename to jump",
                    "<enter>/q back",
                    "Esc close help",
                ]
                .into_iter()
                .map(|hint| vec![Span::raw(hint)])
                .collect()
            };
            for hint in &mut hints {
                monochrome(hint);
            }
            let popup = ui::hint_popup(hints, usize::from(footer.width.saturating_sub(2)));
            ui::render_prefix_popup(frame, area, footer, anchor, popup, None);
        }
    }

    pub(crate) fn draw(&self, frame: &mut Frame<'_>, area: Rect, offset: usize, horizontal_offset: usize) {
        let [tree, diff] = self.areas(area);
        if tree.width != 0 {
            self.draw_tree(frame, tree, offset);
        }
        let colors = self.colors();
        ui::draw_diff(
            frame,
            diff,
            &self.diff.title.to_str_lossy(),
            self.lines
                .iter()
                .map(|(line, language)| colors.highlight(line, *language)),
            self.sticky_header_index(offset)
                .map(|index| colors.highlight(&self.lines[index].0, None)),
            (offset, horizontal_offset),
            "",
        );
        self.tag_tree(frame, diff, offset, horizontal_offset);
        self.draw_directory_counts(frame, area, offset, horizontal_offset);
        self.draw_status(frame, area, offset);
    }

    fn draw_tree(&self, frame: &mut Frame<'_>, tree: Rect, offset: usize) {
        let visible = self.visible_files(tree, offset);
        let colors = self.colors();
        let lines = self
            .lines
            .iter()
            .take(self.tree_line_count())
            .enumerate()
            .map(|(row, (line, _))| {
                let mut line = colors.highlight(line, None);
                if let Ok(index) = self.file_links.binary_search_by_key(&row, |link| link.row)
                    && let link = &self.file_links[index]
                    && visible.contains(&link.target)
                {
                    for span in &mut line.spans[..link.label_spans] {
                        span.style = span.style.add_modifier(Modifier::BOLD | Modifier::REVERSED);
                    }
                }
                line
            });
        let area = Rect::new(tree.x, tree.y, tree.width.saturating_sub(1), tree.height);
        let tree_offset = self.tree_offset(tree, offset);
        crate::ui::draw_diff(frame, area, "Files", lines, None, (tree_offset, 0), "");
        self.tag_tree(frame, area, tree_offset, 0);
        frame.render_widget(
            Block::default().borders(Borders::RIGHT).border_style(colors.metadata),
            tree,
        );
    }

    fn tag_tree(&self, frame: &mut Frame<'_>, area: Rect, offset: usize, horizontal_offset: usize) {
        let tree_end = self.tree_line_count();
        if !self.graphics || offset >= tree_end {
            return;
        }
        let mut tiles = Vec::new();
        for (y, (line, _)) in
            (area.y.saturating_add(1)..area.bottom().saturating_sub(1)).zip(&self.lines[offset..tree_end])
        {
            let Some(prefix) = line.spans.first() else {
                continue;
            };
            tiles.clear();
            tiles.extend(
                prefix
                    .content
                    .chars()
                    .take_while(|ch| matches!(ch, ' ' | '│' | '├' | '└'))
                    .skip(horizontal_offset)
                    .take(usize::from(area.width))
                    .map(|ch| Tile::from_text(ch).unwrap_or_default()),
            );
            crate::ui::tag_graph(frame, Rect::new(area.x, y, area.width, 1), &tiles, 0);
        }
    }
}

fn tree_line_count(lines: &DiffRows, file_headers: &[(usize, usize)]) -> usize {
    file_headers
        .first()
        .map_or(lines.len(), |&(start, _)| start.saturating_sub(1))
}

fn measure_widths(lines: &DiffRows, file_headers: &[(usize, usize)], directories: &[DirectoryStat]) -> (usize, usize) {
    let (tree, diff) = lines.split_at(tree_line_count(lines, file_headers));
    let tree_width = tree
        .iter()
        .map(|(line, _)| line.width())
        .chain(directories.iter().map(|dir| dir.columns.end + dir.counts.width()))
        .max()
        .unwrap_or_default();
    let diff_width = diff.iter().map(|(line, _)| line.width()).max().unwrap_or_default();
    (tree_width, tree_width.max(diff_width))
}

fn file_tree_rows(links: &[FileLink]) -> Vec<(usize, usize)> {
    let mut rows: Vec<_> = links.iter().map(|link| (link.target, link.row)).collect();
    rows.sort_unstable();
    rows
}

fn file_header_rows(lines: &[(Line<'_>, Option<Language>)]) -> Vec<(usize, usize)> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(index, (line, language))| {
            (language.is_none()
                && line
                    .spans
                    .first()
                    .is_some_and(|span| span.content.starts_with("diff --git ")))
            .then_some((index, index))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyModifiers, MouseButton};
    use gix::error::TestResult;

    use super::*;
    use crate::{DiffResource, app::ChangeGroup};

    fn repository() -> TestResult<gix::Repository> {
        let path = gix_testtools::scripted_fixture_read_only("structural-diff.sh")?;
        Ok(crate::test_repository::open_with(
            &path,
            [
                "diff.renames=false",
                "diff.tool=unexpected",
                "difftool.unexpected.cmd=exit 99",
                "diff.external=exit 98",
                "core.pager=exit 97",
            ],
        )?)
    }

    fn production_repository() -> TestResult<gix::Repository> {
        let path = gix_testtools::scripted_fixture_read_only("structural-diff-production.sh")?;
        Ok(crate::test_repository::open(&path)?)
    }

    fn target(repository: &gix::Repository, name: &str, parent: usize) -> TestResult<TreeDiffTarget> {
        Ok(TreeDiffTarget::Commit {
            id: repository.rev_parse_single(name)?.detach(),
            parent,
        })
    }

    fn text(lines: &[(Line<'_>, Option<Language>)]) -> String {
        lines
            .iter()
            .map(|(line, _)| line.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn conventional_non_production_paths_do_not_match_similar_production_names() {
        for path in [
            "tests/gix/mod.rs",
            "crate/benches/run.rs",
            "crate/benchmark/run.rs",
            "benchmarks/compare.rs",
            "examples/demo.rs",
            "support/fixtures/input.bin",
            "src/tests.rs",
            "src/test_repository.rs",
            "src/api_tests.rs",
            "project/test_api.py",
            "project/api_test.py",
            "project/conftest.py",
        ] {
            assert!(
                non_production_path(path.as_bytes().as_bstr()),
                "conventional test and development paths are hidden: {path}"
            );
        }
        for path in [
            "src/latest.rs",
            "src/tester.rs",
            "src/contest.rs",
            "Cargo.toml",
            "docs/guide.md",
            "src/api.rs",
            "project/contest.py",
        ] {
            assert!(
                !non_production_path(path.as_bytes().as_bstr()),
                "production and documentation paths remain visible: {path}"
            );
        }
    }

    #[test]
    fn changed_declarations_have_a_background_and_stronger_word_emphasis() -> TestResult {
        let outline = PreparedDiff::new("fn f(arg: i32) {}", "fn f(arg: u32) {}", Language::Rust)?;
        let changed = outline
            .render_with(
                &Options {
                    word_diff: true,
                    ..Options::default()
                },
                RenderOptions {
                    omit_word_delimiters: true,
                    ..RenderOptions::default()
                },
            )
            .into_iter()
            .find(|line| line.change == Change::Modified)
            .expect("the changed signature has a modified row");
        let colors = Palette::new([220; 3], [20; 3], None);
        let line = structural_line(changed, &colors);
        assert!(
            line.style.bg.is_some(),
            "changed declarations have a subtle line background"
        );
        for word in ["i32", "u32"] {
            let span = line
                .spans
                .iter()
                .find(|span| span.content == word)
                .expect("changed words stay distinct");
            assert_ne!(
                span.style.bg, line.style.bg,
                "{word} has stronger emphasis than its row"
            );
        }
        Ok(())
    }

    #[test]
    fn color_toggle_restores_text_markers_and_disables_diff_colors() -> TestResult {
        let repository = repository()?;
        let diff = prepare_commit_with_repository(&repository, target(&repository, "tip", 0)?, "updated".into())?;
        let mut settings = crate::App::new(24).structural_diff_settings;
        let theme = Palette::new([220; 3], [20; 3], None);
        let mut view = View::new(&diff, &mut settings, &theme, false);
        let colored = text(&view.lines);
        assert!(
            colored.contains("value: i32u32"),
            "highlighted fragments keep both values: {colored}"
        );
        assert!(
            !colored.contains("[-i32-]") && !colored.lines().any(|line| line.starts_with('~')),
            "colored declarations omit word and line markers"
        );
        view.toggle(ViewOption::Color);
        let plain = text(&view.lines);
        assert!(
            plain.contains("[-i32-]{+u32+}") && plain.lines().any(|line| line.starts_with('~')),
            "plain declarations keep textual markers"
        );
        assert!(
            view.lines.iter().all(|(line, _)| line.style.fg.is_none()
                && line.style.bg.is_none_or(|color| color == ratatui::style::Color::Reset)
                && line
                    .spans
                    .iter()
                    .all(|span| span.style.fg.is_none() && span.style.bg.is_none())),
            "plain rows and word changes have no colors"
        );
        view.toggle(ViewOption::Color);
        assert_eq!(
            text(&view.lines),
            colored,
            "turning color back on restores the marker-free view"
        );
        Ok(())
    }

    #[test]
    fn prefix_popouts_keep_the_status_compact_and_accept_consecutive_settings() -> TestResult {
        let repository = repository()?;
        let diff = prepare_commit_with_repository(&repository, target(&repository, "tip", 0)?, "updated".into())?;
        let mut settings = crate::App::new(24).structural_diff_settings;
        let colors = Palette::default();
        let mut view = View::new(&diff, &mut settings, &colors, false);
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(43, 12))?;
        let press = |letter| KeyEvent::new(KeyCode::Char(letter), KeyModifiers::NONE);
        let row = |terminal: &ratatui::Terminal<ratatui::backend::TestBackend>, y| {
            (0..43)
                .map(|x| terminal.backend().buffer()[(x, y)].symbol())
                .collect::<String>()
        };
        terminal.draw(|frame| view.draw(frame, frame.area(), 0, 0))?;
        let footer = row(&terminal, 11);
        assert!(
            footer.contains(" · view · ? help · q back"),
            "position and prefix hints fit on a compact status row"
        );
        assert!(
            !footer.contains("context") && !footer.contains("scroll"),
            "settings and help stay in their popouts"
        );
        assert!(
            !view.key(press('c')) && view.colored(),
            "a setting key by itself does not act"
        );
        assert!(
            view.key(press('v')) && view.colored(),
            "opening View does not toggle a setting"
        );
        assert!(view.key(KeyEvent::new_with_kind(
            KeyCode::Char('v'),
            KeyModifiers::NONE,
            KeyEventKind::Repeat
        )));
        assert!(view.key(press('x')), "View accepts its context shortcut");
        assert_eq!(view.settings.options.context, [1]);
        assert!(view.key(KeyEvent::new_with_kind(
            KeyCode::Char('x'),
            KeyModifiers::NONE,
            KeyEventKind::Repeat
        )));
        assert_eq!(
            view.settings.options.context,
            [1],
            "repeated suffix events do not retoggle the setting"
        );
        terminal.draw(|frame| view.draw(frame, frame.area(), 0, 0))?;
        let items = (7..11).map(|y| row(&terminal, y)).collect::<Vec<_>>().join("\n");
        for hint in [
            "color on",
            "context 1",
            "production only off",
            "modifications shown",
            "renames on",
            "words on",
            "Tab files",
        ] {
            assert!(
                items.contains(hint),
                "the popout keeps {hint} visible, including when it wraps"
            );
        }
        let (prefix, _) = footer.split_once("view").expect("View has a footer label");
        let anchor = u16::try_from(prefix.chars().count())?;
        assert!(
            terminal.backend().buffer()[(anchor, 11)]
                .modifier
                .contains(Modifier::REVERSED),
            "the open prefix label is reversed"
        );
        assert!(
            view.key(press('c')) && !view.colored(),
            "View stays open for consecutive settings"
        );
        assert!(
            view.key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            "Escape dismisses the prefix"
        );
        assert!(!view.key(press('x')), "the prefix is closed after dismissal");
        assert!(
            view.key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::SHIFT)),
            "Shift+/ opens Help"
        );
        terminal.draw(|frame| view.draw(frame, frame.area(), 0, 0))?;
        assert!(
            (0..11).any(|y| row(&terminal, y).contains("click filename to jump")),
            "Help displays navigation instead of settings"
        );
        assert!(
            !view.key(press('j')) && view.prefix.is_none(),
            "scrolling closes Help and continues to the diff"
        );
        Ok(())
    }

    #[test]
    fn tab_shows_and_hides_a_file_tree_beside_the_diff() -> TestResult {
        let repository = repository()?;
        let diff = prepare_commit_with_repository(&repository, target(&repository, "tip", 0)?, "updated".into())?;
        let mut settings = Settings::default();
        let colors = Palette::default();
        let mut view = View::new(&diff, &mut settings, &colors, false);
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 12))?;
        let offset = view.last_file_header();
        for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT, KeyModifiers::SHIFT] {
            assert!(
                !view.key(KeyEvent::new(KeyCode::Tab, modifiers)),
                "only an unmodified Tab toggles the panel"
            );
        }
        for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
            assert!(
                !view.key(KeyEvent::new_with_kind(KeyCode::Tab, KeyModifiers::NONE, kind)),
                "holding or releasing Tab does not toggle the panel"
            );
        }
        assert!(
            view.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
            "Tab opens the file tree panel"
        );
        terminal.draw(|frame| view.draw(frame, frame.area(), offset, 0))?;
        let row: String = (0..40).map(|x| terminal.backend().buffer()[(x, 0)].symbol()).collect();
        assert!(
            row.starts_with("Files"),
            "the tree reappears to the left of the current file"
        );
        assert!(
            view.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
            "a second Tab hides the panel"
        );
        terminal.draw(|frame| view.draw(frame, frame.area(), offset, 0))?;
        let row: String = (0..120).map(|x| terminal.backend().buffer()[(x, 0)].symbol()).collect();
        assert!(
            row.starts_with("Structural diff"),
            "hiding restores the full-width diff"
        );
        Ok(())
    }

    #[test]
    fn tree_panel_highlights_visible_files_and_keeps_click_targets_current() -> TestResult {
        let repository = production_repository()?;
        let diff = prepare_commit_with_repository(&repository, target(&repository, "tip", 0)?, "updated".into())?;
        let mut settings = Settings::default();
        let colors = Palette::default();
        let mut review = crate::DiffView::Structural(View::new(&diff, &mut settings, &colors, false));
        assert!(review.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)));
        let area = Rect::new(3, 2, 120, 80);
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(125, 84))?;
        for option in [
            None,
            Some(ViewOption::Context),
            Some(ViewOption::Words),
            Some(ViewOption::Renames),
            Some(ViewOption::Production),
        ] {
            if let (Some(option), crate::DiffView::Structural(view)) = (option, &mut review) {
                view.toggle(option);
            }
            let crate::DiffView::Structural(view) = &review else {
                unreachable!("this test reviews a structural diff")
            };
            let [tree, patch] = view.areas(area);
            assert_eq!(patch.x, tree.right(), "the panel and diff do not overlap");
            assert_eq!(patch.right(), area.right(), "the diff keeps the remaining width");
            for offset in [view.file_headers[0].0, view.last_file_header()] {
                terminal.draw(|frame| review.draw(frame, area, offset, 0))?;
                let visible = view.visible_files(patch, offset);
                let tree_offset = view.tree_offset(tree, offset);
                let mut highlighted = 0;
                for link in &view.file_links {
                    if link.row < tree_offset || link.row >= tree_offset + usize::from(tree.height - 2) {
                        continue;
                    }
                    let mouse = MouseEvent {
                        kind: MouseEventKind::Down(MouseButton::Left),
                        column: tree.x + u16::try_from(link.columns.start)?,
                        row: tree.y + 1 + u16::try_from(link.row - tree_offset)?,
                        modifiers: KeyModifiers::NONE,
                    };
                    let selected = terminal.backend().buffer()[(mouse.column, mouse.row)]
                        .modifier
                        .contains(Modifier::REVERSED);
                    assert_eq!(
                        selected,
                        visible.contains(&link.target),
                        "only files visible in the diff are highlighted"
                    );
                    let counts_start = tree.x + u16::try_from(link.columns.end)?;
                    let counts_end = tree.x + u16::try_from(view.lines[link.row].0.width())?;
                    for x in counts_start..counts_end {
                        let cell = &terminal.backend().buffer()[(x, mouse.row)];
                        assert_eq!(
                            cell.bg,
                            ratatui::style::Color::Reset,
                            "diffstats use the terminal background"
                        );
                        assert!(
                            !cell.modifier.contains(Modifier::REVERSED),
                            "the entry highlight stops before the diffstat"
                        );
                    }
                    highlighted += usize::from(selected);
                    assert_eq!(
                        review.mouse(mouse, area, offset, 20),
                        Some((link.target, 0)),
                        "panel clicks use their own coordinates and clear diff panning"
                    );
                }
                assert!(highlighted > 0, "the current file is highlighted");
                if offset == view.file_headers[0].0 {
                    assert!(highlighted > 1, "all simultaneously visible files are highlighted");
                }
                assert_eq!(
                    review.mouse(
                        MouseEvent {
                            kind: MouseEventKind::ScrollRight,
                            column: patch.x,
                            row: patch.y + 1,
                            modifiers: KeyModifiers::NONE
                        },
                        area,
                        offset,
                        0
                    ),
                    Some((
                        offset,
                        view.max_width()
                            .saturating_sub(usize::from(patch.width))
                            .min(usize::from(patch.width))
                    )),
                    "horizontal diff scrolling uses the reduced viewport width"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn tree_panel_fits_the_longest_filename_and_counts_as_space_allows() -> TestResult {
        let repository = production_repository()?;
        let mut diff = prepare_commit_with_repository(&repository, target(&repository, "tip", 0)?, "updated".into())?;
        let file = diff
            .files
            .iter_mut()
            .find(|file| file.path.path == "tests/integration.rs")
            .expect("the fixture changes an integration test");
        let filename = "a_long_integration_test_filename_that_needs_more_than_forty_eight_columns.rs";
        file.path.path = format!("tests/{filename}").into();
        let mut settings = Settings::default();
        let colors = Palette::default();
        let mut view = View::new(&diff, &mut settings, &colors, true);
        assert!(view.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)));
        let area = Rect::new(0, 0, 140, 40);
        let link = view
            .file_links
            .iter()
            .find(|link| view.lines[link.row].0.spans[2].content == filename)
            .expect("the tree lists the long filename");
        let line = &view.lines[link.row].0;
        let [tree, patch] = view.areas(area);
        assert_eq!(
            usize::from(tree.width),
            line.width() + 1,
            "the sidebar fits the longest rendered entry plus the divider"
        );
        assert!(patch.width > 0, "the diff gets the remaining space");
        let [small_tree, small_patch] = view.areas(Rect::new(0, 0, 40, 30));
        assert!(
            small_tree.width < tree.width && small_patch.width > 0,
            "the panel fits smaller terminals without consuming the diff"
        );
        view.toggle(ViewOption::Production);
        assert!(
            view.areas(area)[0].width < tree.width,
            "the width follows the filtered tree contents"
        );
        Ok(())
    }

    #[test]
    fn tree_panel_scrolls_independently_and_follows_the_current_file() -> TestResult {
        let repository = production_repository()?;
        let diff = prepare_commit_with_repository(&repository, target(&repository, "tip", 0)?, "updated".into())?;
        let mut settings = Settings::default();
        let colors = Palette::default();
        let mut view = View::new(&diff, &mut settings, &colors, false);
        assert!(view.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)));
        let area = Rect::new(2, 2, 120, 7);
        let [tree, patch] = view.areas(area);
        let offset = view.last_file_header();
        let current = view.tree_offset(tree, offset);
        let mouse = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: tree.x + 1,
            row: tree.y + 1,
            modifiers: KeyModifiers::NONE,
        };
        assert!(
            view.scroll_tree(mouse, area, offset),
            "wheels over the panel scroll its tree"
        );
        assert_eq!(
            view.tree_offset(tree, offset),
            current.saturating_sub(1),
            "manual scrolling is retained while the diff is stationary"
        );
        assert!(
            !view.scroll_tree(
                MouseEvent {
                    column: patch.x,
                    ..mouse
                },
                area,
                offset
            ),
            "wheels over the diff do not scroll the panel"
        );
        let first = view.file_headers[0].0;
        let target_row = view
            .tree_rows
            .iter()
            .find(|&&(target, _)| target == first)
            .expect("each file has a tree row")
            .1;
        let followed = view.tree_offset(tree, first);
        assert!(
            target_row >= followed && target_row < followed + usize::from(tree.height - 2),
            "moving to another file brings its highlighted entry into view"
        );
        view.toggle(ViewOption::Production);
        assert!(
            view.tree_scroll.is_none(),
            "filtering discards obsolete tree scroll coordinates"
        );
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(120, 7))?;
        for width in [1, 3, 6, 20, 120] {
            let bounds = Rect::new(0, 0, width, 7);
            terminal.draw(|frame| view.draw(frame, bounds, first.min(view.last_file_header()), 0))?;
            let [tree, patch] = view.areas(bounds);
            assert_eq!(
                tree.width + patch.width,
                bounds.width,
                "resizing retains a usable diff area"
            );
        }
        Ok(())
    }

    #[test]
    fn context_starts_at_zero_and_can_be_toggled_across_views() -> TestResult {
        let repository = production_repository()?;
        let diff = prepare_commit_with_repository(&repository, target(&repository, "tip", 0)?, "updated".into())?;
        let mut settings = crate::App::new(24).structural_diff_settings;
        let colors = Palette::default();
        {
            let mut view = View::new(&diff, &mut settings, &colors, false);
            let original = text(&view.lines);
            assert!(
                !original.contains("fn unchanged()"),
                "zero context hides unchanged siblings by default"
            );
            view.toggle(ViewOption::Context);
            let output = text(&view.lines);
            assert!(
                output.contains("fn unchanged()"),
                "one context line shows unchanged siblings"
            );
            assert!(output.contains("test_helper"), "changed declarations remain visible");
            view.toggle(ViewOption::Context);
            assert_eq!(
                text(&view.lines),
                original,
                "restoring context reproduces the original diff"
            );
            view.toggle(ViewOption::Context);
        }
        let reopened = View::new(&diff, &mut settings, &colors, false);
        assert!(
            text(&reopened.lines).contains("fn unchanged()"),
            "reopening preserves the selected context"
        );
        Ok(())
    }

    #[test]
    fn file_tree_click_targets_follow_scrolling_panning_and_live_settings() -> TestResult {
        let repository = production_repository()?;
        let diff = prepare_commit_with_repository(&repository, target(&repository, "tip", 0)?, "updated".into())?;
        let mut settings = Settings::default();
        let colors = Palette::default();
        let mut review = crate::DiffView::Structural(View::new(&diff, &mut settings, &colors, false));
        let area = Rect::new(4, 3, 100, 7);
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(110, 12))?;
        for option in [
            None,
            Some(ViewOption::Production),
            Some(ViewOption::Context),
            Some(ViewOption::Words),
            Some(ViewOption::Renames),
            Some(ViewOption::Context),
            Some(ViewOption::Production),
        ] {
            if let (Some(option), crate::DiffView::Structural(view)) = (option, &mut review) {
                view.toggle(option);
            }
            let crate::DiffView::Structural(view) = &review else {
                unreachable!("this test reviews a structural diff")
            };
            assert_eq!(
                view.file_links.len(),
                view.file_headers.len(),
                "each visible file has one jump target"
            );
            for link in &view.file_links {
                let name = &view.lines[link.row].0.spans[2].content;
                let file = diff
                    .files
                    .iter()
                    .find(|file| {
                        quoted(
                            file.path
                                .path
                                .rsplit(|byte| *byte == b'/')
                                .next()
                                .expect("file paths have a basename")
                                .as_bstr(),
                            "",
                        ) == name.as_ref()
                    })
                    .expect("fixture basenames identify individual files");
                assert_eq!(
                    view.lines[link.target].0.to_string(),
                    file.headers[0],
                    "the clicked name points to its own file's header"
                );
                let offset = link.row.saturating_sub(1);
                let mouse = crate::MouseEvent {
                    kind: crate::MouseEventKind::Down(crate::MouseButton::Left),
                    column: area.x,
                    row: area.y + 1 + u16::try_from(link.row - offset)?,
                    modifiers: KeyModifiers::NONE,
                };
                let position = review
                    .mouse(mouse, area, offset, link.columns.start)
                    .expect("a visible filename is clickable after panning");
                assert_eq!(position, (link.target, 0), "jumping clears horizontal panning");
                assert!(
                    position.0 <= review.max_offset(usize::from(area.height - 2)),
                    "even the final short file can be placed at the top"
                );
                terminal.draw(|frame| review.draw(frame, area, position.0, position.1))?;
                let header: String = (area.x..area.right())
                    .map(|x| terminal.backend().buffer()[(x, area.y + 1)].symbol())
                    .collect();
                assert_eq!(
                    header.trim_end(),
                    file.headers[0],
                    "the jump places the selected file at the viewport top"
                );
                for kind in [
                    crate::MouseEventKind::Up(crate::MouseButton::Left),
                    crate::MouseEventKind::Drag(crate::MouseButton::Left),
                    crate::MouseEventKind::Down(crate::MouseButton::Right),
                ] {
                    assert!(
                        review
                            .mouse(crate::MouseEvent { kind, ..mouse }, area, offset, link.columns.start)
                            .is_none(),
                        "only a left-button press activates a filename"
                    );
                }
                assert!(
                    review
                        .mouse(
                            crate::MouseEvent {
                                modifiers: KeyModifiers::CONTROL,
                                ..mouse
                            },
                            area,
                            offset,
                            link.columns.start
                        )
                        .is_none(),
                    "modified clicks do not activate a filename"
                );
                assert!(
                    review.mouse(mouse, area, offset, 0).is_none(),
                    "tree indentation is not a filename"
                );
                assert!(
                    review.mouse(mouse, area, offset, link.columns.end).is_none(),
                    "blank space past the filename is not a link"
                );
            }
            assert!(
                view.file_target(0, 0).is_none(),
                "the overview heading is not a file link"
            );
            assert!(
                view.file_target(view.last_file_header(), 0).is_none(),
                "patch sections are not file-tree links"
            );
        }
        Ok(())
    }

    #[test]
    fn file_line_counts_match_git_and_directory_hover_follows_the_filter() -> TestResult {
        let repository = production_repository()?;
        let diff = prepare_commit_with_repository(&repository, target(&repository, "tip", 0)?, "updated".into())?;
        let workdir = repository.workdir().expect("the fixture has a worktree");
        let numstat = gix_testtools::git(workdir, "diff --numstat base tip -- src/mixed.rs")?;
        assert_eq!(
            numstat.trim(),
            "2\t2\tsrc/mixed.rs",
            "the fixture modifies two lines in this file"
        );
        let mut settings = Settings::default();
        let colors = Palette::default();
        let mut view = View::new(&diff, &mut settings, &colors, false);
        let area = Rect::new(2, 3, 120, 24);
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(125, 30))?;
        for (filtered, expected, aggregate) in [
            (false, "src/  +4 -4", "Changed files (13)  +11 -11"),
            (true, "src/  +2 -2", "Changed files (5)  +4 -4"),
        ] {
            if filtered {
                view.toggle(ViewOption::Production);
            }
            let heading = &view.lines[0].0;
            assert_eq!(heading.to_string(), aggregate, "the heading totals the listed files");
            for (span, kind) in heading
                .spans
                .iter()
                .skip(1)
                .zip([ChangeKind::Added, ChangeKind::Deleted])
            {
                assert_eq!(span.style, colors.path(kind), "the totals use the file diffstat style");
            }
            let output = text(&view.lines[..view.tree_line_count()]);
            assert!(
                output.contains("mixed.rs  +2 -2"),
                "a visible file shows whole-file line counts"
            );
            let dir = view
                .directories
                .iter()
                .find(|dir| view.lines[dir.row].0.spans[1].content == "src/")
                .expect("src contains changes");
            let row = u16::try_from(dir.row)?;
            let column = u16::try_from(dir.columns.start)?;
            let y = area.y + 1 + row;
            for sidebar in [false, true] {
                if sidebar {
                    assert!(view.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)));
                }
                view.set_pointer(Some(Position::new(area.x + column, y)));
                terminal.draw(|frame| view.draw(frame, area, 0, 0))?;
                let buffer = terminal.backend().buffer();
                let row: String = (area.x + column..area.right())
                    .map(|x| buffer[(x, y)].symbol())
                    .collect();
                assert!(
                    row.starts_with(expected),
                    "the hovered label and its visible descendants' totals share a row: {row}"
                );
                for (sign, kind) in [('+', ChangeKind::Added), ('-', ChangeKind::Deleted)] {
                    let index = expected.find(sign).expect("the total has additions and deletions");
                    let cell = &buffer[(area.x + column + u16::try_from(index)?, y)];
                    assert_eq!(
                        Some(cell.fg),
                        colors.path(kind).fg,
                        "directory counts use the file diffstat colors"
                    );
                    assert_eq!(
                        cell.bg,
                        ratatui::style::Color::Reset,
                        "directory counts use the terminal background"
                    );
                }
                assert!(
                    !(area.x..area.right())
                        .map(|x| buffer[(x, area.bottom() - 1)].symbol())
                        .collect::<String>()
                        .contains(expected),
                    "hover keeps the footer available for shortcuts"
                );
            }
            view.set_pointer(None);
            terminal.draw(|frame| view.draw(frame, area, 0, 0))?;
            let row: String = (area.x..area.right())
                .map(|x| terminal.backend().buffer()[(x, y)].symbol())
                .collect();
            assert!(!row.contains(expected), "leaving the directory hides its totals");
            assert!(view.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)));
        }
        Ok(())
    }

    #[test]
    fn file_headers_stick_only_after_reaching_the_viewport_top() -> TestResult {
        let repository = repository()?;
        let diff = prepare_commit_with_repository(&repository, target(&repository, "tip", 0)?, "updated".into())?;
        let mut settings = Settings::default();
        let colors = Palette::new([220; 3], [20; 3], None);
        let mut view = View::new(&diff, &mut settings, &colors, false);
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(160, 7))?;
        let row = |terminal: &ratatui::Terminal<ratatui::backend::TestBackend>, y| {
            (0..160)
                .map(|x| terminal.backend().buffer()[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        };
        for option in [
            None,
            Some(ViewOption::Production),
            Some(ViewOption::Context),
            Some(ViewOption::Words),
            Some(ViewOption::Renames),
            Some(ViewOption::Context),
        ] {
            if let Some(option) = option {
                view.toggle(option);
            }
            terminal.draw(|frame| view.draw(frame, frame.area(), 0, 0))?;
            assert!(
                row(&terminal, 1).starts_with("Changed files"),
                "the overview has no frozen file header"
            );
            let headers: Vec<_> = view
                .lines
                .iter()
                .enumerate()
                .filter_map(|(index, (line, _))| line.to_string().starts_with("diff --git ").then_some(index))
                .collect();
            for (file, header) in headers.into_iter().enumerate() {
                let name = view.lines[header].0.to_string();
                terminal.draw(|frame| view.draw(frame, frame.area(), header - 1, 0))?;
                assert_eq!(
                    row(&terminal, if file == 0 { 2 } else { 3 }),
                    name,
                    "the next header scrolls up in normal flow"
                );
                terminal.draw(|frame| view.draw(frame, frame.area(), header, 0))?;
                assert_eq!(
                    row(&terminal, 1),
                    name,
                    "a header reaching the top replaces the previous file"
                );
                assert_eq!(
                    row(&terminal, 2),
                    view.lines[header + 1].0.to_string(),
                    "the header is not duplicated"
                );
                terminal.draw(|frame| view.draw(frame, frame.area(), header + 2, 0))?;
                assert_eq!(
                    row(&terminal, 1),
                    name,
                    "the current file stays visible while reading its changes"
                );
                assert_eq!(
                    row(&terminal, 2),
                    view.lines[header + 2].0.to_string(),
                    "freezing does not discard the first visible change"
                );
                let buffer = terminal.backend().buffer();
                assert!(
                    buffer[(0, 1)].modifier.contains(Modifier::BOLD | Modifier::REVERSED),
                    "the frozen path stands out from the hunk"
                );
                assert!(
                    buffer[(159, 1)].modifier.contains(Modifier::REVERSED),
                    "the highlight spans the viewport"
                );
                assert!(
                    !buffer[(0, 2)].modifier.contains(Modifier::REVERSED),
                    "hunk text keeps ordinary styling"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn continuous_diff_starts_with_the_file_tree_and_keeps_every_changed_file() -> TestResult {
        let repository = repository()?;
        let diff = prepare_commit_with_repository(&repository, target(&repository, "tip", 0)?, "updated".into())?;
        let (lines, _, _) = diff.render(
            &Options {
                find_renames: true,
                word_diff: true,
                ..Options::default()
            },
            &Palette::default(),
            false,
        );
        let output = text(&lines);
        let first_header = output.find("diff --git").expect("changed files have patch headers");
        let overview = &output[..first_header];
        assert!(overview.contains("binary.rs  n/a"), "binary files have no text counts");
        let without_counts = diff.files.iter().filter(|file| file.path.lines.is_none()).count();
        assert!(
            lines[0]
                .0
                .to_string()
                .ends_with(&format!(" · {without_counts} without line counts")),
            "the overview reports how many files could not contribute to its totals"
        );
        for name in [
            "Changed files",
            "src/",
            "nested/",
            "A added.rs",
            "D removed.rs",
            "M modified.rs",
            "R moved.rs ← src/old.rs",
            "A empty.rs",
            "A broken.rs",
            "M unsupported.txt",
        ] {
            assert!(
                overview.contains(name),
                "{name} precedes every per-file patch: {overview}"
            );
        }
        for header in [
            "rename from src/old.rs",
            "rename to src/moved.rs",
            "--- /dev/null",
            "+++ /dev/null",
            "old mode 100644",
            "new mode 100755",
        ] {
            assert!(output.contains(header), "patch headers retain {header}: {output}");
        }
        assert!(
            output.contains("pub fn") && output.contains("pub struct Config"),
            "structural sections follow the tree"
        );
        assert!(
            !output.contains("secret_"),
            "implementation, values, and prose stay hidden: {output}"
        );
        assert!(
            output.contains("No structural changes."),
            "mode-only and empty changes remain visible"
        );
        assert!(
            output.contains("Outline unavailable:"),
            "unsupported and malformed inputs explain their missing outline"
        );
        Ok(())
    }

    #[test]
    fn kitty_tree_uses_one_cell_per_level_and_keeps_the_filename_clickable() -> TestResult {
        let repository = repository()?;
        let diff = prepare_commit_with_repository(&repository, target(&repository, "tip", 0)?, "updated".into())?;
        let colors = Palette::default();
        let area = Rect::new(2, 3, 100, 10);
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(105, 15))?;
        for (graphics, indent) in [(false, 12), (true, 3)] {
            let mut settings = Settings::default();
            let mut view = View::new(&diff, &mut settings, &colors, graphics);
            let link = view
                .file_links
                .iter()
                .find(|link| view.lines[link.row].0.spans[2].content == "added.rs")
                .expect("the nested fixture file is listed");
            let (row, target) = (link.row, link.target);
            let name = u16::try_from(link.columns.start)?;
            assert_eq!(
                name,
                indent + 2,
                "each level uses the expected number of connector columns"
            );
            let pan = 2;
            terminal.draw(|frame| view.draw(frame, area, row, pan))?;
            let connector = &terminal.backend().buffer()[(area.x + indent - 1 - u16::try_from(pan)?, area.y + 1)];
            assert_eq!(
                Tile::decode(connector.symbol()).map(|tile| tile.paths),
                graphics.then_some(Tile::UP_RIGHT),
                "a clipped overview uses the expected connector renderer"
            );
            let pointer = Position::new(area.x + name - u16::try_from(pan)?, area.y + 1);
            assert_eq!(
                view.tree_target(pointer, area, row, pan),
                Some(target),
                "panning preserves the file's hit region"
            );
            assert!(view.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)));
            let [tree, _] = view.areas(area);
            let y = tree.y + 1 + u16::try_from(row - view.tree_offset(tree, target))?;
            terminal.draw(|frame| view.draw(frame, area, target, 0))?;
            let connector = &terminal.backend().buffer()[(tree.x + indent - 1, y)];
            assert_eq!(
                Tile::decode(connector.symbol()).map(|tile| tile.paths),
                graphics.then_some(Tile::UP_RIGHT),
                "the sidebar uses the same connector geometry"
            );
            assert_eq!(
                view.tree_target(Position::new(tree.x + name, y), area, target, 0),
                Some(target),
                "the same filename stays clickable in the sidebar"
            );
        }
        Ok(())
    }

    #[test]
    fn live_options_change_declarations_and_keep_file_renames_enabled() -> TestResult {
        let repository = repository()?;
        let diff = prepare_commit_with_repository(&repository, target(&repository, "tip", 0)?, "updated".into())?;
        let mut settings = Settings {
            options: Options {
                find_renames: true,
                word_diff: true,
                ..Options::default()
            },
            colored: false,
        };
        let original = text(&diff.render(&settings.options, &Palette::PLAIN, false).0);
        assert!(
            original.contains("[renamed: old -> new]"),
            "declaration matching starts enabled"
        );
        assert!(original.contains("[-i32-]{+u32+}"), "word changes start enabled");
        assert!(
            original.contains("(…, value: ") && !original.contains("limit: usize"),
            "paired signatures collapse unchanged arguments and keep the changed parameter"
        );
        {
            let colors = Palette::default();
            let mut view = View::new(&diff, &mut settings, &colors, false);
            view.toggle(ViewOption::Renames);
            let output = text(&view.lines);
            assert!(
                !output.contains("[renamed:"),
                "r redraws declaration additions and removals"
            );
            assert!(
                output.contains("R moved.rs ← src/old.rs"),
                "file rename detection is independent of the live setting"
            );
            view.toggle(ViewOption::Renames);
            view.toggle(ViewOption::Words);
            let output = text(&view.lines);
            assert!(
                output.contains("[renamed: old -> new]"),
                "declaration matching can be restored"
            );
            assert!(!output.contains("[-i32-]"), "w redraws separate declaration lines");
        }
        assert!(
            !settings.options.word_diff,
            "settings outlive the view and can be reused for the next commit"
        );
        assert!(
            !text(&View::new(&diff, &mut settings, &Palette::default(), false).lines).contains("[-i32-]"),
            "reopening retains the word setting"
        );
        Ok(())
    }

    #[test]
    fn modifications_can_be_hidden_from_the_structural_diff_and_file_tree() -> TestResult {
        let repository = repository()?;
        let diff = prepare_commit_with_repository(&repository, target(&repository, "tip", 0)?, "updated".into())?;
        let mut settings = crate::App::new(24).structural_diff_settings;
        settings.colored = false;
        let colors = Palette::default();
        let press = |letter| KeyEvent::new(KeyCode::Char(letter), KeyModifiers::NONE);
        let filtered = {
            let mut view = View::new(&diff, &mut settings, &colors, false);
            let original = text(&view.lines);
            assert!(
                original.contains("unsupported.txt") && original.contains("~name"),
                "modifications are visible by default"
            );
            assert!(!view.key(press('m')), "the toggle belongs to the View menu");
            assert!(
                view.key(press('v')) && view.key(press('m')),
                "View accepts the filter key"
            );
            let output = text(&view.lines);
            for hidden in ["unsupported.txt", "Cargo.toml", "README.md", "executable.rs"] {
                assert!(!output.contains(hidden), "hide the modification-only file {hidden}");
            }
            for kept in [
                "A added.rs",
                "D removed.rs",
                "R moved.rs",
                "M modified.rs",
                "[renamed: old -> new]",
                "pub struct Config",
            ] {
                assert!(output.contains(kept), "keep {kept} in the filtered view");
            }
            assert!(view.key(press('m')), "the View menu stays open");
            assert_eq!(text(&view.lines), original, "a second toggle restores the full diff");
            assert!(
                view.key(press('m')),
                "the filter can be turned back on before reopening"
            );
            output
        };
        let reopened = View::new(&diff, &mut settings, &colors, false);
        assert_eq!(text(&reopened.lines), filtered, "the filter persists across views");
        Ok(())
    }

    #[test]
    fn whole_worktree_structural_diff_links_both_versions_of_a_path() -> TestResult {
        let repository = repository()?;
        let changes = crate::load_worktree_diff_changes(&repository)?;
        let diff = prepare_changes(&repository, &changes, "Worktree".into());
        let (rows, links, _) = diff.render(&Options::default(), &Palette::PLAIN, false);
        let linked_header = |label: &str| {
            links
                .iter()
                .find(|link| rows[link.row].0.to_string().contains(label))
                .map(|link| rows[link.target].0.to_string())
        };
        for group in ["Staged", "Unstaged"] {
            let label = format!("modified.rs · {group}");
            assert_eq!(
                linked_header(&label),
                Some(format!("diff --git a/src/modified.rs b/src/modified.rs · {group}")),
                "each stage of a path has a distinct jump target"
            );
        }
        let output = text(&rows);
        for content in ["value: u64", "value: u128", "+fn untracked()"] {
            assert!(
                output.contains(content),
                "the whole outline contains {content}: {output}"
            );
        }
        Ok(())
    }

    #[test]
    fn production_only_hides_non_production_files_and_declarations() -> TestResult {
        let repository = production_repository()?;
        let diff = prepare_commit_with_repository(&repository, target(&repository, "tip", 0)?, "updated".into())?;
        let mut settings = Settings::default();
        let colors = Palette::default();
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(160, 24))?;
        {
            let mut view = View::new(&diff, &mut settings, &colors, false);
            terminal.draw(|frame| view.draw(frame, frame.area(), 0, 0))?;
            assert!(
                text(&view.lines).starts_with("Changed files (13)"),
                "all files are visible by default"
            );
            view.toggle(ViewOption::Production);
            let output = text(&view.lines);
            terminal.draw(|frame| view.draw(frame, frame.area(), 0, 0))?;
            let buffer = terminal.backend().buffer();
            let screen: String = (0..24)
                .flat_map(|y| (0..160).map(move |x| buffer[(x, y)].symbol()))
                .collect();
            assert!(
                output.starts_with("Changed files (5)"),
                "the overview counts only visible files: {output}"
            );
            for hidden in [
                "test-only.rs",
                "unit.rs",
                "test-mode.rs",
                "integration.rs",
                "benchmark.rs",
                "compare.rs",
                "demo.rs",
                "input.rs",
                "mod tests",
                "fn smoke",
                "test_helper",
            ] {
                assert!(
                    !output.contains(hidden),
                    "{hidden} is absent from the tree and every file section: {output}"
                );
                assert!(
                    !screen.contains(hidden),
                    "filtered names are erased from the rendered screen: {hidden}"
                );
            }
            for kept in [
                "mixed.rs",
                "mode.rs",
                "moved.rs",
                "guide.md",
                "unknown.txt",
                "pub fn production",
            ] {
                assert!(
                    output.contains(kept),
                    "production and unclassified changes remain visible: {kept}"
                );
            }
            for hidden in ["benches/", "benchmarks/", "examples/", "fixtures/", "tests/"] {
                assert!(
                    !output.contains(hidden),
                    "directories emptied by filtering are absent: {hidden}"
                );
            }
            assert!(
                output.contains("rename from src/old.rs"),
                "production file renames remain visible"
            );
            assert!(
                output.contains("new mode 100755"),
                "production mode-only changes retain their headers"
            );
            assert!(
                output.contains("Outline unavailable:"),
                "unknown file types retain an explicit outline status"
            );
            view.toggle(ViewOption::Renames);
            view.toggle(ViewOption::Words);
            assert!(
                !text(&view.lines).contains("test-only.rs"),
                "other settings preserve the production filter"
            );
            view.toggle(ViewOption::Production);
            assert!(
                text(&view.lines).starts_with("Changed files (13)"),
                "p restores the complete diff"
            );
            assert!(
                text(&view.lines).contains("integration.rs"),
                "hidden files are cached for restoration"
            );
            view.toggle(ViewOption::Production);
        }
        assert!(
            settings.options.production_only,
            "the production setting outlives the view"
        );
        let view = View::new(&diff, &mut settings, &colors, false);
        assert!(
            text(&view.lines).starts_with("Changed files (5)"),
            "reopening retains the filter"
        );
        Ok(())
    }

    #[test]
    fn production_paths_remain_visible_when_renamed_across_the_filter_boundary() -> TestResult {
        let repository = production_repository()?;
        let diff = prepare_commit_with_repository(&repository, target(&repository, "tip", 0)?, "updated".into())?;
        let mut file = diff
            .files
            .into_iter()
            .find(|file| file.path.path == "src/moved.rs")
            .expect("the fixture has a mode-preserving production file rename");
        let options = Options {
            production_only: true,
            ..Options::default()
        };
        for (kind, source, path, visible) in [
            (ChangeKind::Renamed, "src/old.rs", "tests/new.rs", true),
            (ChangeKind::Renamed, "tests/old.rs", "src/new.rs", true),
            (ChangeKind::Renamed, "tests/old.rs", "benches/new.rs", false),
            (ChangeKind::Copied, "src/old.rs", "tests/new.rs", false),
            (ChangeKind::Copied, "tests/old.rs", "src/new.rs", true),
        ] {
            file.path.kind = kind;
            file.path.source = Some(source.into());
            file.path.path = path.into();
            assert_eq!(
                file.visible(&options),
                visible,
                "production renames remain visible, while non-production copies are hidden"
            );
        }
        Ok(())
    }

    #[test]
    fn test_only_file_changes_leave_an_empty_production_view() -> TestResult {
        let repository = production_repository()?;
        let diff = prepare_commit_with_repository(&repository, target(&repository, "tip", 0)?, "updated".into())?;
        let file = diff
            .files
            .into_iter()
            .find(|file| file.path.path == "src/test-only.rs")
            .expect("the fixture changes tests within an otherwise unchanged production file");
        let diff = Diff {
            title: "test-only".into(),
            files: vec![file],
        };
        let options = Options {
            production_only: true,
            ..Options::default()
        };
        assert_eq!(
            text(&diff.render(&options, &Palette::default(), false).0),
            "Changed files (0)  +0 -0\nNo production file changes.",
            "a selected file with only test edits has no tree entry, patch header, or false parent change"
        );
        Ok(())
    }

    #[test]
    fn root_branch_and_merge_parent_scopes_use_the_selected_trees() -> TestResult {
        let repository = repository()?;
        let root = prepare_commit_with_repository(&repository, target(&repository, "base", 0)?, "root".into())?;
        assert!(
            root.files.iter().all(|file| file.path.kind == ChangeKind::Added),
            "root commits compare with an empty tree"
        );
        let branch = prepare_commit_with_repository(
            &repository,
            TreeDiffTarget::Branch {
                base: repository.rev_parse_single("base")?.detach(),
                tip: repository.rev_parse_single("tip")?.detach(),
            },
            "base..tip".into(),
        )?;
        let commit = prepare_commit_with_repository(&repository, target(&repository, "tip", 0)?, "tip".into())?;
        assert_eq!(
            text(&branch.render(&Options::default(), &Palette::default(), false).0),
            text(&commit.render(&Options::default(), &Palette::default(), false).0),
            "branch and commit comparisons use the same selected base"
        );
        let first = prepare_commit_with_repository(&repository, target(&repository, "main", 0)?, "merge".into())?;
        assert_eq!(first.files.len(), 1, "the first parent only lacks the side file");
        assert_eq!(first.files[0].path.path, "side/parent.rs");
        let second = prepare_commit_with_repository(&repository, target(&repository, "main", 1)?, "merge".into())?;
        assert_eq!(
            text(&second.render(&Options::default(), &Palette::default(), false).0),
            text(&commit.render(&Options::default(), &Palette::default(), false).0),
            "the second parent sees the main-side changes"
        );
        Ok(())
    }

    #[test]
    fn staged_unstaged_and_untracked_resources_remain_distinct() -> TestResult {
        let repository = repository()?;
        let path = BString::from("src/modified.rs");
        let old_blob_id = repository.rev_parse_single("HEAD:src/modified.rs")?.detach();
        let index = repository.index()?;
        let staged_blob_id = index
            .entry_by_path(path.as_bstr())
            .expect("the fixture stages the file")
            .id;
        let resource = |blob_id, path: BString| DiffResource {
            id: blob_id,
            mode: gix::objs::tree::EntryKind::Blob.into(),
            path,
        };
        let old = resource(old_blob_id, path.clone());
        let path_change = PathChange {
            kind: ChangeKind::Modified,
            group: ChangeGroup::Staged,
            source: None,
            path: path.clone(),
            lines: None,
        };
        let staged = File::prepare(
            &repository,
            &FileChange::Worktree {
                old: Some(old.clone()),
                new: Some(resource(staged_blob_id, path.clone())),
            },
            &path_change,
        );
        let options = Options {
            word_diff: true,
            ..Options::default()
        };
        let staged_output = text(
            &Diff {
                title: "staged".into(),
                files: vec![staged],
            }
            .render(&options, &Palette::PLAIN, false)
            .0,
        );
        assert!(
            staged_output.contains("{+u64+}") && !staged_output.contains("u128"),
            "the staged blob wins over later worktree edits: {staged_output}"
        );
        let unstaged = File::prepare(
            &repository,
            &FileChange::Worktree {
                old: Some(old),
                new: Some(resource(repository.object_hash().null(), path)),
            },
            &path_change,
        );
        let unstaged_output = text(
            &Diff {
                title: "unstaged".into(),
                files: vec![unstaged],
            }
            .render(&options, &Palette::PLAIN, false)
            .0,
        );
        assert!(
            unstaged_output.contains("{+u128+}"),
            "unstaged comparisons read the current working file"
        );
        let path = BString::from("src/untracked.rs");
        let path_change = PathChange {
            kind: ChangeKind::Added,
            group: ChangeGroup::Unstaged,
            source: None,
            path: path.clone(),
            lines: None,
        };
        let untracked = File::prepare(
            &repository,
            &FileChange::Worktree {
                old: None,
                new: Some(resource(repository.object_hash().null(), path)),
            },
            &path_change,
        );
        let output = text(
            &Diff {
                title: "untracked".into(),
                files: vec![untracked],
            }
            .render(&options, &Palette::PLAIN, false)
            .0,
        );
        assert!(
            output.contains("--- /dev/null") && output.contains("+fn untracked"),
            "untracked files show structural additions: {output}"
        );
        assert!(!output.contains("secret_"), "untracked function bodies remain hidden");
        Ok(())
    }
}
