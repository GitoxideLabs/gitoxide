use gix::bstr::{BStr, BString, ByteSlice};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Margin, Position, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap},
};

use crate::{
    BuiltInDiff,
    app::{
        Alignment as HistoryAlignment, App, AttributionKind, ChangeGroup, ChangeKind, ChangePane, Changes,
        ChangesLayout, ChangesMode, CommitRow, CopyKind, DateMode, HistoryEntry, IdMode, NameMode, Notice, NoticeKind,
        RefMode, SelectionRelation, SignatureState, State,
    },
    command_menu::{self, Command, CommandGroup, CommandId},
    history::{DecorationKind, Decorations},
    menu::{MAX_VISIBLE_ROWS, Menu},
};

const COMPARED_PARENT_COLOR: Color = Color::Cyan;
const COMMIT_PANE_WIDTH: u16 = 84;
const FILESYSTEM_NOTIFICATION_COLOR: Color = Color::Rgb(255, 165, 0);
const NOTE_COLOR: Color = Color::LightMagenta;
const PANE_STATUS_BACKGROUND: Color = Color::DarkGray;
const REVIEW_BACKGROUND: Color = Color::Magenta;

#[derive(Clone)]
struct MarkdownStyle;

impl tui_markdown::StyleSheet for MarkdownStyle {
    fn heading_marker(&self, _level: u8) -> &'static str {
        ""
    }

    fn code_block_fence(&self) -> &'static str {
        ""
    }
}

fn note_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(Color::Yellow)
        .remove_modifier(Modifier::REVERSED)
}

pub(crate) fn notice_area(notice: &Notice, horizontal: Rect, top: u16, bottom: u16) -> Option<Rect> {
    if horizontal.width == 0 || top >= bottom {
        return None;
    }
    let horizontal = horizontal.inner(Margin {
        horizontal: 2.min(horizontal.width.saturating_sub(1) / 2),
        vertical: 0,
    });
    let height = u16::try_from(
        Paragraph::new(notice.text.as_str())
            .wrap(Wrap { trim: false })
            .line_count(horizontal.width),
    )
    .unwrap_or(u16::MAX)
    .max(1)
    .min(bottom.saturating_sub(top));
    Some(Rect::new(
        horizontal.x,
        bottom.saturating_sub(height),
        horizontal.width,
        height,
    ))
}

pub(crate) fn render_notice(frame: &mut Frame<'_>, area: Rect, notice: &Notice) {
    let background = match notice.kind {
        NoticeKind::Success => Color::Green,
        NoticeKind::Attention => Color::Yellow,
        NoticeKind::Error => Color::LightRed,
    };
    frame.render_widget(
        Paragraph::new(notice.text.as_str()).wrap(Wrap { trim: false }).style(
            Style::default()
                .fg(Color::Black)
                .bg(background)
                .add_modifier(Modifier::BOLD),
        ),
        area,
    );
}

pub(crate) fn draw_command_menu(
    frame: &mut Frame<'_>,
    bounds: Rect,
    menu: &mut Menu<CommandId>,
    commands: &[Command],
) -> Option<Position> {
    if !menu.is_open() {
        return None;
    }
    let frame_area = bounds;
    let width = frame_area.width.saturating_sub(2).min(72);
    if width < 4 {
        menu.set_visible_rows(0);
        return None;
    }
    menu.set_visible_rows(usize::from(frame_area.height.saturating_sub(5)).min(MAX_VISIBLE_ROWS));
    let height = frame_area
        .height
        .saturating_sub(2)
        .min(u16::try_from(menu.visible_indices().len().max(1) + 3).unwrap_or(u16::MAX));
    if height < 3 {
        return None;
    }
    let area = Rect::new(
        frame_area.x + (frame_area.width - width) / 2,
        frame_area.y + (frame_area.height - height) / 2,
        width,
        height,
    );
    let block = Block::new().borders(Borders::ALL).title(" Command ");
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    let [query_area, results_area] = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(inner);
    let [prompt_area, input_area] = Layout::horizontal([Constraint::Length(2), Constraint::Min(0)]).areas(query_area);
    frame.render_widget(
        Paragraph::new("> ").style(Style::default().add_modifier(Modifier::BOLD)),
        prompt_area,
    );

    let before_cursor: String = menu.query().chars().take(menu.cursor()).collect();
    let cursor_width = Line::raw(before_cursor).width();
    let input_width = usize::from(input_area.width);
    let scroll = cursor_width.saturating_sub(input_width.saturating_sub(1));
    frame.render_widget(
        Paragraph::new(menu.query()).scroll((0, u16::try_from(scroll).unwrap_or(u16::MAX))),
        input_area,
    );

    let selected = menu.selected_visible_row();
    let mut lines = Vec::new();
    for (row, index) in menu.visible_indices().iter().copied().enumerate() {
        let command = &commands[index];
        let group = command.group.label();
        let mut shortcut = command.shortcut.chars();
        let shortcut = format!(
            "{} {}",
            shortcut.next().expect("a command shortcut has a prefix"),
            shortcut.next().expect("a command shortcut has a key")
        );
        let style = if selected == Some(row) {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        lines.push(Line::styled(
            format!("{}  {group:<11} {}  [{shortcut}]", row + 1, command.label),
            style,
        ));
    }
    if lines.is_empty() {
        lines.push(Line::styled(
            "no matching commands",
            Style::default().add_modifier(Modifier::DIM),
        ));
    }
    frame.render_widget(Paragraph::new(lines), results_area);

    (input_area.width > 0).then(|| {
        Position::new(
            input_area.x + u16::try_from(cursor_width.saturating_sub(scroll)).unwrap_or(u16::MAX),
            input_area.y,
        )
    })
}

pub(crate) fn draw_todo_progress(frame: &mut Frame<'_>, progress: crate::edit::rebase::Progress) {
    let area = frame.area();
    frame.render_widget(Clear, area);
    let width = area.width.min(72);
    let height = area.height.min(4);
    let progress_area = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    let rows = Layout::vertical([Constraint::Length(1); 4]).split(progress_area);
    frame.render_widget(
        Paragraph::new("Rebasing commits")
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::BOLD)),
        rows[0],
    );
    let ratio = if progress.total == 0 {
        1.0
    } else {
        progress.processed.min(progress.total) as f64 / progress.total as f64
    };
    frame.render_widget(
        Gauge::default()
            .ratio(ratio)
            .label(format!("{} / {} commits", progress.processed, progress.total))
            .gauge_style(Style::default().fg(Color::LightBlue)),
        rows[1],
    );
    frame.render_widget(
        Paragraph::new(format!(
            "cherry-picked {} · {:.1?}",
            progress.cherry_picked, progress.cherry_pick_time
        ))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::LightYellow)),
        rows[2],
    );
    frame.render_widget(
        Paragraph::new(format!("signed {} · {:.1?}", progress.signed, progress.signing_time))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Green)),
        rows[3],
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ChangesPaneArea {
    pane: ChangePane,
    outer: Rect,
}

fn index_separator(pane: ChangePane, changes: &Changes) -> Option<usize> {
    if pane != ChangePane::Worktree {
        return None;
    }
    let index = changes
        .paths
        .iter()
        .position(|change| change.group == ChangeGroup::Unstaged)?;
    changes.paths[..index]
        .iter()
        .any(|change| change.group == ChangeGroup::Staged)
        .then_some(index)
}

fn changes_pane_areas(
    area: Rect,
    max_height: u16,
    tree: Option<(u16, usize)>,
    worktree: Option<(u16, usize)>,
) -> (ChangesLayout, Vec<ChangesPaneArea>, u16) {
    match (tree, worktree) {
        (None, None) => (ChangesLayout::SideBySide, Vec::new(), 0),
        (Some((height, _)), None) => {
            let height = height.min(max_height);
            (
                ChangesLayout::SideBySide,
                vec![ChangesPaneArea {
                    pane: ChangePane::Tree,
                    outer: Rect::new(area.x, area.bottom().saturating_sub(height), area.width, height),
                }],
                height,
            )
        }
        (None, Some((height, _))) => {
            let height = height.min(max_height);
            (
                ChangesLayout::SideBySide,
                vec![ChangesPaneArea {
                    pane: ChangePane::Worktree,
                    outer: Rect::new(area.x, area.bottom().saturating_sub(height), area.width, height),
                }],
                height,
            )
        }
        (Some((tree_height, tree_title)), Some((worktree_height, worktree_title))) => {
            let tree_width = area.width / 2;
            let worktree_width = area.width.saturating_sub(tree_width);
            if tree_title <= usize::from(tree_width) && worktree_title <= usize::from(worktree_width) {
                let tree_height = tree_height.min(max_height);
                let worktree_height = worktree_height.min(max_height);
                let height = tree_height.max(worktree_height);
                (
                    ChangesLayout::SideBySide,
                    vec![
                        ChangesPaneArea {
                            pane: ChangePane::Tree,
                            outer: Rect::new(area.x, area.bottom().saturating_sub(height), tree_width, height),
                        },
                        ChangesPaneArea {
                            pane: ChangePane::Worktree,
                            outer: Rect::new(
                                area.x.saturating_add(tree_width),
                                area.bottom().saturating_sub(height),
                                worktree_width,
                                height,
                            ),
                        },
                    ],
                    height,
                )
            } else {
                let total = tree_height.saturating_add(worktree_height);
                let (worktree_height, tree_height) = if total <= max_height {
                    (worktree_height, tree_height)
                } else {
                    let half = max_height / 2;
                    if worktree_height <= half {
                        (worktree_height, max_height.saturating_sub(worktree_height))
                    } else if tree_height <= half {
                        (max_height.saturating_sub(tree_height), tree_height)
                    } else {
                        (half.saturating_add(max_height % 2), half)
                    }
                };
                let height = worktree_height.saturating_add(tree_height);
                let tree_y = area.bottom().saturating_sub(tree_height);
                (
                    ChangesLayout::Stacked,
                    vec![
                        ChangesPaneArea {
                            pane: ChangePane::Worktree,
                            outer: Rect::new(
                                area.x,
                                tree_y.saturating_sub(worktree_height),
                                area.width,
                                worktree_height,
                            ),
                        },
                        ChangesPaneArea {
                            pane: ChangePane::Tree,
                            outer: Rect::new(area.x, tree_y, area.width, tree_height),
                        },
                    ],
                    height,
                )
            }
        }
    }
}

pub(crate) fn draw_file_diff(
    frame: &mut Frame<'_>,
    area: Rect,
    diff: &BuiltInDiff,
    offset: usize,
    horizontal_offset: usize,
) {
    let [header, body, footer] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)]).areas(area);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(diff.title.to_str_lossy()).style(Style::default().add_modifier(Modifier::BOLD)),
        header,
    );
    let mut lines = diff
        .lines
        .iter()
        .map(|line| {
            let style = if line.starts_with(b"@@") {
                Style::default().fg(Color::Cyan)
            } else if line.starts_with(b"+") {
                Style::default().fg(Color::Green)
            } else if line.starts_with(b"-") {
                Style::default().fg(Color::LightRed)
            } else if line.starts_with(b"Binary ") {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            Line::styled(line.to_str_lossy(), style)
        })
        .collect::<Vec<_>>();
    if let Some(summary) = &diff.summary {
        lines.splice(0..0, summary.iter().cloned().chain(std::iter::once(Line::default())));
    }
    frame.render_widget(
        Paragraph::new(Text::from(lines)).scroll((
            u16::try_from(offset).unwrap_or(u16::MAX),
            u16::try_from(horizontal_offset).unwrap_or(u16::MAX),
        )),
        body,
    );
    frame.render_widget(
        Paragraph::new("↑↓/jk move · h/l pan · <enter>/q/Esc back").style(Style::default().add_modifier(Modifier::DIM)),
        footer,
    );
}

#[cfg(test)]
pub(crate) fn draw(
    frame: &mut Frame<'_>,
    app: &mut App,
    decorations: &Decorations,
    mailmap: &gix::mailmap::Snapshot,
    commit_message: Option<&BStr>,
    tree_changes: Option<&Changes>,
) {
    let area = frame.area();
    draw_with_worktree(
        frame,
        area,
        app,
        decorations,
        mailmap,
        commit_message,
        tree_changes,
        None,
    );
}

#[expect(
    clippy::too_many_arguments,
    reason = "the bounds extend the existing drawing context"
)]
pub(crate) fn draw_with_worktree(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &mut App,
    decorations: &Decorations,
    mailmap: &gix::mailmap::Snapshot,
    commit_message: Option<&BStr>,
    tree_changes: Option<&Changes>,
    worktree_changes: Option<&Changes>,
) {
    let background_progress = app.background_progress().cloned();
    let [mut body, progress_area, footer] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(u16::from(background_progress.is_some())),
        Constraint::Length(1),
    ])
    .areas(area);
    let full_body = body;
    let selected_segment = app.selected_is_segment();
    let time_travel_animation = app.time_travel_animation_origin().is_some();
    let compared_parent = if app.changes_visible() && !selected_segment {
        tree_changes.and_then(|changes| changes.parent.map(|parent| parent.id))
    } else {
        None
    };
    let tree_shown = app.changes_visible() && !selected_segment && tree_changes.is_some();
    let worktree_shown =
        app.changes_visible() && app.changes_mode == Some(ChangesMode::Both) && worktree_changes.is_some();
    let tree_summary = tree_changes.map(|changes| changes_summary(ChangePane::Tree, app, changes));
    let worktree_summary = worktree_changes.map(|changes| changes_summary(ChangePane::Worktree, app, changes));
    let mut commit_pane = (app.show_commit && !selected_segment).then(|| {
        let width = COMMIT_PANE_WIDTH.min(full_body.width / 2);
        let [commits, message] = Layout::horizontal([Constraint::Min(0), Constraint::Length(width)]).areas(full_body);
        body.width = body.width.min(commits.width);
        let content = message.inner(Margin {
            horizontal: 2,
            vertical: 1,
        });
        (message, content)
    });
    let pane_height = |pane, changes: &Changes| {
        u16::try_from(changes.paths.len())
            .unwrap_or(u16::MAX)
            .saturating_add(u16::from(index_separator(pane, changes).is_some()))
            .saturating_add(2)
    };
    let (changes_layout, mut changes_panes, _) = changes_pane_areas(
        body,
        area.height / 2,
        tree_shown.then(|| {
            (
                pane_height(ChangePane::Tree, tree_changes.expect("visible tree changes exist")),
                tree_summary.as_ref().map_or(0, Line::width),
            )
        }),
        worktree_shown.then(|| {
            (
                pane_height(
                    ChangePane::Worktree,
                    worktree_changes.expect("visible worktree changes exist"),
                ),
                worktree_summary.as_ref().map_or(0, Line::width),
            )
        }),
    );
    let notice = app.notice();
    let worktree_pane = changes_panes.iter().find(|pane| pane.pane == ChangePane::Worktree);
    let notice_horizontal = worktree_pane.map_or(body, |pane| pane.outer);
    let notice_bottom = worktree_pane.map_or_else(
        || {
            changes_panes
                .iter()
                .map(|pane| pane.outer.y)
                .min()
                .unwrap_or(body.bottom())
        },
        |pane| pane.outer.y,
    );
    let mut notice_area = notice
        .as_ref()
        .and_then(|notice| notice_area(notice, notice_horizontal, body.y, notice_bottom));
    if let Some(notice_area) = notice_area {
        body.height = notice_area.y.saturating_sub(body.y);
    }
    let history_changes_panes = changes_panes.clone();
    let history_notice_area = notice_area;
    if let Some(changes) = worktree_changes {
        app.set_worktree_conflicted(changes.paths.iter().any(|change| change.kind == ChangeKind::Unmerged));
    }
    app.set_new_commit_availability(
        (app.changes_mode == Some(ChangesMode::Both))
            .then_some(worktree_changes)
            .flatten(),
    );
    let selected_is_head = app.selected.and_then(|index| app.rows.get(index)).is_some_and(|row| {
        decorations
            .get(&row.id)
            .is_some_and(|refs| refs.iter().any(|r| r.kind == DecorationKind::Head))
    });
    let selected_has_stash = app
        .selected
        .and_then(|index| app.rows.get(index))
        .and_then(|row| decorations.get(&row.id))
        .is_some_and(|refs| refs.iter().any(|reference| reference.kind == DecorationKind::Stash));
    let stashable = selected_is_head
        && !selected_has_stash
        && worktree_changes.is_some_and(|changes| {
            !changes.paths.is_empty() && !changes.paths.iter().any(|change| change.kind == ChangeKind::Unmerged)
        });
    let worktree_path_amend = worktree_changes.is_some_and(|changes| {
        !changes.paths.iter().any(|change| change.kind == ChangeKind::Unmerged)
            && changes.paths.get(app.worktree_changes.selected).is_some()
    });
    app.set_head_edit_availability(
        selected_is_head && worktree_changes.is_some_and(|changes| !changes.paths.is_empty()),
        stashable,
        selected_is_head && selected_has_stash,
        selected_is_head && worktree_path_amend,
        worktree_changes.is_some_and(|changes| changes.paths.is_empty()),
        selected_is_head && tree_changes.is_some_and(|changes| !changes.paths.is_empty()),
        selected_is_head
            && worktree_changes.is_some_and(|changes| {
                changes.paths.iter().any(|change| change.group == ChangeGroup::Staged)
                    && changes.paths.iter().any(|change| change.group == ChangeGroup::Unstaged)
                    && !changes.paths.iter().any(|change| change.kind == ChangeKind::Unmerged)
            }),
    );
    if app.changes_visible() {
        app.set_changes_layout(
            changes_layout,
            changes_panes
                .iter()
                .any(|pane| pane.pane == ChangePane::Tree && pane.outer.height > 0)
                && tree_changes.is_some_and(Changes::is_visible),
            changes_panes
                .iter()
                .any(|pane| pane.pane == ChangePane::Worktree && pane.outer.height > 0)
                && worktree_changes.is_some_and(Changes::is_visible),
        );
    }
    app.viewport_rows = history_changes_panes
        .iter()
        .map(|pane| pane.outer.y.saturating_sub(body.y))
        .chain(history_notice_area.map(|area| area.y.saturating_sub(body.y)))
        .min()
        .unwrap_or(body.height)
        .max(1) as usize;
    app.center_initial_selection();
    app.prepare_history_viewport();
    let commands =
        if app.history_display_expanded || app.actions_expanded || app.enrich_expanded || app.information_expanded {
            command_menu::commands(app, decorations, app.has_verifiable_signatures())
        } else {
            Vec::new()
        };
    let focus_feedback = app.focus_feedback.take();
    let mut prefix_popup = active_prefix_popup(
        app,
        decorations,
        &commands,
        focus_feedback,
        footer.width.saturating_sub(2) as usize,
    );
    let popup_rows = prefix_popup.as_ref().map_or(0, |(_, rows)| rows.len());
    let mut prefix_popup_allowed = prefix_popup
        .as_ref()
        .is_some_and(|(anchor, _)| prefix_popup_can_render(area, footer, *anchor, popup_rows));
    if prefix_popup_allowed {
        let popup_y = footer.y - popup_rows as u16;
        let shifted_y = |area: Rect| area.y.saturating_sub(popup_rows as u16).max(full_body.y);
        prefix_popup_allowed = changes_panes.iter().all(|pane| popup_y > shifted_y(pane.outer))
            && commit_pane.as_ref().is_none_or(|(outer, _)| popup_y > outer.y)
            && notice_area.is_none_or(|area| popup_y.saturating_sub(shifted_y(area)) >= area.height);
        if prefix_popup_allowed {
            for pane in &mut changes_panes {
                pane.outer.y = shifted_y(pane.outer);
                pane.outer.height = pane.outer.height.min(popup_y.saturating_sub(pane.outer.y));
            }
            if let Some(area) = notice_area.as_mut() {
                area.y = shifted_y(*area);
                area.height = area.height.min(popup_y.saturating_sub(area.y));
            }
            if let Some((outer, content)) = commit_pane.as_mut() {
                outer.height = outer.height.min(popup_y.saturating_sub(outer.y));
                *content = outer.inner(Margin {
                    horizontal: 2,
                    vertical: 1,
                });
            }
        }
    }
    // Keep pane content above the popup while its underlay still covers the history row beneath it.
    let changes_pane_underlays = changes_panes
        .iter()
        .zip(&history_changes_panes)
        .map(|(pane, unshifted)| {
            let mut area = pane.outer;
            if prefix_popup_allowed {
                area.height = unshifted.outer.bottom().saturating_sub(area.y);
            }
            area
        })
        .collect::<Vec<_>>();
    let worktree_dirty = worktree_changes.is_some_and(|changes| !changes.paths.is_empty())
        && changes_panes
            .iter()
            .any(|pane| pane.pane == ChangePane::Worktree && pane.outer.height > 0);
    let start = app.offset.min(app.history_len());
    let render_end = start.saturating_add(body.height as usize).min(app.history_len());
    let visible_entries: Vec<_> = (start..render_end)
        .filter_map(|index| app.history_entry(index))
        .collect();
    let lanes = app.render_lanes(start..render_end);
    let enrichment_gutter = Line::raw(crate::enrich::marker(true, true, true)).width() as u16;
    let has_duplicate_change_id = app.has_duplicate_change_ids();
    let change_id_gutter = if has_duplicate_change_id {
        Line::raw("👯‍♂️").width() as u16
    } else {
        0
    };
    let has_conflict = app.has_conflict_marker();
    let conflict_gutter = if has_conflict {
        Line::raw("💥").width() as u16
    } else {
        0
    };
    let status_x = body
        .x
        .saturating_add(enrichment_gutter)
        .saturating_add(change_id_gutter)
        .saturating_add(conflict_gutter);
    let content = Rect::new(
        status_x.saturating_add(2),
        body.y,
        body.width.saturating_sub(
            enrichment_gutter
                .saturating_add(change_id_gutter)
                .saturating_add(conflict_gutter)
                .saturating_add(2),
        ),
        body.height,
    );
    let requested_alignment = app.alignment;
    let aligned_lane_width = |index: usize| lane_width(lanes.lane(index), requested_alignment);
    let rendered_lane_width = lanes
        .iter()
        .enumerate()
        .map(|(index, _)| aligned_lane_width(index))
        .max()
        .unwrap_or_default();
    let max_lane_width = if rendered_lane_width == 0 {
        app.estimated_lane_width
    } else {
        rendered_lane_width
    };
    let date_mode = app.date_mode;
    let id_mode = app.effective_id_mode();
    let name_mode = app.name_mode;
    let copy_feedback = app.copy_feedback.take();
    let show_author_name = copy_feedback == Some(CopyKind::Author) || name_mode != NameMode::None;
    let show_trailers = name_mode == NameMode::All && app.show_trailers;
    let ref_mode = app.ref_mode;
    let selected = app.selected_history_index();
    let build_metadata_columns = |shorten_titles: bool, compact_history: bool| {
        visible_entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let HistoryEntry::Commit(row_index) = entry else {
                    return None;
                };
                let row = &app.rows[*row_index];
                let row_selected = selected == Some(start + index);
                let note_title = row_selected
                    .then(|| app.note(row.id))
                    .flatten()
                    .map(|note| gix::objs::commit::MessageRef::from_bytes(note).title);
                let mut metadata = metadata_columns(
                    row,
                    app.title(row),
                    app.attributions(row),
                    decorations,
                    mailmap,
                    MetadataOptions {
                        date_mode,
                        id_mode,
                        change_id: app.change_id(row.id),
                        show_author_name,
                        show_emails: app.show_emails && !compact_history,
                        show_trailers,
                        has_notes: !app.notes(row.id).is_empty(),
                        note_title: if compact_history { None } else { note_title },
                        shorten_title: shorten_titles,
                        use_mailmap: app.use_mailmap && copy_feedback != Some(CopyKind::Author),
                        ref_mode,
                        selected: row_selected || compared_parent == Some(row.id),
                        copy_feedback: if row_selected { copy_feedback } else { None },
                    },
                );
                if compact_history {
                    for field in &mut metadata.fields[..5] {
                        *field = Line::default();
                    }
                    metadata.fields[4] = Line::raw(" ");
                }
                if !row_selected && let Some(decorations) = decorations.get(&row.id) {
                    let current_head = decorations
                        .iter()
                        .any(|decoration| decoration.kind == DecorationKind::Head);
                    let foreign_head = decorations.iter().any(|decoration| {
                        matches!(
                            decoration.kind,
                            DecorationKind::WorktreeBranch | DecorationKind::WorktreeDetached
                        )
                    });
                    if current_head || foreign_head {
                        for span in &mut metadata.fields[5].spans {
                            span.style = if current_head {
                                span.style.add_modifier(Modifier::REVERSED)
                            } else {
                                span.style.bg(Color::DarkGray)
                            };
                        }
                    }
                }
                Some(metadata)
            })
            .collect::<Vec<_>>()
    };
    let full_metadata_columns = build_metadata_columns(false, false);
    let title_column = lanes
        .iter()
        .enumerate()
        .zip(&full_metadata_columns)
        .filter_map(|((index, _), metadata)| {
            metadata
                .as_ref()
                .map(|metadata| aligned_lane_width(index).saturating_add(metadata.prefix_width()))
        })
        .max()
        .unwrap_or_default();
    let column_widths = full_metadata_columns
        .iter()
        .flatten()
        .fold([0; 5], |mut widths, metadata| {
            for (width, field) in widths.iter_mut().zip(&metadata.fields[..5]) {
                *width = (*width).max(field.width());
            }
            widths
        });
    let title_start = match requested_alignment {
        HistoryAlignment::None => 0,
        HistoryAlignment::Title | HistoryAlignment::Compressed => title_column,
        HistoryAlignment::Columns => max_lane_width.saturating_add(column_widths.iter().sum()),
    };
    let available_title_width = usize::from(content.width).saturating_sub(title_start);
    let visible_title_widths: Vec<_> = if app.show_emails {
        Vec::new()
    } else {
        visible_entries
            .iter()
            .filter_map(|entry| {
                let HistoryEntry::Commit(row_index) = entry else {
                    return None;
                };
                let title = app.title(&app.rows[*row_index]);
                Some((
                    Line::from(commit_title_spans(title, false)).width(),
                    Line::from(commit_title_spans(title, true)).width(),
                ))
            })
            .collect()
    };
    let shorten_titles = requested_alignment != HistoryAlignment::None
        && less_than_sixty_percent(
            available_title_width,
            visible_title_widths.iter().map(|widths| widths.0),
        );
    let compact_history = shorten_titles
        && less_than_sixty_percent(
            available_title_width,
            visible_title_widths.iter().map(|widths| widths.1),
        );
    let alignment = if compact_history {
        HistoryAlignment::None
    } else {
        requested_alignment
    };
    let displayed_lane = |index: usize| {
        let lane = lanes.lane(index);
        if requested_alignment == HistoryAlignment::None {
            lane
        } else if compact_history && matches!(visible_entries[index], HistoryEntry::Commit(_)) {
            lane_through_node(lane)
        } else {
            lane.trim_end()
        }
    };
    let metadata_columns = if shorten_titles {
        build_metadata_columns(true, compact_history)
    } else {
        full_metadata_columns
    };
    let metadata: Vec<_> = metadata_columns
        .into_iter()
        .enumerate()
        .map(|(index, metadata)| {
            metadata.map(|metadata| match alignment {
                HistoryAlignment::None => {
                    let (metadata, prefix_width) = metadata.into_line_with_prefix();
                    (metadata, 0, prefix_width)
                }
                HistoryAlignment::Title | HistoryAlignment::Compressed => {
                    let lane_width = aligned_lane_width(index);
                    let (metadata, prefix_width) = metadata.align_title(title_column.saturating_sub(lane_width));
                    (metadata, lane_width, prefix_width)
                }
                HistoryAlignment::Columns => {
                    let (metadata, prefix_width) = metadata.align_columns(column_widths);
                    (metadata, max_lane_width, prefix_width)
                }
            })
        })
        .collect();
    let max_offset = visible_entries
        .iter()
        .enumerate()
        .map(|(index, entry)| match (entry, &metadata[index]) {
            (HistoryEntry::Segment { count, .. }, _) => {
                aligned_lane_width(index).saturating_add(format!("[{count}]").chars().count())
            }
            (HistoryEntry::Commit(_), Some((metadata, metadata_x, _))) => match alignment {
                HistoryAlignment::None => displayed_lane(index).chars().count().saturating_add(metadata.width()),
                HistoryAlignment::Title | HistoryAlignment::Columns | HistoryAlignment::Compressed => {
                    metadata_x.saturating_add(metadata.width())
                }
            },
            (HistoryEntry::Commit(_), None) => 0,
        })
        .max()
        .unwrap_or_default()
        .saturating_sub(content.width as usize)
        .min(u16::MAX as usize);
    let horizontal_offset = app.horizontal_offset.min(max_offset);
    let selection_info = selection_info_line(
        (!time_travel_animation && !selected_segment && app.changes_visible())
            .then_some(tree_changes)
            .flatten()
            .filter(|changes| changes.is_visible()),
        (!time_travel_animation && !selected_segment)
            .then_some(app.selection_relation)
            .flatten(),
    );
    let selection_info_width = selection_info.width();
    let mut selection_info_area = None;

    for (index, metadata) in metadata.into_iter().enumerate() {
        let lane = displayed_lane(index);
        let y = body.y.saturating_add(index as u16);
        let row_area = Rect::new(content.x, y, content.width, 1);
        let row_index = match visible_entries[index] {
            HistoryEntry::Commit(index) => index,
            HistoryEntry::Segment { count, .. } => {
                let selected = selected_segment && selected == Some(start + index);
                let selectable = app.history_entry_selectable(start + index);
                let style = if selected {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };
                if selected {
                    frame.render_widget(
                        Paragraph::new(Line::styled("> ", style)),
                        Rect::new(status_x, y, body.right().saturating_sub(status_x).min(2), 1),
                    );
                }
                frame.render_widget(
                    Paragraph::new(Line::styled(
                        format!(
                            "{lane}{}[{count}]",
                            if requested_alignment == HistoryAlignment::None || lane.is_empty() {
                                ""
                            } else {
                                " "
                            }
                        ),
                        style,
                    ))
                    .scroll((0, horizontal_offset as u16)),
                    row_area,
                );
                color_graph(
                    frame,
                    row_area,
                    lane,
                    horizontal_offset,
                    selected.then_some(Color::Reset),
                    SignatureState::Unsigned,
                    None,
                );
                if !selectable {
                    frame.buffer_mut().set_style(
                        Rect::new(body.x, y, body.width, 1),
                        Style::default().add_modifier(Modifier::DIM),
                    );
                }
                continue;
            }
        };
        let row = &app.rows[row_index];
        let (metadata, metadata_x, metadata_prefix_width) = metadata.expect("commit history entries have metadata");
        let selected = app.selected == Some(row_index);
        let head = decorations.get(&row.id).is_some_and(|decorations| {
            decorations
                .iter()
                .any(|decoration| decoration.kind == DecorationKind::Head)
        });
        let attached_head = decorations.get(&row.id).is_some_and(|decorations| {
            decorations
                .iter()
                .any(|decoration| decoration.kind == DecorationKind::CurrentWorktreeBranch)
        });
        let head_has_descendants = app.worktree_head_has_descendants(row.id);
        let head_state = head.then_some(HeadState {
            has_descendants: head_has_descendants,
            attached: attached_head,
        });
        let metadata_width = metadata.width();
        let title_offset = match alignment {
            HistoryAlignment::None => lane.chars().count().saturating_add(metadata_prefix_width),
            HistoryAlignment::Title | HistoryAlignment::Columns | HistoryAlignment::Compressed => {
                metadata_x.saturating_add(metadata_prefix_width)
            }
        };
        let hidden_branch_behind = app.hidden_branch_behind(row.id);
        let line_width = match alignment {
            HistoryAlignment::None => lane
                .chars()
                .count()
                .saturating_add(metadata_width)
                .saturating_sub(horizontal_offset),
            HistoryAlignment::Title | HistoryAlignment::Columns | HistoryAlignment::Compressed => metadata_x
                .saturating_add(metadata_width)
                .saturating_sub(horizontal_offset),
        };
        let hidden_branch_marker = hidden_branch_behind.and_then(|behind| {
            let marker = format!("⇣{behind}");
            let width = marker.chars().count() as u16;
            (body.width > width.saturating_add(2)).then(|| {
                let natural_x = content
                    .x
                    .saturating_add(u16::try_from(line_width).unwrap_or(u16::MAX))
                    .saturating_add(1)
                    .saturating_add(if selected && selection_info_width > 0 {
                        u16::try_from(selection_info_width)
                            .unwrap_or(u16::MAX)
                            .saturating_add(3)
                    } else {
                        0
                    });
                let x = natural_x.min(body.right().saturating_sub(width).saturating_sub(1));
                (marker, x, width)
            })
        });
        let signature_color = signature_color(row.signature);
        let highlight = if selected {
            Some(signature_color)
        } else if compared_parent == Some(row.id) {
            Some(COMPARED_PARENT_COLOR)
        } else {
            None
        };
        let style = highlight.map_or_else(Style::default, |highlight| {
            color(highlight).add_modifier(Modifier::REVERSED)
        });
        let conflict = app.conflict_marker(row.id, head);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw(if app.todo(row.id) { "🚧" } else { "  " }),
                Span::raw(if app.note(row.id).is_some() { "📝" } else { "  " }),
                Span::raw(if app.checks_pass(row.id) { "✔️" } else { "  " }),
            ])),
            Rect::new(body.x, y, enrichment_gutter, 1),
        );
        if app.has_duplicate_change_id(row.id) {
            frame.render_widget(
                Paragraph::new("👯‍♂️"),
                Rect::new(body.x.saturating_add(enrichment_gutter), y, change_id_gutter, 1),
            );
        }
        if conflict {
            frame.render_widget(
                Paragraph::new("💥").style(color(Color::LightRed)),
                Rect::new(
                    body.x
                        .saturating_add(enrichment_gutter)
                        .saturating_add(change_id_gutter),
                    y,
                    conflict_gutter,
                    1,
                ),
            );
        }
        frame.render_widget(
            Paragraph::new(if head && worktree_dirty {
                "🫟"
            } else if selected {
                "> "
            } else {
                "  "
            })
            .style(if selected { style } else { Style::default() }),
            Rect::new(status_x, y, body.right().saturating_sub(status_x).min(2), 1),
        );

        let mut spans = Vec::with_capacity(metadata.spans.len() + 2);
        spans.push(Span::styled(lane, style));
        if alignment != HistoryAlignment::None {
            spans.push(Span::raw(" ".repeat(metadata_x.saturating_sub(lane.chars().count()))));
        }
        spans.extend(metadata.spans);
        frame.render_widget(
            Paragraph::new(Line::from(spans)).scroll((0, horizontal_offset as u16)),
            row_area,
        );
        color_graph(
            frame,
            row_area,
            lane,
            horizontal_offset,
            highlight,
            row.signature,
            head_state,
        );
        if selected {
            let end = if head {
                content.x.saturating_add(u16::try_from(line_width).unwrap_or(u16::MAX))
            } else if title_offset > horizontal_offset {
                content
                    .x
                    .saturating_add(u16::try_from(title_offset - horizontal_offset).unwrap_or(u16::MAX))
                    .saturating_sub(1)
            } else {
                content.x
            }
            .min(body.right());
            frame.buffer_mut().set_style(
                Rect::new(body.x, y, end.saturating_sub(body.x), 1),
                Style::default().add_modifier(Modifier::REVERSED),
            );
        }
        if row.is_review && head && title_offset > horizontal_offset {
            let end = content
                .x
                .saturating_add(u16::try_from(title_offset - horizontal_offset).unwrap_or(u16::MAX))
                .saturating_sub(1)
                .min(body.right());
            let buffer = frame.buffer_mut();
            if let Some(start) = (body.x..end).find(|x| !buffer[(*x, y)].symbol().trim().is_empty()) {
                buffer.set_style(
                    Rect::new(start, y, end - start, 1),
                    Style::default()
                        .fg(Color::Black)
                        .bg(REVIEW_BACKGROUND)
                        .remove_modifier(Modifier::REVERSED),
                );
            }
        }
        if selected && body.width > 0 {
            let marker_limit = hidden_branch_marker
                .as_ref()
                .map_or_else(|| body.right().saturating_sub(1), |(_, x, _)| x.saturating_sub(2));
            let marker_x = content
                .x
                .saturating_add(u16::try_from(line_width).unwrap_or(u16::MAX))
                .saturating_add(1)
                .saturating_add(u16::try_from(selection_info_width).unwrap_or(u16::MAX))
                .saturating_add(1)
                .min(marker_limit);
            if selection_info_width > 0 {
                let width = u16::try_from(selection_info_width)
                    .unwrap_or(u16::MAX)
                    .min(marker_x.saturating_sub(content.x).saturating_sub(2));
                let area = Rect::new(marker_x.saturating_sub(width).saturating_sub(1), y, width, 1);
                if width > 0 {
                    frame.buffer_mut()[(area.x - 1, y)]
                        .set_symbol(" ")
                        .set_style(Style::default().remove_modifier(Modifier::DIM | Modifier::REVERSED));
                    frame.render_widget(
                        Paragraph::new(selection_info.clone())
                            .style(Style::default().remove_modifier(Modifier::REVERSED)),
                        area,
                    );
                    selection_info_area = Some(area);
                }
            }
            let buffer = frame.buffer_mut();
            if marker_x > body.x {
                buffer[(marker_x - 1, y)]
                    .set_symbol(" ")
                    .set_style(Style::default().remove_modifier(Modifier::REVERSED));
            }
            buffer[(marker_x, y)].set_symbol(" ").set_style(style);
        }
        if !app.is_row_reachable(row_index) {
            for x in body.x..body.right() {
                frame.buffer_mut()[(x, y)].set_style(Style::default().add_modifier(Modifier::DIM));
            }
        }
        if app.is_row_hidden(row_index) {
            for x in body.x..body.right() {
                frame.buffer_mut()[(x, y)]
                    .set_fg(Color::Reset)
                    .set_bg(Color::Reset)
                    .set_style(Style::default().add_modifier(Modifier::DIM));
            }
            if selected && let Some(area) = selection_info_area {
                frame.render_widget(
                    Paragraph::new(selection_info.clone()).style(Style::default().remove_modifier(Modifier::REVERSED)),
                    area,
                );
            }
        }
        if let Some((marker, x, width)) = hidden_branch_marker {
            frame.buffer_mut()[(x - 1, y)].set_symbol(" ");
            frame.render_widget(
                Paragraph::new(marker).style(color(Color::LightRed)),
                Rect::new(x, y, width, 1),
            );
            frame.buffer_mut()[(x + width, y)].set_symbol(" ");
            for marker_x in x..x + width {
                let cell = &mut frame.buffer_mut()[(marker_x, y)];
                cell.set_fg(Color::LightRed);
                cell.modifier.remove(Modifier::DIM | Modifier::REVERSED);
            }
        }
    }
    app.set_horizontal_bounds(content.width as usize, max_offset);
    if app.changes_focus.is_some() {
        frame
            .buffer_mut()
            .set_style(body, Style::default().add_modifier(Modifier::DIM));
        if let Some(area) = selection_info_area {
            frame.render_widget(
                Paragraph::new(selection_info).style(Style::default().remove_modifier(Modifier::REVERSED)),
                area,
            );
        }
    }
    for (pane_area, underlay) in changes_panes.iter().zip(&changes_pane_underlays) {
        let outer = pane_area.outer;
        let pane = pane_area.pane;
        let changes = match pane {
            ChangePane::Tree => tree_changes.expect("visible tree changes exist"),
            ChangePane::Worktree => worktree_changes.expect("visible worktree changes exist"),
        };
        let summary = match pane {
            ChangePane::Tree => tree_summary.clone().expect("visible tree summary exists"),
            ChangePane::Worktree => worktree_summary.clone().expect("visible worktree summary exists"),
        };
        let area = outer.inner(Margin {
            horizontal: 2,
            vertical: 1,
        });
        frame.render_widget(Clear, *underlay);
        frame.render_widget(Block::new().borders(Borders::TOP).title(summary), outer);
        render_changes(frame, area, changes, pane, app);
        if app.changes_focus == Some(pane) {
            let status = Rect::new(
                outer.x.saturating_add(2),
                outer.bottom().saturating_sub(1),
                outer.width.saturating_sub(4),
                1,
            );
            let mut spans = Vec::new();
            if pane == ChangePane::Tree
                && let Some(parent) = changes.parent
            {
                spans.extend([
                    Span::styled(
                        format!(
                            "vs parent {}/{} {}",
                            parent.index + 1,
                            parent.total,
                            parent.id.to_hex_with_len(7)
                        ),
                        color(COMPARED_PARENT_COLOR),
                    ),
                    Span::raw(" · "),
                ]);
                spans.extend(shortcut("P next parent", 'P', true));
                spans.push(Span::raw(" · "));
            }
            if let Some(error) = &app.changes(pane).error {
                spans.push(Span::styled(format!("diff: {error}"), color(Color::LightRed)));
            } else {
                spans.push(Span::raw("↑↓/jk move · h/l pan · <enter> diff"));
            }
            spans.push(Span::raw(" · "));
            spans.extend(shortcut("copy", 'y', true));
            if let Some(label) = match app.changes_mode {
                Some(ChangesMode::Both) => Some("cycle tree"),
                Some(ChangesMode::Tree) => Some("close"),
                None => None,
            } {
                spans.push(Span::raw(" · "));
                spans.extend(shortcut(label, 'e', true));
            }
            frame.render_widget(
                Paragraph::new(Line::from(spans)).style(Style::default().bg(PANE_STATUS_BACKGROUND)),
                status,
            );
        } else {
            frame
                .buffer_mut()
                .set_style(outer, Style::default().add_modifier(Modifier::DIM));
        }
    }
    if changes_layout == ChangesLayout::SideBySide {
        render_changes_divider(frame, &changes_panes, app);
    }
    if let Some((outer, area)) = commit_pane {
        frame.render_widget(Clear, outer);
        if let Some((red, green, blue)) = app.commit_pane_background {
            frame
                .buffer_mut()
                .set_style(outer, Style::default().bg(Color::Rgb(red, green, blue)));
        }
        let max_offset = if let Some(message) = commit_message {
            let selected = app.selected.and_then(|index| app.rows.get(index)).map(|row| row.id);
            let notes = selected.map(|id| app.notes(id)).unwrap_or_default();
            let note = selected.and_then(|id| app.note(id));
            render_commit_message(frame, area, message, note, notes, app.commit_offset)
        } else {
            0
        };
        app.set_commit_bounds(area.height as usize, max_offset);
        if max_offset > 0 {
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("PgUp/C-b up page · PgDn/C-f down page · "),
                    Span::styled("m", Style::default().add_modifier(Modifier::UNDERLINED)),
                    Span::raw(" close"),
                ]))
                .style(Style::default().bg(PANE_STATUS_BACKGROUND)),
                Rect::new(
                    outer.x.saturating_add(2),
                    outer.bottom().saturating_sub(1),
                    outer.width.saturating_sub(4),
                    1,
                ),
            );
        }
    }

    let history_state = app.deferred_history_state.unwrap_or(app.state);
    let status = match history_state {
        State::Loading => "",
        State::Cancelling => " · cancelling",
        State::Computing => " · computing",
        State::Complete => "",
        State::Cancelled => " · cancelled",
    };
    let mut footer_spans = vec![Span::raw(status)];
    let mut time_travel = None;
    let mut actions_prefix_spans = Vec::new();
    if !selected_segment && (app.changes_focus != Some(ChangePane::Worktree) || app.can_amend()) {
        time_travel = time_travel_label(app, decorations);
        actions_prefix_spans.push(Span::raw(" · "));
        actions_prefix_spans.push(Span::styled("a", Style::default().add_modifier(Modifier::UNDERLINED)));
        actions_prefix_spans.push(Span::raw("ctions"));
        if app.actions_expanded {
            emphasize_prefix(&mut actions_prefix_spans[1..]);
        }
    }
    if app.changes_focus.is_some() {
        footer_spans.push(Span::raw(" · q/Esc history"));
    }
    let mut view_prefix_spans = Vec::new();
    view_prefix_spans.push(Span::raw(" · "));
    view_prefix_spans.push(Span::styled("v", Style::default().add_modifier(Modifier::UNDERLINED)));
    view_prefix_spans.push(Span::raw("iew"));
    if app.history_display_expanded {
        emphasize_prefix(&mut view_prefix_spans[1..]);
    }
    let mut ordered = vec![Span::raw(history_position(app))];
    if background_progress.is_none()
        && let Some(task) = app.background_task()
    {
        ordered.push(Span::raw(" · "));
        ordered.push(Span::styled(task.to_owned(), Style::default().fg(Color::Yellow)));
    }
    ordered.push(Span::raw(" · "));
    ordered.extend(shortcut("p command", 'p', true));
    if selected_segment {
        ordered.push(Span::raw(" · <enter> expand"));
    }
    ordered.append(&mut view_prefix_spans);
    ordered.append(&mut actions_prefix_spans);
    if !selected_segment {
        ordered.push(Span::raw(" · "));
        let enrich_prefix_start = ordered.len();
        ordered.extend(shortcut("enrich", 'n', true));
        if app.enrich_expanded {
            emphasize_prefix(&mut ordered[enrich_prefix_start..]);
        }
    }
    if let Some(label) = time_travel {
        ordered.push(Span::raw(" · "));
        ordered.extend(shortcut(label, '@', true));
    }
    if app.can_cycle_duplicate() {
        ordered.push(Span::raw(" · "));
        ordered.extend(shortcut("next duplicate", 'x', true));
    }
    if !selected_segment {
        ordered.push(Span::raw(" · "));
        ordered.extend(shortcut("copy", 'y', true));
    }
    ordered.push(Span::raw(" · "));
    ordered.extend(shortcut("refs", 'r', app.ref_mode != RefMode::None));
    ordered.push(Span::raw(" · "));
    let information_prefix_start = ordered.len();
    ordered.push(Span::styled("?", Style::default().add_modifier(Modifier::UNDERLINED)));
    if app.information_expanded {
        emphasize_prefix(&mut ordered[information_prefix_start..]);
    }
    ordered.append(&mut footer_spans);
    footer_spans = ordered;
    if app.changes_focus.is_none() && history_state == State::Loading {
        footer_spans.push(Span::raw(" · Esc cancel"));
    }
    if app.changes_focus.is_none() {
        footer_spans.push(Span::raw(" · "));
        footer_spans.extend(shortcut("quit", 'q', true));
    }
    if app.unseen_filesystem_redraw {
        footer_spans = notification_discs(footer_spans);
        if let Some((_, rows)) = prefix_popup.as_mut() {
            for items in rows {
                *items = notification_discs(std::mem::take(items));
            }
        }
    }
    frame.render_widget(Paragraph::new(Line::from(footer_spans)), footer);
    if let (Some(area), Some(notice)) = (notice_area, notice.as_ref()) {
        render_notice(frame, area, notice);
        if let Some((applied, total, _)) = app.undo_position() {
            render_undo_progress(frame, area, notice.kind, applied, total);
        }
    }
    if let Some(progress) = &background_progress {
        render_background_progress(frame, progress_area, progress);
    }
    let popup_anchor = background_progress.as_ref().map_or(footer, |_| progress_area);
    let _ = prefix_popup
        .filter(|_| prefix_popup_allowed)
        .and_then(|(anchor, items)| render_prefix_popup(frame, area, popup_anchor, anchor, items));
}

