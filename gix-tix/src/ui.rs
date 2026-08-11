use gix::bstr::{BStr, BString, ByteSlice};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::{
    BuiltInDiff,
    app::{
        App, AttributionKind, ChangeGroup, ChangeKind, ChangePane, Changes, ChangesLayout, ChangesMode, CommitRow,
        CopyKind, NameMode, RefMode, SelectionRelation, SignatureState, State,
    },
    history::{DecorationKind, Decorations},
};

const COMPARED_PARENT_COLOR: Color = Color::Cyan;
const COMMIT_PANE_WIDTH: u16 = 84;
const NOTE_COLOR: Color = Color::LightMagenta;
const PANE_STATUS_BACKGROUND: Color = Color::DarkGray;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ChangesPaneArea {
    pane: ChangePane,
    outer: Rect,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct FrameLayout {
    pub history: Rect,
    pub overlays: Vec<Rect>,
    pub rows: Vec<(gix::ObjectId, u16)>,
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
                            outer: Rect::new(
                                area.x,
                                area.bottom().saturating_sub(tree_height),
                                tree_width,
                                tree_height,
                            ),
                        },
                        ChangesPaneArea {
                            pane: ChangePane::Worktree,
                            outer: Rect::new(
                                area.x.saturating_add(tree_width),
                                area.bottom().saturating_sub(worktree_height),
                                worktree_width,
                                worktree_height,
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

pub(crate) fn draw_file_diff(frame: &mut Frame<'_>, diff: &BuiltInDiff, offset: usize, horizontal_offset: usize) {
    let [header, body, footer] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());
    frame.render_widget(Clear, frame.area());
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
        Paragraph::new("↑↓/jk move · h/l pan · Enter/q/Esc back").style(Style::default().add_modifier(Modifier::DIM)),
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
    draw_with_worktree(frame, app, decorations, mailmap, commit_message, tree_changes, None);
}

pub(crate) fn draw_with_worktree(
    frame: &mut Frame<'_>,
    app: &mut App,
    decorations: &Decorations,
    mailmap: &gix::mailmap::Snapshot,
    commit_message: Option<&BStr>,
    tree_changes: Option<&Changes>,
    worktree_changes: Option<&Changes>,
) -> FrameLayout {
    let [mut body, footer] = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());
    let full_body = body;
    let compared_parent = if app.changes_visible() {
        tree_changes.and_then(|changes| changes.parent.map(|parent| parent.id))
    } else {
        None
    };
    let tree_visible = app.changes_visible() && tree_changes.is_some_and(Changes::is_visible);
    let worktree_visible =
        app.changes_visible() && app.changes_mode == Some(ChangesMode::Both) && worktree_changes.is_some();
    let tree_summary = tree_changes.map(|changes| changes_summary(ChangePane::Tree, app, changes));
    let worktree_summary = worktree_changes.map(|changes| changes_summary(ChangePane::Worktree, app, changes));
    let commit_pane = app.show_commit.then(|| {
        let width = COMMIT_PANE_WIDTH.min(full_body.width / 2);
        let [commits, message] = Layout::horizontal([Constraint::Min(0), Constraint::Length(width)]).areas(full_body);
        body.width = body.width.min(commits.width);
        let mut content = message.inner(Margin {
            horizontal: 2,
            vertical: 0,
        });
        content.height = content.height.saturating_sub(1);
        (message, content)
    });
    let pane_height = |changes: &Changes| u16::try_from(changes.paths.len()).unwrap_or(u16::MAX).saturating_add(2);
    let (changes_layout, changes_panes, _) = changes_pane_areas(
        body,
        frame.area().height / 2,
        tree_visible.then(|| {
            (
                pane_height(tree_changes.expect("visible tree changes exist")),
                tree_summary.as_ref().map_or(0, Line::width),
            )
        }),
        worktree_visible.then(|| {
            (
                pane_height(worktree_changes.expect("visible worktree changes exist")),
                worktree_summary.as_ref().map_or(0, Line::width),
            )
        }),
    );
    if app.changes_visible() {
        app.set_changes_layout(
            changes_layout,
            changes_panes
                .iter()
                .any(|pane| pane.pane == ChangePane::Tree && pane.outer.height > 0),
            changes_panes
                .iter()
                .any(|pane| pane.pane == ChangePane::Worktree && pane.outer.height > 0)
                && worktree_changes.is_some_and(Changes::is_visible),
        );
    }
    app.viewport_rows = changes_panes
        .iter()
        .map(|pane| pane.outer.y.saturating_sub(body.y))
        .min()
        .unwrap_or(body.height)
        .max(1) as usize;
    app.ensure_visible();
    let start = app.offset.min(app.rows.len());
    let render_end = start.saturating_add(body.height as usize).min(app.rows.len());
    let visible_rows = &app.rows[start..render_end];
    let has_verifiable_signatures = visible_rows.iter().enumerate().any(|(index, row)| {
        !app.is_row_hidden(start + index)
            && matches!(row.signature, SignatureState::Unverified | SignatureState::Verifying)
    });
    let lanes = app.render_lanes(start..render_end);
    let content = Rect::new(
        body.x.saturating_add(2),
        body.y,
        body.width.saturating_sub(2),
        body.height,
    );
    let rendered_lane_width = lanes
        .iter()
        .filter(|lane| !lane.is_empty())
        .map(|lane| lane.trim_end().chars().count().saturating_add(1))
        .max()
        .unwrap_or_default();
    let max_lane_width = if rendered_lane_width == 0 {
        app.estimated_lane_width
    } else {
        rendered_lane_width
    };
    let align_limit = ((body.width as usize) / 3)
        .saturating_sub(2)
        .max(1)
        .min(content.width as usize);
    let align_width = max_lane_width.min(align_limit);
    let align_metadata = app.align_metadata;
    let show_committer_date = app.show_committer_date;
    let name_mode = app.name_mode;
    let preview_author_copy = app.preview_author_copy;
    let copy_feedback = app.copy_feedback.take();
    let show_author_name =
        preview_author_copy || copy_feedback == Some(CopyKind::Author) || name_mode != NameMode::None;
    let show_trailers = name_mode == NameMode::All && app.show_trailers;
    let ref_mode = app.ref_mode;
    let selected = app.selected;
    let metadata: Vec<_> = visible_rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            metadata_line(
                row,
                app.title(row),
                app.attributions(row),
                decorations,
                mailmap,
                MetadataOptions {
                    show_committer_date,
                    show_author_name,
                    show_emails: app.show_emails,
                    show_trailers,
                    has_notes: !app.notes(row.id).is_empty(),
                    use_mailmap: app.use_mailmap && !preview_author_copy && copy_feedback != Some(CopyKind::Author),
                    ref_mode,
                    selected: (selected == Some(start + index) && app.show_selection_tail)
                        || compared_parent == Some(row.id),
                    preview_author_copy,
                    copy_feedback: if selected == Some(start + index) {
                        copy_feedback
                    } else {
                        None
                    },
                },
            )
        })
        .collect();
    let graph_max_offset = max_lane_width.saturating_sub(align_width);
    let max_offset = if align_metadata {
        graph_max_offset
    } else {
        lanes
            .iter()
            .zip(&metadata)
            .map(|(lane, metadata)| lane.chars().count().saturating_add(metadata.width()))
            .max()
            .unwrap_or_default()
            .saturating_sub(content.width as usize)
    }
    .min(u16::MAX as usize);
    let horizontal_offset = app.horizontal_offset.min(max_offset);
    let graph_offset = horizontal_offset.min(graph_max_offset);
    let selection_info = selection_info_line(
        app.changes_visible()
            .then_some(tree_changes)
            .flatten()
            .filter(|changes| changes.is_visible()),
        app.selection_relation,
    );
    let selection_info_width = selection_info.width();
    let mut selection_info_area = None;

    let rows = visible_rows
        .iter()
        .enumerate()
        .map(|(index, row)| (row.id, body.y.saturating_add(index as u16)))
        .collect();
    for (index, metadata) in metadata.into_iter().enumerate() {
        let lane = lanes.lane(index);
        let y = body.y.saturating_add(index as u16);
        let selected = app.selected == Some(start + index);
        let metadata_width = metadata.width();
        let signature_color = signature_color(visible_rows[index].signature);
        let highlight = if selected && app.show_selection_tail {
            Some(signature_color)
        } else if compared_parent == Some(visible_rows[index].id) {
            Some(COMPARED_PARENT_COLOR)
        } else {
            None
        };
        let style = highlight.map_or_else(Style::default, |highlight| {
            color(highlight).add_modifier(Modifier::REVERSED)
        });
        frame.render_widget(
            Paragraph::new(if selected { "> " } else { "  " }).style(style),
            Rect::new(body.x, y, body.width.min(2), 1),
        );

        let row_area = Rect::new(content.x, y, content.width, 1);
        if align_metadata {
            frame.render_widget(
                Paragraph::new(lane).style(style).scroll((0, graph_offset as u16)),
                row_area,
            );
            color_graph(
                frame,
                row_area,
                lane,
                graph_offset,
                highlight,
                visible_rows[index].signature,
            );
            let aligned = Rect::new(
                content.x.saturating_add(align_width as u16),
                y,
                content.width.saturating_sub(align_width as u16),
                1,
            );
            frame.render_widget(Clear, aligned);
            frame.render_widget(Paragraph::new(metadata), aligned);
        } else {
            let mut spans = Vec::with_capacity(metadata.spans.len() + 1);
            spans.push(Span::styled(lane, style));
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
                visible_rows[index].signature,
            );
        }
        let lane_offset = if align_metadata {
            graph_offset
        } else {
            horizontal_offset
        };
        if let (Some(parent), Some(disk)) = (
            app.junction_parent(start + index),
            lane.chars().position(|symbol| symbol == '●'),
        ) && disk >= lane_offset
        {
            let number = parent.to_string();
            let x = disk + 1 - lane_offset;
            if x < row_area.width as usize {
                let width = number
                    .chars()
                    .count()
                    .min(lane.chars().count().saturating_sub(disk + 1))
                    .min(row_area.width as usize - x);
                if width > 0 {
                    frame.render_widget(
                        Paragraph::new(number).style(style),
                        Rect::new(row_area.x + x as u16, y, width as u16, 1),
                    );
                }
            }
        }
        if selected && app.show_selection_tail && body.width > 0 {
            let line_width = if align_metadata {
                align_width.saturating_add(metadata_width)
            } else {
                lane.chars()
                    .count()
                    .saturating_add(metadata_width)
                    .saturating_sub(horizontal_offset)
            };
            let marker_x = content
                .x
                .saturating_add(u16::try_from(line_width).unwrap_or(u16::MAX))
                .saturating_add(1)
                .saturating_add(u16::try_from(selection_info_width).unwrap_or(u16::MAX))
                .saturating_add(1)
                .min(body.right().saturating_sub(1));
            if selection_info_width > 0 {
                let width = u16::try_from(selection_info_width)
                    .unwrap_or(u16::MAX)
                    .min(marker_x.saturating_sub(content.x).saturating_sub(2));
                let area = Rect::new(marker_x.saturating_sub(width).saturating_sub(1), y, width, 1);
                if width > 0 {
                    frame.buffer_mut()[(area.x - 1, y)].set_symbol(" ");
                    frame.render_widget(Paragraph::new(selection_info.clone()), area);
                    selection_info_area = Some(area);
                }
            }
            let buffer = frame.buffer_mut();
            if marker_x > body.x {
                buffer[(marker_x - 1, y)].set_symbol(" ");
            }
            buffer[(marker_x, y)].set_symbol(" ").set_style(style);
        }
        if !app.is_row_reachable(start + index) {
            for x in body.x..body.right() {
                frame.buffer_mut()[(x, y)].set_style(Style::default().add_modifier(Modifier::DIM));
            }
        }
        if app.is_row_hidden(start + index) {
            for x in body.x..body.right() {
                frame.buffer_mut()[(x, y)]
                    .set_fg(Color::Reset)
                    .set_bg(Color::Reset)
                    .set_style(Style::default().add_modifier(Modifier::DIM));
            }
        }
    }
    app.set_horizontal_bounds(content.width as usize, max_offset);
    if app.changes_focus.is_some() {
        frame
            .buffer_mut()
            .set_style(body, Style::default().add_modifier(Modifier::DIM));
        if let Some(area) = selection_info_area {
            frame.render_widget(Paragraph::new(selection_info), area);
        }
    }
    for pane_area in &changes_panes {
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
        frame.render_widget(Clear, outer);
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
                    Span::raw(" · p next parent · "),
                ]);
            }
            if let Some(error) = &app.changes(pane).error {
                spans.push(Span::styled(format!("diff: {error}"), color(Color::LightRed)));
            } else {
                spans.push(Span::raw("↑↓/jk move · h/l pan · Enter diff"));
            }
            spans.push(Span::raw(" · y copy"));
            spans.push(Span::raw(match app.changes_mode {
                Some(ChangesMode::Both) => " · c tree",
                Some(ChangesMode::Tree) => " · c to hide",
                None => "",
            }));
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
        frame.render_widget(Block::new().borders(Borders::LEFT), outer);
        let max_offset = if let Some(message) = commit_message {
            let notes = app
                .selected
                .and_then(|index| app.rows.get(index))
                .map(|row| app.notes(row.id))
                .unwrap_or_default();
            render_commit_message(frame, area, message, notes, app.commit_offset)
        } else {
            0
        };
        app.set_commit_bounds(area.height as usize, max_offset);
        if max_offset > 0 {
            frame.render_widget(
                Paragraph::new("PgUp/C-b up page · PgDn/C-f down page · o to hide")
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
    let mut footer_spans = vec![Span::raw(format!(
        "{}{status} · ↑↓/jk move · h/l pan",
        history_position(app)
    ))];
    if app.changes_focus.is_none() {
        footer_spans.push(Span::raw(" · Enter diff"));
    }
    if app.tree_changes_visible || app.worktree_changes_visible {
        footer_spans.push(match app.focus_feedback.take() {
            Some(destination) => Span::raw(format!(" · Tab → {destination}")),
            None => Span::raw(" · Tab switch"),
        });
    }
    if app.changes_focus.is_some() {
        footer_spans.push(Span::raw(" · q/Esc history"));
    }
    footer_spans.extend([Span::raw(" · "), toggle("[ align", app.align_metadata)]);
    footer_spans.extend([Span::raw(" · "), toggle("o commit", app.show_commit)]);
    footer_spans.extend([Span::raw(" · "), toggle("c changes", app.changes_mode.is_some())]);
    if app.history_display_expanded {
        footer_spans.extend([Span::raw(" · "), toggle("d date", app.show_committer_date)]);
        footer_spans.extend([Span::raw(" · "), toggle("e emails", app.show_emails)]);
        let (name_label, names_visible) = match app.name_mode {
            NameMode::All => ("n names", true),
            NameMode::Author => ("n name", true),
            NameMode::None => ("n name", false),
        };
        footer_spans.extend([Span::raw(" · "), toggle(name_label, names_visible)]);
        for (label, enabled) in [("m mailmap", app.use_mailmap), ("t trailers", app.show_trailers)] {
            footer_spans.extend([Span::raw(" · "), toggle(label, enabled)]);
        }
        let ref_label = match app.ref_mode {
            RefMode::All => "r all refs",
            RefMode::Default => "r refs",
            RefMode::None => "r no refs",
        };
        footer_spans.extend([Span::raw(" · "), toggle(ref_label, app.ref_mode != RefMode::None)]);
        if app.has_hidden_filter {
            footer_spans.extend([
                Span::raw(" · "),
                toggle(
                    if app.show_hidden {
                        "h hide hidden"
                    } else {
                        "h show hidden"
                    },
                    app.show_hidden,
                ),
            ]);
        }
    } else {
        footer_spans.push(Span::raw(" · v view"));
    }
    if app.preview_author_copy && app.manual_refresh {
        footer_spans.extend([
            Span::raw(" · "),
            toggle("R refresh", matches!(history_state, State::Complete | State::Cancelled)),
        ]);
    }
    footer_spans.push(Span::raw(if app.preview_author_copy {
        " · Y copy author"
    } else {
        " · y copy"
    }));
    if app.signature_failures > 0 {
        footer_spans.extend([
            Span::raw(format!(" · s {} ", app.signature_failures)),
            Span::styled("●", color(Color::LightRed)),
        ]);
    } else if has_verifiable_signatures {
        footer_spans.extend([
            Span::raw(" · s "),
            Span::styled("●", color(Color::Rgb(255, 165, 0))),
            Span::raw(" -> "),
            Span::styled("●", color(Color::Green)),
        ]);
    }
    if app.changes_focus.is_none() {
        if history_state == State::Loading {
            footer_spans.push(Span::raw(" · Esc cancel"));
        }
        footer_spans.push(Span::raw(" · q quit"));
    }
    if let Some(notice) = &app.notice {
        footer_spans = vec![Span::raw(notice)];
    }
    frame.render_widget(Paragraph::new(Line::from(footer_spans)), footer);
    FrameLayout {
        history: body,
        overlays: changes_panes
            .iter()
            .map(|pane| pane.outer)
            .chain(commit_pane.map(|(outer, _)| outer))
            .collect(),
        rows,
    }
}

fn history_position(app: &App) -> String {
    match (app.deferred_history_state.unwrap_or(app.state), app.selected) {
        (State::Complete, Some(selected)) => format!("#{}", app.rows.len().saturating_sub(selected)),
        _ => format!("{} commits", app.rows.len()),
    }
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
    Style::default().fg(color).remove_modifier(Modifier::DIM)
}

fn push_selection_span(spans: &mut Vec<Span<'static>>, span: Span<'static>) {
    if !spans.is_empty() {
        spans.push(Span::raw(" "));
    }
    spans.push(span);
}

fn render_changes(frame: &mut Frame<'_>, area: Rect, changes: &Changes, pane: ChangePane, app: &mut App) {
    if area.height == 0 {
        app.set_changes_bounds(pane, 0, 0, area.width as usize, 0);
        return;
    }
    let focused = app.changes_focus == Some(pane);
    let selected_index = app.changes(pane).selected.min(changes.paths.len().saturating_sub(1));
    let path_capacity = usize::from(area.height);
    let overflow = changes.paths.len() > 1 && changes.paths.len() > path_capacity;
    let visible_paths = if overflow {
        path_capacity.saturating_sub(1)
    } else {
        path_capacity.min(changes.paths.len())
    };
    let lines: Vec<_> = changes
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
    let horizontal_max = lines
        .iter()
        .map(Line::width)
        .max()
        .unwrap_or_default()
        .saturating_sub(area.width as usize);
    app.set_changes_bounds(
        pane,
        visible_paths,
        changes.paths.len(),
        area.width as usize,
        horizontal_max,
    );
    let offset = app.changes(pane).offset;
    let horizontal_offset = app.changes(pane).horizontal_offset;
    let path_area = Rect::new(
        area.x,
        area.y,
        area.width,
        u16::try_from(visible_paths).unwrap_or(u16::MAX),
    );
    frame.render_widget(
        Paragraph::new(Text::from(
            lines.into_iter().skip(offset).take(visible_paths).collect::<Vec<_>>(),
        ))
        .scroll((0, u16::try_from(horizontal_offset).unwrap_or(u16::MAX))),
        path_area,
    );
    let hidden = changes.paths.len().saturating_sub(offset.saturating_add(visible_paths));
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
    let graph_width = max_changes.min(40);
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
                        (u64::from(count) * graph_width / max_changes.max(1)).max(u64::from(count > 0)) as usize
                    };
                    spans.push(Span::styled("+".repeat(scaled(*added)), color(Color::Green)));
                    spans.push(Span::styled("-".repeat(scaled(*removed)), color(Color::LightRed)));
                }
                None => spans.push(Span::raw(format!("{:>count_width$}", "Bin"))),
            }
            Line::from(spans)
        })
        .collect::<Vec<_>>();
    let mut spans = match changes.parent {
        Some(parent) if parent.total > 1 => vec![Span::styled(
            format!(
                "vs parent {}/{} {} · ",
                parent.index + 1,
                parent.total,
                parent.id.to_hex_with_len(7)
            ),
            color(COMPARED_PARENT_COLOR),
        )],
        Some(parent) => vec![Span::styled(
            format!("vs parent {} · ", parent.id.to_hex_with_len(7)),
            color(COMPARED_PARENT_COLOR),
        )],
        None => vec![Span::styled("root · ", color(COMPARED_PARENT_COLOR))],
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
            let id = app
                .selected
                .and_then(|index| app.rows.get(index))
                .map_or_else(|| "-------".into(), |row| row.id.to_hex_with_len(7).to_string());
            vec![Span::raw(format!("─ Tree {id} ── "))]
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

fn render_commit_message(frame: &mut Frame<'_>, area: Rect, message: &BStr, notes: &[BString], offset: usize) -> usize {
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
            Paragraph::new(commit_text(parsed.title, parsed.body, notes)).wrap(Wrap { trim: false }),
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
                Paragraph::new(commit_text(parsed.title, parsed.body, notes)).wrap(Wrap { trim: false }),
                offset,
            );
        }
        let mut text = commit_text(parsed.title, body_message, notes);
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

    let text = commit_text(parsed.title, body_message, notes);
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

