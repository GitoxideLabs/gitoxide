use std::{collections::BTreeMap, path::Path};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use gix::{
    Result,
    bstr::{BStr, BString, ByteSlice},
    error::{OptionExt, ResultExt, bail, message},
};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::{
    FileChange, OBJECT_CACHE_SIZE,
    app::{ChangeKind, PathChange, TreeDiffTarget},
    diff_colors::Palette,
};

pub(crate) mod outline;

use outline::{Change, Language, Options, PreparedDiff};

pub(crate) struct Diff {
    title: BString,
    files: Vec<File>,
}

struct File {
    path: PathChange,
    headers: Vec<String>,
    language: Option<Language>,
    outline: std::result::Result<PreparedDiff, String>,
}

pub(crate) fn prepare_file(repository_path: &Path, bare: bool, change: &FileChange, path: &PathChange) -> Result<Diff> {
    let mut repository = crate::open_repository(repository_path, bare, false)?;
    repository.object_cache_size(OBJECT_CACHE_SIZE);
    Ok(Diff {
        title: format!(
            "Structural diff · {} {}",
            path.kind.letter(),
            quoted(path.path.as_bstr(), "")
        )
        .into(),
        files: vec![File::prepare(&repository, change, path)],
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
    let changes = crate::load_changes_with_diff_options(
        repository,
        target,
        Some(gix::diff::Options::default().with_rewrites(Some(Default::default()))),
    )?;
    Ok(Diff {
        title: format!("Structural diff · {}", title.to_str_lossy()).into(),
        files: changes
            .diffs
            .iter()
            .zip(&changes.paths)
            .map(|(change, path)| File::prepare(repository, change, path))
            .collect(),
    })
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
            let language =
                language.ok_or_raise(|| message("Unsupported structural language; supported: Rust, TOML, Markdown"))?;
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
        Self {
            path: path.clone(),
            headers: file_headers(path, change),
            language,
            outline,
        }
    }
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
    files: BTreeMap<BString, usize>,
}

impl FileTree {
    fn render(&self, files: &[File], prefix: &str, lines: &mut Vec<Line<'static>>, colors: &Palette) {
        let count = self.directories.len() + self.files.len();
        for (index, (name, tree)) in self.directories.iter().enumerate() {
            let last = index + 1 == count;
            lines.push(Line::styled(
                format!(
                    "{prefix}{} {}/",
                    if last { "└──" } else { "├──" },
                    quoted(name.as_bstr(), "")
                ),
                colors.metadata,
            ));
            tree.render(
                files,
                &format!("{prefix}{}", if last { "    " } else { "│   " }),
                lines,
                colors,
            );
        }
        for (index, (name, file)) in self.files.iter().enumerate() {
            let path = &files[*file].path;
            let last = self.directories.len() + index + 1 == count;
            let mut spans = vec![
                Span::raw(format!("{prefix}{} ", if last { "└──" } else { "├──" })),
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
            lines.push(Line::from(spans));
        }
    }
}

impl Diff {
    fn render(&self, options: &Options, colors: &Palette) -> Vec<(Line<'static>, Option<Language>)> {
        let mut lines = vec![Line::styled(
            format!("Changed files ({})", self.files.len()),
            colors.header,
        )];
        if self.files.is_empty() {
            lines.push(Line::from("No file changes."));
        }
        let mut tree = FileTree::default();
        for (index, file) in self.files.iter().enumerate() {
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
            }
            node.files.insert(BString::from(*name), index);
        }
        tree.render(&self.files, "", &mut lines, colors);
        let mut rows = lines.into_iter().map(|line| (line, None)).collect::<Vec<_>>();
        for file in &self.files {
            rows.push((Line::default(), None));
            rows.extend(file.headers.iter().map(|header| {
                (
                    Line::styled(
                        header.clone(),
                        if header.starts_with("diff --git") {
                            colors.header
                        } else {
                            colors.metadata
                        },
                    ),
                    None,
                )
            }));
            match &file.outline {
                Ok(outline) => {
                    let rendered = outline.render(options);
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
        rows
    }
}

fn structural_line(line: outline::Line, colors: &Palette) -> Line<'static> {
    let mut spans = vec![Span::styled(
        line.change.marker().to_string(),
        colors.marker(line.change),
    )];
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

pub(crate) struct View<'a> {
    diff: &'a Diff,
    options: &'a mut Options,
    lines: Vec<(Line<'static>, Option<Language>)>,
    max_width: usize,
    colors: &'a Palette,
}

impl<'a> View<'a> {
    pub(crate) fn new(diff: &'a Diff, options: &'a mut Options, colors: &'a Palette) -> Self {
        let lines = diff.render(options, colors);
        let max_width = lines.iter().map(|(line, _)| line.width()).max().unwrap_or_default();
        Self {
            diff,
            options,
            lines,
            max_width,
            colors,
        }
    }

    pub(crate) fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub(crate) fn max_width(&self) -> usize {
        self.max_width
    }

    pub(crate) fn key(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press || key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
            return false;
        }
        match key.code {
            KeyCode::Char('r') => self.options.find_renames = !self.options.find_renames,
            KeyCode::Char('w') => self.options.word_diff = !self.options.word_diff,
            _ => return false,
        }
        self.lines = self.diff.render(self.options, self.colors);
        self.max_width = self
            .lines
            .iter()
            .map(|(line, _)| line.width())
            .max()
            .unwrap_or_default();
        true
    }

    pub(crate) fn draw(&self, frame: &mut Frame<'_>, area: Rect, offset: usize, horizontal_offset: usize) {
        let footer = format!(
            "r declaration renames:{} · w words:{} · ↑↓/jk · g/G first/last · h/l pan · <enter>/q/Esc back",
            if self.options.find_renames { "on" } else { "off" },
            if self.options.word_diff { "on" } else { "off" },
        );
        crate::ui::draw_diff(
            frame,
            area,
            &self.diff.title.to_str_lossy(),
            self.lines
                .iter()
                .map(|(line, language)| self.colors.highlight(line, *language)),
            (offset, horizontal_offset),
            &footer,
        );
    }
}

#[cfg(test)]
mod tests {
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
    fn changed_declarations_have_a_background_and_stronger_word_emphasis() -> TestResult {
        let outline = PreparedDiff::new("fn f(arg: i32) {}", "fn f(arg: u32) {}", Language::Rust)?;
        let changed = outline
            .render(&Options {
                word_diff: true,
                ..Options::default()
            })
            .into_iter()
            .find(|line| line.change == Change::Modified)
            .expect("the changed signature has a modified row");
        let colors = Palette::new([220; 3], [20; 3], None);
        let line = structural_line(changed, &colors);
        assert!(
            line.style.bg.is_some(),
            "changed declarations have a subtle line background"
        );
        for word in ["[-i32-]", "{+u32+}"] {
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
    fn continuous_diff_starts_with_the_file_tree_and_keeps_every_changed_file() -> TestResult {
        let repository = repository()?;
        let diff = prepare_commit_with_repository(&repository, target(&repository, "tip", 0)?, "updated".into())?;
        let lines = diff.render(
            &Options {
                find_renames: true,
                word_diff: true,
                ..Options::default()
            },
            &Palette::default(),
        );
        let output = text(&lines);
        let first_header = output.find("diff --git").expect("changed files have patch headers");
        let overview = &output[..first_header];
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
    fn live_options_change_declarations_and_keep_file_renames_enabled() -> TestResult {
        let repository = repository()?;
        let diff = prepare_commit_with_repository(&repository, target(&repository, "tip", 0)?, "updated".into())?;
        let mut options = Options {
            find_renames: true,
            word_diff: true,
            ..Options::default()
        };
        let original = text(&diff.render(&options, &Palette::default()));
        assert!(
            original.contains("[renamed: old -> new]"),
            "declaration matching starts enabled"
        );
        assert!(original.contains("[-i32-]{+u32+}"), "word changes start enabled");
        {
            let colors = Palette::default();
            let mut view = View::new(&diff, &mut options, &colors);
            assert!(view.key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE)));
            let output = text(&view.lines);
            assert!(
                !output.contains("[renamed:"),
                "r redraws declaration additions and removals"
            );
            assert!(
                output.contains("R moved.rs ← src/old.rs"),
                "file rename detection is independent of the live setting"
            );
            assert!(
                !view.key(KeyEvent::new_with_kind(
                    KeyCode::Char('r'),
                    KeyModifiers::NONE,
                    KeyEventKind::Repeat
                )),
                "holding a setting key does not oscillate it"
            );
            assert!(
                !view.key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL)),
                "modified keys do not toggle settings"
            );
            view.key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE));
            view.key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));
            let output = text(&view.lines);
            assert!(
                output.contains("[renamed: old -> new]"),
                "declaration matching can be restored"
            );
            assert!(!output.contains("[-i32-]"), "w redraws separate declaration lines");
        }
        assert!(
            !options.word_diff,
            "settings outlive the view and can be reused for the next commit"
        );
        assert!(
            !text(&View::new(&diff, &mut options, &Palette::default()).lines).contains("[-i32-]"),
            "reopening retains the word setting"
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
            text(&branch.render(&Options::default(), &Palette::default())),
            text(&commit.render(&Options::default(), &Palette::default())),
            "branch and commit comparisons use the same selected base"
        );
        let first = prepare_commit_with_repository(&repository, target(&repository, "main", 0)?, "merge".into())?;
        assert_eq!(first.files.len(), 1, "the first parent only lacks the side file");
        assert_eq!(first.files[0].path.path, "side/parent.rs");
        let second = prepare_commit_with_repository(&repository, target(&repository, "main", 1)?, "merge".into())?;
        assert_eq!(
            text(&second.render(&Options::default(), &Palette::default())),
            text(&commit.render(&Options::default(), &Palette::default())),
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
            .render(&options, &Palette::default()),
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
            .render(&options, &Palette::default()),
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
            .render(&options, &Palette::default()),
        );
        assert!(
            output.contains("--- /dev/null") && output.contains("+fn untracked"),
            "untracked files show structural additions: {output}"
        );
        assert!(!output.contains("secret_"), "untracked function bodies remain hidden");
        Ok(())
    }
}