fn render_background_progress(frame: &mut Frame<'_>, area: Rect, progress: &crate::app::BackgroundProgress) {
    frame.render_widget(
        Paragraph::new(progress.text.as_str()).style(Style::default().bg(Color::Reset)),
        area,
    );
    let completed_width = if progress.total == 0 {
        0
    } else {
        (u128::from(area.width) * progress.completed.min(progress.total) as u128 / progress.total as u128) as u16
    };
    frame.buffer_mut().set_style(
        Rect::new(area.x, area.y, completed_width, area.height),
        Style::default().bg(Color::DarkGray),
    );
}

fn render_undo_progress(frame: &mut Frame<'_>, area: Rect, kind: NoticeKind, applied: usize, total: usize) {
    let bright_width = if total == 0 {
        0
    } else {
        (u128::from(area.width) * applied.min(total) as u128 / total as u128) as u16
    };
    let (bright, dim) = match kind {
        NoticeKind::Success => (Color::LightGreen, Color::Green),
        NoticeKind::Attention => (Color::LightYellow, Color::Yellow),
        NoticeKind::Error => (Color::LightRed, Color::Red),
    };
    frame.buffer_mut().set_style(
        Rect::new(area.x, area.y, bright_width, area.height),
        Style::default().bg(bright),
    );
    frame.buffer_mut().set_style(
        Rect::new(
            area.x.saturating_add(bright_width),
            area.y,
            area.width.saturating_sub(bright_width),
            area.height,
        ),
        Style::default().bg(dim),
    );
}

fn history_position(app: &App) -> String {
    if let (State::Complete, Some(selected)) = (app.deferred_history_state.unwrap_or(app.state), app.selected)
        && let Some(count) = app.visual_count(selected)
    {
        format!("#{count}")
    } else {
        format!("{} commits", app.rows.len())
    }
}

fn time_travel_label(app: &App, decorations: &Decorations) -> Option<&'static str> {
    if !app.time_travel_shortcut_visible()
        || !decorations
            .values()
            .flatten()
            .any(|decoration| decoration.kind == DecorationKind::Head)
    {
        return None;
    }
    let selected = app.selected.and_then(|index| app.rows.get(index))?;
    let selected_refs = decorations.get(&selected.id).map(Vec::as_slice).unwrap_or_default();
    if selected_refs
        .iter()
        .any(|decoration| decoration.kind == DecorationKind::Head)
    {
        None
    } else if selected_refs
        .iter()
        .any(|decoration| decoration.kind == DecorationKind::Pin)
    {
        Some("@ return")
    } else {
        Some("@ travel")
    }
}

fn active_prefix_popup_anchor(app: &App, decorations: &Decorations) -> Option<usize> {
    let mut width = history_position(app).chars().count();
    if app.background_progress().is_none()
        && let Some(task) = app.background_task()
    {
        width += 3 + task.chars().count();
    }
    width += 3 + "p command".len();
    let selected_segment = app.selected_is_segment();
    if selected_segment {
        width += " · <enter> expand".chars().count();
    }
    width += 3;
    let view = width;
    width += "view".len();
    let mut active = app.history_display_expanded.then_some(view);

    let actions_visible = !selected_segment && (app.changes_focus != Some(ChangePane::Worktree) || app.can_amend());
    if actions_visible {
        width += 3;
        let actions = width;
        width += "actions".len();
        if app.actions_expanded {
            active = Some(actions);
        }
    }
    if !selected_segment {
        width += 3;
        let enrich = width;
        width += "enrich".len();
        if app.enrich_expanded {
            active = Some(enrich);
        }
    }
    if actions_visible && let Some(label) = time_travel_label(app, decorations) {
        width += 3 + label.len();
    }
    if app.can_cycle_duplicate() {
        width += 3 + "next duplicate".len();
    }
    if !selected_segment {
        width += 3 + "copy".len();
    }
    width += 3 + "refs".len();
    width += 3;
    if app.information_expanded {
        active = Some(width);
    }
    active
}

fn render_changes_divider(frame: &mut Frame<'_>, panes: &[ChangesPaneArea], app: &App) {
    let Some(tree) = panes.iter().find(|pane| pane.pane == ChangePane::Tree) else {
        return;
    };
    let Some(worktree) = panes.iter().find(|pane| pane.pane == ChangePane::Worktree) else {
        return;
    };
    let x = worktree.outer.x;
    let top = tree.outer.y.min(worktree.outer.y);
    let bottom = tree.outer.bottom().max(worktree.outer.bottom());
    let style = if app.changes_focus.is_none() {
        Style::default().add_modifier(Modifier::DIM)
    } else {
        Style::default()
    };
    for y in top..bottom {
        let symbol = if tree.outer.y == worktree.outer.y && y == tree.outer.y {
            "┬"
        } else if y == tree.outer.y {
            if tree.outer.y < worktree.outer.y { "┐" } else { "┤" }
        } else if y == worktree.outer.y {
            if worktree.outer.y < tree.outer.y { "┌" } else { "├" }
        } else {
            "│"
        };
        frame.buffer_mut()[(x, y)].set_symbol(symbol).set_style(style);
    }
}

fn selection_info_line(changes: Option<&Changes>, relation: Option<SelectionRelation>) -> Line<'static> {
    let mut spans = Vec::new();
    if let Some(changes) = changes {
        if changes.lines_added > 0 {
            push_selection_span(
                &mut spans,
                Span::styled(format!("+{}", changes.lines_added), selection_color(Color::Green)),
            );
        }
        if changes.lines_removed > 0 {
            push_selection_span(
                &mut spans,
                Span::styled(format!("-{}", changes.lines_removed), selection_color(Color::LightRed)),
            );
        }
    }
    match relation {
        Some(SelectionRelation::Tracking { ahead, behind }) => {
            if ahead > 0 {
                push_selection_span(
                    &mut spans,
                    Span::styled(format!("⇡{ahead}"), selection_color(Color::Green)),
                );
            }
            if behind > 0 {
                if ahead == 0 {
                    push_selection_span(
                        &mut spans,
                        Span::styled(format!("⇣{behind}"), selection_color(Color::LightRed)),
                    );
                } else {
                    spans.push(Span::styled(format!("⇣{behind}"), selection_color(Color::LightRed)));
                }
            }
        }
        Some(SelectionRelation::Visible(commits)) => {
            push_selection_span(
                &mut spans,
                Span::styled(format!("⇡{commits}"), selection_color(Color::Green)),
            );
        }
        None => {}
    }
    Line::from(spans)
}

fn selection_color(color: Color) -> Style {
    Style::default()
        .fg(color)
        .remove_modifier(Modifier::DIM | Modifier::REVERSED)
}

fn push_selection_span(spans: &mut Vec<Span<'static>>, span: Span<'static>) {
    if !spans.is_empty() {
        spans.push(Span::raw(" "));
    }
    spans.push(span);
}

fn index_divider(width: u16) -> Line<'static> {
    const LABEL: &str = "↑ index ↑ ";
    let width = usize::from(width);
    let label: String = LABEL.chars().take(width).collect();
    let rail_width = width - label.chars().count();
    Line::from(vec![
        Span::styled(label, Style::default().add_modifier(Modifier::DIM)),
        Span::styled("─".repeat(rail_width), color(Color::Green)),
    ])
}

fn render_changes(frame: &mut Frame<'_>, area: Rect, changes: &Changes, pane: ChangePane, app: &mut App) {
    if area.height == 0 {
        app.set_changes_bounds(pane, 0, 0, None, area.width as usize, 0);
        return;
    }
    let focused = app.changes_focus == Some(pane);
    let selected_index = app.changes(pane).selected.min(changes.paths.len().saturating_sub(1));
    let separator = index_separator(pane, changes);
    let display_len = changes.paths.len() + usize::from(separator.is_some());
    let path_capacity = usize::from(area.height);
    let overflow = display_len > 1 && display_len > path_capacity;
    let visible_rows = if overflow {
        path_capacity.saturating_sub(1)
    } else {
        path_capacity.min(display_len)
    };
    let mut lines: Vec<_> = changes
        .paths
        .iter()
        .enumerate()
        .map(|(index, change)| {
            let selected = focused && index == selected_index;
            let path_style = if selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            let mut spans = vec![
                Span::styled(change.kind.letter().to_string(), color(path_change_color(change))),
                Span::raw(" "),
            ];
            if let Some(source) = &change.source {
                spans.extend([
                    Span::styled(source.to_str_lossy(), path_style),
                    Span::styled(" -> ", path_style),
                    Span::styled(change.path.to_str_lossy(), path_style),
                ]);
            } else {
                spans.push(Span::styled(change.path.to_str_lossy(), path_style));
            }
            if selected && let Some((insertions, removals)) = change.lines {
                if insertions > 0 {
                    spans.extend([
                        Span::raw(" "),
                        Span::styled(format!("+{insertions}"), color(Color::Green)),
                    ]);
                }
                if removals > 0 {
                    spans.extend([
                        Span::raw(" "),
                        Span::styled(format!("-{removals}"), color(Color::LightRed)),
                    ]);
                }
            }
            Line::from(spans)
        })
        .collect();
    if let Some(separator) = separator {
        lines.insert(separator, index_divider(area.width));
    }
    let horizontal_max = lines
        .iter()
        .map(Line::width)
        .max()
        .unwrap_or_default()
        .saturating_sub(area.width as usize);
    app.set_changes_bounds(
        pane,
        visible_rows,
        changes.paths.len(),
        separator,
        area.width as usize,
        horizontal_max,
    );
    let offset = app.changes(pane).offset;
    let horizontal_offset = app.changes(pane).horizontal_offset;
    for (row, line) in lines.into_iter().skip(offset).take(visible_rows).enumerate() {
        let display_index = offset + row;
        let horizontal_offset = if separator == Some(display_index) {
            0
        } else {
            horizontal_offset
        };
        frame.render_widget(
            Paragraph::new(line).scroll((0, u16::try_from(horizontal_offset).unwrap_or(u16::MAX))),
            Rect::new(
                area.x,
                area.y.saturating_add(u16::try_from(row).unwrap_or(u16::MAX)),
                area.width,
                1,
            ),
        );
    }
    let visible_end = offset.saturating_add(visible_rows);
    let hidden = (0..changes.paths.len())
        .filter(|index| *index + usize::from(separator.is_some_and(|separator| *index >= separator)) >= visible_end)
        .count();
    if overflow && hidden > 0 {
        frame.render_widget(
            Paragraph::new(Line::styled(
                format!("… {hidden} {} not shown", if hidden == 1 { "line" } else { "lines" }),
                Style::default().add_modifier(Modifier::DIM),
            )),
            Rect::new(
                area.x,
                area.bottom().saturating_sub(1),
                area.width,
                u16::from(area.height > 0),
            ),
        );
    }
}

fn change_color(kind: ChangeKind) -> Color {
    match kind {
        ChangeKind::Added => Color::Green,
        ChangeKind::Modified => Color::Yellow,
        ChangeKind::Deleted => Color::LightRed,
        ChangeKind::Renamed | ChangeKind::Copied => Color::Cyan,
        ChangeKind::TypeChanged => Color::Magenta,
        ChangeKind::Unmerged => Color::LightRed,
    }
}

fn path_change_color(change: &crate::app::PathChange) -> Color {
    match change.group {
        ChangeGroup::Tree => change_color(change.kind),
        ChangeGroup::Staged => Color::Green,
        ChangeGroup::Unstaged => Color::LightRed,
    }
}

pub(crate) fn commit_diff_title(
    row: &CommitRow,
    title: &BStr,
    mailmap: &gix::mailmap::Snapshot,
    use_mailmap: bool,
    show_emails: bool,
) -> BString {
    let author = author_label(row.author, mailmap, use_mailmap, show_emails && !row.author.is_bot());
    let author = if row.author.is_bot() {
        format!("[{author}]")
    } else {
        author
    };
    let mut out: BString = format!("{} {author} ", row.id.to_hex_with_len(7)).into();
    out.extend_from_slice(title);
    out
}

pub(crate) fn commit_diff_summary(
    changes: &Changes,
    line_counts: &[Option<(u32, u32)>],
    lines_added: u64,
    lines_removed: u64,
) -> Vec<Line<'static>> {
    let paths = changes
        .paths
        .iter()
        .map(|change| match &change.source {
            Some(source) => format!("{} -> {}", source.to_str_lossy(), change.path.to_str_lossy()),
            None => change.path.to_str_lossy().into_owned(),
        })
        .collect::<Vec<_>>();
    let path_width = paths
        .iter()
        .map(|path| Line::from(path.as_str()).width())
        .max()
        .unwrap_or_default();
    let count_width = line_counts
        .iter()
        .map(|counts| {
            counts.map_or(3, |(added, removed)| {
                (u64::from(added) + u64::from(removed)).to_string().len()
            })
        })
        .max()
        .unwrap_or_default();
    let max_changes = line_counts
        .iter()
        .flatten()
        .map(|(added, removed)| u64::from(*added) + u64::from(*removed))
        .max()
        .unwrap_or_default();
    let graph_width = max_changes.min(40) as usize;
    let delta = |added: u32, removed: u32| i64::from(added) - i64::from(removed);
    let format_delta = |delta: i64| {
        if delta > 0 {
            format!("+{delta}")
        } else {
            delta.to_string()
        }
    };
    let delta_width = line_counts
        .iter()
        .flatten()
        .map(|(added, removed)| format_delta(delta(*added, *removed)).len())
        .max()
        .unwrap_or_default();
    let mut lines = paths
        .into_iter()
        .zip(line_counts)
        .map(|(path, counts)| {
            let padding = " ".repeat(path_width.saturating_sub(Line::from(path.as_str()).width()));
            let mut spans = vec![Span::raw(format!(" {path}{padding} | "))];
            match counts {
                Some((added, removed)) => {
                    let total = u64::from(*added) + u64::from(*removed);
                    spans.push(Span::raw(format!("{total:>count_width$} ")));
                    let scaled = |count: u32| {
                        (u64::from(count) * graph_width as u64 / max_changes.max(1)).max(u64::from(count > 0)) as usize
                    };
                    let added_width = scaled(*added);
                    let removed_width = scaled(*removed);
                    spans.push(Span::styled("+".repeat(added_width), color(Color::Green)));
                    spans.push(Span::styled("-".repeat(removed_width), color(Color::LightRed)));
                    let delta = delta(*added, *removed);
                    let delta = format_delta(delta);
                    spans.push(Span::raw(" ".repeat(
                        graph_width.saturating_sub(added_width + removed_width) + 1 + delta_width - delta.len(),
                    )));
                    spans.push(Span::styled(
                        delta,
                        color(match added.cmp(removed) {
                            std::cmp::Ordering::Greater => Color::Green,
                            std::cmp::Ordering::Less => Color::LightRed,
                            std::cmp::Ordering::Equal => Color::Reset,
                        }),
                    ));
                }
                None => spans.push(Span::raw(format!("{:>count_width$}", "Bin"))),
            }
            Line::from(spans)
        })
        .collect::<Vec<_>>();
    let mut spans = match (changes.range, changes.parent) {
        (Some(range), _) => vec![Span::styled(
            format!("{}..{} · ", range.base.to_hex_with_len(7), range.tip.to_hex_with_len(7)),
            color(COMPARED_PARENT_COLOR),
        )],
        (None, Some(parent)) if parent.total > 1 => vec![Span::styled(
            format!(
                "vs parent {}/{} {} · ",
                parent.index + 1,
                parent.total,
                parent.id.to_hex_with_len(7)
            ),
            color(COMPARED_PARENT_COLOR),
        )],
        (None, Some(parent)) => vec![Span::styled(
            format!("vs parent {} · ", parent.id.to_hex_with_len(7)),
            color(COMPARED_PARENT_COLOR),
        )],
        (None, None) => vec![Span::styled("root · ", color(COMPARED_PARENT_COLOR))],
    };
    if changes.paths.is_empty() {
        spans.push(Span::styled("No changes", Style::default().add_modifier(Modifier::DIM)));
    } else {
        append_change_aggregate(
            &mut spans,
            tree_change_counts(changes),
            changes.paths.len(),
            lines_added,
            lines_removed,
        );
    }
    lines.push(Line::from(spans));
    lines
}