fn commit_text<'a>(title: &'a BStr, body: Option<&'a BStr>, notes: &'a [BString]) -> Text<'a> {
    let mut text = Text::raw(title.to_str_lossy());
    for line in &mut text.lines {
        line.style = Style::default().add_modifier(Modifier::BOLD);
    }
    if let Some(body) = body.filter(|body| !body.is_empty()) {
        text.lines.push(Line::default());
        text.lines.extend(Text::raw(body.to_str_lossy()).lines);
    }
    for note in notes {
        text.lines.push(Line::default());
        text.lines.push(Line::from(vec![
            Span::styled("Notes", color(NOTE_COLOR).add_modifier(Modifier::BOLD)),
            Span::styled(":", color(NOTE_COLOR)),
        ]));
        let mut note = Text::raw(note.to_str_lossy());
        for line in &mut note.lines {
            line.style = color(NOTE_COLOR);
        }
        text.lines.extend(note.lines);
    }
    text
}

fn toggle(label: &'static str, enabled: bool) -> Span<'static> {
    Span::styled(
        label,
        if enabled {
            Style::default()
        } else {
            Style::default().add_modifier(Modifier::DIM)
        },
    )
}

#[derive(Clone, Copy)]
struct MetadataOptions {
    show_committer_date: bool,
    show_author_name: bool,
    show_emails: bool,
    show_trailers: bool,
    has_notes: bool,
    use_mailmap: bool,
    ref_mode: RefMode,
    selected: bool,
    preview_author_copy: bool,
    copy_feedback: Option<CopyKind>,
}

fn metadata_line<'a>(
    row: &'a CommitRow,
    title: &'a BStr,
    attributions: &'a [crate::app::Attribution],
    decorations: &'a Decorations,
    mailmap: &'a gix::mailmap::Snapshot,
    options: MetadataOptions,
) -> Line<'a> {
    debug_assert!(row.metadata_loaded, "visible rows have metadata");
    let MetadataOptions {
        show_committer_date,
        show_author_name,
        show_emails,
        show_trailers,
        has_notes,
        use_mailmap,
        ref_mode,
        selected,
        preview_author_copy,
        copy_feedback,
    } = options;
    let id = row.id.to_hex().to_string();
    let id_style = if preview_author_copy || copy_feedback == Some(CopyKind::Id) {
        Style::default()
    } else {
        color(Color::Magenta).add_modifier(Modifier::BOLD)
    };
    let mut spans = vec![Span::styled(
        id[..7].to_owned(),
        if selected {
            id_style.add_modifier(Modifier::REVERSED)
        } else {
            id_style
        },
    )];
    let mut labels = decorations
        .get(&row.id)
        .into_iter()
        .flatten()
        .filter(|decoration| match ref_mode {
            RefMode::All => true,
            RefMode::Default => decoration.kind != DecorationKind::Special,
            RefMode::None => false,
        })
        .peekable();
    if labels.peek().is_some() {
        spans.push(Span::raw(" ("));
        for (index, decoration) in labels.enumerate() {
            if index != 0 {
                spans.push(Span::raw(", "));
            }
            spans.push(Span::styled(
                decoration.name.to_str_lossy(),
                decoration_style(decoration.kind),
            ));
        }
        spans.push(Span::raw(") "));
    } else {
        spans.push(Span::raw(" "));
    }
    if show_committer_date {
        spans.push(Span::styled(
            format!("{} ", row.committer_time.format_or_unix(gix::date::time::format::SHORT)),
            color(Color::Blue),
        ));
    }
    if show_author_name {
        let author = author_label(row.author, mailmap, use_mailmap, show_emails && !row.author.is_bot());
        let mut author_style = if copy_feedback == Some(CopyKind::Author) {
            Style::default()
        } else if preview_author_copy {
            color(Color::Magenta).add_modifier(Modifier::BOLD)
        } else {
            color(Color::Green)
        };
        if row.author.is_github_noreply() {
            author_style = author_style.add_modifier(Modifier::ITALIC);
        }
        spans.push(Span::styled(
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
                spans.push(Span::styled(
                    if markers.len() == 1 {
                        marker.to_owned()
                    } else {
                        format!("{}: ", markers.join(", "))
                    },
                    color(Color::Green).add_modifier(Modifier::DIM),
                ));
                for (index, (name, style)) in actors.into_iter().enumerate() {
                    if index != 0 {
                        spans.push(Span::raw(", "));
                    }
                    spans.push(Span::styled(name, style));
                }
                spans.push(Span::raw(" "));
            }
        }
    }
    if row.has_agent_marker {
        spans.push(Span::styled("[A] ", color(NOTE_COLOR)));
    }
    if has_notes {
        spans.push(Span::styled("[N] ", color(NOTE_COLOR)));
    }
    if !show_emails {
        spans.push(Span::raw(title.to_str_lossy()));
    }
    Line::from(spans)
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