fn changes_summary(pane: ChangePane, app: &App, changes: &Changes) -> Line<'static> {
    let mut spans = match pane {
        ChangePane::Tree => {
            let label = changes.range.map_or_else(
                || {
                    app.time_travel_animation_origin()
                        .or_else(|| app.selected.and_then(|index| app.rows.get(index)).map(|row| row.id))
                        .map_or_else(|| "-------".into(), |id| id.to_hex_with_len(7).to_string())
                },
                |range| format!("{}..{}", range.base.to_hex_with_len(7), range.tip.to_hex_with_len(7)),
            );
            let mut spans = vec![Span::raw(format!("─ Tree {label} "))];
            if changes.paths.is_empty() {
                spans.push(Span::styled("empty", color(Color::Green)));
                spans.push(Span::raw(" "));
            }
            spans.push(Span::raw("── "));
            spans
        }
        ChangePane::Worktree if changes.paths.is_empty() => vec![
            Span::raw("─ Worktree "),
            Span::styled("clean", color(Color::Green)),
            Span::raw(" ── "),
        ],
        ChangePane::Worktree => vec![Span::raw("─ Worktree ── ")],
    };
    let counts: Vec<_> = match pane {
        ChangePane::Tree => tree_change_counts(changes),
        ChangePane::Worktree => {
            let staged = changes
                .paths
                .iter()
                .filter(|change| change.group == ChangeGroup::Staged)
                .count();
            let unstaged = changes.paths.len().saturating_sub(staged);
            [
                ("S".to_owned(), staged, Color::Green),
                ("U".to_owned(), unstaged, Color::LightRed),
            ]
            .into_iter()
            .filter(|(_, count, _)| *count > 0)
            .collect()
        }
    };
    append_change_aggregate(
        &mut spans,
        counts,
        changes.paths.len(),
        changes.lines_added,
        changes.lines_removed,
    );
    Line::from(spans)
}

fn tree_change_counts(changes: &Changes) -> Vec<(String, usize, Color)> {
    [
        ChangeKind::Added,
        ChangeKind::Modified,
        ChangeKind::Deleted,
        ChangeKind::Renamed,
        ChangeKind::Copied,
        ChangeKind::TypeChanged,
    ]
    .into_iter()
    .filter_map(|kind| {
        let count = changes.paths.iter().filter(|change| change.kind == kind).count();
        (count > 0).then(|| (kind.letter().to_string(), count, change_color(kind)))
    })
    .collect()
}

fn append_change_aggregate(
    spans: &mut Vec<Span<'static>>,
    counts: Vec<(String, usize, Color)>,
    total: usize,
    lines_added: u64,
    lines_removed: u64,
) {
    let has_counts = !counts.is_empty();
    let show_total = has_counts && (counts.len() != 1 || counts[0].1 != total);
    for (index, (label, count, count_color)) in counts.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::raw(" + "));
        }
        spans.push(Span::styled(format!("{label} {count}"), color(count_color)));
    }
    if show_total {
        spans.push(Span::raw(format!("{}= {}", if has_counts { " " } else { "" }, total)));
    }
    if lines_added > 0 || lines_removed > 0 {
        spans.push(Span::raw(" · "));
        if lines_added > 0 {
            spans.push(Span::styled(format!("+{lines_added}"), color(Color::Green)));
        }
        if lines_removed > 0 {
            if lines_added > 0 {
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(format!("-{lines_removed}"), color(Color::LightRed)));
        }
        spans.push(Span::raw(" "));
    }
}

fn render_commit_message(
    frame: &mut Frame<'_>,
    area: Rect,
    message: &BStr,
    note: Option<&BStr>,
    notes: &[BString],
    offset: usize,
) -> usize {
    let parsed = gix::objs::commit::MessageRef::from_bytes(message);
    let mut body_message = BString::default();
    let mut trailers = Vec::new();
    if let Some(body) = parsed.body() {
        for block in body.message_blocks() {
            body_message.extend_from_slice(block.message);
            trailers.extend(block.trailers());
        }
    }
    let body_message = body_message.trim_end().as_bstr();
    let body_message = (!body_message.is_empty()).then_some(body_message);
    if trailers.is_empty() {
        return render_scrolling_paragraph(
            frame,
            area,
            Paragraph::new(commit_text(note, parsed.title, parsed.body, notes, area.width)).wrap(Wrap { trim: false }),
            offset,
        );
    }
    let key_width = trailers
        .iter()
        .map(|trailer| Line::raw(trailer.token.to_str_lossy()).width())
        .max()
        .unwrap_or_default();
    if area.width < 3 || key_width > area.width.saturating_sub(3) as usize {
        if notes.is_empty() {
            return render_scrolling_paragraph(
                frame,
                area,
                Paragraph::new(commit_text(note, parsed.title, parsed.body, notes, area.width))
                    .wrap(Wrap { trim: false }),
                offset,
            );
        }
        let mut text = commit_text(note, parsed.title, body_message, notes, area.width);
        text.lines.push(Line::default());
        for trailer in trailers {
            text.lines.extend(
                Text::raw(format!(
                    "{}: {}",
                    trailer.token.to_str_lossy(),
                    trailer.value.to_str_lossy()
                ))
                .lines,
            );
        }
        return render_scrolling_paragraph(frame, area, Paragraph::new(text).wrap(Wrap { trim: false }), offset);
    }
    let key_width = key_width as u16;

    let text = commit_text(note, parsed.title, body_message, notes, area.width);
    let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
    let body_height = paragraph.line_count(area.width);
    let value_x = area.x.saturating_add(key_width).saturating_add(2);
    let value_width = area.right().saturating_sub(value_x);
    let trailers: Vec<_> = trailers
        .into_iter()
        .map(|trailer| {
            let value = Paragraph::new(trailer.value.to_str_lossy()).wrap(Wrap { trim: false });
            let height = value.line_count(value_width).max(1);
            (trailer, height)
        })
        .collect();
    let total_height = body_height
        .saturating_add(1)
        .saturating_add(trailers.iter().map(|(_, height)| height).sum::<usize>());
    let max_offset = total_height.saturating_sub(area.height as usize).min(u16::MAX as usize);
    let offset = offset.min(max_offset);
    frame.render_widget(paragraph.scroll((u16::try_from(offset).unwrap_or(u16::MAX), 0)), area);

    let viewport_end = offset.saturating_add(area.height as usize);
    let mut start = body_height.saturating_add(1);
    for (trailer, height) in trailers {
        let end = start.saturating_add(height);
        if start >= viewport_end {
            break;
        }
        if end > offset {
            let skipped = offset.saturating_sub(start);
            let y = area
                .y
                .saturating_add(u16::try_from(start.saturating_sub(offset)).unwrap_or_default());
            let visible_height = height
                .saturating_sub(skipped)
                .min(area.bottom().saturating_sub(y) as usize);
            if skipped == 0 {
                frame.render_widget(
                    Paragraph::new(format!("{}:", trailer.token.to_str_lossy()))
                        .style(color(Color::Green))
                        .right_aligned(),
                    Rect::new(area.x, y, key_width.saturating_add(1), 1),
                );
            }
            let value = Paragraph::new(trailer.value.to_str_lossy()).wrap(Wrap { trim: false });
            frame.render_widget(
                value.scroll((u16::try_from(skipped).unwrap_or(u16::MAX), 0)),
                Rect::new(
                    value_x,
                    y,
                    value_width,
                    u16::try_from(visible_height).unwrap_or(u16::MAX),
                ),
            );
        }
        start = end;
    }
    max_offset
}

fn render_scrolling_paragraph(frame: &mut Frame<'_>, area: Rect, paragraph: Paragraph<'_>, offset: usize) -> usize {
    let max_offset = paragraph
        .line_count(area.width)
        .saturating_sub(area.height as usize)
        .min(u16::MAX as usize);
    frame.render_widget(
        paragraph.scroll((u16::try_from(offset.min(max_offset)).unwrap_or(u16::MAX), 0)),
        area,
    );
    max_offset
}

fn commit_text<'a>(
    note: Option<&'a BStr>,
    title: &'a BStr,
    body: Option<&'a BStr>,
    notes: &'a [BString],
    width: u16,
) -> Text<'a> {
    let mut text = message_text(title, body);
    if let Some(note) = note {
        let parsed = gix::objs::commit::MessageRef::from_bytes(note);
        let mut prefixed = message_text(parsed.title, parsed.body);
        prefixed.lines.push(Line::styled(
            "─".repeat(width as usize),
            Style::default().add_modifier(Modifier::DIM),
        ));
        prefixed.lines.append(&mut text.lines);
        text = prefixed;
    }
    for note in notes {
        text.lines.push(Line::default());
        text.lines.push(Line::from(vec![
            Span::styled("Notes", color(NOTE_COLOR).add_modifier(Modifier::BOLD)),
            Span::styled(":", color(NOTE_COLOR)),
        ]));
        text.lines.extend(markdown_text(note.as_bstr()).lines);
    }
    text
}

fn message_text(title: &BStr, body: Option<&BStr>) -> Text<'static> {
    let mut text = markdown_text(title);
    for line in &mut text.lines {
        line.style = line.style.add_modifier(Modifier::BOLD);
    }
    if let Some(body) = body.filter(|body| !body.is_empty()) {
        text.lines.push(Line::default());
        text.lines.extend(markdown_text(body).lines);
    }
    text
}

fn markdown_text(input: &BStr) -> Text<'static> {
    let input = input.to_str_lossy();
    let rendered = tui_markdown::from_str_with_options(&input, &tui_markdown::Options::new(MarkdownStyle));
    Text {
        alignment: rendered.alignment,
        style: rendered.style,
        lines: rendered
            .lines
            .into_iter()
            .map(|line| Line {
                style: line.style,
                alignment: line.alignment,
                spans: line
                    .spans
                    .into_iter()
                    .map(|span| Span::styled(span.content.into_owned(), span.style))
                    .collect(),
            })
            .collect(),
    }
}

fn markdown_title_spans(title: &BStr) -> Vec<Span<'static>> {
    let text = markdown_text(title);
    let text_style = text.style;
    let mut out = Vec::new();
    for line in text.lines.into_iter().filter(|line| !line.spans.is_empty()) {
        if !out.is_empty() {
            out.push(Span::raw(" "));
        }
        out.extend(line.spans.into_iter().map(|mut span| {
            span.style = text_style.patch(line.style).patch(span.style);
            span
        }));
    }
    out
}

fn commit_title_spans(title: &BStr, shorten_conventional_prefix: bool) -> Vec<Span<'static>> {
    let Some(subject) = shorten_conventional_prefix
        .then(|| conventional_title_subject(title))
        .flatten()
    else {
        return markdown_title_spans(title);
    };
    let mut shortened = BString::from("…:");
    shortened.extend_from_slice(subject);
    markdown_title_spans(shortened.as_bstr())
}

fn less_than_sixty_percent(available_width: usize, title_widths: impl IntoIterator<Item = usize>) -> bool {
    let (sum, count) = title_widths
        .into_iter()
        .fold((0_u128, 0_u128), |(sum, count), width| (sum + width as u128, count + 1));
    count > 0 && available_width as u128 * count * 5 < sum * 3
}

fn lane_width(lane: &str, alignment: HistoryAlignment) -> usize {
    let lane = if alignment == HistoryAlignment::None {
        lane
    } else {
        lane.trim_end()
    };
    Line::raw(lane).width() + usize::from(alignment != HistoryAlignment::None && !lane.is_empty())
}

fn lane_through_node(lane: &str) -> &str {
    lane.char_indices()
        .find(|(_, symbol)| matches!(symbol, '●' | '◆' | '@' | '0'..='9' | '+'))
        .map_or_else(
            || lane.trim_end(),
            |(offset, symbol)| &lane[..offset + symbol.len_utf8()],
        )
}

fn conventional_title_subject(title: &BStr) -> Option<&BStr> {
    let separator = title.find(b": ")?;
    let mut prefix: &[u8] = &title[..separator];
    if let Some(without_bang) = prefix.strip_suffix(b"!") {
        prefix = without_bang;
    }
    let valid_type = |value: &[u8]| {
        value.first().is_some_and(u8::is_ascii_lowercase)
            && value
                .iter()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
    };
    let valid = prefix.iter().position(|byte| *byte == b'(').map_or_else(
        || valid_type(prefix),
        |open| {
            let scope = &prefix[open + 1..];
            valid_type(&prefix[..open])
                && scope.len() > 1
                && scope.ends_with(b")")
                && !scope[..scope.len() - 1].iter().any(|byte| matches!(byte, b'(' | b')'))
        },
    );
    valid.then(|| title[separator + 2..].as_bstr())
}

fn shortcut(label: &'static str, key: char, enabled: bool) -> Vec<Span<'static>> {
    let key_start = label.find(key).expect("shortcut key is present in its label");
    let key_end = key_start + key.len_utf8();
    let style = if enabled {
        Style::default()
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };
    vec![
        Span::styled(&label[..key_start], style),
        Span::styled(&label[key_start..key_end], style.add_modifier(Modifier::UNDERLINED)),
        Span::styled(&label[key_end..], style),
    ]
}

fn command_items(commands: &[Command], group: CommandGroup, row: usize) -> Vec<Vec<Span<'static>>> {
    let mut items = Vec::new();
    for command in commands
        .iter()
        .filter(|command| command.group == group && command.row == row)
    {
        let item = if command.id == CommandId::StackInsert {
            vec![
                Span::raw("stack-inser"),
                Span::styled("t", Style::default().add_modifier(Modifier::UNDERLINED)),
            ]
        } else {
            shortcut(command.label, command.key(), command.active)
        };
        items.push(item);
    }
    if items.is_empty() {
        items.push(vec![Span::raw("no actions")]);
    }
    items
}

fn active_prefix_popup(
    app: &App,
    decorations: &Decorations,
    commands: &[Command],
    focus_feedback: Option<&'static str>,
    content_width: usize,
) -> Option<(usize, Vec<Vec<Span<'static>>>)> {
    let selected_segment = app.selected_is_segment();
    let mut logical_rows = app
        .history_display_expanded
        .then(|| vec![command_items(commands, CommandGroup::View, 0)]);
    if !selected_segment && (app.changes_focus != Some(ChangePane::Worktree) || app.can_amend()) && app.actions_expanded
    {
        logical_rows = Some(vec![
            command_items(commands, CommandGroup::Actions, 0),
            command_items(commands, CommandGroup::Actions, 1),
        ]);
    }
    if !selected_segment && app.enrich_expanded {
        logical_rows = Some(vec![command_items(commands, CommandGroup::Enrich, 0)]);
    }
    if app.information_expanded {
        let information = commands
            .iter()
            .filter(|command| command.group == CommandGroup::Information)
            .map(|command| {
                if command.id == CommandId::VerifySignatures {
                    if app.signature_failures > 0 {
                        vec![
                            Span::raw(format!("s {} ", app.signature_failures)),
                            Span::styled("●", color(Color::LightRed)),
                        ]
                    } else {
                        vec![
                            Span::raw("s "),
                            Span::styled("●", color(Color::Rgb(255, 165, 0))),
                            Span::raw(" -> "),
                            Span::styled("●", color(Color::Green)),
                        ]
                    }
                } else {
                    shortcut(command.label, command.key(), command.active)
                }
            })
            .collect();
        let mut navigation = vec![shortcut("p command", 'p', true)];
        if !selected_segment && (app.tree_changes_visible || app.worktree_changes_visible) {
            navigation.push(vec![match focus_feedback {
                Some(destination) => Span::raw(format!("<tab> → {destination}")),
                None => Span::raw("<tab> switch"),
            }]);
        }
        navigation.extend([
            vec![Span::raw("↑↓/jk move")],
            vec![Span::raw("h/l pan")],
            vec![Span::raw("J/K topo")],
            vec![Span::raw("PgUp/PgDn move")],
            vec![Span::raw("Shift+PgUp/PgDn pan")],
        ]);
        if app.changes_focus.is_none() {
            navigation.push(vec![Span::raw(if selected_segment {
                "<enter> expand"
            } else {
                "<enter> diff"
            })]);
        }
        logical_rows = Some(vec![information, navigation]);
    }
    Some((
        active_prefix_popup_anchor(app, decorations)?,
        wrap_prefix_popup_rows(logical_rows?, content_width),
    ))
}

fn wrap_prefix_popup_rows(logical_rows: Vec<Vec<Vec<Span<'static>>>>, content_width: usize) -> Vec<Vec<Span<'static>>> {
    let mut rows = Vec::new();
    for items in logical_rows {
        let mut row = Vec::new();
        let mut row_width = 0usize;
        for mut item in items {
            let item_width = spans_width(&item);
            if !row.is_empty() && row_width.saturating_add(3).saturating_add(item_width) > content_width {
                rows.push(row);
                row = Vec::new();
                row_width = 0;
            }
            if !row.is_empty() {
                row.push(Span::raw(" · "));
                row_width += 3;
            }
            row_width = row_width.saturating_add(item_width);
            row.append(&mut item);
        }
        if row.is_empty() {
            rows.push(Vec::new());
        } else {
            rows.push(row);
        }
    }
    rows
}

fn emphasize_prefix(spans: &mut [Span<'_>]) {
    for span in spans {
        span.style = span.style.add_modifier(Modifier::REVERSED);
    }
}

fn spans_width(spans: &[Span<'_>]) -> usize {
    spans.iter().map(Span::width).sum()
}

fn render_prefix_popup(
    frame: &mut Frame<'_>,
    bounds: Rect,
    footer: Rect,
    anchor: usize,
    mut rows: Vec<Vec<Span<'static>>>,
) -> Option<Rect> {
    let height = u16::try_from(rows.len()).unwrap_or(u16::MAX);
    if !prefix_popup_can_render(bounds, footer, anchor, rows.len()) {
        return None;
    }
    let mut width = 0;
    for items in &mut rows {
        items.insert(0, Span::raw(" "));
        items.push(Span::raw(" "));
        width = width.max(spans_width(items));
    }
    let width = u16::try_from(width).unwrap_or(u16::MAX).min(footer.width);
    let anchor = footer.x.saturating_add(u16::try_from(anchor).unwrap_or(u16::MAX));
    let area = Rect::new(
        anchor.min(footer.right().saturating_sub(width)),
        footer.y - height,
        width,
        height,
    );
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(rows.into_iter().map(Line::from).collect::<Vec<_>>())
            .style(Style::default().add_modifier(Modifier::REVERSED)),
        area,
    );
    Some(area)
}

fn prefix_popup_can_render(frame: Rect, footer: Rect, anchor: usize, rows: usize) -> bool {
    footer.width > 0 && usize::from(footer.y.saturating_sub(frame.y)) >= rows && anchor < usize::from(footer.width)
}

fn notification_discs(spans: Vec<Span<'_>>) -> Vec<Span<'_>> {
    let mut out = Vec::with_capacity(spans.len());
    for span in spans {
        let style = span.style;
        for (index, text) in span.content.split('·').enumerate() {
            if index > 0 {
                out.push(Span::styled("●", style.fg(FILESYSTEM_NOTIFICATION_COLOR)));
            }
            if !text.is_empty() {
                out.push(Span::styled(text.to_owned(), style));
            }
        }
    }
    out
}

#[derive(Clone, Copy)]
struct MetadataOptions<'a> {
    date_mode: DateMode,
    id_mode: IdMode,
    change_id: gix::hash::ChangeId,
    show_author_name: bool,
    show_emails: bool,
    show_trailers: bool,
    has_notes: bool,
    note_title: Option<&'a BStr>,
    shorten_title: bool,
    use_mailmap: bool,
    ref_mode: RefMode,
    selected: bool,
    copy_feedback: Option<CopyKind>,
}

struct MetadataColumns<'a> {
    fields: [Line<'a>; 6],
}

impl<'a> MetadataColumns<'a> {
    fn prefix_width(&self) -> usize {
        self.fields[..5].iter().map(Line::width).sum()
    }

    fn into_line(self) -> Line<'a> {
        Line::from(
            self.fields
                .into_iter()
                .flat_map(|field| field.spans)
                .collect::<Vec<_>>(),
        )
    }

    fn into_line_with_prefix(self) -> (Line<'a>, usize) {
        let prefix_width = self.prefix_width();
        (self.into_line(), prefix_width)
    }

    fn align_title(mut self, width: usize) -> (Line<'a>, usize) {
        let padding = width.saturating_sub(self.prefix_width());
        self.fields[4].spans.push(Span::raw(" ".repeat(padding)));
        self.into_line_with_prefix()
    }

    fn align_columns(mut self, widths: [usize; 5]) -> (Line<'a>, usize) {
        for (field, width) in self.fields[..5].iter_mut().zip(widths) {
            field
                .spans
                .push(Span::raw(" ".repeat(width.saturating_sub(field.width()))));
        }
        self.into_line_with_prefix()
    }
}

fn metadata_columns<'a>(
    row: &'a CommitRow,
    title: &'a BStr,
    attributions: &'a [crate::app::Attribution],
    decorations: &'a Decorations,
    mailmap: &'a gix::mailmap::Snapshot,
    options: MetadataOptions<'a>,
) -> MetadataColumns<'a> {
    debug_assert!(row.metadata_loaded, "visible rows have metadata");
    let MetadataOptions {
        date_mode,
        id_mode,
        change_id,
        show_author_name,
        show_emails,
        show_trailers,
        has_notes,
        note_title,
        shorten_title,
        use_mailmap,
        ref_mode,
        selected,
        copy_feedback,
    } = options;
    let commit_style = if copy_feedback == Some(CopyKind::Id) {
        Style::default()
    } else {
        color(Color::Magenta).add_modifier(Modifier::BOLD)
    };
    let change_style = color(Color::LightCyan).add_modifier(Modifier::BOLD);
    let selected_style = |style: Style| {
        if selected {
            style.add_modifier(Modifier::REVERSED)
        } else {
            style
        }
    };
    let mut id = Vec::new();
    match id_mode {
        IdMode::Commit => id.push(Span::styled(
            row.id.to_hex_with_len(7).to_string(),
            selected_style(commit_style),
        )),
        IdMode::Change => id.push(Span::styled(
            change_id.to_reverse_hex_with_len(7).to_string(),
            selected_style(change_style),
        )),
        IdMode::Off => {}
    }
    let mut refs = Vec::new();
    let row_decorations = decorations.get(&row.id).map(Vec::as_slice).unwrap_or_default();
    if row_decorations
        .iter()
        .any(|decoration| decoration.kind == DecorationKind::Pin)
    {
        refs.push(Span::styled(" 📌", decoration_style(DecorationKind::Pin)));
    }
    if row_decorations
        .iter()
        .any(|decoration| decoration.kind == DecorationKind::Stash)
    {
        let marker = if row_decorations
            .iter()
            .any(|decoration| decoration.kind == DecorationKind::Pin)
        {
            "🎁"
        } else {
            " 🎁"
        };
        refs.push(Span::styled(marker, decoration_style(DecorationKind::Stash)));
    }
    let mut labels = row_decorations
        .iter()
        .filter(|decoration| !matches!(decoration.kind, DecorationKind::Pin | DecorationKind::Stash))
        .filter(|decoration| decoration.kind != DecorationKind::CurrentWorktreeDetached)
        .filter(|decoration| match ref_mode {
            _ if decoration.kind == DecorationKind::Head => false,
            _ if decoration.kind == DecorationKind::Review => true,
            RefMode::All => true,
            RefMode::Default => decoration.kind != DecorationKind::Special,
            RefMode::None => {
                selected
                    && matches!(
                        decoration.kind,
                        DecorationKind::WorktreeBranch
                            | DecorationKind::WorktreeDetached
                            | DecorationKind::CurrentWorktreeDetached
                    )
            }
        })
        .peekable();
    if labels.peek().is_some() {
        refs.push(Span::raw(" ("));
        for (index, decoration) in labels.enumerate() {
            if index != 0 {
                refs.push(Span::raw(", "));
            }
            let name = decoration.name.to_str_lossy();
            refs.push(Span::styled(
                if matches!(
                    decoration.kind,
                    DecorationKind::WorktreeBranch | DecorationKind::WorktreeDetached
                ) {
                    format!("{name}@").into()
                } else if decoration.kind == DecorationKind::HeadPinBranch {
                    format!("★{name}").into()
                } else if matches!(
                    decoration.kind,
                    DecorationKind::CurrentWorktreeBranch | DecorationKind::CurrentWorktreeDetached
                ) {
                    format!("@{name}").into()
                } else {
                    name
                },
                decoration_style(decoration.kind),
            ));
        }
        refs.push(Span::raw(") "));
    } else {
        refs.push(Span::raw(" "));
    }
    let mut date_spans = Vec::new();
    let date = match date_mode {
        DateMode::Author => Some(row.author_time),
        DateMode::Committer => Some(row.committer_time),
        DateMode::None => None,
    };
    if let Some(date) = date {
        date_spans.push(Span::styled(
            format!("{} ", date.format_or_unix(gix::date::time::format::SHORT)),
            color(Color::Blue),
        ));
    }
    let mut author_spans = Vec::new();
    let mut attribution_spans = Vec::new();
    if show_author_name {
        let author = author_label(row.author, mailmap, use_mailmap, show_emails && !row.author.is_bot());
        let mut author_style = if copy_feedback == Some(CopyKind::Author) {
            Style::default()
        } else {
            color(Color::Green)
        };
        if row.author.is_github_noreply() {
            author_style = author_style.add_modifier(Modifier::ITALIC);
        }
        author_spans.push(Span::styled(
            if row.author.is_bot() {
                format!("[{author}] ")
            } else {
                format!("{author} ")
            },
            author_style,
        ));
        if show_trailers {
            type Group = (&'static str, Vec<&'static str>, Vec<(String, Style)>);
            let mut groups: Vec<Group> = Vec::new();
            for (kind, marker, grouped_marker) in [
                (AttributionKind::CoAuthor, "Co: ", "Co"),
                (AttributionKind::Assisted, "As: ", "A"),
                (AttributionKind::Reviewed, "Re: ", "Re"),
                (AttributionKind::Acked, "Ack: ", "Ack"),
                (AttributionKind::Tested, "Te: ", "Te"),
                (AttributionKind::SignedOff, "So: ", "So"),
            ] {
                let actors: Vec<_> = attributions
                    .iter()
                    .filter(|actor| actor.kind == kind)
                    .map(|actor| {
                        let name = if actor.author == row.author {
                            "*".to_owned()
                        } else {
                            let name =
                                author_label(actor.author, mailmap, use_mailmap, show_emails && !actor.is_agent());
                            if actor.is_agent() { format!("[{name}]") } else { name }
                        };
                        let style = if actor.author.is_github_noreply() {
                            color(Color::Green).add_modifier(Modifier::ITALIC)
                        } else {
                            color(Color::Green)
                        };
                        (name, style)
                    })
                    .collect();
                if actors.is_empty() {
                    continue;
                }
                if let Some((_, markers, _)) = groups
                    .iter_mut()
                    .find(|(_, _, displayed_actors)| *displayed_actors == actors)
                {
                    markers.push(grouped_marker);
                } else {
                    groups.push((marker, vec![grouped_marker], actors));
                }
            }
            for (marker, markers, actors) in groups {
                attribution_spans.push(Span::styled(
                    if markers.len() == 1 {
                        marker.to_owned()
                    } else {
                        format!("{}: ", markers.join(", "))
                    },
                    color(Color::Green).add_modifier(Modifier::DIM),
                ));
                for (index, (name, style)) in actors.into_iter().enumerate() {
                    if index != 0 {
                        attribution_spans.push(Span::raw(", "));
                    }
                    attribution_spans.push(Span::styled(name, style));
                }
                attribution_spans.push(Span::raw(" "));
            }
        }
    }
    if row.has_agent_marker {
        attribution_spans.push(Span::styled("[A] ", color(NOTE_COLOR)));
    }
    if has_notes {
        attribution_spans.push(Span::styled("[N] ", color(NOTE_COLOR)));
    }
    let mut title_spans = Vec::new();
    if !show_emails {
        if let Some(note_title) = note_title {
            let mut note_title = markdown_title_spans(note_title);
            for span in &mut note_title {
                span.style = span.style.patch(note_style());
            }
            title_spans.extend(note_title);
            title_spans.push(Span::raw(" "));
        }
        title_spans.extend(commit_title_spans(title, shorten_title));
    }
    MetadataColumns {
        fields: [
            Line::from(id),
            Line::from(refs),
            Line::from(date_spans),
            Line::from(author_spans),
            Line::from(attribution_spans),
            Line::from(title_spans),
        ],
    }
}

fn metadata_line<'a>(
    row: &'a CommitRow,
    title: &'a BStr,
    attributions: &'a [crate::app::Attribution],
    decorations: &'a Decorations,
    mailmap: &'a gix::mailmap::Snapshot,
    options: MetadataOptions<'a>,
) -> Line<'a> {
    metadata_columns(row, title, attributions, decorations, mailmap, options).into_line()
}

pub(crate) fn plain_history_metadata(
    app: &App,
    row: &CommitRow,
    decorations: &Decorations,
    mailmap: &gix::mailmap::Snapshot,
    has_notes: bool,
    change_id: Option<gix::hash::ChangeId>,
) -> String {
    let mut line = metadata_line(
        row,
        app.title(row),
        app.attributions(row),
        decorations,
        mailmap,
        MetadataOptions {
            date_mode: app.date_mode,
            id_mode: app.effective_id_mode(),
            change_id: app.change_id(row.id),
            show_author_name: app.name_mode != NameMode::None,
            show_emails: app.show_emails,
            show_trailers: app.name_mode == NameMode::All && app.show_trailers,
            has_notes,
            note_title: None,
            shorten_title: false,
            use_mailmap: app.use_mailmap,
            ref_mode: app.ref_mode,
            selected: false,
            copy_feedback: None,
        },
    );
    if let Some(change_id) = change_id {
        line.spans
            .insert(1, Span::raw(format!(" {}", change_id.to_reverse_hex_with_len(7))));
    }
    line.spans.into_iter().map(|span| span.content.into_owned()).collect()
}

pub(crate) fn todo_metadata(app: &App, row: &CommitRow, mailmap: &gix::mailmap::Snapshot) -> String {
    let title = app.title(row);
    let decorations = Decorations::new();
    let line = metadata_line(
        row,
        title,
        app.attributions(row),
        &decorations,
        mailmap,
        MetadataOptions {
            date_mode: app.date_mode,
            id_mode: IdMode::Off,
            change_id: row.id.into(),
            show_author_name: app.name_mode != crate::app::NameMode::None,
            show_emails: app.show_emails,
            show_trailers: app.name_mode == crate::app::NameMode::All && app.show_trailers,
            has_notes: !app.notes(row.id).is_empty(),
            note_title: None,
            shorten_title: false,
            use_mailmap: app.use_mailmap,
            ref_mode: app.ref_mode,
            selected: false,
            copy_feedback: None,
        },
    );
    let mut out = line
        .spans
        .into_iter()
        .map(|span| span.content.into_owned())
        .collect::<String>()
        .trim()
        .to_owned();
    if app.show_emails {
        if !out.is_empty() {
            out.push(' ');
        }
        let rendered = markdown_title_spans(title)
            .into_iter()
            .map(|span| span.content.into_owned())
            .collect::<String>();
        out.push_str(&rendered);
    }
    out
}

fn author_label(
    author: &crate::app::Author,
    mailmap: &gix::mailmap::Snapshot,
    use_mailmap: bool,
    show_email: bool,
) -> String {
    let resolved = use_mailmap
        .then(|| {
            mailmap.try_resolve_ref(gix::actor::SignatureRef {
                name: author.name,
                email: author.email,
                time: "",
            })
        })
        .flatten();
    let name = resolved.as_ref().and_then(|actor| actor.name).unwrap_or(author.name);
    if show_email {
        let email = resolved.as_ref().and_then(|actor| actor.email).unwrap_or(author.email);
        format!("{} <{}>", name.to_str_lossy(), email.to_str_lossy())
    } else {
        name.to_str_lossy().into_owned()
    }
}

pub(crate) fn decoration_style(kind: DecorationKind) -> Style {
    match kind {
        DecorationKind::Head => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        DecorationKind::Pin => Style::default().fg(Color::Blue),
        DecorationKind::HeadPinBranch => Style::default().fg(Color::Cyan),
        DecorationKind::Stash => Style::default().fg(Color::LightMagenta).add_modifier(Modifier::BOLD),
        DecorationKind::Review => Style::default().fg(Color::LightMagenta).add_modifier(Modifier::BOLD),
        DecorationKind::CurrentWorktreeBranch | DecorationKind::CurrentWorktreeDetached => {
            Style::default().fg(Color::Cyan)
        }
        DecorationKind::WorktreeBranch | DecorationKind::WorktreeDetached => Style::default().fg(Color::LightBlue),
        DecorationKind::Local => Style::default().fg(Color::Cyan),
        DecorationKind::Remote => Style::default().fg(Color::Yellow),
        DecorationKind::Tag => Style::default().fg(Color::Magenta),
        DecorationKind::AnnotatedTag => Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        DecorationKind::Special => Style::default().fg(Color::Blue),
    }
}

fn color(color: Color) -> Style {
    Style::default().fg(color)
}

#[derive(Clone, Copy)]
struct HeadState {
    has_descendants: bool,
    attached: bool,
}

fn color_graph(
    frame: &mut Frame<'_>,
    area: Rect,
    graph: &str,
    offset: usize,
    highlight: Option<Color>,
    signature: SignatureState,
    head: Option<HeadState>,
) {
    for (x, symbol) in graph.chars().skip(offset).take(area.width as usize).enumerate() {
        if symbol.is_whitespace() {
            continue;
        }
        let node = matches!(symbol, '●' | '◆');
        let mut style = if let Some(highlight) = highlight {
            color(highlight).add_modifier(Modifier::REVERSED)
        } else if symbol == '◆' && head.is_none() {
            decoration_style(DecorationKind::Review)
        } else if node {
            color(signature_color(signature))
        } else {
            graph_style(offset.saturating_add(x) / 2)
        };
        if head.is_some_and(|head| head.has_descendants) && node {
            style = style.add_modifier(Modifier::BOLD);
        }
        let cell = &mut frame.buffer_mut()[(area.x + x as u16, area.y)];
        if head.is_some() && node {
            cell.set_symbol("@");
            if head.is_some_and(|head| head.attached) {
                style = style.add_modifier(Modifier::ITALIC);
            }
        }
        cell.set_style(style);
    }
}

fn signature_color(signature: SignatureState) -> Color {
    match signature {
        SignatureState::Unsigned => Color::Blue,
        SignatureState::Unverified | SignatureState::Verifying => Color::Rgb(255, 165, 0),
        SignatureState::Verified => Color::Green,
        SignatureState::Failed => Color::LightRed,
        SignatureState::PendingRebase => Color::Gray,
    }
}