fn decoration_style(kind: DecorationKind) -> Style {
    match kind {
        DecorationKind::Head => Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
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

fn color_graph(
    frame: &mut Frame<'_>,
    area: Rect,
    graph: &str,
    offset: usize,
    highlight: Option<Color>,
    signature: SignatureState,
) {
    for (x, symbol) in graph.chars().skip(offset).take(area.width as usize).enumerate() {
        if symbol.is_whitespace() {
            continue;
        }
        let style = if let Some(highlight) = highlight {
            color(highlight).add_modifier(Modifier::REVERSED)
        } else if symbol == '●' {
            color(signature_color(signature))
        } else {
            graph_style(offset.saturating_add(x) / 2)
        };
        frame.buffer_mut()[(area.x + x as u16, area.y)].set_style(style);
    }
}

fn signature_color(signature: SignatureState) -> Color {
    match signature {
        SignatureState::Unsigned => Color::Blue,
        SignatureState::Unverified | SignatureState::Verifying => Color::Rgb(255, 165, 0),
        SignatureState::Verified => Color::Green,
        SignatureState::Failed => Color::LightRed,
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
    fn counts_commits_until_the_graph_is_complete_then_tracks_the_selected_row() {
        let mut app = App::new(3);
        app.extend_commits(
            (1..=3)
                .map(|byte| Commit {
                    id: gix::ObjectId::Sha1([byte; 20]),
                    parent_ids: Default::default(),
                    committer_time: gix::date::Time::default(),
                    author: author(b"author", b"author@example.com"),
                    attributions: 0..0,
                    title: "subject".into(),
                    metadata_loaded: true,
                    has_agent_marker: false,
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

        assert_eq!(history_position(&app), "#3");
        app.update(Action::MoveDown);
        assert_eq!(history_position(&app), "#2");
        app.update(Action::MoveDown);
        assert_eq!(history_position(&app), "#1");
    }

    #[test]
    fn keeps_the_completed_footer_while_background_progress_is_deferred() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(1);
        app.extend_commits(vec![Commit {
            id: gix::ObjectId::Sha1([1; 20]),
            parent_ids: Default::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: false,
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
            computing.contains("1 commits · computing"),
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
    fn renders_selection_info_beside_the_right_marker_without_dimming_it() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(2);
        app.extend_commits(vec![Commit {
            id: gix::ObjectId::Sha1([1; 20]),
            parent_ids: Default::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "a subject which is deliberately too long".into(),
            metadata_loaded: true,
            has_agent_marker: false,
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

        app.show_selection_tail = false;
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
        assert!(!rendered_row(&terminal).contains(info));

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
    fn renders_a_colored_file_diff_pager() -> Result<(), Box<dyn std::error::Error>> {
        let diff = BuiltInDiff::new(
            "M file".into(),
            ["--- a/file", "+++ b/file", "@@ -1 +1 @@", "-old", "+new"]
                .into_iter()
                .map(Into::into)
                .collect(),
        );
        let mut terminal = Terminal::new(TestBackend::new(40, 7))?;

        terminal.draw(|frame| draw_file_diff(frame, &diff, 0, 0))?;

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
        assert!(rendered_line(&terminal, 6).contains("Enter/q/Esc back"));
        Ok(())
    }

    #[test]
    fn renders_and_streams_compact_commit_diff_summaries() -> Result<(), Box<dyn std::error::Error>> {
        let row = Commit {
            id: gix::ObjectId::Sha1([1; 20]),
            parent_ids: Default::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: 0..0,
            metadata_loaded: true,
            has_agent_marker: false,
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
            ],
            ..Changes::default()
        };
        let diff = BuiltInDiff::new(
            title.clone(),
            ["--- a/old", "+++ b/old"].into_iter().map(Into::into).collect(),
        )
        .with_summary(commit_diff_summary(&changes, &[Some((2, 0)), Some((1, 1))], 3, 1));
        let mut terminal = Terminal::new(TestBackend::new(64, 9))?;

        terminal.draw(|frame| draw_file_diff(frame, &diff, 0, 0))?;

        assert_eq!(rendered_line(&terminal, 0).trim(), title);
        assert_eq!(rendered_line(&terminal, 1).trim(), "new | 2 ++");
        assert_eq!(rendered_line(&terminal, 2).trim(), "old | 2 +-");
        let summary = "root · A 1 + M 1 = 2 · +3 -1";
        assert_eq!(rendered_line(&terminal, 3).trim(), summary);
        let buffer = terminal.backend().buffer();
        let summary_x = |needle| {
            summary[..summary.find(needle).expect("summary term is present")]
                .chars()
                .count() as u16
        };
        assert_eq!(buffer[(0, 3)].fg, COMPARED_PARENT_COLOR);
        assert_eq!(buffer[(summary_x("A 1"), 3)].fg, Color::Green);
        assert_eq!(buffer[(summary_x("-1"), 3)].fg, Color::LightRed);
        assert_eq!(buffer[(9, 1)].fg, Color::Green);
        assert_eq!(buffer[(10, 2)].fg, Color::LightRed);
        assert_eq!(rendered_line(&terminal, 4).trim(), "");
        assert_eq!(rendered_line(&terminal, 5).trim(), "--- a/old");

        let mut streamed = Vec::new();
        diff.write_to(&mut streamed)?;
        assert_eq!(
            streamed,
            b"0101010 mapped author subject\n new | 2 ++\n old | 2 +-\nroot \xc2\xb7 A 1 + M 1 = 2 \xc2\xb7 +3 -1 \n\n--- a/old\n+++ b/old\n"
        );
        Ok(())
    }

    #[test]
    fn renders_grouped_attributions_and_bot_names() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(1);
        app.extend_commits(LoadedCommits {
            rows: vec![Commit {
                id: gix::ObjectId::Sha1([1; 20]),
                parent_ids: Default::default(),
                committer_time: gix::date::Time::default(),
                author: author(b"Codex", b"codex@openai.com"),
                attributions: 0..8,
                title: "subject".into(),
                metadata_loaded: true,
                has_agent_marker: false,
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
        let mut terminal = Terminal::new(TestBackend::new(160, 2))?;

        let mailmap =
            gix::mailmap::Snapshot::from_bytes(b"Mapped Human <mapped@example.com> Human <human@example.com>\n");
        terminal.draw(|frame| super::draw(frame, &mut app, &Decorations::new(), &mailmap, None, None))?;

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
            rendered_line(&terminal, 1).contains("t trailers"),
            "the footer advertises the trailer toggle"
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
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "unique comment".into(),
            metadata_loaded: true,
            has_agent_marker: false,
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
                committer_time: gix::date::Time::default(),
                author: author(b"Author", b"1+author@users.noreply.github.com"),
                attributions: 0..1,
                title: "subject".into(),
                metadata_loaded: true,
                has_agent_marker: false,
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
        app.extend_commits(vec![Commit {
            id,
            parent_ids: Default::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: false,
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

        let footer_text =
            "#1 · ↑↓/jk move · h/l pan · Enter diff · [ align · o commit · c changes · v view · y copy · q quit";
        let selected_line = "> ● 0101010 (HEAD) 1970-01-01 mapped author subject";
        let mut expected = Buffer::with_lines([format!("{selected_line:<180}"), format!("{footer_text:<180}")]);
        for x in 0..11 {
            expected[(x, 0)].set_style(Style::default().add_modifier(Modifier::REVERSED));
        }
        for x in 0..4 {
            expected[(x, 0)].set_style(Style::default().fg(Color::Blue).add_modifier(Modifier::REVERSED));
        }
        for x in 4..11 {
            expected[(x, 0)].set_style(
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::REVERSED | Modifier::BOLD),
            );
        }
        for x in 13..17 {
            expected[(x, 0)].set_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
        }
        for x in 19..30 {
            expected[(x, 0)].set_style(Style::default().fg(Color::Blue));
        }
        for x in 30..44 {
            expected[(x, 0)].set_style(Style::default().fg(Color::Green));
        }
        expected[(selected_line.chars().count() as u16 + 2, 0)]
            .set_style(Style::default().fg(Color::Blue).add_modifier(Modifier::REVERSED));
        let commit = footer_text[..footer_text.find("o commit").expect("the commit toggle is present")]
            .chars()
            .count();
        for x in commit..commit + "o commit".len() {
            expected[(x as u16, 1)].set_style(Style::default().add_modifier(Modifier::DIM));
        }
        terminal.backend().assert_buffer(&expected);

        let row = terminal.backend().buffer();
        assert!(
            row[(10, 0)].modifier.contains(Modifier::REVERSED),
            "selection includes the final hash character"
        );
        assert!(
            !row[(11, 0)].modifier.contains(Modifier::REVERSED),
            "selection ends immediately after the hash"
        );
        assert_eq!(row[(13, 0)].fg, Color::Cyan, "reference colors remain visible");
        assert!(
            !rendered_line(&terminal, 1).contains("Esc cancel"),
            "completed work cannot be cancelled"
        );

        app.notice = Some("worktree removed; using common repository".into());
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert_eq!(
            rendered_line(&terminal, 1).trim(),
            "worktree removed; using common repository",
            "recovery information replaces the status until the next action"
        );

        app.history_display_expanded = true;
        app.update(Action::ToggleMailmap);
        assert!(app.notice.is_none(), "the next action restores the normal status");
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert!(
            rendered_row(&terminal).contains(" author subject"),
            "m restores the original author name"
        );
        assert!(footer_is_dim(&terminal, "m mailmap"), "disabled mailmap is dimmed");
        app.update(Action::ToggleMailmap);

        app.update(Action::ToggleDate);
        app.update(Action::ToggleName);
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        let row = rendered_row(&terminal);
        assert!(!row.contains("1970-01-01"), "d hides the committer date");
        assert!(
            !row.contains("author"),
            "the first n hides the author when there are no attributions"
        );
        assert!(!row.contains("refs/patches"), "special refs are hidden until requested");
        assert!(row.contains("subject"), "the commit subject remains visible");
        assert!(footer_is_dim(&terminal, "d date"), "disabled date is dimmed");
        assert!(footer_is_dim(&terminal, "n name"), "disabled name is dimmed");

        app.update(Action::ToggleName);
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert!(
            rendered_row(&terminal).contains("author"),
            "the second n restores the author name"
        );
        assert!(
            !footer_is_dim(&terminal, "n name"),
            "the restored name mode is not dimmed"
        );

        app.update(Action::ToggleRefs);
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert!(!rendered_row(&terminal).contains("HEAD"), "no refs hides regular refs");
        assert!(
            !rendered_row(&terminal).contains("refs/patches"),
            "no refs hides special refs"
        );
        assert!(footer_is_dim(&terminal, "r no refs"), "no refs is dimmed");

        app.update(Action::ToggleRefs);
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert!(rendered_row(&terminal).contains("HEAD"), "all refs shows regular refs");
        assert!(
            rendered_row(&terminal).contains("refs/patches"),
            "all refs shows special refs"
        );
        assert!(!footer_is_dim(&terminal, "r all refs"), "all refs is not dimmed");

        app.update(Action::ToggleRefs);
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert!(rendered_row(&terminal).contains("HEAD"), "refs shows regular refs");
        assert!(
            !rendered_row(&terminal).contains("refs/patches"),
            "refs hides special refs"
        );
        assert!(!footer_is_dim(&terminal, "r refs"), "refs is not dimmed");

        app.has_hidden_filter = true;
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert!(
            rendered_line(&terminal, 1).contains("h show hidden"),
            "the footer advertises the configured hidden-history toggle"
        );
        app.show_hidden = true;
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert!(
            rendered_line(&terminal, 1).contains("h hide hidden"),
            "the footer reflects the unfiltered view"
        );

        app.manual_refresh = true;
        app.update(Action::PreviewAuthorCopy(true));
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        let row = rendered_row(&terminal);
        assert!(
            row.contains("author subject"),
            "holding Shift reveals the raw author even when names are hidden"
        );
        let buffer = terminal.backend().buffer();
        let hash_x = row.find("0101010").expect("the commit hash is rendered") as u16;
        let author_x = row.find("author").expect("the author is rendered") as u16;
        assert_ne!(buffer[(hash_x, 0)].fg, Color::Magenta, "the hash yields its copy color");
        assert_eq!(
            buffer[(author_x, 0)].fg,
            Color::Magenta,
            "the author takes the copy color"
        );
        assert!(
            rendered_line(&terminal, 1).contains("Y copy author"),
            "the footer previews the shifted shortcut"
        );
        assert!(
            rendered_line(&terminal, 1).contains("R refresh"),
            "the footer previews the shifted refresh shortcut"
        );
        assert!(
            !rendered_line(&terminal, 1).contains("r refs"),
            "the shifted refresh shortcut replaces the reference toggle"
        );
        Ok(())
    }

    #[test]
    fn removes_the_copied_fields_color_from_only_the_selected_row_for_one_frame()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(2);
        app.extend_commits(
            (1..=2)
                .map(|n| Commit {
                    id: gix::ObjectId::Sha1([n; 20]),
                    parent_ids: Default::default(),
                    committer_time: gix::date::Time::default(),
                    author: author(b"author", b"author@example.com"),
                    attributions: 0..0,
                    title: format!("subject {n}").into(),
                    metadata_loaded: true,
                    has_agent_marker: false,
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

        drop(app.update(Action::PreviewAuthorCopy(true)));
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
            Color::Magenta,
            "author feedback does not affect other rows"
        );

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert_eq!(
            terminal.backend().buffer()[(selected_author, 0)].fg,
            Color::Magenta,
            "the author color returns on the next frame"
        );
        Ok(())
    }

    #[test]
    fn colors_commit_disks_by_signature_state() -> Result<(), Box<dyn std::error::Error>> {
        let states = [
            (SignatureState::Unsigned, Color::Blue),
            (SignatureState::Unverified, Color::Rgb(255, 165, 0)),
            (SignatureState::Verified, Color::Green),
            (SignatureState::Failed, Color::LightRed),
        ];
        let mut terminal = Terminal::new(TestBackend::new(2, states.len() as u16))?;
        terminal.draw(|frame| {
            for (y, (state, _)) in states.iter().enumerate() {
                color_graph(
                    frame,
                    Rect::new(0, y as u16, 2, 1),
                    "●─",
                    0,
                    Some(signature_color(*state)),
                    *state,
                );
            }
        })?;

        for (y, (_, expected)) in states.iter().enumerate() {
            for x in 0..2 {
                let cell = &terminal.backend().buffer()[(x, y as u16)];
                assert_eq!(cell.fg, *expected);
                assert!(cell.modifier.contains(Modifier::REVERSED));
            }
        }
        Ok(())
    }

    #[test]
    fn shows_signature_action_only_while_actionable() -> Result<(), Box<dyn std::error::Error>> {
        let id = gix::ObjectId::Sha1([1; 20]);
        let mut app = App::new(1);
        app.extend_commits(vec![Commit {
            id,
            parent_ids: Default::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: false,
            signature: SignatureState::Unverified,
        }]);
        complete(&mut app);
        let mut terminal = Terminal::new(TestBackend::new(160, 2))?;

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(rendered_line(&terminal, 1).contains("s ● -> ●"));

        app.finish_signature_verification(vec![(id, false)]);
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(rendered_line(&terminal, 1).contains("s 1 ●"));
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
        app.extend_commits(vec![Commit {
            id: gix::ObjectId::Sha1([1; 20]),
            parent_ids: Default::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: false,
            signature: SignatureState::Unsigned,
        }]);
        let mut terminal = Terminal::new(TestBackend::new(120, 6))?;

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(footer_is_dim(&terminal, "o commit"), "the closed commit pane is dimmed");

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
            terminal.backend().buffer()[(62, 0)].symbol(),
            "s",
            "the title starts on the first pane row after two columns of horizontal margin"
        );
        assert_eq!(
            terminal.backend().buffer()[(60, 0)].symbol(),
            "│",
            "the pane has a left border"
        );
        assert_eq!(
            terminal.backend().buffer()[(62, 2)].symbol(),
            "b",
            "the commit body remains separated from its title"
        );
        assert!(
            !footer_is_dim(&terminal, "o commit"),
            "the open commit pane is not dimmed"
        );

        app.update(Action::ToggleCommit);
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert_eq!(
            terminal.backend().buffer()[(62, 3)].symbol(),
            " ",
            "closing the pane removes the commit body"
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
            wide_terminal.backend().buffer()[(118, 0)].symbol(),
            "x",
            "the pane reserves eighty content columns on a wide screen"
        );
        assert!(
            rendered_line(&wide_terminal, 0)
                .chars()
                .skip(118)
                .take(80)
                .collect::<String>()
                .ends_with(" word")
                && rendered_line(&wide_terminal, 1)
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
        app.extend_commits(vec![Commit {
            id: gix::ObjectId::Sha1([1; 20]),
            parent_ids: Default::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: false,
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
    fn changing_the_changes_height_keeps_history_alignment_stable() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(11);
        app.extend_commits(
            (1..=10)
                .map(|n| Commit {
                    id: gix::ObjectId::Sha1([n; 20]),
                    parent_ids: Default::default(),
                    committer_time: gix::date::Time::default(),
                    author: author(b"author", b"author@example.com"),
                    attributions: 0..0,
                    title: format!("subject {n}").into(),
                    metadata_loaded: true,
                    has_agent_marker: false,
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
        Ok(())
    }

    #[test]
    fn shows_changed_paths_in_a_bottom_pane_below_the_summary() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(6);
        app.extend_commits(vec![
            Commit {
                id: gix::ObjectId::Sha1([1; 20]),
                parent_ids: [gix::ObjectId::Sha1([2; 20]), gix::ObjectId::Sha1([3; 20])]
                    .into_iter()
                    .collect(),
                committer_time: gix::date::Time::default(),
                author: author(b"author", b"author@example.com"),
                attributions: 0..0,
                title: "merge".into(),
                metadata_loaded: true,
                has_agent_marker: false,
                signature: SignatureState::Unsigned,
            },
            Commit {
                id: gix::ObjectId::Sha1([2; 20]),
                parent_ids: Default::default(),
                committer_time: gix::date::Time::default(),
                author: author(b"author", b"author@example.com"),
                attributions: 0..0,
                title: "parent".into(),
                metadata_loaded: true,
                has_agent_marker: false,
                signature: SignatureState::Unsigned,
            },
        ]);
        complete(&mut app);
        let changes = Changes {
            parent: None,
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
        assert!(rendered_line(&terminal, 15).contains("Tab switch"));

        app.changes_suppressed = true;
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
            app.changes_mode.is_some() && !footer_is_dim(&terminal, "c changes"),
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
        assert!(rendered_line(&terminal, 15).contains("Tab → tree changes"));
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
        assert!(
            rendered_line(&terminal, 15).contains("Tab switch"),
            "focus feedback lasts for one redraw"
        );
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
            rendered_line(&terminal, 14).contains("Enter diff · y copy · c tree"),
            "the changes pane advertises the next cycle mode"
        );
        assert!(
            rendered_line(&terminal, 14).contains("y copy"),
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
                "vs parent 1/2 0202020 · p next parent · ↑↓/jk move · h/l pan · Enter diff · y copy · c tree"
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
            !rendered_line(&terminal, 15).contains("p next parent"),
            "parent cycling is absent from the main status line"
        );

        app.update(Action::ToggleCommit);
        let worktree_changes = Changes::default();
        terminal.draw(|frame| {
            super::draw_with_worktree(
                frame,
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
        assert_eq!(terminal.backend().buffer()[(60, 7)].symbol(), "│");
        assert_eq!(
            app.viewport_rows, 7,
            "history remains bounded above the highest overlay"
        );
        assert!(rendered_line(&terminal, 0).starts_with('>'));

        let mut wide_terminal = Terminal::new(TestBackend::new(240, 16))?;
        wide_terminal.draw(|frame| {
            super::draw_with_worktree(
                frame,
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
        assert_eq!(wide_terminal.backend().buffer()[(156, 7)].symbol(), "│");
        Ok(())
    }

    #[test]
    fn summarizes_staged_and_unstaged_changes_in_the_top_border() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(1);
        app.changes_mode = Some(ChangesMode::Both);
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
        let mut terminal = Terminal::new(TestBackend::new(80, 8))?;
        terminal.draw(|frame| {
            super::draw_with_worktree(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                Some(&changes),
            );
        })?;

        let (header_y, header) = (0..8)
            .map(|y| (y, rendered_line(&terminal, y)))
            .find(|(_, line)| line.contains("Worktree"))
            .expect("the worktree border is visible");
        assert!(
            header.contains("Worktree ── S 1 + U 1 = 2 · +3 -1"),
            "the border distinguishes staged and unstaged rows: {header:?}"
        );
        let staged_y = header_y + 1;
        let unstaged_y = header_y + 2;
        let staged_x = rendered_line(&terminal, staged_y).find('A').expect("staged letter") as u16;
        let unstaged_x = rendered_line(&terminal, unstaged_y).find('M').expect("unstaged letter") as u16;
        assert_eq!(terminal.backend().buffer()[(staged_x, staged_y)].fg, Color::Green);
        assert_eq!(
            terminal.backend().buffer()[(unstaged_x, unstaged_y)].fg,
            Color::LightRed
        );

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
            super::draw_with_worktree(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                None,
                None,
                Some(&Changes::default()),
            );
        })?;
        let (clean_y, clean_header) = (0..8)
            .map(|y| (y, rendered_line(&terminal, y)))
            .find(|(_, line)| line.contains("Worktree clean"))
            .expect("an enabled clean worktree remains visible as an empty block");
        let clean_x = clean_header.find("clean").expect("clean label") as u16;
        assert_eq!(terminal.backend().buffer()[(clean_x, clean_y)].fg, Color::Green);
        assert!(
            !(0..8).any(|y| rendered_line(&terminal, y).contains("+0") || rendered_line(&terminal, y).contains("-0")),
            "a clean worktree omits empty diff counts"
        );
        assert!(
            !(0..8).any(|y| rendered_line(&terminal, y).contains("= 0")),
            "a clean worktree has no empty aggregate"
        );
        assert!(!app.worktree_changes_visible, "an empty block is not focusable");
        Ok(())
    }

    #[test]
    fn lays_out_tree_and_worktree_changes_by_available_width() -> Result<(), Box<dyn std::error::Error>> {
        let (layout, panes, height) = changes_pane_areas(Rect::new(0, 0, 120, 20), 10, Some((5, 30)), Some((3, 25)));
        assert_eq!(layout, ChangesLayout::SideBySide);
        assert_eq!(height, 5);
        assert_eq!(panes[0].outer, Rect::new(0, 15, 60, 5));
        assert_eq!(panes[1].outer, Rect::new(60, 17, 60, 3));

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
            super::draw_with_worktree(
                frame,
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
        let (left, _) = halves(rendered_line(&terminal, 5));
        assert!(left.contains("Tree"));
        let (left, right) = halves(rendered_line(&terminal, 6));
        assert!(left.contains("tree-file"));
        assert!(right.contains("Worktree"));
        let (left, right) = halves(rendered_line(&terminal, 7));
        assert!(left.contains("tree-file-2"));
        assert!(right.contains("worktree-file"));
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(60, 5)].symbol(), "┐");
        assert_eq!(buffer[(60, 6)].symbol(), "├");
        assert_eq!(buffer[(60, 7)].symbol(), "│");
        assert_eq!(buffer[(60, 8)].symbol(), "│");
        Ok(())
    }

    #[test]
    fn aligns_commit_trailers_and_wraps_only_in_the_value_column() -> Result<(), Box<dyn std::error::Error>> {
        let mut terminal = Terminal::new(TestBackend::new(40, 8))?;
        let message = b"subject\n\nbody\n\nShort: one two three four five six seven\nCo-authored-by: Alice".as_bstr();

        terminal.draw(|frame| {
            render_commit_message(frame, frame.area(), message, &[], 0);
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
            render_commit_message(frame, frame.area(), b"plain subject\n\nplain body".as_bstr(), &[], 0);
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
            render_commit_message(frame, frame.area(), message, &[], 0);
        })?;
        assert!(
            rendered_line(&terminal, 2).contains("not a trailer")
                && rendered_line(&terminal, 3).contains("another note"),
            "mixed message parts are combined ahead of the trailers"
        );
        assert!(
            rendered_line(&terminal, 4).trim().is_empty(),
            "the combined message remains separated from its trailers"
        );
        assert_eq!(
            rendered_line(&terminal, 5).find("Alice"),
            Some(15),
            "the first trailer moves below all message parts"
        );
        assert_eq!(
            rendered_line(&terminal, 6).find("Bob"),
            Some(15),
            "later trailer runs share the aligned value column"
        );
        Ok(())
    }

    #[test]
    fn renders_note_markers_and_notes_before_trailers() -> Result<(), Box<dyn std::error::Error>> {
        let id = gix::ObjectId::Sha1([1; 20]);
        let mut app = App::new(1);
        app.extend_commits(vec![Commit {
            id,
            parent_ids: Default::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: true,
            signature: SignatureState::Unsigned,
        }]);
        app.set_notes(id, vec!["review note".into()]);
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
                &["review note".into()],
                0,
            );
        })?;
        assert_eq!(rendered_line(&message, 4).trim(), "Notes:");
        assert_eq!(rendered_line(&message, 5).trim(), "review note");
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
        app.extend_commits(
            (1..=3)
                .map(|n| Commit {
                    id: gix::ObjectId::Sha1([n; 20]),
                    parent_ids: Default::default(),
                    committer_time: gix::date::Time::default(),
                    author: author(b"author", b"author@example.com"),
                    attributions: 0..0,
                    title: format!("subject {n}").into(),
                    metadata_loaded: true,
                    has_agent_marker: false,
                    signature: SignatureState::Unsigned,
                })
                .collect::<Vec<_>>(),
        );
        complete(&mut app);
        app.update(Action::Last);
        let mut terminal = Terminal::new(TestBackend::new(24, 3))?;

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;

        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(5, 0)].symbol(), "2", "the viewport starts at the second row");
        assert_eq!(buffer[(5, 1)].symbol(), "3", "the selected third row remains visible");
        assert!(
            buffer[(0, 1)].modifier.contains(Modifier::REVERSED),
            "the slice-local selection highlights the global selection"
        );
        assert!(
            buffer[(23, 1)].modifier.contains(Modifier::REVERSED),
            "a clipped selection marker uses the right border"
        );
        let hash_color = buffer[(5, 1)].fg;
        assert_eq!(app.selected, Some(2), "drawing preserves the global selection");
        assert_eq!(app.offset, 1, "drawing preserves the global offset");

        app.show_selection_tail = false;
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        let buffer = terminal.backend().buffer();
        assert!(
            !buffer[(0, 1)].modifier.contains(Modifier::REVERSED | Modifier::DIM),
            "the inactive marker has no selection modifiers"
        );
        assert!(
            !buffer[(5, 1)].modifier.contains(Modifier::REVERSED | Modifier::DIM),
            "the inactive hash has no selection modifiers"
        );
        assert_eq!(buffer[(0, 1)].symbol(), ">", "the inactive row keeps its marker");
        assert_eq!(buffer[(0, 1)].fg, Color::Reset, "the marker uses normal text color");
        assert_eq!(
            buffer[(0, 1)].bg,
            Color::Reset,
            "the marker has no selection background"
        );
        assert_eq!(buffer[(5, 1)].fg, hash_color, "the hash returns to its normal color");
        assert_eq!(buffer[(5, 1)].bg, Color::Reset, "the hash has no selection background");
        assert!(
            !buffer[(23, 1)].modifier.contains(Modifier::REVERSED),
            "the final frame hides the trailing selection marker"
        );
        Ok(())
    }

    #[test]
    fn dims_rows_outside_the_shift_reachability_set() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(2);
        app.extend_commits(
            (1..=2)
                .map(|n| Commit {
                    id: gix::ObjectId::Sha1([n; 20]),
                    parent_ids: Default::default(),
                    committer_time: gix::date::Time::default(),
                    author: author(b"author", b"author@example.com"),
                    attributions: 0..0,
                    title: format!("subject {n}").into(),
                    metadata_loaded: true,
                    has_agent_marker: false,
                    signature: SignatureState::Unsigned,
                })
                .collect::<Vec<_>>(),
        );
        complete(&mut app);
        app.update(Action::PreviewAuthorCopy(true));
        let mut terminal = Terminal::new(TestBackend::new(80, 3))?;
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;

        assert!(
            !terminal.backend().buffer()[(10, 0)].modifier.contains(Modifier::DIM),
            "the anchor row remains bright"
        );
        assert!(
            terminal.backend().buffer()[(10, 1)].modifier.contains(Modifier::DIM),
            "an unreachable row is dimmed"
        );
        Ok(())
    }

    #[test]
    fn renders_hidden_boundary_rows_without_colors() -> Result<(), Box<dyn std::error::Error>> {
        let commit = |n: u8| Commit {
            id: gix::ObjectId::Sha1([n; 20]),
            parent_ids: Default::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: format!("subject {n}").into(),
            metadata_loaded: true,
            has_agent_marker: false,
            signature: SignatureState::Unverified,
        };
        let mut app = App::new(2);
        app.extend_commits(vec![commit(1)]);
        app.extend_hidden_commits(vec![commit(2)]);
        complete(&mut app);
        app.set_lane(0, "● ");
        app.set_lane(1, "● ");
        let mut terminal = Terminal::new(TestBackend::new(80, 3))?;

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;

        let line = rendered_line(&terminal, 1);
        assert!(line.contains("subject 2"), "the hidden commit keeps its normal content");
        let visible = rendered_line(&terminal, 0);
        let visible_hash = visible.find("0101010").expect("the visible hash is present") as u16;
        assert_ne!(terminal.backend().buffer()[(visible_hash, 0)].fg, Color::Reset);
        let hash = line.find("0202020").expect("the hidden hash is visible") as u16;
        assert!(
            terminal.backend().buffer()[(hash, 1)].modifier.contains(Modifier::BOLD),
            "non-color styling is retained"
        );
        for x in 0..terminal.backend().buffer().area.width {
            let cell = &terminal.backend().buffer()[(x, 1)];
            assert_eq!(cell.fg, Color::Reset, "the hidden row has no foreground colors");
            assert_eq!(cell.bg, Color::Reset, "the hidden row has no background colors");
            assert!(cell.modifier.contains(Modifier::DIM), "the hidden row is dimmed");
        }
        assert_ne!(
            terminal.backend().buffer()[(0, 1)].symbol(),
            ">",
            "the hidden row is not selected"
        );
        Ok(())
    }

    #[test]
    fn shows_the_selected_parent_beside_the_junction_disk() -> Result<(), Box<dyn std::error::Error>> {
        let commit = |n: u8, parents: &[u8]| Commit {
            id: gix::ObjectId::Sha1([n; 20]),
            parent_ids: parents
                .iter()
                .map(|parent| gix::ObjectId::Sha1([*parent; 20]))
                .collect(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: format!("subject {n}").into(),
            metadata_loaded: true,
            has_agent_marker: false,
            signature: SignatureState::Unsigned,
        };
        let mut app = App::new(4);
        app.extend_commits(vec![
            commit(4, &[3, 2]),
            commit(3, &[1]),
            commit(2, &[1]),
            commit(1, &[]),
        ]);
        complete(&mut app);
        app.update(Action::PreviewAuthorCopy(true));
        let mut terminal = Terminal::new(TestBackend::new(80, 5))?;

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        let metadata_x = rendered_row(&terminal)
            .find("0404040")
            .expect("the junction metadata is visible");
        assert_eq!(
            terminal.backend().buffer()[(3, 0)].symbol(),
            "2",
            "the initial parent number replaces the connector beside the disk"
        );

        app.update(Action::ScrollRight);
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert_eq!(terminal.backend().buffer()[(3, 0)].symbol(), "1");
        assert_eq!(
            rendered_row(&terminal).find("0404040"),
            Some(metadata_x),
            "cycling parents does not shift metadata"
        );
        Ok(())
    }

    #[test]
    fn uses_the_tig_palette_without_coloring_the_selection() -> Result<(), Box<dyn std::error::Error>> {
        let id = gix::ObjectId::Sha1([1; 20]);
        let commit = Commit {
            id,
            parent_ids: Default::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
            metadata_loaded: true,
            has_agent_marker: false,
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
                show_committer_date: true,
                show_author_name: true,
                show_emails: false,
                show_trailers: true,
                has_notes: false,
                use_mailmap: false,
                ref_mode: RefMode::All,
                selected: false,
                preview_author_copy: false,
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
        assert_eq!(
            style("HEAD"),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
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
        assert_eq!(buffer[(2, 0)].fg, Color::Blue, "commit dots use graph-commit");
        assert_eq!(buffer[(4, 0)].fg, Color::Yellow, "lanes cycle through tig's palette");
        assert_eq!(
            buffer[(16, 0)].fg,
            Color::Magenta,
            "the palette repeats after seven lanes"
        );
        assert!(
            buffer[(16, 0)].modifier.contains(Modifier::BOLD),
            "the second palette cycle is bold"
        );
        Ok(())
    }

    #[test]
    fn overlays_metadata_on_wide_graphs_and_allows_natural_flow() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(1);
        app.extend_commits(vec![Commit {
            id: gix::ObjectId::Sha1([1; 20]),
            parent_ids: Default::default(),
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: format!("{} subject-tail", "a".repeat(50)).into(),
            metadata_loaded: true,
            has_agent_marker: false,
            signature: SignatureState::Unsigned,
        }]);
        complete(&mut app);
        app.set_lane(0, &format!("{}{}", "A".repeat(40), "B".repeat(40)));
        let mut terminal = Terminal::new(TestBackend::new(60, 2))?;

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        let aligned_column = rendered_row(&terminal)
            .find("0101010")
            .expect("alignment keeps metadata visible beside wide graphs");
        assert!(aligned_column < 60, "alignment keeps metadata within the viewport");

        app.update(Action::ScrollRight);
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert_eq!(terminal.backend().buffer()[(2, 0)].symbol(), "B");
        assert_eq!(
            rendered_row(&terminal).find("0101010"),
            Some(aligned_column),
            "horizontal graph scrolling leaves aligned metadata fixed"
        );

        app.update(Action::ScrollLeft);
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        app.update(Action::ToggleAlign);
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(
            !rendered_row(&terminal).contains("0101010"),
            "[ restores natural post-graph placement"
        );
        assert!(footer_is_dim(&terminal, "[ align"), "disabled alignment is dimmed");

        app.update(Action::ScrollRight);
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(
            rendered_row(&terminal).contains("0101010"),
            "l pages far enough right to reveal natural metadata"
        );

        app.update(Action::ScrollLeft);
        app.set_lane(0, "● ");
        app.update(Action::ToggleAlign);
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert_eq!(
            terminal.backend().buffer()[(4, 0)].symbol(),
            "0",
            "aligned metadata starts immediately after the widest visible lane"
        );
        assert!(
            !rendered_row(&terminal).contains("subject-tail"),
            "long aligned metadata starts clipped"
        );

        app.update(Action::ScrollRight);
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert_eq!(
            terminal.backend().buffer()[(4, 0)].symbol(),
            "0",
            "l leaves aligned metadata fixed when there is no graph left to pan"
        );
        assert!(
            !rendered_row(&terminal).contains("subject-tail"),
            "aligned metadata remains clipped instead of becoming horizontal-scroll content"
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

    fn footer_is_dim(terminal: &Terminal<TestBackend>, label: &str) -> bool {
        let y = terminal.backend().buffer().area.height - 1;
        let footer = rendered_line(terminal, y);
        let x = footer[..footer.find(label).expect("toggle is visible")].chars().count() as u16;
        terminal.backend().buffer()[(x, y)].modifier.contains(Modifier::DIM)
    }
}