fn graph_style(column: usize) -> Style {
    const COLORS: [Color; 7] = [
        Color::Magenta,
        Color::Yellow,
        Color::Cyan,
        Color::Green,
        Color::Reset,
        Color::White,
        Color::LightRed,
    ];
    let index = column % 14;
    let style = Style::default().fg(COLORS[index % COLORS.len()]);
    if index >= COLORS.len() {
        style.add_modifier(Modifier::BOLD)
    } else {
        style
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    use super::*;
    use crate::{
        app::{Action, Attribution, AttributionKind, Author, Commit, LoadedCommits},
        history::{Decoration, DecorationKind},
    };

    fn author(name: &'static [u8], email: &'static [u8]) -> &'static Author {
        Box::leak(Box::new(Author {
            name: name.as_bstr(),
            email: email.as_bstr(),
        }))
    }

    fn draw(frame: &mut Frame<'_>, app: &mut App, decorations: &Decorations) {
        super::draw(frame, app, decorations, &gix::mailmap::Snapshot::default(), None, None);
    }

    fn complete(app: &mut App) {
        let rows = app
            .start_lane_computation()
            .expect("a loading app starts lane computation");
        let (rows, lanes, lane_time) = crate::app::compute_lanes(rows);
        app.finish_lane_computation(rows, lanes, lane_time);
    }

    #[test]
    fn embeds_status_shortcuts_in_their_labels() {
        let spans = shortcut("copy", 'y', false);
        assert_eq!(Line::from(spans.clone()).to_string(), "copy");
        assert!(spans[1].style.add_modifier.contains(Modifier::UNDERLINED));
        assert!(spans[1].style.add_modifier.contains(Modifier::DIM));
    }

    #[test]
    fn time_travel_animation_keeps_the_tree_summary_on_its_origin() {
        let ids = [1, 2].map(|byte| gix::ObjectId::Sha1([byte; 20]));
        let mut app = App::new(ids.len());
        app.extend_commits(
            ids.into_iter()
                .map(|id| Commit {
                    id,
                    parent_ids: Default::default(),
                    author_time: gix::date::Time::default(),
                    committer_time: gix::date::Time::default(),
                    author: author(b"author", b"author@example.com"),
                    attributions: 0..0,
                    title: "subject".into(),
                    metadata_loaded: true,
                    has_agent_marker: false,
                    is_review: false,
                    signature: SignatureState::Unsigned,
                })
                .collect::<Vec<_>>(),
        );
        complete(&mut app);
        app.select_commit(ids[1]);
        app.begin_time_travel_animation();
        app.select_commit_for_time_travel(ids[0]);

        assert!(
            changes_summary(ChangePane::Tree, &app, &Changes::default())
                .to_string()
                .contains("0202020"),
            "the cached tree diff remains labelled with its origin while selection moves"
        );
        app.finish_time_travel_animation();
    }

    #[test]
    fn prefix_popout_clamps_and_clears_the_row_beneath_it() -> Result<(), Box<dyn std::error::Error>> {
        let mut terminal = Terminal::new(TestBackend::new(20, 2))?;
        let mut popup = None;
        terminal.draw(|frame| {
            frame.render_widget(Paragraph::new("underlying history"), Rect::new(0, 0, 20, 1));
            popup = render_prefix_popup(
                frame,
                Rect::new(0, 0, 20, 2),
                Rect::new(0, 1, 20, 1),
                12,
                vec![vec![Span::raw("abcdefghijklmnopqrstuvwxyz")]],
            );
        })?;

        assert_eq!(popup, Some(Rect::new(0, 0, 20, 1)));
        assert_eq!(rendered_line(&terminal, 0), " abcdefghijklmnopqrs");
        assert!(
            (0..20).all(|x| terminal.backend().buffer()[(x, 0)]
                .modifier
                .contains(Modifier::REVERSED)),
            "the complete clipped popout keeps its floating treatment"
        );
        Ok(())
    }

    #[test]
    fn prefix_popout_wraps_whole_items_and_preserves_logical_rows() -> Result<(), Box<dyn std::error::Error>> {
        let rows = wrap_prefix_popup_rows(
            vec![
                vec![
                    shortcut("one", 'o', true),
                    shortcut("two", 't', false),
                    shortcut("three", 't', true),
                ],
                vec![shortcut("four", 'f', true)],
            ],
            9,
        );
        assert_eq!(
            rows.iter()
                .cloned()
                .map(Line::from)
                .map(|line| line.to_string())
                .collect::<Vec<_>>(),
            ["one · two", "three", "four"],
            "items wrap without orphaning separators and logical rows still start new display rows"
        );

        let mut terminal = Terminal::new(TestBackend::new(11, 4))?;
        let mut popup = None;
        terminal.draw(|frame| {
            popup = render_prefix_popup(frame, Rect::new(0, 0, 11, 4), Rect::new(0, 3, 11, 1), 0, rows);
        })?;

        assert_eq!(popup, Some(Rect::new(0, 0, 11, 3)));
        assert_eq!(rendered_line(&terminal, 0), " one · two ");
        assert_eq!(rendered_line(&terminal, 1).trim_end(), " three");
        assert_eq!(rendered_line(&terminal, 2).trim_end(), " four");
        let styled_shortcut = terminal.backend().buffer()[(7, 0)].modifier;
        assert!(styled_shortcut.contains(Modifier::UNDERLINED));
        assert!(styled_shortcut.contains(Modifier::DIM));
        assert!(styled_shortcut.contains(Modifier::REVERSED));
        Ok(())
    }

    #[test]
    fn wrapped_prefix_popout_reserves_all_rows_or_stays_hidden() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(1);
        app.changes_mode = None;
        app.configure_hidden_filter(true);
        app.history_display_expanded = true;
        app.leave_success("notice");
        let mut terminal = Terminal::new(TestBackend::new(35, 5))?;

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;

        assert!(rendered_line(&terminal, 0).contains("notice"));
        assert!(rendered_line(&terminal, 1).contains("author date"));
        assert!(rendered_line(&terminal, 2).contains("names"));
        assert!(rendered_line(&terminal, 3).contains("show hidden"));

        let mut short_app = App::new(1);
        short_app.changes_mode = None;
        short_app.configure_hidden_filter(true);
        short_app.history_display_expanded = true;
        let mut short = Terminal::new(TestBackend::new(35, 3))?;
        short.draw(|frame| draw(frame, &mut short_app, &Decorations::new()))?;

        assert!(
            !(0..2).any(|row| {
                let line = rendered_line(&short, row);
                line.contains("author date") || line.contains("names") || line.contains("show hidden")
            }),
            "a popup that does not fit is not partially rendered"
        );
        Ok(())
    }

    #[test]
    fn command_menu_renders_filtered_numbered_results_and_a_cursor() -> Result<(), Box<dyn std::error::Error>> {
        let app = App::new(1);
        let commands = command_menu::commands(&app, &Decorations::new(), false);
        let items = commands
            .iter()
            .map(|command| {
                crate::menu::Item::with_search_prefix(
                    command.label,
                    command.group.label(),
                    command.group.prefix(),
                    command.id,
                )
            })
            .collect::<Vec<_>>();
        let mut menu = Menu::default();
        menu.open(&items);
        for character in "rft".chars() {
            menu.insert(character, &items);
        }
        let mut cursor = None;
        let mut terminal = Terminal::new(TestBackend::new(60, 12))?;
        terminal.draw(|frame| {
            let area = frame.area();
            cursor = draw_command_menu(frame, area, &mut menu, &commands);
        })?;

        let rendered = (0..12)
            .map(|row| rendered_line(&terminal, row))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("Command"), "the popup has a title");
        assert!(rendered.contains("> rft"), "the popup shows the query");
        assert!(
            rendered.contains("1  Information ref-tree  [? t]"),
            "matching commands are numbered and retain their shortcut"
        );
        let result_y = (0..12)
            .find(|row| rendered_line(&terminal, *row).contains("ref-tree"))
            .expect("the selected result is visible");
        assert!(
            (0..60).any(|x| terminal.backend().buffer()[(x, result_y)]
                .modifier
                .contains(Modifier::REVERSED)),
            "the selected command is highlighted"
        );
        assert!(cursor.is_some(), "the query exposes a terminal cursor");

        menu.open(&items);
        let mut short = Terminal::new(TestBackend::new(60, 7))?;
        short.draw(|frame| {
            let area = frame.area();
            assert!(draw_command_menu(frame, area, &mut menu, &commands).is_some());
        })?;
        assert_eq!(menu.visible_indices().len(), 2, "only rendered rows are selectable");
        assert_eq!(menu.submit_digit('3', &items), None, "a clipped row cannot execute");

        menu.open(&items);
        let mut tiny = Terminal::new(TestBackend::new(3, 12))?;
        tiny.draw(|frame| {
            let area = frame.area();
            assert_eq!(draw_command_menu(frame, area, &mut menu, &commands), None);
        })?;
        assert_eq!(
            menu.submit_selected(&items),
            None,
            "an invisible selection cannot execute"
        );
        Ok(())
    }

    #[test]
    fn tix_view_and_overlays_stay_inside_their_area() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(1);
        let decorations = Decorations::new();
        let commands = command_menu::commands(&app, &decorations, false);
        let items = commands
            .iter()
            .map(|command| {
                crate::menu::Item::with_search_prefix(
                    command.label,
                    command.group.label(),
                    command.group.prefix(),
                    command.id,
                )
            })
            .collect::<Vec<_>>();
        let mut menu = Menu::default();
        menu.open(&items);
        let diff = BuiltInDiff::new("M file".into(), vec!["+line".into()]);
        let bounds = Rect::new(5, 3, 30, 7);
        let mut cursor = None;
        let mut terminal = Terminal::new(TestBackend::new(40, 12))?;

        terminal.draw(|frame| {
            for y in 0..frame.area().height {
                for x in 0..frame.area().width {
                    frame.buffer_mut()[(x, y)].set_symbol("x");
                }
            }
            super::draw_with_worktree(
                frame,
                bounds,
                &mut app,
                &decorations,
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                None,
            );
            draw_file_diff(frame, bounds, &diff, 0, 0);
            cursor = draw_command_menu(frame, bounds, &mut menu, &commands);
        })?;

        for y in 0..12 {
            for x in 0..40 {
                if x < bounds.x || x >= bounds.right() || y < bounds.y || y >= bounds.bottom() {
                    assert_eq!(
                        terminal.backend().buffer()[(x, y)].symbol(),
                        "x",
                        "drawing escaped its supplied area at ({x}, {y})"
                    );
                }
            }
        }
        assert!(
            cursor.is_some_and(|position| bounds.contains(position)),
            "the command cursor remains inside the supplied area"
        );
        Ok(())
    }

    #[test]
    fn undo_progress_uses_the_message_line_without_shifting_the_body() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(4);
        app.update(Action::ToggleInformation);
        app.show_undo_position(1, 4, "reword commit");
        let mut terminal = Terminal::new(TestBackend::new(80, 5))?;

        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                None,
            );
        })?;

        let notice_y = (0..4)
            .find(|y| rendered_line(&terminal, *y).contains("reword commit · 1 undo · 3 redo"))
            .expect("the undo message is visible");
        assert!(
            rendered_line(&terminal, notice_y).contains("reword commit · 1 undo · 3 redo"),
            "the message line names the current operation"
        );
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(2, notice_y)].bg, Color::LightGreen);
        assert_eq!(buffer[(20, notice_y)].bg, Color::LightGreen);
        assert_eq!(buffer[(21, notice_y)].bg, Color::Green);
        assert_eq!(buffer[(77, notice_y)].bg, Color::Green);
        assert_eq!(buffer[(78, notice_y)].bg, Color::Reset, "the notice margin stays clear");
        assert!(
            rendered_line(&terminal, 4).starts_with("0 commits"),
            "the footer remains full-width"
        );

        app.update(Action::MoveDown);
        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                None,
            );
        })?;
        assert_eq!(app.undo_position(), None, "ordinary movement dismisses undo progress");
        Ok(())
    }

    #[test]
    fn undo_progress_colors_all_notice_rows_and_handles_edges() -> Result<(), Box<dyn std::error::Error>> {
        let mut terminal = Terminal::new(TestBackend::new(8, 2))?;
        for (kind, applied, total, bright_width, bright, dim) in [
            (NoticeKind::Success, 4, 4, 8, Color::LightGreen, Color::Green),
            (NoticeKind::Success, 3, 4, 6, Color::LightGreen, Color::Green),
            (NoticeKind::Success, 0, 4, 0, Color::LightGreen, Color::Green),
            (NoticeKind::Success, 0, 0, 0, Color::LightGreen, Color::Green),
            (NoticeKind::Attention, 3, 4, 6, Color::LightYellow, Color::Yellow),
            (NoticeKind::Error, 3, 4, 6, Color::LightRed, Color::Red),
        ] {
            let notice = Notice {
                kind,
                text: "notice".into(),
            };
            terminal.draw(|frame| {
                render_notice(frame, frame.area(), &notice);
                render_undo_progress(frame, frame.area(), kind, applied, total);
            })?;
            let buffer = terminal.backend().buffer();
            for y in 0..2 {
                for x in 0..8 {
                    assert_eq!(
                        buffer[(x, y)].bg,
                        if x < bright_width { bright } else { dim },
                        "every row uses the same progress boundary"
                    );
                }
            }
            assert_eq!(buffer[(0, 0)].fg, Color::Black, "progress preserves notice text color");
            assert!(
                buffer[(0, 0)].modifier.contains(Modifier::BOLD),
                "progress preserves notice emphasis"
            );
        }
        Ok(())
    }

    #[test]
    fn renders_todo_progress_with_operation_counts_and_times() -> Result<(), Box<dyn std::error::Error>> {
        let mut terminal = Terminal::new(TestBackend::new(80, 8))?;
        terminal.draw(|frame| {
            draw_todo_progress(
                frame,
                crate::edit::rebase::Progress {
                    total: 100,
                    processed: 42,
                    cherry_picked: 31,
                    signed: 28,
                    cherry_pick_time: std::time::Duration::from_millis(1234),
                    signing_time: std::time::Duration::from_millis(45),
                },
            );
        })?;
        assert_eq!(rendered_line(&terminal, 2).trim(), "Rebasing commits");
        assert!(
            rendered_line(&terminal, 3).contains("42 / 100 commits"),
            "the gauge labels current and total source commits"
        );
        assert_eq!(rendered_line(&terminal, 4).trim(), "cherry-picked 31 · 1.2s");
        assert_eq!(rendered_line(&terminal, 5).trim(), "signed 28 · 45.0ms");
        Ok(())
    }

    #[test]
    fn counts_commits_until_the_graph_is_complete_then_tracks_the_selected_row() {
        let ids = [1, 2, 3].map(|byte| gix::ObjectId::Sha1([byte; 20]));
        let mut app = App::new(3);
        app.extend_commits(
            ids.into_iter()
                .rev()
                .enumerate()
                .map(|(index, id)| Commit {
                    id,
                    parent_ids: (index < 2).then(|| ids[1 - index]).into_iter().collect(),
                    author_time: gix::date::Time::default(),
                    committer_time: gix::date::Time::default(),
                    author: author(b"author", b"author@example.com"),
                    attributions: 0..0,
                    title: "subject".into(),
                    metadata_loaded: true,
                    has_agent_marker: false,
                    is_review: false,
                    signature: SignatureState::Unsigned,
                })
                .collect::<Vec<_>>(),
        );
        assert_eq!(history_position(&app), "3 commits");

        let rows = app
            .start_lane_computation()
            .expect("a loading app starts lane computation");
        assert_eq!(history_position(&app), "3 commits");
        let (rows, lanes, lane_time) = crate::app::compute_lanes(rows);
        app.finish_lane_computation(rows, lanes, lane_time);

        assert_eq!(history_position(&app), "#2");
        app.update(Action::MoveDown);
        assert_eq!(history_position(&app), "#1");
        app.update(Action::MoveDown);
        assert_eq!(history_position(&app), "#0");
    }

    #[test]
    fn visual_counts_restart_at_each_root_and_merges_choose_the_closest() {
        let ids = [2, 3, 4, 5, 6, 7, 8].map(|byte| gix::ObjectId::Sha1([byte; 20]));
        let parents = [
            &[ids[5], ids[3]][..],
            &[ids[2]][..],
            &[ids[2]][..],
            &[ids[1]][..],
            &[][..],
            &[ids[0]][..],
            &[][..],
        ];
        let mut app = App::new(ids.len());
        app.extend_commits(
            ids.into_iter()
                .rev()
                .zip(parents)
                .map(|(id, parents)| Commit {
                    id,
                    parent_ids: parents.iter().copied().collect(),
                    author_time: gix::date::Time::default(),
                    committer_time: gix::date::Time::default(),
                    author: author(b"author", b"author@example.com"),
                    attributions: 0..0,
                    title: "subject".into(),
                    metadata_loaded: true,
                    has_agent_marker: false,
                    is_review: false,
                    signature: SignatureState::Unsigned,
                })
                .collect::<Vec<_>>(),
        );
        complete(&mut app);

        let expected = [ids[6], ids[5], ids[3], ids[1], ids[0], ids[4], ids[2]];
        assert_eq!(app.rows.iter().map(|row| row.id).collect::<Vec<_>>(), expected);
        app.selected = Some(0);
        assert_eq!(
            history_position(&app),
            "#4",
            "the merge uses its visually closest root and counts an interleaved row"
        );
        app.selected = Some(4);
        assert_eq!(history_position(&app), "#0", "the first root starts at zero");
        app.selected = Some(6);
        assert_eq!(history_position(&app), "#0", "the second root restarts at zero");
    }

    #[test]
    fn enrichments_have_a_gutter_beside_selection_and_the_graph() -> Result<(), Box<dyn std::error::Error>> {
        let id = gix::ObjectId::Sha1([1; 20]);
        let mut app = App::new(1);
        app.extend_commits(vec![Commit {
            id,
            parent_ids: Default::default(),
            author_time: gix::date::Time::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unsigned,
        }]);
        complete(&mut app);
        app.set_enrichment(
            id,
            crate::enrich::Enrichment {
                todo: true,
                note: Some("follow-up *title*\n\nwhy it matters\n".into()),
            },
        );
        app.set_tree_enrichment(id, crate::enrich::TreeEnrichment { checks_pass: true });
        let mut terminal = Terminal::new(TestBackend::new(80, 2))?;
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;

        let row = terminal.backend().buffer();
        assert_eq!(row[(0, 0)].symbol(), "🚧", "todo leads the row");
        assert_eq!(row[(2, 0)].symbol(), "📝", "note directly follows todo");
        assert_eq!(row[(4, 0)].symbol(), "✔️", "tree status follows commit enrichments");
        assert_eq!(row[(6, 0)].symbol(), ">", "selection directly follows enrichments");
        assert_eq!(row[(8, 0)].symbol(), "●", "the graph remains separate");
        assert!(
            rendered_line(&terminal, 0).contains("follow-up title subject"),
            "the selected todo prefixes its title with the note title"
        );
        let title_x = (0..80)
            .find(|x| row[(*x, 0)].symbol() == "f")
            .expect("the note title is visible");
        assert_eq!(row[(title_x, 0)].fg, Color::Black);
        assert_eq!(row[(title_x, 0)].bg, Color::Yellow);
        let separator = &row[(title_x + 15, 0)];
        assert_eq!(separator.symbol(), " ", "a space separates both titles");
        assert_eq!(separator.fg, Color::Reset, "the separator has no foreground color");
        assert_eq!(separator.bg, Color::Reset, "the separator has no background color");
        assert!(separator.modifier.is_empty(), "the separator has no modifiers");
        let commit_title = &row[(title_x + 16, 0)];
        assert_eq!(commit_title.symbol(), "s", "the commit title follows the note title");
        assert_eq!(
            commit_title.fg,
            Color::Reset,
            "the commit title has its normal foreground"
        );
        assert_eq!(
            commit_title.bg,
            Color::Reset,
            "the commit title has its normal background"
        );
        assert_eq!(
            commit_title.modifier,
            Modifier::empty(),
            "the commit title has its normal modifiers"
        );
        assert!(
            (0..80).any(|x| row[(x, 0)].symbol() == "t" && row[(x, 0)].modifier.contains(Modifier::ITALIC)),
            "the Markdown title is italicized"
        );
        app.enrich_expanded = true;
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(
            rendered_line(&terminal, 0).contains(" todo · note · checks-pass · git note "),
            "the enrich group advertises all note actions"
        );
        app.enrich_expanded = false;

        app.selected = None;
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(
            rendered_line(&terminal, 0).contains("subject"),
            "unselected rows retain their commit title"
        );
        app.selected = Some(0);
        app.set_enrichment(
            id,
            crate::enrich::Enrichment {
                todo: false,
                note: Some("follow-up *title*\n\nwhy it matters\n".into()),
            },
        );
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(
            rendered_line(&terminal, 0).contains("follow-up title subject"),
            "a note prefixes the selected title independently of todo"
        );
        assert_eq!(terminal.backend().buffer()[(0, 0)].symbol(), " ");
        assert_eq!(terminal.backend().buffer()[(2, 0)].symbol(), "📝");
        assert_eq!(terminal.backend().buffer()[(4, 0)].symbol(), "✔️");
        assert_eq!(terminal.backend().buffer()[(6, 0)].symbol(), ">");
        assert_eq!(terminal.backend().buffer()[(8, 0)].symbol(), "●");
        Ok(())
    }

    #[test]
    fn gutter_columns_and_history_offset_stay_fixed_while_scrolling() -> Result<(), Box<dyn std::error::Error>> {
        let ids = [5, 4, 3, 2, 1].map(|byte| gix::ObjectId::Sha1([byte; 20]));
        let mut app = App::new(1);
        app.extend_commits(
            ids.into_iter()
                .enumerate()
                .map(|(index, id)| Commit {
                    id,
                    parent_ids: ids.get(index + 1).copied().into_iter().collect(),
                    author_time: gix::date::Time::default(),
                    committer_time: gix::date::Time::default(),
                    author: author(b"author", b"author@example.com"),
                    attributions: 0..0,
                    title: "subject".into(),
                    metadata_loaded: true,
                    has_agent_marker: false,
                    is_review: false,
                    signature: SignatureState::Unsigned,
                })
                .collect::<Vec<_>>(),
        );
        complete(&mut app);
        let ids: Vec<_> = app.rows.iter().map(|row| row.id).collect();
        app.set_enrichment(ids[0], crate::enrich::Enrichment { todo: true, note: None });
        app.set_enrichment(
            ids[1],
            crate::enrich::Enrichment {
                todo: false,
                note: Some("note".into()),
            },
        );
        app.set_tree_enrichment(ids[2], crate::enrich::TreeEnrichment { checks_pass: true });
        app.set_change_ids(
            std::collections::HashMap::new(),
            std::collections::HashSet::from([ids[3]]),
        );
        app.arm_rebase_conflict(ids[4]);

        let mut terminal = Terminal::new(TestBackend::new(80, 3))?;
        for (index, marker_x, marker) in [(4, 8, "💥"), (3, 6, "👯‍♂️"), (2, 4, "✔️"), (1, 2, "📝"), (0, 0, "🚧")]
        {
            terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
            let row = terminal.backend().buffer();
            assert_eq!(row[(marker_x, 0)].symbol(), marker, "row {index} keeps its marker slot");
            assert_eq!(row[(10, 0)].symbol(), ">", "row {index} keeps the status column");
            assert_eq!(row[(12, 0)].symbol(), "●", "row {index} keeps the graph column");
            for empty_x in [0, 2, 4, 6, 8].into_iter().filter(|x| *x != marker_x) {
                assert_eq!(
                    row[(empty_x, 0)].symbol(),
                    " ",
                    "row {index} clears the unused gutter slot at {empty_x}"
                );
            }
            app.update(Action::MoveUp);
        }
        Ok(())
    }

    #[test]
    fn keeps_the_completed_footer_while_background_progress_is_deferred() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(1);
        app.extend_commits(vec![Commit {
            id: gix::ObjectId::Sha1([1; 20]),
            parent_ids: Default::default(),
            author_time: gix::date::Time::new(0, 0),
            committer_time: gix::date::Time::new(86_400, 0),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unsigned,
        }]);
        complete(&mut app);
        let mut terminal = Terminal::new(TestBackend::new(160, 2))?;
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        let completed = rendered_line(&terminal, 1);

        app.deferred_history_state = Some(State::Complete);
        app.state = State::Computing;
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert_eq!(
            rendered_line(&terminal, 1),
            completed,
            "short lane computation preserves the completed footer"
        );

        app.deferred_history_state = None;
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        let computing = rendered_line(&terminal, 1);
        assert!(
            computing.contains("1 commits · p command · view · actions · enrich · copy")
                && computing.contains("computing"),
            "expired deferral reveals computation progress"
        );
        assert_ne!(computing, completed, "visible progress changes the footer");

        app.deferred_history_state = Some(State::Complete);
        app.state = State::Loading;
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert_eq!(
            rendered_line(&terminal, 1),
            completed,
            "short traversal setup also preserves the completed footer"
        );
        Ok(())
    }

    #[test]
    fn shows_the_running_background_task_in_yellow_in_the_footer() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(1);
        app.start_background_task("pushing topic to origin…");
        let mut terminal = Terminal::new(TestBackend::new(120, 2))?;

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;

        let footer = rendered_line(&terminal, 1);
        let label = "pushing topic to origin…";
        let start = footer[..footer.find(label).expect("the running task is visible")]
            .chars()
            .count() as u16;
        for x in start..start + label.chars().count() as u16 {
            assert_eq!(
                terminal.backend().buffer()[(x, 1)].fg,
                Color::Yellow,
                "task cell {x} ({:?}) is yellow",
                terminal.backend().buffer()[(x, 1)].symbol()
            );
        }
        Ok(())
    }

    #[cfg(feature = "blocking-network-client")]
    #[test]
    fn fetch_progress_reserves_a_row_above_the_footer_and_below_notices_and_popups()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(1);
        complete(&mut app);
        app.set_active_branch(Some("topic".into()));
        app.start_background_task_with_progress("fetching origin…");
        assert!(app.update_background_progress("fetching origin: indexing 40/100".into(), 40, 100));
        app.leave_attention("working tree notice");
        let mut terminal = Terminal::new(TestBackend::new(100, 5))?;

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;

        assert!(rendered_line(&terminal, 2).contains("working tree notice"));
        assert!(rendered_line(&terminal, 3).contains("fetching origin: indexing 40/100"));
        assert!(!rendered_line(&terminal, 4).contains("fetching origin"));
        assert_eq!(terminal.backend().buffer()[(39, 3)].bg, Color::DarkGray);
        assert_eq!(
            terminal.backend().buffer()[(40, 3)].bg,
            Color::Reset,
            "the unfilled share keeps the status background"
        );

        app.update(Action::ToggleActions);
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(
            (0..3).any(|row| rendered_line(&terminal, row).contains(" no actions ")),
            "the prefix popup is rendered above progress"
        );
        assert!(rendered_line(&terminal, 3).contains("fetching origin"));
        Ok(())
    }

    #[test]
    fn materialized_rebase_continuation_uses_a_persistent_notice_above_the_footer()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(1);
        complete(&mut app);
        app.arm_rebase_continuation();
        app.set_worktree_conflicted(true);
        app.leave_attention("materialized conflict");
        let mut terminal = Terminal::new(TestBackend::new(120, 4))?;

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        let notice = rendered_line(&terminal, 2);
        assert!(
            notice
                .trim_start()
                .starts_with("REBASE PAUSED · resolve conflicts, then <enter> continue · Esc stop"),
            "an unresolved continuation owns the notice: {notice:?}"
        );
        assert!(
            notice.contains("materialized conflict"),
            "operation context is retained beside the persistent prompt"
        );
        assert_eq!(terminal.backend().buffer()[(2, 2)].bg, Color::Yellow);
        assert!(
            rendered_line(&terminal, 3).contains("view"),
            "the ordinary footer remains visible"
        );

        app.information_expanded = true;
        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                None,
            );
        })?;
        assert!(rendered_line(&terminal, 0).trim_start().starts_with("REBASE PAUSED"));
        assert!(
            rendered_line(&terminal, 1).contains("[ title"),
            "the popup keeps its action row"
        );
        assert!(rendered_line(&terminal, 2).contains("p command"));
        app.information_expanded = false;

        app.update(Action::MoveDown);
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(
            rendered_line(&terminal, 2).trim_start().starts_with("REBASE PAUSED"),
            "navigation cannot dismiss the continuation"
        );
        app.leave_error("continue failed");
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert_eq!(terminal.backend().buffer()[(2, 2)].bg, Color::LightRed);
        app.update(Action::MoveDown);
        app.set_worktree_conflicted(false);
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert_eq!(terminal.backend().buffer()[(2, 2)].bg, Color::Yellow);
        assert!(
            rendered_line(&terminal, 2)
                .trim_start()
                .starts_with("REBASE PAUSED · <enter> continue · Esc stop")
        );

        app.clear_rebase_continuation();
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(!(0..4).any(|y| rendered_line(&terminal, y).contains("REBASE PAUSED")));
        Ok(())
    }

    #[test]
    fn an_offscreen_popup_does_not_move_a_notice() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(1);
        app.information_expanded = true;
        app.leave_success("notice");
        let mut terminal = Terminal::new(TestBackend::new(10, 3))?;

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;

        assert!(rendered_line(&terminal, 1).contains("notice"));
        assert!(!rendered_line(&terminal, 0).contains("notice"));
        Ok(())
    }

    #[test]
    fn a_popup_is_suppressed_when_a_notice_cannot_move() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(1);
        app.information_expanded = true;
        app.leave_success("notice");
        let mut terminal = Terminal::new(TestBackend::new(120, 2))?;

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;

        assert!(rendered_line(&terminal, 0).contains("notice"));
        assert!(!rendered_line(&terminal, 0).contains("[ title"));
        Ok(())
    }

    #[test]
    fn renders_selection_info_beside_the_right_marker_without_dimming_it() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(2);
        app.extend_commits(vec![Commit {
            id: gix::ObjectId::Sha1([1; 20]),
            parent_ids: Default::default(),
            author_time: gix::date::Time::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "a subject which is deliberately too long".into(),
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unsigned,
        }]);
        complete(&mut app);
        app.selection_relation = Some(SelectionRelation::Tracking { ahead: 1, behind: 2 });
        let changes = Changes {
            paths: vec![crate::app::PathChange {
                kind: ChangeKind::Modified,
                group: ChangeGroup::Tree,
                source: None,
                path: "file".into(),
                lines: Some((3, 4)),
            }],
            lines_added: 3,
            lines_removed: 4,
            ..Changes::default()
        };
        app.changes_focus = Some(ChangePane::Tree);
        let mut terminal = Terminal::new(TestBackend::new(38, 7))?;

        terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&changes),
            );
        })?;
        let row = rendered_row(&terminal);
        let info = "+3 -4 ⇡1⇣2";
        let info_byte = row.find(info).expect("selection info wins over the long subject");
        let info_x = row[..info_byte].chars().count() as u16;
        let buffer = terminal.backend().buffer();
        assert_eq!(
            buffer[(info_x - 1, 0)].symbol(),
            " ",
            "selection info has a left margin"
        );
        assert_eq!(buffer[(info_x, 0)].fg, Color::Green);
        assert_eq!(buffer[(info_x + 3, 0)].fg, Color::LightRed);
        assert!(!buffer[(info_x, 0)].modifier.contains(Modifier::DIM));
        assert!(
            !buffer[(info_x, 0)].modifier.contains(Modifier::REVERSED),
            "contextual selection information stays outside the row inversion"
        );
        let spacer_x = info_x + info.chars().count() as u16;
        assert_eq!(buffer[(spacer_x, 0)].symbol(), " ", "the marker has a left spacer");
        assert!(
            buffer[(spacer_x + 1, 0)].modifier.contains(Modifier::REVERSED),
            "the right selection block follows the spacer"
        );
        assert_eq!(
            buffer[(spacer_x + 1, 0)].symbol(),
            " ",
            "the right selection block never inverts text"
        );

        app.selection_relation = None;
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(36, 0)].symbol(), " ", "a plain marker has a left spacer");
        assert_eq!(buffer[(37, 0)].symbol(), " ", "a plain marker never inverts text");
        assert!(buffer[(37, 0)].modifier.contains(Modifier::REVERSED));

        let text = |relation| {
            selection_info_line(None, relation)
                .spans
                .into_iter()
                .map(|span| span.content.into_owned())
                .collect::<String>()
        };
        assert_eq!(text(Some(SelectionRelation::Tracking { ahead: 0, behind: 2 })), "⇣2");
        assert_eq!(text(Some(SelectionRelation::Tracking { ahead: 0, behind: 0 })), "");
        assert!(
            selection_info_line(Some(&Changes::default()), None).spans.is_empty(),
            "selection information hides empty diff counts"
        );
        Ok(())
    }

    #[test]
    fn renders_a_pending_topological_choice_in_the_source_disc() -> Result<(), Box<dyn std::error::Error>> {
        let ids = [1, 2, 3].map(|byte| gix::ObjectId::Sha1([byte; 20]));
        let commit = |id, parent: Option<gix::ObjectId>, title: &'static str| Commit {
            id,
            parent_ids: parent.into_iter().collect(),
            author_time: gix::date::Time::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: title.into(),
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unsigned,
        };
        let mut app = App::new(3);
        app.extend_commits(vec![
            commit(ids[2], Some(ids[0]), "first child"),
            commit(ids[1], Some(ids[0]), "second child"),
            commit(ids[0], None, "fork"),
        ]);
        complete(&mut app);
        app.select_commit(ids[0]);
        let selected = app.selected.expect("the fork is selected");
        std::sync::Arc::make_mut(&mut app.rows[selected]).is_review = true;
        app.update(Action::TopologicalUp);
        let mut terminal = Terminal::new(TestBackend::new(80, 6))?;

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;

        let row = rendered_line(&terminal, 2);
        assert!(
            row.trim_start().starts_with("> 1"),
            "the pending choice replaces the selected commit disk: {row:?}"
        );
        assert!(!row.contains("1/2"), "the old persistent choice annotation is gone");
        Ok(())
    }

    #[test]
    fn renders_a_colored_file_diff_pager() -> Result<(), Box<dyn std::error::Error>> {
        let diff = BuiltInDiff::new(
            "M file".into(),
            ["--- a/file", "+++ b/file", "@@ -1 +1 @@", "-old", "+new"]
                .into_iter()
                .map(Into::into)
                .collect(),
        );
        let mut terminal = Terminal::new(TestBackend::new(48, 7))?;

        terminal.draw(|frame| {
            let area = frame.area();
            draw_file_diff(frame, area, &diff, 0, 0);
        })?;

        assert_eq!(rendered_line(&terminal, 0).trim(), "M file");
        for (y, color) in [
            (1, Color::LightRed),
            (2, Color::Green),
            (3, Color::Cyan),
            (4, Color::LightRed),
            (5, Color::Green),
        ] {
            assert_eq!(terminal.backend().buffer()[(0, y)].fg, color);
        }
        assert!(rendered_line(&terminal, 6).contains("<enter>/q/Esc back"));
        Ok(())
    }

    #[test]
    fn renders_and_streams_compact_commit_diff_summaries() -> Result<(), Box<dyn std::error::Error>> {
        let row = Commit {
            id: gix::ObjectId::Sha1([1; 20]),
            parent_ids: Default::default(),
            author_time: gix::date::Time::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: 0..0,
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unsigned,
        };
        let mailmap =
            gix::mailmap::Snapshot::from_bytes(b"mapped author <mapped@example.com> author <author@example.com>\n");
        let title = commit_diff_title(&row, b"subject".as_bstr(), &mailmap, true, false);
        assert_eq!(title, "0101010 mapped author subject");
        assert_eq!(
            commit_diff_title(&row, b"subject".as_bstr(), &mailmap, true, true),
            "0101010 mapped author <mapped@example.com> subject"
        );
        let changes = Changes {
            paths: vec![
                crate::app::PathChange {
                    kind: ChangeKind::Added,
                    group: ChangeGroup::Tree,
                    source: None,
                    path: "new".into(),
                    lines: None,
                },
                crate::app::PathChange {
                    kind: ChangeKind::Modified,
                    group: ChangeGroup::Tree,
                    source: None,
                    path: "old".into(),
                    lines: None,
                },
                crate::app::PathChange {
                    kind: ChangeKind::Deleted,
                    group: ChangeGroup::Tree,
                    source: None,
                    path: "gone".into(),
                    lines: None,
                },
                crate::app::PathChange {
                    kind: ChangeKind::Modified,
                    group: ChangeGroup::Tree,
                    source: None,
                    path: "image.bin".into(),
                    lines: None,
                },
            ],
            ..Changes::default()
        };
        let diff = BuiltInDiff::new(
            title.clone(),
            ["--- a/old", "+++ b/old"].into_iter().map(Into::into).collect(),
        )
        .with_summary(commit_diff_summary(
            &changes,
            &[Some((2, 0)), Some((1, 1)), Some((0, 3)), None],
            3,
            4,
        ));
        let mut terminal = Terminal::new(TestBackend::new(64, 9))?;

        terminal.draw(|frame| {
            let area = frame.area();
            draw_file_diff(frame, area, &diff, 0, 0);
        })?;

        assert_eq!(rendered_line(&terminal, 0).trim(), title);
        assert_eq!(rendered_line(&terminal, 1).trim(), "new       |   2 ++  +2");
        assert_eq!(rendered_line(&terminal, 2).trim(), "old       |   2 +-   0");
        assert_eq!(rendered_line(&terminal, 3).trim(), "gone      |   3 --- -3");
        assert_eq!(rendered_line(&terminal, 4).trim(), "image.bin | Bin");
        let summary = "root · A 1 + M 2 + D 1 = 4 · +3 -4";
        assert_eq!(rendered_line(&terminal, 5).trim(), summary);
        let buffer = terminal.backend().buffer();
        let summary_x = |needle| {
            summary[..summary.find(needle).expect("summary term is present")]
                .chars()
                .count() as u16
        };
        assert_eq!(buffer[(0, 5)].fg, COMPARED_PARENT_COLOR);
        assert_eq!(buffer[(summary_x("A 1"), 5)].fg, Color::Green);
        assert_eq!(buffer[(summary_x("-4"), 5)].fg, Color::LightRed);
        let delta_x = rendered_line(&terminal, 1).find("+2").expect("positive delta") as u16;
        let zero_x = rendered_line(&terminal, 2).rfind('0').expect("zero delta") as u16;
        assert_eq!(
            delta_x + 2,
            zero_x + 1,
            "deltas are right-aligned despite different sign widths"
        );
        assert_eq!(
            delta_x,
            rendered_line(&terminal, 3).find("-3").expect("negative delta") as u16
        );
        assert_eq!(buffer[(delta_x, 1)].fg, Color::Green);
        assert_eq!(buffer[(zero_x, 2)].fg, Color::Reset);
        assert_eq!(buffer[(delta_x, 3)].fg, Color::LightRed);
        assert_eq!(rendered_line(&terminal, 6).trim(), "");
        assert_eq!(rendered_line(&terminal, 7).trim(), "--- a/old");

        let mut streamed = Vec::new();
        diff.write_to(&mut streamed)?;
        assert_eq!(
            streamed,
            b"0101010 mapped author subject\n new       |   2 ++  +2\n old       |   2 +-   0\n gone      |   3 --- -3\n image.bin | Bin\nroot \xc2\xb7 A 1 + M 2 + D 1 = 4 \xc2\xb7 +3 -4 \n\n--- a/old\n+++ b/old\n"
        );
        Ok(())
    }

    #[test]
    fn renders_grouped_attributions_and_bot_names() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(1);
        app.id_mode = IdMode::Commit;
        app.extend_commits(LoadedCommits {
            rows: vec![Commit {
                id: gix::ObjectId::Sha1([1; 20]),
                parent_ids: Default::default(),
                author_time: gix::date::Time::default(),
                committer_time: gix::date::Time::default(),
                author: author(b"Codex", b"codex@openai.com"),
                attributions: 0..8,
                title: "subject".into(),
                metadata_loaded: true,
                has_agent_marker: false,
                is_review: false,
                signature: SignatureState::Unsigned,
            }],
            attributions: vec![
                Attribution {
                    kind: AttributionKind::CoAuthor,
                    author: author(b"Claude", b"noreply@anthropic.com"),
                },
                Attribution {
                    kind: AttributionKind::CoAuthor,
                    author: author(b"Codex", b"codex@openai.com"),
                },
                Attribution {
                    kind: AttributionKind::Assisted,
                    author: author(b"Claude", b"noreply@anthropic.com"),
                },
                Attribution {
                    kind: AttributionKind::Assisted,
                    author: author(b"Codex", b"codex@openai.com"),
                },
                Attribution {
                    kind: AttributionKind::Reviewed,
                    author: author(b"Human", b"human@example.com"),
                },
                Attribution {
                    kind: AttributionKind::Acked,
                    author: author(b"Acknowledger", b"ack@example.com"),
                },
                Attribution {
                    kind: AttributionKind::Tested,
                    author: author(b"Tester", b"tester@example.com"),
                },
                Attribution {
                    kind: AttributionKind::SignedOff,
                    author: author(b"Signer", b"signer@example.com"),
                },
            ],
        });
        app.selected = None;
        app.history_display_expanded = true;
        let mut terminal = Terminal::new(TestBackend::new(160, 3))?;

        let mailmap =
            gix::mailmap::Snapshot::from_bytes(b"Mapped Human <mapped@example.com> Human <human@example.com>\n");
        terminal.draw(|frame| super::draw(frame, &mut app, &Decorations::new(), &mailmap, None, None))?;

        assert!(
            rendered_line(&terminal, 2).contains(" · actions ·"),
            "the commit and actions prefixes remain available beside the view group"
        );

        let row = rendered_row(&terminal);
        assert!(
            row.contains("[Codex] Co, A: [Claude], * Re: Mapped Human Ack: Acknowledger Te: Tester So: Signer subject"),
            "attributions with identical displayed actors share their markers"
        );
        let buffer = terminal.backend().buffer();
        let style_at = |needle: &str| {
            let x = row.find(needle).expect("rendered metadata contains the named actor") as u16;
            buffer[(x, 0)].fg
        };
        assert_eq!(style_at("[Codex]"), Color::Green, "bot authors use the agent color");
        assert_eq!(
            style_at("Co, A:"),
            Color::Green,
            "grouped attribution markers use the agent color"
        );
        let marker_x = row.find("Co, A:").expect("rendered metadata contains a trailer marker") as u16;
        assert!(
            buffer[(marker_x, 0)].modifier.contains(Modifier::DIM),
            "attribution markers are dimmed"
        );
        assert_eq!(style_at("Human"), Color::Green, "human trailer actors are green");
        assert_eq!(style_at("[Claude]"), Color::Green, "bot co-authors use agent styling");
        assert!(
            rendered_line(&terminal, 1).contains("trailers"),
            "the popout advertises the trailer toggle"
        );

        app.update(Action::ToggleTrailers);
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(!rendered_row(&terminal).contains("Co:"), "t hides trailer attribution");

        app.update(Action::ToggleTrailers);
        app.update(Action::ToggleName);
        terminal.draw(|frame| super::draw(frame, &mut app, &Decorations::new(), &mailmap, None, None))?;
        let row = rendered_row(&terminal);
        assert!(row.contains("Codex"), "the first n keeps the primary actor");
        assert!(
            !row.contains("Mapped Human"),
            "the first n hides trailer actors while trailers are enabled"
        );
        app.update(Action::ToggleName);
        terminal.draw(|frame| super::draw(frame, &mut app, &Decorations::new(), &mailmap, None, None))?;
        let row = rendered_row(&terminal);
        assert!(!row.contains("Codex"), "the second n hides the primary actor");
        assert!(
            !row.contains("Mapped Human"),
            "the second n keeps trailer actors hidden"
        );
        app.update(Action::ToggleName);
        app.update(Action::ToggleMailmap);
        terminal.draw(|frame| super::draw(frame, &mut app, &Decorations::new(), &mailmap, None, None))?;
        assert!(
            rendered_row(&terminal).contains("Re: Human"),
            "m restores original trailer actor names"
        );

        app.update(Action::ToggleEmail);
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        let row = rendered_row(&terminal);
        assert!(row.contains("Human <human@example.com>"));
        assert!(!row.contains("codex@openai.com"));
        assert!(!row.contains("noreply@anthropic.com"));
        Ok(())
    }

    #[test]
    fn toggles_full_actor_and_comment() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(1);
        app.extend_commits(vec![Commit {
            id: gix::ObjectId::Sha1([1; 20]),
            parent_ids: Default::default(),
            author_time: gix::date::Time::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "unique comment".into(),
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unsigned,
        }]);
        app.selected = None;
        let mut terminal = Terminal::new(TestBackend::new(100, 2))?;

        app.update(Action::ToggleEmail);
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(rendered_row(&terminal).contains("author <author@example.com>"));
        assert!(!rendered_row(&terminal).contains("unique comment"));

        app.update(Action::ToggleEmail);
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(!rendered_row(&terminal).contains("<author@example.com>"));
        assert!(rendered_row(&terminal).contains("unique comment"));
        Ok(())
    }

    #[test]
    fn italicizes_github_noreply_actors() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(1);
        app.extend_commits(LoadedCommits {
            rows: vec![Commit {
                id: gix::ObjectId::Sha1([1; 20]),
                parent_ids: Default::default(),
                author_time: gix::date::Time::default(),
                committer_time: gix::date::Time::default(),
                author: author(b"Author", b"1+author@users.noreply.github.com"),
                attributions: 0..1,
                title: "subject".into(),
                metadata_loaded: true,
                has_agent_marker: false,
                is_review: false,
                signature: SignatureState::Unsigned,
            }],
            attributions: vec![Attribution {
                kind: AttributionKind::Reviewed,
                author: author(b"Reviewer", b"reviewer@USERS.NOREPLY.GITHUB.COM"),
            }],
        });
        app.selected = None;
        app.update(Action::ToggleEmail);
        let mut terminal = Terminal::new(TestBackend::new(160, 2))?;
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;

        let row = rendered_row(&terminal);
        for actor in [
            "Author <1+author@users.noreply.github.com>",
            "Reviewer <reviewer@USERS.NOREPLY.GITHUB.COM>",
        ] {
            let start = row.find(actor).expect("the full actor is rendered") as u16;
            for x in start..start + actor.len() as u16 {
                assert!(terminal.backend().buffer()[(x, 0)].modifier.contains(Modifier::ITALIC));
            }
        }
        Ok(())
    }

    #[test]
    fn renders_rows_decorations_selection_and_footer() -> Result<(), Box<dyn std::error::Error>> {
        let id = gix::ObjectId::Sha1([1; 20]);
        let mut app = App::new(2);
        app.id_mode = IdMode::Commit;
        app.extend_commits(vec![Commit {
            id,
            parent_ids: Default::default(),
            author_time: gix::date::Time::new(0, 0),
            committer_time: gix::date::Time::new(86_400, 0),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unsigned,
        }]);
        complete(&mut app);
        let decorations = Decorations::from([(
            id,
            vec![
                Decoration {
                    name: "HEAD".into(),
                    kind: DecorationKind::Head,
                },
                Decoration {
                    name: "refs/patches/main/patch".into(),
                    kind: DecorationKind::Special,
                },
            ],
        )]);
        let mailmap =
            gix::mailmap::Snapshot::from_bytes(b"mapped author <mapped@example.com> author <author@example.com>\n");
        let mut terminal = Terminal::new(TestBackend::new(180, 2))?;

        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;

        assert_eq!(
            super::todo_metadata(&app, &app.rows[0], &mailmap),
            "1970-01-01 mapped author subject",
            "todo commit metadata uses the author date and excludes separately represented refs"
        );

        let footer_text = "#0 · p command · view · actions · enrich · copy · refs · ? · quit";
        let selected_line = "      > @ 0101010 1970-01-01 mapped author subject";
        let mut expected = Buffer::with_lines([format!("{selected_line:<180}"), format!("{footer_text:<180}")]);
        for x in 0..selected_line.chars().count() as u16 {
            expected[(x, 0)].set_style(Style::default().add_modifier(Modifier::REVERSED));
        }
        for x in 6..9 {
            expected[(x, 0)].set_style(Style::default().fg(Color::Blue).add_modifier(Modifier::REVERSED));
        }
        for x in 10..17 {
            expected[(x, 0)].set_style(
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::REVERSED | Modifier::BOLD),
            );
        }
        for x in 18..29 {
            expected[(x, 0)].set_style(Style::default().fg(Color::Blue).add_modifier(Modifier::REVERSED));
        }
        for x in 29..43 {
            expected[(x, 0)].set_style(Style::default().fg(Color::Green).add_modifier(Modifier::REVERSED));
        }
        expected[(selected_line.chars().count() as u16 + 2, 0)]
            .set_style(Style::default().fg(Color::Blue).add_modifier(Modifier::REVERSED));
        for (label, key) in [
            ("p command", 'p'),
            ("view", 'v'),
            ("actions", 'a'),
            ("enrich", 'n'),
            ("copy", 'y'),
            ("refs", 'r'),
            ("quit", 'q'),
        ] {
            let label_start = footer_text[..footer_text.find(label).expect("shortcut label is present")]
                .chars()
                .count();
            let key_offset = label[..label.find(key).expect("shortcut key is in its label")]
                .chars()
                .count();
            expected[((label_start + key_offset) as u16, 1)]
                .modifier
                .insert(Modifier::UNDERLINED);
        }
        let information = footer_text[..footer_text.find('?').expect("the information prefix is present")]
            .chars()
            .count();
        expected[(information as u16, 1)].modifier.insert(Modifier::UNDERLINED);
        terminal.backend().assert_buffer(&expected);

        let row = terminal.backend().buffer();
        assert!(
            (0..selected_line.chars().count() as u16).all(|x| row[(x, 0)].modifier.contains(Modifier::REVERSED)),
            "the current worktree selection is reversed through its title"
        );
        assert!(
            !row[(selected_line.chars().count() as u16, 0)]
                .modifier
                .contains(Modifier::REVERSED),
            "the current worktree selection ends after its title"
        );
        assert!(
            !rendered_row(&terminal).contains("HEAD"),
            "the graph marker makes textual HEAD redundant"
        );
        assert!(
            !rendered_line(&terminal, 1).contains("Esc cancel"),
            "completed work cannot be cancelled"
        );

        terminal.draw(|frame| super::draw(frame, &mut app, &Decorations::new(), &mailmap, None, None))?;
        let rendered = rendered_row(&terminal);
        let title_byte = rendered.find("subject").expect("the selected commit title is visible");
        let title_x = rendered[..title_byte].chars().count() as u16;
        let row = terminal.backend().buffer();
        assert!(
            (0..title_x.saturating_sub(1)).all(|x| row[(x, 0)].modifier.contains(Modifier::REVERSED)),
            "an off-worktree selection reverses through its non-title metadata"
        );
        assert!(
            (title_x.saturating_sub(1)..title_x + "subject".len() as u16)
                .all(|x| !row[(x, 0)].modifier.contains(Modifier::REVERSED)),
            "an off-worktree selection leaves a margin and its title uninverted"
        );

        app.unseen_filesystem_redraw = true;
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        for (x, _) in footer_text
            .chars()
            .enumerate()
            .filter(|(_, character)| *character == '·')
        {
            assert_eq!(
                terminal.backend().buffer()[(x as u16, 1)].symbol(),
                "●",
                "status separators become prominent notification discs"
            );
            assert_eq!(
                terminal.backend().buffer()[(x as u16, 1)].fg,
                FILESYSTEM_NOTIFICATION_COLOR,
                "every status separator marks an unseen filesystem redraw"
            );
        }
        assert_eq!(
            terminal.backend().buffer()[(0, 1)].fg,
            Color::Reset,
            "notification coloring does not affect status text"
        );
        app.unseen_filesystem_redraw = false;

        app.leave_attention("worktree removed; using common repository");
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert_eq!(
            rendered_line(&terminal, 0).trim(),
            "worktree removed; using common repository",
            "recovery information uses the body row above the status"
        );
        assert_eq!(terminal.backend().buffer()[(2, 0)].bg, Color::Yellow);
        assert_eq!(
            rendered_line(&terminal, 1).trim(),
            footer_text,
            "the status remains visible"
        );

        app.history_display_expanded = true;
        app.update(Action::ToggleMailmap);
        assert!(app.notice().is_none(), "the next action clears the notice");
        let mut terminal = Terminal::new(TestBackend::new(180, 3))?;
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert!(
            rendered_row(&terminal).contains(" author subject"),
            "m restores the original author name"
        );
        assert!(popup_is_dim(&terminal, "mailmap"), "disabled mailmap is dimmed");

        app.history_display_expanded = false;
        app.leave_attention("no commit created: no input was provided");
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert_eq!(
            rendered_line(&terminal, 1).trim(),
            "no commit created: no input was provided",
            "an unchanged new-commit editor explains why nothing happened"
        );
        app.leave_error("operation failed");
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert_eq!(terminal.backend().buffer()[(2, 1)].bg, Color::LightRed);
        app.history_display_expanded = true;
        app.update(Action::ToggleMailmap);

        app.update(Action::ToggleDate);
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert!(
            rendered_row(&terminal).contains("1970-01-02"),
            "the first d switches to the committer date"
        );
        assert!(rendered_line(&terminal, 1).contains("committer date"));

        app.update(Action::ToggleDate);
        app.update(Action::ToggleName);
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        let row = rendered_row(&terminal);
        assert!(!row.contains("1970-01-0"), "the second d hides dates");
        assert!(
            !row.contains("author"),
            "the first n hides the author when there are no attributions"
        );
        assert!(!row.contains("refs/patches"), "special refs are hidden until requested");
        assert!(row.contains("subject"), "the commit subject remains visible");
        assert!(popup_is_dim(&terminal, "date"), "disabled date is dimmed");
        assert!(popup_is_dim(&terminal, "name"), "disabled name is dimmed");

        app.update(Action::ToggleName);
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert!(
            rendered_row(&terminal).contains("author"),
            "the second n restores the author name"
        );
        assert!(!popup_is_dim(&terminal, "name"), "the restored name mode is not dimmed");

        app.update(Action::CycleRefs);
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert!(!rendered_row(&terminal).contains("HEAD"), "no refs hides regular refs");
        assert!(
            rendered_row(&terminal).trim_start().starts_with("> @"),
            "HEAD keeps its graph marker"
        );
        assert!(
            !rendered_row(&terminal).contains("refs/patches"),
            "no refs hides special refs"
        );
        assert!(popup_is_dim(&terminal, "no refs"), "no refs is dimmed");

        app.update(Action::CycleRefs);
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert!(
            !rendered_row(&terminal).contains("HEAD"),
            "all refs still omits redundant HEAD"
        );
        assert!(
            rendered_row(&terminal).contains("refs/patches"),
            "all refs shows special refs"
        );
        assert!(!popup_is_dim(&terminal, "all refs"), "all refs is not dimmed");

        app.update(Action::CycleRefs);
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert!(
            !rendered_row(&terminal).contains("HEAD"),
            "refs still omits redundant HEAD"
        );
        assert!(
            !rendered_row(&terminal).contains("refs/patches"),
            "refs hides special refs"
        );
        assert!(!popup_is_dim(&terminal, "refs"), "refs is not dimmed");

        app.has_hidden_filter = true;
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert!(
            rendered_line(&terminal, 1).contains("show hidden"),
            "the popout advertises the configured hidden-history toggle"
        );
        app.show_hidden = true;
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert!(
            rendered_line(&terminal, 1).contains("hide hidden"),
            "the popout reflects the unfiltered view"
        );

        Ok(())
    }

    #[test]
    fn advertises_travel_and_return_for_non_head_rows() -> Result<(), Box<dyn std::error::Error>> {
        let selected = gix::ObjectId::Sha1([1; 20]);
        let head = gix::ObjectId::Sha1([2; 20]);
        let mut app = App::new(2);
        app.id_mode = IdMode::Commit;
        app.extend_commits(vec![Commit {
            id: selected,
            parent_ids: Default::default(),
            author_time: gix::date::Time::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unsigned,
        }]);
        complete(&mut app);
        let mut decorations = Decorations::from([
            (
                selected,
                vec![
                    Decoration {
                        name: "pin:01010101".into(),
                        kind: DecorationKind::Pin,
                    },
                    Decoration {
                        name: "pin:01010102".into(),
                        kind: DecorationKind::Pin,
                    },
                    Decoration {
                        name: "stash".into(),
                        kind: DecorationKind::Stash,
                    },
                ],
            ),
            (
                head,
                vec![Decoration {
                    name: "HEAD".into(),
                    kind: DecorationKind::Head,
                }],
            ),
        ]);
        let mut terminal = Terminal::new(TestBackend::new(140, 4))?;
        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        let row = rendered_row(&terminal);
        assert!(row.contains("📌"), "a pinned commit has an obvious pin marker: {row:?}");
        assert_eq!(row.matches("📌").count(), 1, "multiple pins share one visual marker");
        assert!(!row.contains("pin:01010101"), "pin identities stay out of history rows");
        let hash = row.find("0101010").expect("the row contains its hash");
        let pin = row.find("📌").expect("the row contains its pin");
        let gift = row.find("🎁").expect("the row contains its stash marker");
        let date = row.find("1970-01-01").expect("the row contains its date");
        assert!(
            pin < gift && gift < date,
            "resource markers form one compact group: {row:?}"
        );
        assert!(
            hash < pin && pin < date,
            "the pin sits between the hash and metadata: {row:?}"
        );
        std::sync::Arc::make_mut(&mut app.rows[0]).is_review = true;
        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        let row = rendered_row(&terminal);
        let review = row.find("◆").expect("a review replaces its graph disc");
        let hash = row.find("0101010").expect("the row contains its hash");
        let pin = row.find("📌").expect("the row contains its pin");
        let gift = row.find("🎁").expect("the row contains its stash marker");
        assert_eq!(row.matches("◆").count(), 1, "a review has one diamond: {row:?}");
        assert!(
            review < hash && hash < pin && pin < gift,
            "the graph diamond does not disturb resource ordering: {row:?}"
        );
        decorations
            .get_mut(&selected)
            .expect("the selected commit has resources")
            .retain(|decoration| decoration.kind == DecorationKind::Stash);
        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        assert!(
            rendered_row(&terminal).contains("0101010 🎁"),
            "a lone stash remains separated from the hash"
        );
        decorations
            .get_mut(&selected)
            .expect("the selected commit has a stash")
            .push(Decoration {
                name: "pin:01010101".into(),
                kind: DecorationKind::Pin,
            });
        std::sync::Arc::make_mut(&mut app.rows[0]).is_review = false;
        app.ref_mode = RefMode::None;
        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        let row = rendered_row(&terminal);
        assert!(
            row.contains("📌") && row.contains("🎁"),
            "hiding refs retains resource markers: {row:?}"
        );
        app.actions_expanded = true;
        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        assert!(
            rendered_line(&terminal, 1).contains(" reword · new · new-empty · d forget · unpin "),
            "the commit actions float above their prefix"
        );
        assert!(
            rendered_line(&terminal, 3).contains("actions · enrich · @ return · copy"),
            "time travel stays outside the active actions prefix"
        );

        decorations.remove(&selected);
        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        assert!(rendered_line(&terminal, 3).contains(" · @ travel · copy"));
        assert!(!rendered_line(&terminal, 1).contains("unpin"));

        app.ref_mode = RefMode::Default;
        decorations.insert(
            selected,
            vec![Decoration {
                name: "main".into(),
                kind: DecorationKind::HeadPinBranch,
            }],
        );
        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        let row = rendered_row(&terminal);
        assert!(
            row.contains("★main"),
            "the remembered branch has its dedicated marker: {row:?}"
        );
        assert!(!row.contains("📌"), "the HEAD pin is not an ordinary pin: {row:?}");
        assert!(rendered_line(&terminal, 3).contains(" · @ travel · copy"));
        assert!(!rendered_line(&terminal, 1).contains("unpin"));

        decorations.remove(&head);
        decorations.insert(
            selected,
            vec![
                Decoration {
                    name: "HEAD".into(),
                    kind: DecorationKind::Head,
                },
                Decoration {
                    name: "main".into(),
                    kind: DecorationKind::HeadPinBranch,
                },
            ],
        );
        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        let footer = rendered_line(&terminal, 3);
        assert!(
            !footer.contains("@ travel") && !footer.contains("@ return"),
            "time travel is hidden at HEAD: {footer}"
        );
        Ok(())
    }

    #[test]
    fn visually_groups_active_prefix_options() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(1);
        app.changes_mode = None;
        app.configure_hidden_filter(true);
        app.history_display_expanded = true;
        let mut terminal = Terminal::new(TestBackend::new(240, 3))?;

        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                None,
            );
        })?;

        let footer = rendered_line(&terminal, 2);
        let compact = "0 commits · p command · view · actions · enrich · copy · refs · ? · Esc cancel · quit";
        assert_eq!(footer.trim_end(), compact, "the footer keeps every prefix compact");
        let view = "author date · ids · emails · names · mailmap · trailers · refs · show hidden";
        let popup = rendered_line(&terminal, 1);
        let view_x = footer[..footer.find("view").expect("the view prefix is visible")]
            .chars()
            .count() as u16;
        let popup_x = popup[..popup.find(view).expect("the view items are visible")]
            .chars()
            .count() as u16
            - 1;
        assert_eq!(popup_x, view_x, "the popout is connected to its footer prefix");
        assert!(
            terminal.backend().buffer()[(view_x, 2)]
                .modifier
                .contains(Modifier::UNDERLINED),
            "the prefix key is underlined in its verb"
        );
        assert_reversed_group(&terminal, 2, "view");
        assert_reversed_group(&terminal, 1, &format!(" {view} "));

        app.history_display_expanded = false;
        app.actions_expanded = true;
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert_eq!(rendered_line(&terminal, 2).trim_end(), compact);
        assert!(rendered_line(&terminal, 0).contains(" no actions "));
        assert!(rendered_line(&terminal, 1).contains(" no actions "));
        assert_reversed_group(&terminal, 2, "actions");
        assert_reversed_group(&terminal, 0, " no actions ");
        assert_reversed_group(&terminal, 1, " no actions ");

        app.actions_expanded = false;
        app.enrich_expanded = true;
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert_eq!(rendered_line(&terminal, 2).trim_end(), compact);
        assert!(rendered_line(&terminal, 1).contains(" no actions "));
        assert_reversed_group(&terminal, 2, "enrich");
        assert_reversed_group(&terminal, 1, " no actions ");

        app.enrich_expanded = false;
        app.information_expanded = true;
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert_eq!(rendered_line(&terminal, 2).trim_end(), compact);
        let information = "[ title · ref-tree · message · changes";
        let navigation =
            "p command · ↑↓/jk move · h/l pan · J/K topo · PgUp/PgDn move · Shift+PgUp/PgDn pan · <enter> diff";
        assert!(rendered_line(&terminal, 0).contains(information));
        assert!(rendered_line(&terminal, 1).contains(navigation));
        assert_reversed_group(&terminal, 2, "?");
        for (row, text) in [(0, information), (1, navigation)] {
            let line = rendered_line(&terminal, row);
            let x = line.find(text).expect("the popup row is visible") as u16;
            assert!(
                terminal.backend().buffer()[(x, row)]
                    .modifier
                    .contains(Modifier::REVERSED),
                "the popup row is reversed"
            );
        }
        Ok(())
    }

    #[test]
    fn actions_popup_shows_push_and_insert_shortcuts_without_the_cherry_prefix()
    -> Result<(), Box<dyn std::error::Error>> {
        let head = gix::ObjectId::Sha1([1; 20]);
        let base = gix::ObjectId::Sha1([2; 20]);
        let parent = gix::ObjectId::Sha1([3; 20]);
        let target = gix::ObjectId::Sha1([4; 20]);
        let root = gix::ObjectId::Sha1([5; 20]);
        let commit = |id, parent: Option<gix::ObjectId>| Commit {
            id,
            parent_ids: parent.into_iter().collect(),
            author_time: gix::date::Time::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unsigned,
        };
        let mut app = App::new(5);
        app.set_worktree_head(Some(head), false);
        app.set_worktree_branch(Some((head, true)));
        app.extend_commits(vec![
            commit(head, Some(base)),
            commit(base, Some(parent)),
            commit(parent, Some(root)),
            commit(root, None),
            commit(target, None),
        ]);
        complete(&mut app);
        app.selected = app.rows.iter().position(|row| row.id == head);
        app.set_active_branch(Some("topic".into()));
        #[cfg(feature = "blocking-network-client")]
        app.set_fetch_remote(Some("origin".into()));
        app.actions_expanded = true;
        let mut terminal = Terminal::new(TestBackend::new(160, 5))?;

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;

        let popup = rendered_line(&terminal, 3);
        assert!(popup.contains("copy-insert"));
        assert!(popup.contains("move-insert"));
        assert!(popup.contains("fork"));
        assert!(popup.contains("attach"));
        #[cfg(feature = "blocking-network-client")]
        assert!(popup.contains("F fetch"));
        assert!(popup.contains("P push"));
        assert!(!popup.contains("cherry-"));
        let push = popup[..popup.find("P push").expect("the push action is visible")]
            .chars()
            .count() as u16;
        assert!(
            terminal.backend().buffer()[(push, 3)]
                .modifier
                .contains(Modifier::UNDERLINED)
        );
        let label = "stack-insert";
        let start = popup[..popup.find(label).expect("the stack-insert action is visible")]
            .chars()
            .count() as u16;
        let shortcut = start + label.len() as u16 - 1;
        assert!(
            terminal.backend().buffer()[(shortcut, 3)]
                .modifier
                .contains(Modifier::UNDERLINED)
        );
        assert!(
            (start..shortcut).all(|x| !terminal.backend().buffer()[(x, 3)]
                .modifier
                .contains(Modifier::UNDERLINED)),
            "only the t in insert is underlined"
        );
        Ok(())
    }

    #[test]
    fn focused_paths_offer_only_their_scoped_edit_in_the_main_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let id = gix::ObjectId::Sha1([1; 20]);
        let mut app = App::new(4);
        app.extend_commits(vec![Commit {
            id,
            parent_ids: Default::default(),
            author_time: gix::date::Time::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unsigned,
        }]);
        app.set_worktree_head(Some(id), false);
        complete(&mut app);
        app.changes_focus = Some(ChangePane::Tree);
        app.actions_expanded = true;
        let changes = Changes {
            paths: vec![crate::app::PathChange {
                kind: ChangeKind::Added,
                group: ChangeGroup::Tree,
                source: None,
                path: "file".into(),
                lines: None,
            }],
            ..Changes::default()
        };
        let mut decorations = Decorations::from([(
            id,
            vec![Decoration {
                name: "HEAD".into(),
                kind: DecorationKind::Head,
            }],
        )]);
        let mut terminal = Terminal::new(TestBackend::new(120, 8))?;
        terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &decorations,
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&changes),
            );
        })?;
        let popup = rendered_line(&terminal, 5);
        assert!(
            popup.contains(" spill "),
            "tree focus keeps the scoped edit visible: {popup}"
        );
        assert!(!popup.contains("amend"), "tree paths cannot be amended");

        app.changes_focus = Some(ChangePane::Worktree);
        let mut worktree = Changes {
            paths: vec![crate::app::PathChange {
                kind: ChangeKind::Modified,
                group: ChangeGroup::Unstaged,
                source: None,
                path: "file".into(),
                lines: None,
            }],
            ..Changes::default()
        };
        app.set_head_edit_availability(true, false, false, true, false, false, false);
        assert!(app.can_amend(), "the focused worktree path is amendable");
        assert!(app.actions_expanded, "the actions prefix remains expanded");
        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &decorations,
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                Some(&worktree),
            );
        })?;
        assert!(app.can_amend(), "drawing retains scoped amend availability");
        assert!(app.actions_expanded, "drawing retains the expanded actions prefix");
        let popup = rendered_line(&terminal, 5);
        assert!(
            popup.contains(" amend "),
            "worktree focus keeps the scoped edit visible: {popup}"
        );
        assert!(!popup.contains("spill"), "worktree paths cannot be spilled");

        app.changes_focus = None;
        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &decorations,
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                Some(&worktree),
            );
        })?;
        assert!(
            rendered_line(&terminal, 6).contains("stash"),
            "loaded unconflicted worktree changes offer stashing"
        );
        decorations
            .get_mut(&id)
            .expect("HEAD has decorations")
            .push(Decoration {
                name: "stash".into(),
                kind: DecorationKind::Stash,
            });
        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &decorations,
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                Some(&worktree),
            );
        })?;
        assert!(
            rendered_line(&terminal, 6).contains("unstash"),
            "an existing commit stash offers in-place restoration even with worktree changes"
        );
        decorations
            .get_mut(&id)
            .expect("HEAD has decorations")
            .retain(|decoration| decoration.kind != DecorationKind::Stash);
        app.changes_focus = Some(ChangePane::Worktree);

        std::sync::Arc::make_mut(&mut app.rows[0]).is_review = true;
        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &decorations,
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                Some(&worktree),
            );
        })?;
        assert!(
            rendered_line(&terminal, 5).contains(" amend "),
            "a review may amend its selected unstaged path"
        );
        worktree.paths[0].group = ChangeGroup::Staged;
        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &decorations,
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                Some(&worktree),
            );
        })?;
        assert!(
            rendered_line(&terminal, 5).contains(" amend "),
            "a review may amend its selected staged path"
        );
        Ok(())
    }

    #[test]
    fn renders_worktree_labels_and_keeps_them_selected_when_refs_are_hidden() -> Result<(), Box<dyn std::error::Error>>
    {
        let checked_out = gix::ObjectId::Sha1([1; 20]);
        let other = gix::ObjectId::Sha1([2; 20]);
        let commit = |id| Commit {
            id,
            parent_ids: Default::default(),
            author_time: gix::date::Time::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unsigned,
        };
        let mut app = App::new(2);
        app.extend_commits(vec![commit(checked_out), commit(other)]);
        complete(&mut app);
        let decorations = Decorations::from([(
            checked_out,
            vec![
                Decoration {
                    name: "current".into(),
                    kind: DecorationKind::CurrentWorktreeBranch,
                },
                Decoration {
                    name: "main".into(),
                    kind: DecorationKind::WorktreeBranch,
                },
                Decoration {
                    name: "detached".into(),
                    kind: DecorationKind::WorktreeDetached,
                },
                Decoration {
                    name: "current-detached".into(),
                    kind: DecorationKind::CurrentWorktreeDetached,
                },
                Decoration {
                    name: "HEAD".into(),
                    kind: DecorationKind::Head,
                },
            ],
        )]);
        let mut terminal = Terminal::new(TestBackend::new(140, 3))?;
        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        let row = rendered_row(&terminal);
        assert!(row.contains("@current"));
        assert!(row.contains("main@"));
        assert!(row.contains("detached@"));
        assert!(
            !row.contains("current-detached"),
            "the graph @ already identifies the current detached worktree"
        );
        assert!(!row.contains("HEAD"), "a worktree label replaces textual HEAD");
        let x = row.find("main@").expect("the worktree label is visible") as u16;
        assert_eq!(terminal.backend().buffer()[(x, 0)].fg, Color::LightBlue);
        let x = row.find("@current").expect("the current branch label is visible") as u16;
        assert_eq!(terminal.backend().buffer()[(x, 0)].fg, Color::Cyan);

        app.update(Action::ToggleRefs);
        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        assert!(
            rendered_row(&terminal).contains("main@"),
            "the selected row retains worktrees"
        );
        assert!(
            !rendered_row(&terminal).contains("@current"),
            "the current branch follows ordinary reference visibility"
        );
        app.update(Action::MoveDown);
        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        assert!(
            !rendered_row(&terminal).contains("main@"),
            "hidden refs omit worktrees from unselected rows"
        );
        Ok(())
    }

    #[test]
    fn removes_the_copied_fields_color_from_only_the_selected_row_for_one_frame()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(2);
        app.id_mode = IdMode::Commit;
        app.extend_commits(
            (1..=2)
                .map(|n| Commit {
                    id: gix::ObjectId::Sha1([n; 20]),
                    parent_ids: Default::default(),
                    author_time: gix::date::Time::default(),
                    committer_time: gix::date::Time::default(),
                    author: author(b"author", b"author@example.com"),
                    attributions: 0..0,
                    title: format!("subject {n}").into(),
                    metadata_loaded: true,
                    has_agent_marker: false,
                    is_review: false,
                    signature: SignatureState::Unsigned,
                })
                .collect::<Vec<_>>(),
        );
        let mut terminal = Terminal::new(TestBackend::new(80, 3))?;

        drop(app.update(Action::Copy));
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        let selected_hash = rendered_line(&terminal, 0)
            .find("0101010")
            .expect("the selected hash is visible") as u16;
        let other_hash = rendered_line(&terminal, 1)
            .find("0202020")
            .expect("the other hash is visible") as u16;
        assert_eq!(
            terminal.backend().buffer()[(selected_hash, 0)].fg,
            Color::Reset,
            "the copied hash loses its color"
        );
        assert_eq!(
            terminal.backend().buffer()[(other_hash, 1)].fg,
            Color::Magenta,
            "copy feedback does not affect other rows"
        );

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert_eq!(
            terminal.backend().buffer()[(selected_hash, 0)].fg,
            Color::Magenta,
            "the hash color returns on the next frame"
        );

        drop(app.update(Action::CopyAuthor));
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        let selected_author = rendered_line(&terminal, 0)
            .find("author")
            .expect("the selected author is visible") as u16;
        let other_author = rendered_line(&terminal, 1)
            .find("author")
            .expect("the other author is visible") as u16;
        assert_eq!(
            terminal.backend().buffer()[(selected_author, 0)].fg,
            Color::Reset,
            "the copied author loses its color"
        );
        assert_eq!(
            terminal.backend().buffer()[(other_author, 1)].fg,
            Color::Green,
            "author feedback does not affect other rows"
        );

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert_eq!(
            terminal.backend().buffer()[(selected_author, 0)].fg,
            Color::Green,
            "the author color returns on the next frame"
        );
        Ok(())
    }

    #[test]
    fn colors_commit_markers_by_signature_state() -> Result<(), Box<dyn std::error::Error>> {
        let states = [
            (SignatureState::Unsigned, Color::Blue),
            (SignatureState::Unverified, Color::Rgb(255, 165, 0)),
            (SignatureState::Verified, Color::Green),
            (SignatureState::Failed, Color::LightRed),
            (SignatureState::PendingRebase, Color::Gray),
        ];
        let mut terminal = Terminal::new(TestBackend::new(4, states.len() as u16))?;
        terminal.draw(|frame| {
            for (y, (state, _)) in states.iter().enumerate() {
                for (x, head) in [(0, false), (2, true)] {
                    frame.render_widget(Paragraph::new("●─"), Rect::new(x, y as u16, 2, 1));
                    color_graph(
                        frame,
                        Rect::new(x, y as u16, 2, 1),
                        "●─",
                        0,
                        Some(signature_color(*state)),
                        *state,
                        head.then_some(HeadState {
                            has_descendants: false,
                            attached: false,
                        }),
                    );
                }
            }
        })?;

        for (y, (_, expected)) in states.iter().enumerate() {
            assert_eq!(terminal.backend().buffer()[(0, y as u16)].symbol(), "●");
            assert_eq!(terminal.backend().buffer()[(2, y as u16)].symbol(), "@");
            for x in 0..4 {
                let cell = &terminal.backend().buffer()[(x, y as u16)];
                assert_eq!(cell.fg, *expected);
                assert!(cell.modifier.contains(Modifier::REVERSED));
            }
        }
        Ok(())
    }

    #[test]
    fn review_diamond_uses_review_style_unless_head() -> Result<(), Box<dyn std::error::Error>> {
        let mut terminal = Terminal::new(TestBackend::new(4, 1))?;
        terminal.draw(|frame| {
            frame.render_widget(Paragraph::new("◆─◆─"), Rect::new(0, 0, 4, 1));
            color_graph(
                frame,
                Rect::new(0, 0, 2, 1),
                "◆─",
                0,
                None,
                SignatureState::Unsigned,
                None,
            );
            color_graph(
                frame,
                Rect::new(2, 0, 2, 1),
                "◆─",
                0,
                None,
                SignatureState::Unsigned,
                Some(HeadState {
                    has_descendants: true,
                    attached: false,
                }),
            );
        })?;
        let review = &terminal.backend().buffer()[(0, 0)];
        assert_eq!(review.symbol(), "◆");
        assert_eq!(review.fg, Color::LightMagenta);
        assert!(review.modifier.contains(Modifier::BOLD));
        assert_eq!(terminal.backend().buffer()[(2, 0)].symbol(), "@");
        assert_eq!(terminal.backend().buffer()[(2, 0)].fg, Color::Blue);
        assert_eq!(terminal.backend().buffer()[(3, 0)].symbol(), "─");
        Ok(())
    }

    #[test]
    fn italicizes_only_an_attached_head_marker() -> Result<(), Box<dyn std::error::Error>> {
        let id = gix::ObjectId::Sha1([1; 20]);
        let mut app = App::new(1);
        app.extend_commits(vec![Commit {
            id,
            parent_ids: Default::default(),
            author_time: gix::date::Time::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unsigned,
        }]);
        complete(&mut app);
        app.selected = None;
        app.set_lane(0, "● ");
        let mut terminal = Terminal::new(TestBackend::new(80, 2))?;
        let head = Decoration {
            name: "HEAD".into(),
            kind: DecorationKind::Head,
        };

        let attached = Decorations::from([(
            id,
            vec![
                head.clone(),
                Decoration {
                    name: "main".into(),
                    kind: DecorationKind::CurrentWorktreeBranch,
                },
            ],
        )]);
        terminal.draw(|frame| draw(frame, &mut app, &attached))?;
        let marker = &terminal.backend().buffer()[(8, 0)];
        assert_eq!(marker.symbol(), "@");
        assert!(marker.modifier.contains(Modifier::ITALIC), "attached HEAD is italic");

        let detached = Decorations::from([(
            id,
            vec![
                head,
                Decoration {
                    name: "worktree".into(),
                    kind: DecorationKind::CurrentWorktreeDetached,
                },
            ],
        )]);
        terminal.draw(|frame| draw(frame, &mut app, &detached))?;
        let marker = &terminal.backend().buffer()[(8, 0)];
        assert_eq!(marker.symbol(), "@");
        assert!(!marker.modifier.contains(Modifier::ITALIC), "detached HEAD is upright");
        Ok(())
    }

    #[test]
    fn reverses_an_unselected_head_title_and_the_full_selected_row() -> Result<(), Box<dyn std::error::Error>> {
        let head = gix::ObjectId::Sha1([1; 20]);
        let child = gix::ObjectId::Sha1([2; 20]);
        let commit = |id: gix::ObjectId, parent: Option<gix::ObjectId>| Commit {
            id,
            parent_ids: parent.into_iter().collect(),
            author_time: gix::date::Time::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unsigned,
        };
        let mut app = App::new(2);
        app.id_mode = IdMode::Commit;
        app.set_worktree_head(Some(head), false);
        app.extend_commits(vec![commit(child, Some(head)), commit(head, None)]);
        complete(&mut app);
        app.selected = Some(0);
        let decorations = Decorations::from([(
            head,
            vec![Decoration {
                name: "HEAD".into(),
                kind: DecorationKind::Head,
            }],
        )]);
        let mut terminal = Terminal::new(TestBackend::new(80, 3))?;

        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        let head_row = 1;
        let line = rendered_line(&terminal, head_row);
        let title = line.find("subject").expect("the HEAD title is visible") as u16;
        for x in title..title + "subject".len() as u16 {
            assert!(
                terminal.backend().buffer()[(x, head_row)]
                    .modifier
                    .contains(Modifier::REVERSED),
                "the unselected HEAD title is reversed"
            );
        }
        for x in [0, 6, 8, 11, 18, 79] {
            assert!(
                !terminal.backend().buffer()[(x, head_row)]
                    .modifier
                    .contains(Modifier::REVERSED),
                "HEAD emphasis does not invert the gutter or metadata"
            );
        }
        assert!(
            (0..80).all(|x| !terminal.backend().buffer()[(x, head_row)]
                .modifier
                .contains(Modifier::UNDERLINED)),
            "the old HEAD underline is gone"
        );
        assert!(
            terminal.backend().buffer()[(8, head_row)]
                .modifier
                .contains(Modifier::BOLD),
            "the non-tip @ is bold"
        );

        app.selected = Some(1);
        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        assert!(
            terminal.backend().buffer()[(title, head_row)]
                .modifier
                .contains(Modifier::REVERSED),
            "selection reverses the HEAD title as part of the full row"
        );
        assert!(
            terminal.backend().buffer()[(8, head_row)]
                .modifier
                .contains(Modifier::BOLD),
            "the selected non-tip @ remains bold"
        );

        app.set_worktree_head(Some(child), false);
        let decorations = Decorations::from([(
            child,
            vec![Decoration {
                name: "HEAD".into(),
                kind: DecorationKind::Head,
            }],
        )]);
        app.selected = Some(1);
        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        let tip_title = rendered_line(&terminal, 0)
            .find("subject")
            .expect("the tip title is visible") as u16;
        assert!(
            terminal.backend().buffer()[(tip_title, 0)]
                .modifier
                .contains(Modifier::REVERSED),
            "an unselected tip HEAD title is reversed too"
        );
        assert!(
            !terminal.backend().buffer()[(8, 0)].modifier.contains(Modifier::BOLD),
            "a tip @ keeps its normal weight"
        );
        Ok(())
    }

    #[test]
    fn highlights_foreign_worktree_head_titles_without_competing_with_current_head()
    -> Result<(), Box<dyn std::error::Error>> {
        let current = gix::ObjectId::Sha1([1; 20]);
        let attached = gix::ObjectId::Sha1([2; 20]);
        let detached = gix::ObjectId::Sha1([3; 20]);
        let ordinary = gix::ObjectId::Sha1([4; 20]);
        let commit = |id: gix::ObjectId, title: &'static str| Commit {
            id,
            parent_ids: Default::default(),
            author_time: gix::date::Time::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: title.into(),
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unsigned,
        };
        let mut app = App::new(4);
        app.id_mode = IdMode::Commit;
        app.ref_mode = RefMode::None;
        app.set_worktree_head(Some(current), false);
        app.extend_commits(vec![
            commit(current, "current title"),
            commit(attached, "attached title"),
            commit(detached, "detached title"),
            commit(ordinary, "ordinary title"),
        ]);
        complete(&mut app);
        app.selected = None;
        let decorations = Decorations::from([
            (
                current,
                vec![
                    Decoration {
                        name: "HEAD".into(),
                        kind: DecorationKind::Head,
                    },
                    Decoration {
                        name: "shared".into(),
                        kind: DecorationKind::WorktreeBranch,
                    },
                ],
            ),
            (
                attached,
                vec![Decoration {
                    name: "topic".into(),
                    kind: DecorationKind::WorktreeBranch,
                }],
            ),
            (
                detached,
                vec![Decoration {
                    name: "agent-wt".into(),
                    kind: DecorationKind::WorktreeDetached,
                }],
            ),
        ]);
        let mut terminal = Terminal::new(TestBackend::new(100, 5))?;

        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        let current_x = rendered_line(&terminal, 0)
            .find("current title")
            .expect("the current HEAD title is visible") as u16;
        assert!(
            terminal.backend().buffer()[(current_x, 0)]
                .modifier
                .contains(Modifier::REVERSED),
            "the current HEAD keeps its stronger reverse-video title"
        );
        assert_eq!(
            terminal.backend().buffer()[(current_x, 0)].bg,
            Color::Reset,
            "foreign worktrees do not replace current HEAD reverse video"
        );
        for (row, title) in [(1, "attached title"), (2, "detached title")] {
            let line = rendered_line(&terminal, row);
            let title_x = line.find(title).expect("the foreign worktree title is visible") as u16;
            let title_cell = &terminal.backend().buffer()[(title_x, row)];
            assert_eq!(title_cell.bg, Color::DarkGray, "the foreign HEAD title is shaded");
            assert!(
                !title_cell.modifier.contains(Modifier::REVERSED),
                "foreign HEAD titles remain less prominent than current HEAD"
            );
            assert_eq!(
                terminal.backend().buffer()[(0, row)].bg,
                Color::Reset,
                "the foreign HEAD background is limited to its title"
            );
            assert!(
                !line.contains('@'),
                "hidden reference labels do not provide the highlight"
            );
        }
        let ordinary_line = rendered_line(&terminal, 3);
        let ordinary_x = ordinary_line
            .find("ordinary title")
            .expect("the ordinary title is visible") as u16;
        assert_eq!(
            terminal.backend().buffer()[(ordinary_x, 3)].bg,
            Color::Reset,
            "ordinary commit titles keep the terminal background"
        );

        app.selected = Some(1);
        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        let selected_line = rendered_line(&terminal, 1);
        let selected_title = selected_line
            .find("attached title")
            .expect("the selected foreign worktree title is visible") as u16;
        assert_eq!(
            terminal.backend().buffer()[(selected_title, 1)].bg,
            Color::Reset,
            "selection takes precedence over foreign HEAD emphasis"
        );
        assert!(
            !terminal.backend().buffer()[(selected_title, 1)]
                .modifier
                .contains(Modifier::REVERSED),
            "a selected foreign worktree title stays outside the row inversion"
        );
        Ok(())
    }

    #[test]
    fn marks_and_highlights_a_dirty_review_head_independently_of_selection() -> Result<(), Box<dyn std::error::Error>> {
        let head = gix::ObjectId::Sha1([1; 20]);
        let other = gix::ObjectId::Sha1([2; 20]);
        let mut app = App::new(5);
        app.extend_commits(
            [head, other]
                .into_iter()
                .enumerate()
                .map(|(index, id)| Commit {
                    id,
                    parent_ids: Default::default(),
                    author_time: gix::date::Time::default(),
                    committer_time: gix::date::Time::default(),
                    author: author(b"author", b"author@example.com"),
                    attributions: 0..0,
                    title: if index == 0 {
                        "feat(scope)!: subject"
                    } else {
                        "fix(scope)!: subject"
                    }
                    .into(),
                    metadata_loaded: true,
                    has_agent_marker: false,
                    is_review: false,
                    signature: SignatureState::Unsigned,
                })
                .collect::<Vec<_>>(),
        );
        complete(&mut app);
        let decorations = Decorations::from([(
            head,
            vec![Decoration {
                name: "HEAD".into(),
                kind: DecorationKind::Head,
            }],
        )]);
        let dirty = Changes {
            paths: vec![crate::app::PathChange {
                kind: ChangeKind::Modified,
                group: ChangeGroup::Unstaged,
                source: None,
                path: "dirty".into(),
                lines: None,
            }],
            ..Changes::default()
        };
        let mut terminal = Terminal::new(TestBackend::new(80, 8))?;

        app.selected = Some(1);
        std::sync::Arc::make_mut(&mut app.rows[0]).is_review = true;
        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &decorations,
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                Some(&dirty),
            );
        })?;
        assert!(rendered_line(&terminal, 0).trim_start().starts_with("🫟 @"));
        assert!(rendered_line(&terminal, 1).trim_start().starts_with("> ●"));
        assert_eq!(terminal.backend().buffer()[(6, 0)].bg, REVIEW_BACKGROUND);
        assert!(
            !terminal.backend().buffer()[(6, 0)]
                .modifier
                .contains(Modifier::REVERSED)
        );
        assert!(
            terminal.backend().buffer()[(6, 1)]
                .modifier
                .contains(Modifier::REVERSED)
        );

        app.selected = Some(0);
        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &decorations,
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                Some(&dirty),
            );
        })?;
        let line = rendered_line(&terminal, 0);
        assert!(line.trim_start().starts_with("🫟 @"));
        let start = line[..line.find("🫟").expect("the dirty marker is visible")]
            .chars()
            .count() as u16;
        let title = line[..line.find("feat(scope)!: subject").expect("the title is visible")]
            .chars()
            .count() as u16;
        let end = title - 1;
        let buffer = terminal.backend().buffer();
        let highlighted = |x| {
            buffer[(x, 0)].fg == Color::Black
                && buffer[(x, 0)].bg == REVIEW_BACKGROUND
                && !buffer[(x, 0)].modifier.contains(Modifier::REVERSED)
        };
        assert!(
            highlighted(start) && (start + Line::raw("🫟").width() as u16..end).all(highlighted),
            "the review background wins from the first visible gutter through its metadata"
        );
        assert_ne!(buffer[(end, 0)].bg, REVIEW_BACKGROUND, "one space separates the title");
        assert!(
            buffer[(end, 0)].modifier.contains(Modifier::REVERSED),
            "ordinary selection remains visible outside the review background"
        );

        app.changes_mode = Some(ChangesMode::Both);
        app.set_lane(0, "│ ◆─┐ ");
        app.set_lane(1, "│ ● ");
        let mut shortened = Terminal::new(TestBackend::new(41, 8))?;
        shortened.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &decorations,
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&dirty),
                Some(&dirty),
            );
        })?;
        assert_eq!(app.changes_layout, ChangesLayout::Stacked);
        let line = rendered_line(&shortened, 0);
        let other_line = rendered_line(&shortened, 1);
        assert!(
            line.contains("│ @─┐")
                && line.contains("1970-01-01 author …:")
                && other_line.contains("1970-01-01 author")
                && other_line.contains("…:"),
            "stacking does not minimize rows when shortening is sufficient: {line:?} / {other_line:?}"
        );
        let head_x = line.chars().position(|symbol| symbol == '@').expect("HEAD is visible") as u16;
        let title_x = line[..line.find("…:").expect("the title is visible")].chars().count() as u16;
        let buffer = shortened.backend().buffer();
        assert!(title_x > head_x + 2, "metadata remains between the disc and title");
        assert_eq!(buffer[(head_x, 0)].bg, REVIEW_BACKGROUND);
        assert_ne!(buffer[(title_x - 1, 0)].bg, REVIEW_BACKGROUND);
        assert!(buffer[(title_x, 0)].modifier.contains(Modifier::REVERSED));

        app.changes_suppressed = true;
        shortened.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &decorations,
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&dirty),
                Some(&dirty),
            );
        })?;
        assert!(
            rendered_line(&shortened, 0).contains("…:") && rendered_line(&shortened, 1).contains("…:"),
            "repeat suppression retains the width-derived row layout"
        );
        app.changes_suppressed = false;
        app.changes_mode = None;
        let mut compact = Terminal::new(TestBackend::new(31, 3))?;
        compact.draw(|frame| draw(frame, &mut app, &decorations))?;
        let line = rendered_line(&compact, 0);
        assert!(
            line.contains("│ @ …:subject") && !line.contains("1970-01-01") && !line.contains("─┐"),
            "narrow history minimizes without a changes pane: {line:?}"
        );
        let head_x = line.chars().position(|symbol| symbol == '@').expect("HEAD is visible") as u16;
        assert_eq!(compact.backend().buffer()[(head_x, 0)].bg, REVIEW_BACKGROUND);

        app.alignment = HistoryAlignment::None;
        compact.draw(|frame| draw(frame, &mut app, &decorations))?;
        app.update(Action::ScrollRight);
        compact.draw(|frame| draw(frame, &mut app, &decorations))?;
        let line = rendered_line(&compact, 0);
        assert!(
            line.contains("feat(scope)!:") && !line.contains("…:"),
            "unaligned history retains the complete prefix while scrolling: {line:?}"
        );
        app.alignment = HistoryAlignment::Title;
        app.horizontal_offset = 0;
        app.changes_mode = Some(ChangesMode::Both);
        app.set_lane(0, "◆ ");
        app.set_lane(1, "● ");
        std::sync::Arc::make_mut(&mut app.rows[0]).is_review = false;

        let conflicted = Changes {
            paths: vec![crate::app::PathChange {
                kind: ChangeKind::Unmerged,
                group: ChangeGroup::Unstaged,
                source: None,
                path: "dirty".into(),
                lines: None,
            }],
            ..Changes::default()
        };
        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &decorations,
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                Some(&conflicted),
            );
        })?;
        assert!(
            rendered_line(&terminal, 0).trim_start().starts_with("💥 🫟 @"),
            "the conflict gutter precedes the ordinary status: {:?}",
            rendered_line(&terminal, 0)
        );
        assert_eq!(terminal.backend().buffer()[(6, 0)].fg, Color::LightRed);
        assert!(
            !terminal.backend().buffer()[(6, 0)]
                .modifier
                .contains(Modifier::SLOW_BLINK),
            "the conflict marker remains steady"
        );

        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &decorations,
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                Some(&dirty),
            );
        })?;
        assert!(
            rendered_line(&terminal, 0).trim_start().starts_with("🫟 @"),
            "resolving the index restores the ordinary dirty marker"
        );

        app.arm_rebase_conflict(other);
        app.selected = Some(1);
        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &decorations,
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                Some(&dirty),
            );
        })?;
        assert!(
            rendered_line(&terminal, 1).trim_start().starts_with("💥 > ●"),
            "the conflict gutter does not replace selection: {:?}",
            rendered_line(&terminal, 1)
        );
        app.clear_rebase_conflict();
        app.selected = Some(0);

        app.changes_mode = Some(ChangesMode::Tree);
        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &decorations,
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                Some(&dirty),
            );
        })?;
        assert!(rendered_line(&terminal, 0).trim_start().starts_with("> @"));
        Ok(())
    }

    #[test]
    fn shows_signature_action_only_while_actionable() -> Result<(), Box<dyn std::error::Error>> {
        let id = gix::ObjectId::Sha1([1; 20]);
        let mut app = App::new(1);
        app.extend_commits(vec![Commit {
            id,
            parent_ids: Default::default(),
            author_time: gix::date::Time::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unverified,
        }]);
        complete(&mut app);
        let mut terminal = Terminal::new(TestBackend::new(160, 3))?;

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(rendered_line(&terminal, 2).contains("copy · refs · ? ·"));
        assert!(!rendered_line(&terminal, 2).contains("s ● -> ●"));

        app.information_expanded = true;
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(rendered_line(&terminal, 0).contains(" s ● -> ● · [ title"));
        assert!(rendered_line(&terminal, 1).contains("p command"));
        assert!(rendered_line(&terminal, 2).contains("copy · refs · ?"));

        app.finish_signature_verification(vec![(id, false)]);
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(rendered_line(&terminal, 0).contains(" s 1 ● · [ title"));
        Ok(())
    }

    #[test]
    fn advertises_cancel_only_while_loading() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(1);
        let mut terminal = Terminal::new(TestBackend::new(180, 2))?;

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(
            !rendered_line(&terminal, 1).contains("loading"),
            "loading is already apparent from the streaming history"
        );
        assert!(rendered_line(&terminal, 1).contains("Esc cancel"));

        app.update(Action::Cancel);
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(!rendered_line(&terminal, 1).contains("Esc cancel"));
        Ok(())
    }

    #[test]
    fn toggles_the_full_commit_message_in_a_padded_half_width_pane() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(3);
        app.commit_pane_background = Some((15, 16, 17));
        app.extend_commits(vec![Commit {
            id: gix::ObjectId::Sha1([1; 20]),
            parent_ids: Default::default(),
            author_time: gix::date::Time::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unsigned,
        }]);
        let mut terminal = Terminal::new(TestBackend::new(120, 8))?;

        app.information_expanded = true;
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(popup_is_dim(&terminal, "message"), "the closed commit pane is dimmed");

        app.update(Action::ToggleCommit);
        terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                Some(b"subject\n\nbody".as_bstr()),
                None,
            );
        })?;
        assert_eq!(
            terminal.backend().buffer()[(62, 1)].symbol(),
            "s",
            "the title starts after horizontal and vertical margins"
        );
        assert_eq!(
            terminal.backend().buffer()[(60, 0)].symbol(),
            " ",
            "the pane starts with padding instead of a border"
        );
        assert_eq!(
            terminal.backend().buffer()[(60, 0)].bg,
            Color::Rgb(15, 16, 17),
            "the commit pane has the derived terminal-background shade"
        );
        assert_eq!(
            terminal.backend().buffer()[(62, 1)].bg,
            Color::Rgb(15, 16, 17),
            "the shade extends behind the commit message"
        );
        assert_eq!(
            terminal.backend().buffer()[(59, 0)].bg,
            Color::Reset,
            "the history background is unchanged"
        );
        assert_eq!(
            terminal.backend().buffer()[(62, 3)].symbol(),
            "b",
            "the commit body remains separated from its title"
        );
        assert!(
            !popup_is_dim(&terminal, "message"),
            "the open commit pane is not dimmed"
        );

        app.update(Action::ToggleCommit);
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert_eq!(
            terminal.backend().buffer()[(62, 3)].symbol(),
            " ",
            "closing the pane removes the commit body"
        );
        assert_eq!(
            terminal.backend().buffer()[(62, 3)].bg,
            Color::Reset,
            "closing the pane removes its background shade"
        );

        app.update(Action::ToggleCommit);
        let mut wide_terminal = Terminal::new(TestBackend::new(200, 6))?;
        let conventional_line = format!("{} word", "x".repeat(75));
        wide_terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                Some(conventional_line.as_bytes().as_bstr()),
                None,
            );
        })?;
        assert_eq!(
            wide_terminal.backend().buffer()[(118, 1)].symbol(),
            "x",
            "the pane reserves eighty content columns on a wide screen"
        );
        assert!(
            rendered_line(&wide_terminal, 1)
                .chars()
                .skip(118)
                .take(80)
                .collect::<String>()
                .ends_with(" word")
                && rendered_line(&wide_terminal, 2)
                    .chars()
                    .skip(118)
                    .take(80)
                    .collect::<String>()
                    .trim()
                    .is_empty(),
            "an eighty-column message line does not wrap its final word"
        );
        Ok(())
    }

    #[test]
    fn pages_overflowing_commit_messages_and_hides_the_status_when_they_fit() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut app = App::new(4);
        app.commit_pane_background = Some((15, 16, 17));
        app.extend_commits(vec![Commit {
            id: gix::ObjectId::Sha1([1; 20]),
            parent_ids: Default::default(),
            author_time: gix::date::Time::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unsigned,
        }]);
        app.update(Action::ToggleCommit);
        let message = b"subject\n\none\ntwo\nthree\nfour\nfive\nsix\n\nSigned-off-by: Alice".as_bstr();
        let mut terminal = Terminal::new(TestBackend::new(120, 7))?;

        terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                Some(message),
                None,
            );
        })?;
        assert!(
            rendered_line(&terminal, 5).contains("PgUp/C-b up page · PgDn/C-f down page"),
            "overflowing commit messages advertise both full-page key pairs"
        );
        assert_eq!(
            terminal.backend().buffer()[(62, 5)].bg,
            PANE_STATUS_BACKGROUND,
            "the commit status has the shared pane-status background"
        );
        assert_eq!(
            terminal.backend().buffer()[(0, 6)].bg,
            Color::Reset,
            "the main status keeps its original background"
        );

        app.information_expanded = true;
        terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                Some(message),
                None,
            );
        })?;
        assert!(
            rendered_line(&terminal, 3).contains("PgUp/C-b up page · PgDn/C-f down page"),
            "the popup moves the commit-message status up"
        );
        assert!(rendered_line(&terminal, 4).contains("[ title"));
        assert!(rendered_line(&terminal, 5).contains("p command"));
        app.information_expanded = false;

        app.update(Action::PageDown);
        app.update(Action::PageDown);
        terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                Some(message),
                None,
            );
        })?;
        assert!(
            rendered_line(&terminal, 4).contains("Alice"),
            "the last page reaches aligned trailers"
        );

        terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                Some(b"subject".as_bstr()),
                None,
            );
        })?;
        assert!(
            !rendered_line(&terminal, 5).contains("PgUp"),
            "the commit status disappears when all content fits"
        );
        assert_eq!(app.commit_offset, 0, "shorter content clamps the old offset");
        Ok(())
    }

    #[test]
    fn popup_keeps_a_focused_changes_error_visible() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(1);
        app.id_mode = IdMode::Commit;
        app.extend_commits(
            (1..=5)
                .map(|n| Commit {
                    id: gix::ObjectId::Sha1([n; 20]),
                    parent_ids: Default::default(),
                    author_time: gix::date::Time::default(),
                    committer_time: gix::date::Time::default(),
                    author: author(b"author", b"author@example.com"),
                    attributions: 0..0,
                    title: format!("history {n}").into(),
                    metadata_loaded: true,
                    has_agent_marker: false,
                    is_review: false,
                    signature: SignatureState::Unsigned,
                })
                .collect::<Vec<_>>(),
        );
        complete(&mut app);
        app.changes_focus = Some(ChangePane::Tree);
        app.tree_changes.error = Some("failed deliberately".into());
        app.information_expanded = true;
        let changes = Changes {
            paths: vec![crate::app::PathChange {
                kind: ChangeKind::Modified,
                group: ChangeGroup::Tree,
                source: None,
                path: "file".into(),
                lines: None,
            }],
            ..Changes::default()
        };
        let mut terminal = Terminal::new(TestBackend::new(120, 6))?;

        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&changes),
                None,
            );
        })?;

        assert!(rendered_line(&terminal, 2).contains("diff: failed deliberately"));
        assert_eq!(terminal.backend().buffer()[(2, 2)].bg, PANE_STATUS_BACKGROUND);
        assert!(rendered_line(&terminal, 3).contains("[ title"));
        assert!(rendered_line(&terminal, 4).contains("p command"));
        Ok(())
    }

    #[test]
    fn changing_the_changes_height_keeps_history_alignment_stable() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(11);
        app.id_mode = IdMode::Commit;
        app.extend_commits(
            (1..=10)
                .map(|n| Commit {
                    id: gix::ObjectId::Sha1([n; 20]),
                    parent_ids: Default::default(),
                    author_time: gix::date::Time::default(),
                    committer_time: gix::date::Time::default(),
                    author: author(b"author", b"author@example.com"),
                    attributions: 0..0,
                    title: format!("subject {n}").into(),
                    metadata_loaded: true,
                    has_agent_marker: false,
                    is_review: false,
                    signature: SignatureState::Unsigned,
                })
                .collect::<Vec<_>>(),
        );
        complete(&mut app);
        app.selected = Some(7);
        app.ensure_visible();
        let selection = app.selected;
        app.set_lane(6, "●──────── ");
        let path = crate::app::PathChange {
            kind: ChangeKind::Modified,
            group: ChangeGroup::Tree,
            source: None,
            path: "path".into(),
            lines: None,
        };
        let changes = |len| Changes {
            paths: vec![path.clone(); len],
            ..Changes::default()
        };
        let mut terminal = Terminal::new(TestBackend::new(80, 12))?;

        terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&changes(1)),
            );
        })?;
        let short = rendered_line(&terminal, 0)
            .find("0101010")
            .expect("metadata is visible with a short changes pane");
        assert_eq!((app.selected, app.offset), (selection, 0));

        terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&changes(8)),
            );
        })?;
        assert_eq!(
            rendered_line(&terminal, 0).find("0404040"),
            Some(short),
            "changes pane height does not move aligned history metadata"
        );
        assert_eq!(
            (app.selected, app.offset),
            (selection, 3),
            "the selected commit stays immediately above the taller changes pane"
        );

        app.update(Action::MoveDown);
        assert_eq!(
            (app.selected, app.offset),
            (Some(8), 4),
            "moving down advances the commit and scrolls history at the pane boundary"
        );

        terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&changes(1)),
            );
        })?;
        assert_eq!(
            (app.selected, app.offset),
            (Some(8), 4),
            "a shorter changes pane does not pull history back into the freed space"
        );
        Ok(())
    }

    #[test]
    fn shows_changed_paths_in_a_bottom_pane_below_the_summary() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(6);
        app.id_mode = IdMode::Commit;
        app.commit_pane_background = Some((15, 16, 17));
        app.extend_commits(vec![
            Commit {
                id: gix::ObjectId::Sha1([1; 20]),
                parent_ids: [gix::ObjectId::Sha1([2; 20]), gix::ObjectId::Sha1([3; 20])]
                    .into_iter()
                    .collect(),
                author_time: gix::date::Time::default(),
                committer_time: gix::date::Time::default(),
                author: author(b"author", b"author@example.com"),
                attributions: 0..0,
                title: "merge".into(),
                metadata_loaded: true,
                has_agent_marker: false,
                is_review: false,
                signature: SignatureState::Unsigned,
            },
            Commit {
                id: gix::ObjectId::Sha1([2; 20]),
                parent_ids: Default::default(),
                author_time: gix::date::Time::default(),
                committer_time: gix::date::Time::default(),
                author: author(b"author", b"author@example.com"),
                attributions: 0..0,
                title: "parent".into(),
                metadata_loaded: true,
                has_agent_marker: false,
                is_review: false,
                signature: SignatureState::Unsigned,
            },
        ]);
        complete(&mut app);
        let changes = Changes {
            parent: None,
            range: None,
            paths: vec![
                crate::app::PathChange {
                    kind: ChangeKind::Added,
                    group: ChangeGroup::Tree,
                    source: None,
                    path: "added".into(),
                    lines: Some((10, 0)),
                },
                crate::app::PathChange {
                    kind: ChangeKind::Modified,
                    group: ChangeGroup::Tree,
                    source: None,
                    path: "modified".into(),
                    lines: Some((5, 2)),
                },
                crate::app::PathChange {
                    kind: ChangeKind::Deleted,
                    group: ChangeGroup::Tree,
                    source: None,
                    path: "deleted".into(),
                    lines: Some((0, 7)),
                },
                crate::app::PathChange {
                    kind: ChangeKind::Renamed,
                    group: ChangeGroup::Tree,
                    source: Some("old".into()),
                    path: "new".into(),
                    lines: Some((3, 3)),
                },
                crate::app::PathChange {
                    kind: ChangeKind::Copied,
                    group: ChangeGroup::Tree,
                    source: Some("source".into()),
                    path: "copy".into(),
                    lines: Some((0, 0)),
                },
                crate::app::PathChange {
                    kind: ChangeKind::TypeChanged,
                    group: ChangeGroup::Tree,
                    source: None,
                    path: format!("{}tail", "x".repeat(130)).into(),
                    lines: Some((24, 5)),
                },
            ],
            diffs: Vec::new(),
            lines_added: 42,
            lines_removed: 17,
            has_tracked_changes: false,
        };
        let mut terminal = Terminal::new(TestBackend::new(120, 16))?;

        terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&changes),
            );
        })?;
        let mut footer_terminal = Terminal::new(TestBackend::new(240, 16))?;
        footer_terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&changes),
            );
        })?;
        assert!(
            rendered_line(&footer_terminal, 15).contains("? · quit"),
            "the collapsed information prefix is followed only by quit"
        );
        assert!(
            !rendered_line(&footer_terminal, 15).contains("<tab> switch")
                && !rendered_line(&footer_terminal, 15).contains("↑↓/jk move")
                && !rendered_line(&footer_terminal, 15).contains("h/l pan")
                && !rendered_line(&footer_terminal, 15).contains("<enter> diff"),
            "information actions are hidden with their prefix"
        );
        app.information_expanded = true;
        footer_terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&changes),
            );
        })?;
        assert!(
            rendered_line(&footer_terminal, 13).contains(" [ title · ref-tree · message · changes "),
            "the expanded information prefix floats its actions above the navigation"
        );
        assert!(
            rendered_line(&footer_terminal, 14).contains(
                " p command · <tab> switch · ↑↓/jk move · h/l pan · J/K topo · PgUp/PgDn move · Shift+PgUp/PgDn pan · <enter> diff "
            ),
            "the expanded information prefix keeps keyboard help next to the footer"
        );
        assert!(rendered_line(&footer_terminal, 15).contains("? · quit"));
        app.information_expanded = false;

        assert_eq!(
            terminal.backend().buffer()[(119, 7)].symbol(),
            "─",
            "the changes pane starts at the screen's halfway point"
        );
        assert!(
            terminal.backend().buffer()[(119, 7)].modifier.contains(Modifier::DIM),
            "the inactive changes border is dimmed"
        );
        assert!(
            !terminal.backend().buffer()[(5, 0)].modifier.contains(Modifier::DIM)
                && !terminal.backend().buffer()[(2, 15)].modifier.contains(Modifier::DIM),
            "the focused history and its status use their normal intensity"
        );
        let summary = rendered_line(&terminal, 7);
        assert_eq!(
            terminal.backend().buffer()[(0, 7)].symbol(),
            "─",
            "the tree title border reaches the left edge"
        );
        assert!(
            summary.contains("Tree 0101010 ── A 1 + M 1 + D 1 + R 1 + C 1 + T 1 = 6 · +42 -17"),
            "the top border contains the tree identity and aggregates"
        );
        let position = |needle| {
            summary[..summary.find(needle).expect("aggregate is visible")]
                .chars()
                .count() as u16
        };
        let added_x = position("A 1");
        let deleted_x = position("D 1");
        assert_eq!(terminal.backend().buffer()[(added_x, 7)].fg, Color::Green);
        assert_eq!(terminal.backend().buffer()[(deleted_x, 7)].fg, Color::LightRed);
        assert!(
            terminal.backend().buffer()[(added_x, 7)]
                .modifier
                .contains(Modifier::DIM),
            "the inactive summary is dimmed without losing its colors"
        );
        assert!(
            rendered_line(&terminal, 8).contains("A added"),
            "changed paths follow the summary border in diff order"
        );
        let inactive_path = rendered_line(&terminal, 8);
        let inactive_x = inactive_path.find("A added").expect("changed path is visible") as u16;
        assert!(
            terminal.backend().buffer()[(inactive_x, 8)]
                .modifier
                .contains(Modifier::DIM)
                && terminal.backend().buffer()[(inactive_x + 2, 8)]
                    .modifier
                    .contains(Modifier::DIM),
            "the inactive change kind and path are dimmed"
        );
        assert!(
            !rendered_line(&terminal, 8).contains("+10"),
            "inactive panes do not display a path selection"
        );
        assert!(
            rendered_line(&terminal, 13).contains("T "),
            "reclaiming the summary row lets all paths fit"
        );
        assert!(
            !rendered_line(&terminal, 14).contains("↑↓/jk move · h/l pan"),
            "the unfocused changes status is hidden"
        );
        assert_eq!(
            terminal.backend().buffer()[(2, 15)].bg,
            Color::Reset,
            "the main status keeps its original background"
        );
        assert!(!rendered_line(&terminal, 15).contains("<tab> switch"));

        app.changes_suppressed = true;
        app.information_expanded = true;
        terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&changes),
            );
        })?;
        assert!(
            !rendered_line(&terminal, 7).contains("files changed"),
            "repeated history navigation temporarily hides the changes pane"
        );
        assert!(
            app.changes_mode.is_some() && !popup_is_dim(&terminal, "changes"),
            "temporary suppression leaves the persistent changes setting enabled"
        );
        app.changes_suppressed = false;

        app.update(Action::ToggleChangesFocus);
        app.update(Action::MoveDown);
        terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&changes),
            );
        })?;
        assert!(
            !terminal.backend().buffer()[(119, 7)].modifier.contains(Modifier::DIM),
            "the focused changes border uses its normal style"
        );
        assert!(
            rendered_line(&terminal, 14).contains("↑↓/jk move · h/l pan"),
            "the focused changes status advertises its navigation keys"
        );
        assert_eq!(
            terminal.backend().buffer()[(2, 14)].bg,
            PANE_STATUS_BACKGROUND,
            "the focused changes status uses the shared pane-status background"
        );
        assert!(
            terminal.backend().buffer()[(5, 0)].modifier.contains(Modifier::DIM)
                && !terminal.backend().buffer()[(2, 15)].modifier.contains(Modifier::DIM),
            "the inactive history is dimmed without dimming the main status"
        );
        assert!(!rendered_line(&terminal, 15).contains("<tab> → tree changes"));
        assert!(rendered_line(&terminal, 15).contains("q/Esc history"));
        terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&changes),
            );
        })?;
        assert!(!rendered_line(&terminal, 15).contains("<tab> switch"));
        assert!(
            !terminal.backend().buffer()[(added_x, 7)]
                .modifier
                .contains(Modifier::DIM)
                && !terminal.backend().buffer()[(2, 14)].modifier.contains(Modifier::DIM),
            "the focused summary and status use their normal intensity"
        );
        let selected = rendered_line(&terminal, 9);
        assert!(selected.contains("M modified +5 -2"));
        let path_x = selected.find("modified").expect("selected path is visible") as u16;
        let kind_x = selected.find("M modified").expect("selected kind is visible") as u16;
        let added_x = selected.find("+5").expect("selected additions are visible") as u16;
        let removed_x = selected.find("-2").expect("selected removals are visible") as u16;
        assert!(
            !terminal.backend().buffer()[(kind_x, 9)]
                .modifier
                .contains(Modifier::DIM)
                && !terminal.backend().buffer()[(path_x, 9)]
                    .modifier
                    .contains(Modifier::DIM),
            "focused paths use their normal intensity"
        );
        assert!(
            terminal.backend().buffer()[(path_x, 9)]
                .modifier
                .contains(Modifier::REVERSED),
            "the selected filepath is inverted"
        );
        assert_eq!(terminal.backend().buffer()[(added_x, 9)].fg, Color::Green);
        assert_eq!(terminal.backend().buffer()[(removed_x, 9)].fg, Color::LightRed);
        assert!(
            !terminal.backend().buffer()[(added_x, 9)]
                .modifier
                .contains(Modifier::REVERSED),
            "the diff-line suffix keeps its normal background"
        );
        assert!(
            !rendered_line(&terminal, 8).contains("+10"),
            "only the selected path displays its line counts"
        );
        assert!(rendered_line(&terminal, 13).contains("T "));
        assert!(rendered_line(&terminal, 14).contains("↑↓/jk move · h/l pan"));

        assert!(
            rendered_line(&terminal, 14).contains("<enter> diff · copy · cycle tree"),
            "the changes pane advertises the next cycle mode"
        );
        assert!(
            rendered_line(&terminal, 14).contains("copy"),
            "the changes pane advertises path copying"
        );

        app.update(Action::MoveUp);
        terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&changes),
            );
        })?;
        assert!(rendered_line(&terminal, 8).contains("A added +10"));
        assert!(
            !rendered_line(&terminal, 8).contains("-0"),
            "selected paths hide empty counts"
        );

        app.update(Action::Last);
        app.update(Action::ScrollRight);
        terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&changes),
            );
        })?;
        assert_eq!(app.tree_changes.horizontal_offset, 20);
        assert!(
            rendered_line(&terminal, 13).contains("tail"),
            "h/l pans long path rows while the summary remains fixed"
        );
        assert!(
            !rendered_line(&terminal, 13).contains("not shown"),
            "the overflow indicator disappears at the end"
        );

        let mut short_terminal = Terminal::new(TestBackend::new(120, 8))?;
        short_terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&changes),
            );
        })?;
        assert!(
            !rendered_line(&short_terminal, 5).contains("not shown"),
            "the overflow count disappears once the selected final path is visible"
        );

        let mut merge_changes = changes.clone();
        merge_changes.parent = Some(crate::app::ComparedParent {
            index: 0,
            total: 2,
            id: gix::ObjectId::Sha1([2; 20]),
        });
        terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&merge_changes),
            );
        })?;
        assert!(
            rendered_line(&terminal, 7).contains("Tree 0101010 ── A 1"),
            "parent context no longer crowds the aggregate border"
        );
        assert!(
            rendered_line(&terminal, 14).contains(
                "vs parent 1/2 0202020 · P next parent · ↑↓/jk move · h/l pan · <enter> diff · copy · cycle tree"
            ),
            "merge diffs keep parent controls alongside navigation"
        );
        let parent = rendered_line(&terminal, 1);
        let disk_x = parent.find('●').expect("the parent disk is visible") as u16;
        let hash_x = parent.find("0202020").expect("the parent hash is visible") as u16;
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(disk_x, 1)].fg, COMPARED_PARENT_COLOR);
        assert!(buffer[(disk_x, 1)].modifier.contains(Modifier::REVERSED));
        assert!(
            buffer[(hash_x, 1)].modifier.contains(Modifier::REVERSED),
            "the compared parent's hash is inverted"
        );
        assert!(
            !rendered_line(&terminal, 15).contains("next parent"),
            "parent cycling is absent from the main status line"
        );

        app.update(Action::ToggleCommit);
        let worktree_changes = Changes::default();
        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                Some(b"subject".as_bstr()),
                Some(&changes),
                Some(&worktree_changes),
            );
        })?;
        assert_eq!(
            app.changes_layout,
            ChangesLayout::Stacked,
            "both change blocks adapt to the width left by the commit pane"
        );
        assert!(
            rendered_line(&terminal, 7)
                .chars()
                .take(60)
                .collect::<String>()
                .contains("Worktree")
        );
        assert!(
            rendered_line(&terminal, 9)
                .chars()
                .take(60)
                .collect::<String>()
                .contains("Tree")
        );
        assert_eq!(terminal.backend().buffer()[(60, 7)].symbol(), " ");
        assert_eq!(
            terminal.backend().buffer()[(60, 7)].bg,
            Color::Rgb(15, 16, 17),
            "the shaded commit pane separates the overlays without a border"
        );
        assert_eq!(
            app.viewport_rows, 7,
            "history remains bounded above the highest overlay"
        );
        assert!(rendered_line(&terminal, 0).trim_start().starts_with('>'));

        let mut wide_terminal = Terminal::new(TestBackend::new(240, 16))?;
        wide_terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                Some(b"subject".as_bstr()),
                Some(&changes),
                Some(&worktree_changes),
            );
        })?;
        assert_eq!(
            app.changes_layout,
            ChangesLayout::SideBySide,
            "sufficient remaining width still permits side-by-side changes"
        );
        assert_eq!(wide_terminal.backend().buffer()[(156, 7)].symbol(), " ");
        assert_eq!(wide_terminal.backend().buffer()[(156, 7)].bg, Color::Rgb(15, 16, 17));
        Ok(())
    }

    #[test]
    fn summarizes_staged_and_unstaged_changes_in_the_top_border() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(1);
        app.changes_mode = Some(ChangesMode::Both);
        app.changes_focus = Some(ChangePane::Worktree);
        let changes = Changes {
            paths: vec![
                crate::app::PathChange {
                    kind: ChangeKind::Added,
                    group: ChangeGroup::Staged,
                    source: None,
                    path: "same".into(),
                    lines: Some((1, 0)),
                },
                crate::app::PathChange {
                    kind: ChangeKind::Modified,
                    group: ChangeGroup::Unstaged,
                    source: None,
                    path: "same".into(),
                    lines: Some((2, 1)),
                },
            ],
            lines_added: 3,
            lines_removed: 1,
            ..Changes::default()
        };
        let mut terminal = Terminal::new(TestBackend::new(80, 12))?;
        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                Some(&changes),
            );
        })?;

        let (header_y, header) = (0..12)
            .map(|y| (y, rendered_line(&terminal, y)))
            .find(|(_, line)| line.contains("Worktree"))
            .expect("the worktree border is visible");
        assert!(
            header.contains("Worktree ── S 1 + U 1 = 2 · +3 -1"),
            "the border distinguishes staged and unstaged rows: {header:?}"
        );
        let staged_y = header_y + 1;
        let divider_y = header_y + 2;
        let unstaged_y = header_y + 3;
        let divider = rendered_line(&terminal, divider_y);
        let label_x = divider[..divider.find("↑ index ↑").expect("the index label is visible")]
            .chars()
            .count() as u16;
        let staged_x = rendered_line(&terminal, staged_y).find('A').expect("staged letter") as u16;
        assert_eq!(label_x, staged_x, "the index label aligns with path kinds");
        assert_eq!(terminal.backend().buffer()[(label_x, divider_y)].fg, Color::Reset);
        assert!(
            terminal.backend().buffer()[(label_x, divider_y)]
                .modifier
                .contains(Modifier::DIM),
            "the label uses dimmed normal text"
        );
        let rail_x = label_x + "↑ index ↑ ".chars().count() as u16;
        assert_eq!(terminal.backend().buffer()[(rail_x, divider_y)].fg, Color::Green);
        assert!(
            !terminal.backend().buffer()[(rail_x, divider_y)]
                .modifier
                .contains(Modifier::DIM),
            "the colored divider rail is not dimmed"
        );
        let unstaged_x = rendered_line(&terminal, unstaged_y).find('M').expect("unstaged letter") as u16;
        assert_eq!(terminal.backend().buffer()[(staged_x, staged_y)].fg, Color::Green);
        assert_eq!(
            terminal.backend().buffer()[(unstaged_x, unstaged_y)].fg,
            Color::LightRed
        );

        let staged_only = Changes {
            paths: vec![changes.paths[0].clone()],
            ..Changes::default()
        };
        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                Some(&staged_only),
            );
        })?;
        assert!(
            !(0..12).any(|y| rendered_line(&terminal, y).contains("index")),
            "a single change group has no divider"
        );
        assert_eq!(index_divider(5).to_string(), "↑ ind", "narrow panes clip the label");

        let modified = Changes {
            paths: (0..12)
                .map(|index| crate::app::PathChange {
                    kind: ChangeKind::Modified,
                    group: ChangeGroup::Tree,
                    source: None,
                    path: format!("file-{index}").into(),
                    lines: Some((0, 0)),
                })
                .collect(),
            ..Changes::default()
        };
        let summary = changes_summary(ChangePane::Tree, &app, &modified)
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert!(summary.contains("M 12"));
        assert!(
            !summary.contains("+0") && !summary.contains("-0"),
            "empty diff counts are hidden"
        );
        assert!(!summary.contains("= 12"), "a single term already expresses the total");

        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                Some(&Changes::default()),
            );
        })?;
        let (clean_y, clean_header) = (0..12)
            .map(|y| (y, rendered_line(&terminal, y)))
            .find(|(_, line)| line.contains("Worktree clean"))
            .expect("an enabled clean worktree remains visible as an empty block");
        let clean_x = clean_header.find("clean").expect("clean label") as u16;
        assert_eq!(terminal.backend().buffer()[(clean_x, clean_y)].fg, Color::Green);
        assert!(
            !(0..12).any(|y| rendered_line(&terminal, y).contains("+0") || rendered_line(&terminal, y).contains("-0")),
            "a clean worktree omits empty diff counts"
        );
        assert!(
            !(0..12).any(|y| rendered_line(&terminal, y).contains("= 0")),
            "a clean worktree has no empty aggregate"
        );
        assert!(!app.worktree_changes_visible, "an empty block is not focusable");

        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&Changes::default()),
                Some(&Changes::default()),
            );
        })?;
        let (empty_y, empty_tree) = (0..12)
            .map(|y| (y, rendered_line(&terminal, y)))
            .find(|(_, line)| line.contains("Tree ------- empty"))
            .expect("an empty tree remains visible and says it is empty");
        let empty_x = empty_tree.find("empty").expect("empty tree label") as u16;
        assert_eq!(terminal.backend().buffer()[(empty_x, empty_y)].fg, Color::Green);
        assert!(!app.tree_changes_visible, "an empty tree block is not focusable");
        Ok(())
    }

    #[test]
    fn worktree_index_divider_scrolls_with_paths() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(1);
        app.changes_mode = Some(ChangesMode::Both);
        app.changes_focus = Some(ChangePane::Worktree);
        let changes = Changes {
            paths: [
                ChangeGroup::Staged,
                ChangeGroup::Staged,
                ChangeGroup::Unstaged,
                ChangeGroup::Unstaged,
            ]
            .into_iter()
            .enumerate()
            .map(|(index, group)| crate::app::PathChange {
                kind: ChangeKind::Modified,
                group,
                source: None,
                path: format!("file-{index}").into(),
                lines: None,
            })
            .collect(),
            ..Changes::default()
        };
        let mut terminal = Terminal::new(TestBackend::new(80, 10))?;
        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                Some(&changes),
            );
        })?;

        app.update(Action::MoveDown);
        app.update(Action::MoveDown);
        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                Some(&changes),
            );
        })?;

        let divider_y = (0..10)
            .find(|y| rendered_line(&terminal, *y).contains("↑ index ↑"))
            .expect("the boundary scrolls into view");
        assert!(rendered_line(&terminal, divider_y + 1).contains("M file-2"));
        assert!(
            rendered_line(&terminal, divider_y + 2).contains("… 1 line not shown"),
            "the overflow count excludes the divider"
        );
        assert_eq!(
            app.update(Action::OpenDiff),
            vec![crate::app::Effect::OpenDiff(ChangePane::Worktree, 2)],
            "the selected row after the divider retains its path index"
        );
        Ok(())
    }

    #[test]
    fn lays_out_tree_and_worktree_changes_by_available_width() -> Result<(), Box<dyn std::error::Error>> {
        let (layout, panes, height) = changes_pane_areas(Rect::new(0, 0, 120, 20), 10, Some((5, 30)), Some((3, 25)));
        assert_eq!(layout, ChangesLayout::SideBySide);
        assert_eq!(height, 5);
        assert_eq!(panes[0].outer, Rect::new(0, 15, 60, 5));
        assert_eq!(panes[1].outer, Rect::new(60, 15, 60, 5));

        let (layout, panes, height) = changes_pane_areas(Rect::new(0, 0, 60, 20), 10, Some((8, 31)), Some((3, 31)));
        assert_eq!(layout, ChangesLayout::Stacked);
        assert_eq!(height, 10);
        assert_eq!(
            panes[0],
            ChangesPaneArea {
                pane: ChangePane::Worktree,
                outer: Rect::new(0, 10, 60, 3),
            }
        );
        assert_eq!(
            panes[1],
            ChangesPaneArea {
                pane: ChangePane::Tree,
                outer: Rect::new(0, 13, 60, 7),
            }
        );

        let (_, panes, height) = changes_pane_areas(Rect::new(0, 0, 120, 20), 10, None, Some((3, 25)));
        assert_eq!(height, 3);
        assert_eq!(panes[0].outer, Rect::new(0, 17, 120, 3));

        let mut app = App::new(1);
        app.changes_mode = Some(ChangesMode::Both);
        let path = |group, path: &'static str| Changes {
            paths: vec![crate::app::PathChange {
                kind: ChangeKind::Modified,
                group,
                source: None,
                path: path.into(),
                lines: Some((1, 1)),
            }],
            lines_added: 1,
            lines_removed: 1,
            ..Changes::default()
        };
        let mut tree = path(ChangeGroup::Tree, "tree-file");
        tree.paths.push(crate::app::PathChange {
            kind: ChangeKind::Added,
            group: ChangeGroup::Tree,
            source: None,
            path: "tree-file-2".into(),
            lines: Some((0, 0)),
        });
        let worktree = path(ChangeGroup::Staged, "worktree-file");
        let mut terminal = Terminal::new(TestBackend::new(120, 10))?;
        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&tree),
                Some(&worktree),
            );
        })?;
        let halves = |row: String| {
            let left = row.chars().take(60).collect::<String>();
            let right = row.chars().skip(60).collect::<String>();
            (left, right)
        };
        let (left, right) = halves(rendered_line(&terminal, 5));
        assert!(left.contains("Tree"));
        assert!(right.contains("Worktree"));
        let (left, right) = halves(rendered_line(&terminal, 6));
        assert!(left.contains("tree-file"));
        assert!(right.contains("worktree-file"));
        let (left, right) = halves(rendered_line(&terminal, 7));
        assert!(left.contains("tree-file-2"));
        assert_eq!(right.trim(), "│");
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(60, 5)].symbol(), "┬");
        assert_eq!(buffer[(60, 6)].symbol(), "│");
        assert_eq!(buffer[(60, 7)].symbol(), "│");
        assert_eq!(buffer[(60, 8)].symbol(), "│");

        app.leave_success(
            "success success success success success success success success success success success success",
        );
        terminal.draw(|frame| {
            let area = frame.area();
            super::draw_with_worktree(
                frame,
                area,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                Some(&tree),
                Some(&worktree),
            );
        })?;
        assert_eq!(terminal.backend().buffer()[(62, 4)].bg, Color::Green);
        assert_eq!(
            terminal.backend().buffer()[(60, 4)].bg,
            Color::Reset,
            "the notice has horizontal margin within the worktree pane"
        );
        assert_eq!(
            terminal.backend().buffer()[(60, 9)].bg,
            Color::Reset,
            "the footer is never covered"
        );
        Ok(())
    }

    #[test]
    fn aligns_commit_trailers_and_wraps_only_in_the_value_column() -> Result<(), Box<dyn std::error::Error>> {
        let mut terminal = Terminal::new(TestBackend::new(40, 8))?;
        let message = b"subject\n\nbody\n\nShort: one two three four five six seven\nCo-authored-by: Alice".as_bstr();

        terminal.draw(|frame| {
            render_commit_message(frame, frame.area(), message, None, &[], 0);
        })?;

        assert_eq!(
            rendered_line(&terminal, 4).find("one"),
            Some(16),
            "values start after the widest trailer key"
        );
        assert_eq!(
            rendered_line(&terminal, 5).find("six"),
            Some(16),
            "wrapped values remain in the value column"
        );
        assert_eq!(
            rendered_line(&terminal, 6).find("Alice"),
            Some(16),
            "all trailer values share the same column"
        );
        assert!(
            rendered_line(&terminal, 5)[..16].trim().is_empty(),
            "wrapped values never occupy key space"
        );
        let key_x = rendered_line(&terminal, 4)
            .find("Short:")
            .expect("the trailer key is visible") as u16;
        let key = &terminal.backend().buffer()[(key_x, 4)];
        assert_eq!(key.fg, Color::Green, "trailer keys use the listing color");
        assert!(
            !key.modifier.contains(Modifier::DIM),
            "trailer keys remain fully visible"
        );
        assert!(
            terminal.backend().buffer()[(0, 0)].modifier.contains(Modifier::BOLD),
            "the commit title is bold"
        );
        assert!(
            !terminal.backend().buffer()[(0, 2)].modifier.contains(Modifier::BOLD),
            "the commit body is not bold"
        );

        let mut plain_terminal = Terminal::new(TestBackend::new(40, 4))?;
        plain_terminal.draw(|frame| {
            render_commit_message(
                frame,
                frame.area(),
                b"plain subject\n\nplain body".as_bstr(),
                None,
                &[],
                0,
            );
        })?;
        assert!(
            plain_terminal.backend().buffer()[(0, 0)]
                .modifier
                .contains(Modifier::BOLD),
            "titles remain bold without trailers"
        );
        assert!(
            !plain_terminal.backend().buffer()[(0, 2)]
                .modifier
                .contains(Modifier::BOLD),
            "plain commit bodies remain unstyled"
        );

        let mut terminal = Terminal::new(TestBackend::new(60, 8))?;
        let message = b"subject\n\nnot a trailer\nSigned-off-by: Alice\nanother note\nSigned-off-by: Bob".as_bstr();
        terminal.draw(|frame| {
            render_commit_message(frame, frame.area(), message, None, &[], 0);
        })?;
        assert!(
            rendered_line(&terminal, 2).contains("not a trailer another note"),
            "Markdown soft breaks combine message lines ahead of the trailers"
        );
        assert!(
            rendered_line(&terminal, 3).trim().is_empty(),
            "the combined message remains separated from its trailers"
        );
        assert_eq!(
            rendered_line(&terminal, 4).find("Alice"),
            Some(15),
            "the first trailer moves below all message parts"
        );
        assert_eq!(
            rendered_line(&terminal, 5).find("Bob"),
            Some(15),
            "later trailer runs share the aligned value column"
        );
        Ok(())
    }

    #[test]
    fn note_precedes_the_commit_message_and_its_trailers() -> Result<(), Box<dyn std::error::Error>> {
        let mut terminal = Terminal::new(TestBackend::new(40, 10))?;
        terminal.draw(|frame| {
            render_commit_message(
                frame,
                frame.area(),
                b"commit *subject*\n\ncommit ~~body~~\n\nSigned-off-by: Alice".as_bstr(),
                Some(b"todo *subject*\n\ntodo **body**".as_bstr()),
                &[],
                0,
            );
        })?;

        assert_eq!(rendered_line(&terminal, 0).trim(), "todo subject");
        assert_eq!(rendered_line(&terminal, 2).trim(), "todo body");
        assert_eq!(terminal.backend().buffer()[(0, 0)].bg, Color::Reset);
        assert_eq!(terminal.backend().buffer()[(0, 2)].bg, Color::Reset);
        assert!(terminal.backend().buffer()[(5, 0)].modifier.contains(Modifier::ITALIC));
        assert!(terminal.backend().buffer()[(5, 2)].modifier.contains(Modifier::BOLD));
        assert!(
            rendered_line(&terminal, 3).chars().all(|char| char == '─'),
            "a horizontal rule separates note from the commit"
        );
        assert_eq!(rendered_line(&terminal, 4).trim(), "commit subject");
        assert_eq!(rendered_line(&terminal, 6).trim(), "commit body");
        assert!(terminal.backend().buffer()[(7, 4)].modifier.contains(Modifier::ITALIC));
        assert!(
            terminal.backend().buffer()[(7, 6)]
                .modifier
                .contains(Modifier::CROSSED_OUT)
        );
        assert!(
            rendered_line(&terminal, 8).contains("Alice"),
            "commit trailers retain their layout"
        );
        Ok(())
    }

    #[test]
    fn markdown_hides_structural_delimiters() {
        let text = markdown_text(b"# Heading\n\n```rust\nlet value = 1;\n```".as_bstr());
        let rendered = text.to_string();
        assert!(rendered.contains("Heading") && rendered.contains("let value = 1;"));
        assert!(!rendered.contains('#') && !rendered.contains("```"));
        assert!(text.lines[0].style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn shortens_only_conventional_commit_prefixes() {
        for (input, expected) in [
            ("feat: subject", "…:subject"),
            ("feat(gix-tix)!: subject", "…:subject"),
            ("change(cli-tools): subject", "…:subject"),
            ("feat: **🧪 subject**", "…:🧪 subject"),
            ("feat: # heading", "…:# heading"),
            ("feat: ---", "…:---"),
            ("Title: subject", "Title: subject"),
            ("feat(scope: subject", "feat(scope: subject"),
            ("feat:subject", "feat:subject"),
        ] {
            assert_eq!(
                commit_title_spans(input.as_bytes().as_bstr(), true)
                    .into_iter()
                    .map(|span| span.content.into_owned())
                    .collect::<String>(),
                expected,
                "prefix classification for {input:?}"
            );
        }
    }

    #[test]
    fn adapts_history_detail_below_sixty_percent_of_the_average_title() {
        assert!(!less_than_sixty_percent(6, [10]));
        assert!(less_than_sixty_percent(5, [10]));
        assert!(!less_than_sixty_percent(6, [8, 12]));
        assert!(less_than_sixty_percent(5, [8, 12]));
        assert!(!less_than_sixty_percent(0, []));
        assert_eq!(
            Line::from(commit_title_spans("feat: 🧪".as_bytes().as_bstr(), true)).width(),
            Line::raw("…:🧪").width(),
            "title widths use terminal cells rather than bytes"
        );
        assert_eq!(lane_width("●       ", HistoryAlignment::Title), 2);
        assert_eq!(lane_width("●       ", HistoryAlignment::None), 8);
    }

    #[test]
    fn renders_note_markers_and_notes_before_trailers() -> Result<(), Box<dyn std::error::Error>> {
        let id = gix::ObjectId::Sha1([1; 20]);
        let mut app = App::new(1);
        app.extend_commits(vec![Commit {
            id,
            parent_ids: Default::default(),
            author_time: gix::date::Time::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: true,
            is_review: false,
            signature: SignatureState::Unsigned,
        }]);
        app.set_notes(id, vec!["review *note*".into()]);
        app.selected = None;
        let mut history = Terminal::new(TestBackend::new(100, 2))?;
        history.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        let row = rendered_row(&history);
        let agent_x = row.find("[A]").expect("the agent marker is visible") as u16;
        let note_x = row.find("[N]").expect("the note marker is visible") as u16;
        assert!(
            row.contains("[A] [N] subject"),
            "agent and note markers precede the title"
        );
        assert_eq!(history.backend().buffer()[(agent_x, 0)].fg, Color::LightMagenta);
        assert_eq!(history.backend().buffer()[(note_x, 0)].fg, Color::LightMagenta);

        let mut message = Terminal::new(TestBackend::new(40, 9))?;
        message.draw(|frame| {
            render_commit_message(
                frame,
                frame.area(),
                b"subject\n\nbody\n\nSigned-off-by: Alice".as_bstr(),
                None,
                &["review *note*".into()],
                0,
            );
        })?;
        assert_eq!(rendered_line(&message, 4).trim(), "Notes:");
        assert_eq!(rendered_line(&message, 5).trim(), "review note");
        assert!(message.backend().buffer()[(7, 5)].modifier.contains(Modifier::ITALIC));
        assert!(rendered_line(&message, 7).contains("Alice"), "trailers follow notes");
        let notes_label = &message.backend().buffer()[(0, 4)];
        assert_eq!(notes_label.fg, NOTE_COLOR);
        assert!(
            notes_label.modifier.contains(Modifier::BOLD),
            "only the Notes label is bold"
        );
        assert!(
            !message.backend().buffer()[(5, 4)].modifier.contains(Modifier::BOLD)
                && !message.backend().buffer()[(0, 5)].modifier.contains(Modifier::BOLD),
            "the colon and note body are not bold"
        );
        Ok(())
    }

    #[test]
    fn renders_only_the_visible_rows() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(2);
        app.id_mode = IdMode::Commit;
        app.extend_commits(
            (1..=3)
                .map(|n| Commit {
                    id: gix::ObjectId::Sha1([n; 20]),
                    parent_ids: Default::default(),
                    author_time: gix::date::Time::default(),
                    committer_time: gix::date::Time::default(),
                    author: author(b"author", b"author@example.com"),
                    attributions: 0..0,
                    title: format!("subject {n}").into(),
                    metadata_loaded: true,
                    has_agent_marker: false,
                    is_review: false,
                    signature: SignatureState::Unsigned,
                })
                .collect::<Vec<_>>(),
        );
        complete(&mut app);
        app.alignment = HistoryAlignment::None;
        app.update(Action::Last);
        let mut terminal = Terminal::new(TestBackend::new(24, 3))?;

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;

        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(11, 0)].symbol(), "2", "the viewport starts at the second row");
        assert_eq!(buffer[(11, 1)].symbol(), "3", "the selected third row remains visible");
        assert!(
            buffer[(6, 1)].modifier.contains(Modifier::REVERSED),
            "the slice-local selection highlights the global selection"
        );
        assert!(
            buffer[(23, 1)].modifier.contains(Modifier::REVERSED),
            "a clipped selection marker uses the right border"
        );
        assert_eq!(app.selected, Some(2), "drawing preserves the global selection");
        assert_eq!(app.offset, 1, "drawing preserves the global offset");
        Ok(())
    }

    #[test]
    fn renders_hidden_boundary_rows_without_colors() -> Result<(), Box<dyn std::error::Error>> {
        let commit = |n: u8| Commit {
            id: gix::ObjectId::Sha1([n; 20]),
            parent_ids: Default::default(),
            author_time: gix::date::Time::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: format!("subject {n}").into(),
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unverified,
        };
        let mut app = App::new(2);
        app.id_mode = IdMode::Commit;
        app.extend_commits(vec![commit(1)]);
        std::sync::Arc::make_mut(&mut app.rows[0]).parent_ids = [gix::ObjectId::Sha1([2; 20])].into_iter().collect();
        app.extend_hidden_commits(vec![commit(2)]);
        complete(&mut app);
        app.select_commit(gix::ObjectId::Sha1([2; 20]));
        app.set_hidden_branch_updates(std::collections::HashMap::from([(
            gix::ObjectId::Sha1([2; 20]),
            (2, gix::ObjectId::Sha1([3; 20])),
        )]));
        app.set_lane(0, "● ");
        app.set_lane(1, "● ");
        let changes = Changes {
            range: Some(crate::app::ComparedRange {
                base: gix::ObjectId::Sha1([2; 20]),
                tip: gix::ObjectId::Sha1([1; 20]),
            }),
            paths: vec![crate::app::PathChange {
                kind: ChangeKind::Modified,
                group: ChangeGroup::Tree,
                source: None,
                path: "changed".into(),
                lines: Some((3, 1)),
            }],
            lines_added: 3,
            lines_removed: 1,
            ..Changes::default()
        };
        let mut terminal = Terminal::new(TestBackend::new(120, 8))?;

        terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &Default::default(),
                None,
                Some(&changes),
            );
        })?;

        let line = rendered_line(&terminal, 1);
        assert!(
            line.contains("subject 2"),
            "the hidden commit keeps its normal content: {line:?}"
        );
        let visible = rendered_line(&terminal, 0);
        let visible_hash = visible.find("0101010").expect("the visible hash is present") as u16;
        assert_ne!(terminal.backend().buffer()[(visible_hash, 0)].fg, Color::Reset);
        let hash = line.find("0202020").expect("the hidden hash is visible") as u16;
        assert!(
            terminal.backend().buffer()[(hash, 1)].modifier.contains(Modifier::BOLD),
            "non-color styling is retained"
        );
        let marker_byte = line.find("⇣2").expect("the behind count is visible");
        let marker = line[..marker_byte].chars().count() as u16;
        let added_byte = line.find("+3").expect("the branch additions are visible");
        let added = line[..added_byte].chars().count() as u16;
        let removed_byte = line.find("-1").expect("the branch removals are visible");
        let removed = line[..removed_byte].chars().count() as u16;
        assert_eq!(&line[marker_byte - 1..marker_byte], " ", "the marker has a left margin");
        assert_eq!(
            terminal.backend().buffer()[(marker + 2, 1)].symbol(),
            " ",
            "the marker has a right margin"
        );
        assert!(
            line.find("subject 2") < line.find("+3")
                && line.find("+3") < line.find("-1")
                && line.find("-1") < line.find("⇣2"),
            "the branch diff-stat and behind marker follow the title: {line:?}"
        );
        for x in 0..terminal.backend().buffer().area.width {
            let cell = &terminal.backend().buffer()[(x, 1)];
            if (marker..marker + 2).contains(&x) {
                assert_eq!(cell.fg, Color::LightRed, "behind information uses its usual color");
                assert!(!cell.modifier.contains(Modifier::DIM));
                continue;
            }
            if (added..added + 2).contains(&x) || (removed..removed + 2).contains(&x) {
                assert!(
                    !cell.modifier.contains(Modifier::DIM),
                    "branch diff-stat remains prominent"
                );
                continue;
            }
            assert_eq!(cell.fg, Color::Reset, "the hidden row has no foreground colors");
            assert_eq!(cell.bg, Color::Reset, "the hidden row has no background colors");
            assert!(cell.modifier.contains(Modifier::DIM), "the hidden row is dimmed");
        }
        assert_eq!(
            terminal.backend().buffer()[(6, 1)].symbol(),
            ">",
            "the hidden base is selectable"
        );

        app.alignment = HistoryAlignment::None;
        let mut narrow = Terminal::new(TestBackend::new(28, 3))?;
        narrow.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(
            rendered_line(&narrow, 1).ends_with(" ⇣2 "),
            "the terminal edge pushes the marker over clipped title text"
        );
        app.actions_expanded = true;
        let mut commit = Terminal::new(TestBackend::new(120, 3))?;
        commit.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(rendered_line(&commit, 0).contains(" pin "));
        assert!(
            rendered_line(&commit, 1).contains(" rebase · rebase-update ")
                && rendered_line(&commit, 2).contains("actions"),
            "the selected hidden base offers rebasing from actions: {:?}",
            rendered_line(&commit, 1)
        );
        Ok(())
    }

    #[test]
    fn uses_the_tig_palette_without_coloring_the_selection() -> Result<(), Box<dyn std::error::Error>> {
        let id = gix::ObjectId::Sha1([1; 20]);
        let commit = Commit {
            id,
            parent_ids: Default::default(),
            author_time: gix::date::Time::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unsigned,
        };
        let decorations = Decorations::from([(
            id,
            vec![
                Decoration {
                    name: "HEAD".into(),
                    kind: DecorationKind::Head,
                },
                Decoration {
                    name: "main".into(),
                    kind: DecorationKind::Local,
                },
                Decoration {
                    name: "origin/main".into(),
                    kind: DecorationKind::Remote,
                },
                Decoration {
                    name: "tag: v1".into(),
                    kind: DecorationKind::AnnotatedTag,
                },
                Decoration {
                    name: "refs/stash".into(),
                    kind: DecorationKind::Special,
                },
            ],
        )]);
        let mut app = App::new(1);
        app.extend_commits(vec![commit]);
        app.set_lane(0, "● │ │ │ │ │ │ │ ");
        let row = &app.rows[0];
        let mailmap = gix::mailmap::Snapshot::default();
        let line = metadata_line(
            row,
            app.title(row),
            app.attributions(row),
            &decorations,
            &mailmap,
            MetadataOptions {
                date_mode: DateMode::Committer,
                id_mode: IdMode::Commit,
                change_id: row.id.into(),
                show_author_name: true,
                show_emails: false,
                show_trailers: true,
                has_notes: false,
                note_title: None,
                shorten_title: false,
                use_mailmap: false,
                ref_mode: RefMode::All,
                selected: false,
                copy_feedback: None,
            },
        );
        let style = |text| {
            line.spans
                .iter()
                .find(|span| span.content == text)
                .expect("the styled field is present")
                .style
        };
        assert_eq!(
            style("0101010"),
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
        );
        assert_eq!(style("1970-01-01 "), Style::default().fg(Color::Blue));
        assert_eq!(style("author "), Style::default().fg(Color::Green));
        assert!(
            line.spans.iter().all(|span| span.content != "HEAD"),
            "the graph marker makes textual HEAD redundant"
        );
        assert_eq!(style("main"), Style::default().fg(Color::Cyan));
        assert_eq!(style("origin/main"), Style::default().fg(Color::Yellow));
        assert_eq!(
            style("tag: v1"),
            Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
        );
        assert_eq!(style("refs/stash"), Style::default().fg(Color::Blue));

        app.selected = None;
        let mut terminal = Terminal::new(TestBackend::new(80, 2))?;
        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(8, 0)].fg, Color::Blue, "commit dots use graph-commit");
        assert_eq!(buffer[(10, 0)].fg, Color::Yellow, "lanes cycle through tig's palette");
        assert_eq!(
            buffer[(22, 0)].fg,
            Color::Magenta,
            "the palette repeats after seven lanes"
        );
        assert!(
            buffer[(22, 0)].modifier.contains(Modifier::BOLD),
            "the second palette cycle is bold"
        );
        Ok(())
    }

    #[test]
    fn hides_ids_by_default_and_marks_duplicate_change_ids() -> Result<(), Box<dyn std::error::Error>> {
        let id = gix::ObjectId::Sha1([1; 20]);
        let duplicate_id = gix::ObjectId::Sha1([2; 20]);
        let mut app = App::new(1);
        let commit = Commit {
            id,
            parent_ids: Default::default(),
            author_time: gix::date::Time::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unsigned,
        };
        app.extend_commits(vec![
            commit.clone(),
            Commit {
                id: duplicate_id,
                title: "duplicate".into(),
                ..commit
            },
        ]);
        let row = &app.rows[0];
        let plain = plain_history_metadata(
            &app,
            row,
            &Decorations::new(),
            &gix::mailmap::Snapshot::default(),
            false,
            None,
        );
        assert!(!plain.contains("0101010"), "history rows hide IDs by default");

        let change_id = gix::hash::ChangeId::from(duplicate_id);
        app.set_change_ids(
            std::collections::HashMap::from([(id, change_id)]),
            std::collections::HashSet::from([id, duplicate_id]),
        );
        let row = &app.rows[0];
        let decorations = Decorations::new();
        let mailmap = gix::mailmap::Snapshot::default();
        let line = metadata_line(
            row,
            app.title(row),
            app.attributions(row),
            &decorations,
            &mailmap,
            MetadataOptions {
                date_mode: DateMode::None,
                id_mode: app.effective_id_mode(),
                change_id: app.change_id(id),
                show_author_name: false,
                show_emails: false,
                show_trailers: false,
                has_notes: false,
                note_title: None,
                shorten_title: false,
                use_mailmap: false,
                ref_mode: RefMode::None,
                selected: false,
                copy_feedback: None,
            },
        );
        assert_eq!(line.spans[0].content, change_id.to_reverse_hex_with_len(7).to_string());
        assert_eq!(
            line.spans[0].style,
            color(Color::LightCyan).add_modifier(Modifier::BOLD)
        );
        assert!(
            line.spans
                .iter()
                .all(|span| span.style != color(Color::Magenta).add_modifier(Modifier::BOLD)),
            "the commit ID is not mixed into an ambiguous change ID"
        );

        complete(&mut app);
        app.selected = Some(0);
        let mut terminal = Terminal::new(TestBackend::new(80, 2))?;
        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        assert!(
            rendered_line(&terminal, 0).trim_start().starts_with("👯‍♂️ > "),
            "the ambiguity gutter does not replace selection: {:?}",
            rendered_line(&terminal, 0)
        );
        assert!(
            rendered_line(&terminal, 1).contains("next duplicate"),
            "the footer advertises duplicate cycling for the selected commit"
        );
        Ok(())
    }

    #[test]
    fn aligns_titles_or_all_columns_from_only_visible_rows() -> Result<(), Box<dyn std::error::Error>> {
        fn column(line: &str, needle: &str) -> usize {
            line[..line.find(needle).expect("field is visible")].chars().count()
        }

        let ids = [1, 2, 3].map(|byte| gix::ObjectId::Sha1([byte; 20]));
        let mut app = App::new(2);
        app.extend_commits(LoadedCommits {
            rows: vec![
                Commit {
                    id: ids[0],
                    parent_ids: Default::default(),
                    author_time: gix::date::Time::default(),
                    committer_time: gix::date::Time::default(),
                    author: author(b"Byron", b"byron@example.com"),
                    attributions: 0..0,
                    title: "first-title".into(),
                    metadata_loaded: true,
                    has_agent_marker: false,
                    is_review: false,
                    signature: SignatureState::Unsigned,
                },
                Commit {
                    id: ids[1],
                    parent_ids: Default::default(),
                    author_time: gix::date::Time::default(),
                    committer_time: gix::date::Time::default(),
                    author: author(b"Byron", b"byron@example.com"),
                    attributions: 0..1,
                    title: "second-title".into(),
                    metadata_loaded: true,
                    has_agent_marker: true,
                    is_review: false,
                    signature: SignatureState::Unsigned,
                },
                Commit {
                    id: ids[2],
                    parent_ids: Default::default(),
                    author_time: gix::date::Time::default(),
                    committer_time: gix::date::Time::default(),
                    author: author(b"An extraordinarily long author", b"long@example.com"),
                    attributions: 1..1,
                    title: "third-title".into(),
                    metadata_loaded: true,
                    has_agent_marker: false,
                    is_review: false,
                    signature: SignatureState::Unsigned,
                },
            ],
            attributions: vec![Attribution {
                kind: AttributionKind::CoAuthor,
                author: author(b"GPT", b"gpt@example.com"),
            }],
        });
        complete(&mut app);
        app.selected = None;
        app.set_lane(0, "● ");
        app.set_lane(1, "●─┐ ");
        app.set_lane(2, "● ");
        let decorations = Decorations::from([(
            ids[0],
            vec![Decoration {
                name: "various-improvements".into(),
                kind: DecorationKind::Local,
            }],
        )]);
        let mut terminal = Terminal::new(TestBackend::new(120, 3))?;

        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        let first_title = column(&rendered_line(&terminal, 0), "first-title");
        let second_title = column(&rendered_line(&terminal, 1), "second-title");
        assert_eq!(
            first_title,
            second_title,
            "[ aligns only the visible titles: {:?} / {:?}",
            rendered_line(&terminal, 0),
            rendered_line(&terminal, 1)
        );
        assert_ne!(
            column(&rendered_line(&terminal, 0), "1970-01-01"),
            column(&rendered_line(&terminal, 1), "1970-01-01"),
            "title mode leaves earlier fields at natural positions"
        );
        app.set_lane(0, "●                                                  ");
        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        assert_eq!(
            column(&rendered_line(&terminal, 0), "first-title"),
            first_title,
            "trailing graph storage does not consume visible columns"
        );
        app.set_lane(0, "● ");

        let mut narrow_title = Terminal::new(TestBackend::new(40, 3))?;
        narrow_title.draw(|frame| draw(frame, &mut app, &decorations))?;
        assert!(
            rendered_line(&narrow_title, 0).contains("● first-title")
                && !rendered_line(&narrow_title, 0).contains("1970-01-01"),
            "narrow title alignment minimizes metadata"
        );

        app.update(Action::ToggleAlign);
        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        let first = rendered_line(&terminal, 0);
        let second = rendered_line(&terminal, 1);
        assert_eq!(
            column(&first, "1970-01-01"),
            column(&second, "1970-01-01"),
            "dates align"
        );
        assert_eq!(column(&first, "Byron"), column(&second, "Byron"), "authors align");
        assert_eq!(
            column(&first, "first-title"),
            column(&second, "second-title"),
            "titles align"
        );
        assert!(
            second.contains("Co: GPT [A] second-title"),
            "attribution and markers share one column"
        );

        let mut narrow = Terminal::new(TestBackend::new(46, 3))?;
        narrow.draw(|frame| draw(frame, &mut app, &decorations))?;
        assert!(
            rendered_line(&narrow, 0).contains("● first-title") && !rendered_line(&narrow, 0).contains("1970-01-01"),
            "narrow column alignment minimizes metadata"
        );

        let visible_title = column(&first, "first-title");
        app.offset = 1;
        terminal.draw(|frame| draw(frame, &mut app, &decorations))?;
        assert!(
            column(&rendered_line(&terminal, 0), "second-title") > visible_title,
            "an off-screen wide author affects alignment only after entering the viewport"
        );

        app.offset = 0;
        app.update(Action::ToggleAlign);
        assert_eq!(app.alignment, HistoryAlignment::None);
        narrow.draw(|frame| draw(frame, &mut app, &decorations))?;
        let before = rendered_line(&narrow, 0);
        assert!(before.contains("1970-01-01"), "unaligned rows retain metadata");
        app.update(Action::ScrollRight);
        assert!(app.horizontal_offset > 0, "unaligned rows expose their clipped width");
        narrow.draw(|frame| draw(frame, &mut app, &decorations))?;
        assert_ne!(rendered_line(&narrow, 0), before, "l pans the unaligned row");
        Ok(())
    }

    #[test]
    fn renders_selected_compressed_segments_as_expandable_without_stale_commit_ui()
    -> Result<(), Box<dyn std::error::Error>> {
        let ids = [1, 2, 3, 4].map(|byte| gix::ObjectId::Sha1([byte; 20]));
        let mut app = App::new(2);
        app.extend_commits(
            ids.iter()
                .enumerate()
                .map(|(index, id)| Commit {
                    id: *id,
                    parent_ids: ids.get(index + 1).copied().into_iter().collect(),
                    author_time: gix::date::Time::default(),
                    committer_time: gix::date::Time::default(),
                    author: author(b"Byron", b"byron@example.com"),
                    attributions: 0..0,
                    title: if index == 0 { "tip" } else { "hidden" }.into(),
                    metadata_loaded: true,
                    has_agent_marker: false,
                    is_review: false,
                    signature: SignatureState::Unsigned,
                })
                .collect::<Vec<_>>(),
        );
        complete(&mut app);
        app.set_view_tips(&ids[..1]);
        for _ in 0..3 {
            app.update(Action::ToggleAlign);
        }
        app.update(Action::MoveDown);
        app.show_commit = true;

        let mut terminal = Terminal::new(TestBackend::new(80, 3))?;
        let decorations = Decorations::new();
        let mailmap = gix::mailmap::Snapshot::default();
        let stale_tree_changes = Changes::default();
        terminal.draw(|frame| {
            let area = frame.area();
            draw_with_worktree(
                frame,
                area,
                &mut app,
                &decorations,
                &mailmap,
                Some(BStr::new(b"stale commit message")),
                Some(&stale_tree_changes),
                None,
            );
        })?;

        assert!(
            rendered_line(&terminal, 0).contains("tip"),
            "the retained tip keeps its metadata"
        );
        let line = rendered_line(&terminal, 1);
        let summary = "> ○ [2]";
        let summary_x = line.find(summary).expect("the compressed summary is visible") as u16;
        assert_eq!(line.trim(), summary);
        assert!(
            (summary_x..summary_x + summary.chars().count() as u16).all(|x| terminal.backend().buffer()[(x, 1)]
                .modifier
                .contains(Modifier::REVERSED)),
            "the selected summary, including its status and count, is reversed"
        );
        let footer = rendered_line(&terminal, 2);
        assert!(footer.contains("<enter> expand"));
        for hidden in [" · commit", " · actions", " · enrich", " · copy"] {
            assert!(!footer.contains(hidden), "synthetic selection hides {hidden:?}");
        }
        assert!(
            (0..3).all(|y| !rendered_line(&terminal, y).contains("stale commit message")),
            "stale commit-message and tree-change panes stay hidden"
        );
        Ok(())
    }

    fn rendered_row(terminal: &Terminal<TestBackend>) -> String {
        rendered_line(terminal, 0)
    }

    fn rendered_line(terminal: &Terminal<TestBackend>, y: u16) -> String {
        (0..terminal.backend().buffer().area.width).fold(String::new(), |mut out, x| {
            out.push_str(terminal.backend().buffer()[(x, y)].symbol());
            out
        })
    }

    fn assert_reversed_group(terminal: &Terminal<TestBackend>, y: u16, group: &str) {
        let line = rendered_line(terminal, y);
        let start = line[..line.rfind(group).expect("the active prefix group is visible")]
            .chars()
            .count() as u16;
        let end = start + group.chars().count() as u16;
        let buffer = terminal.backend().buffer();
        assert!(
            (start..end).all(|x| buffer[(x, y)].modifier.contains(Modifier::REVERSED)),
            "every cell in {group:?} is reversed"
        );
        assert!(
            !buffer[(start - 1, y)].modifier.contains(Modifier::REVERSED)
                && !buffer[(end, y)].modifier.contains(Modifier::REVERSED),
            "the active treatment is bounded to {group:?}"
        );
    }

    fn popup_is_dim(terminal: &Terminal<TestBackend>, label: &str) -> bool {
        let height = terminal.backend().buffer().area.height;
        let (y, popup) = (0..height.saturating_sub(1))
            .rev()
            .find_map(|y| {
                let line = rendered_line(terminal, y);
                line.contains(label).then_some((y, line))
            })
            .expect("toggle is visible in a popup");
        let x = popup[..popup.rfind(label).expect("toggle is visible")].chars().count() as u16;
        terminal.backend().buffer()[(x, y)].modifier.contains(Modifier::DIM)
    }
}
