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
    app::{App, AttributionKind, ChangeKind, Changes, CommitRow, CopyKind, NameMode, RefMode, SignatureState, State},
    history::{DecorationKind, Decorations},
};

const COMPARED_PARENT_COLOR: Color = Color::Cyan;
const NOTE_COLOR: Color = Color::LightMagenta;
const PANE_STATUS_BACKGROUND: Color = Color::DarkGray;

pub(crate) fn draw_file_diff(frame: &mut Frame<'_>, diff: &BuiltInDiff, offset: usize, horizontal_offset: usize) {
    let [header, body, footer] =
        Layout::vertical([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());
    frame.render_widget(Clear, frame.area());
    frame.render_widget(
        Paragraph::new(diff.title.to_str_lossy()).style(Style::default().add_modifier(Modifier::BOLD)),
        header,
    );
    let lines = diff
        .lines
        .iter()
        .map(|line| {
            let style = if line.starts_with(b"@@") {
                Style::default().fg(Color::Cyan)
            } else if line.starts_with(b"+") {
                Style::default().fg(Color::Green)
            } else if line.starts_with(b"-") {
                Style::default().fg(Color::Red)
            } else if line.starts_with(b"Binary ") {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            Line::styled(line.to_str_lossy(), style)
        })
        .collect::<Vec<_>>();
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

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    app: &mut App,
    decorations: &Decorations,
    mailmap: &gix::mailmap::Snapshot,
    commit_message: Option<&BStr>,
    changes: Option<&Changes>,
) {
    let [top_spacer, mut body, bottom_spacer, footer] = Layout::vertical([
        Constraint::Length(u16::from(app.inline)),
        Constraint::Min(0),
        Constraint::Length(u16::from(app.inline)),
        Constraint::Length(1),
    ])
    .areas(frame.area());
    frame.render_widget(Clear, top_spacer);
    frame.render_widget(Clear, bottom_spacer);
    let full_body = body;
    let compared_parent = if app.changes_visible() {
        changes.and_then(|changes| changes.parent.map(|parent| parent.id))
    } else {
        None
    };
    let changes_pane = app.changes_visible().then(|| {
        let desired_height = changes.filter(|changes| changes.is_visible()).map_or(0, |changes| {
            u16::try_from(changes.paths.len()).unwrap_or(u16::MAX).saturating_add(3)
        });
        let max_height = frame.area().height / 2;
        let height = desired_height.min(max_height);
        let [commits, changes] = Layout::vertical([Constraint::Min(0), Constraint::Length(height)]).areas(full_body);
        body = commits;
        (
            changes,
            changes.inner(Margin {
                horizontal: 2,
                vertical: 1,
            }),
        )
    });
    let commit_pane = app.show_commit.then(|| {
        let width = 80.min(full_body.width / 2);
        let [commits, message] = Layout::horizontal([Constraint::Min(0), Constraint::Length(width)]).areas(full_body);
        body.width = body.width.min(commits.width);
        (
            message,
            message.inner(Margin {
                horizontal: 2,
                vertical: 1,
            }),
        )
    });
    app.viewport_rows = body.height as usize;
    app.ensure_visible();
    let start = app.offset.min(app.rows.len());
    let end = start.saturating_add(app.viewport_rows).min(app.rows.len());
    let lane_end = start.saturating_add(full_body.height as usize).min(app.rows.len());
    let visible_rows = &app.rows[start..end];
    let has_verifiable_signatures = visible_rows.iter().enumerate().any(|(index, row)| {
        !app.is_row_hidden(start + index)
            && matches!(row.signature, SignatureState::Unverified | SignatureState::Verifying)
    });
    let lanes = app.render_lanes(start..lane_end);
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
                .min(body.right().saturating_sub(1));
            frame.buffer_mut()[(marker_x, y)].set_style(style);
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
    if let Some((outer, area)) = changes_pane {
        frame.render_widget(Clear, outer);
        frame.render_widget(Block::new().borders(Borders::TOP), outer);
        if let Some(changes) = changes.filter(|changes| changes.is_visible()) {
            render_changes(frame, area, changes, app);
            if app.changes_focused {
                let status = Rect::new(
                    outer.x.saturating_add(2),
                    outer.bottom().saturating_sub(1),
                    outer.width.saturating_sub(4),
                    1,
                );
                let mut spans = Vec::new();
                if let Some(parent) = changes.parent {
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
                if let Some(error) = &app.diff_error {
                    spans.push(Span::styled(format!("diff: {error}"), color(Color::Red)));
                } else {
                    spans.push(Span::raw("↑↓/jk move · h/l pan · Enter diff"));
                }
                spans.push(Span::raw(" · c to hide"));
                frame.render_widget(
                    Paragraph::new(Line::from(spans)).style(Style::default().bg(PANE_STATUS_BACKGROUND)),
                    status,
                );
            }
        }
        if !app.changes_focused {
            frame
                .buffer_mut()
                .set_style(outer, Style::default().add_modifier(Modifier::DIM));
        }
    }
    if let Some((outer, area)) = commit_pane {
        frame.render_widget(Clear, outer);
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

    let status = match app.state {
        State::Loading => "",
        State::Cancelling => " · cancelling",
        State::Computing => " · computing",
        State::Complete => "",
        State::Cancelled => " · cancelled",
    };
    let mut footer_spans = vec![Span::raw(format!(
        "{} commits{status} · ↑↓/jk move · h/l pan",
        app.rows.len()
    ))];
    if app.changes_visible() && changes.is_some_and(Changes::is_visible) {
        footer_spans.push(match app.focus_feedback.take() {
            Some(destination) => Span::raw(format!(" · Tab → {destination}")),
            None => Span::raw(" · Tab switch"),
        });
    }
    if app.changes_focused {
        footer_spans.push(Span::raw(" · q/Esc history"));
    }
    footer_spans.extend([Span::raw(" · "), toggle("[ align", app.align_metadata)]);
    footer_spans.extend([Span::raw(" · "), toggle("o commit", app.show_commit)]);
    footer_spans.extend([Span::raw(" · "), toggle("c changes", app.show_changes)]);
    if app.has_hidden_filter {
        footer_spans.extend([
            Span::raw(" · "),
            toggle(
                if app.show_hidden {
                    "v hide hidden"
                } else {
                    "v show hidden"
                },
                app.show_hidden,
            ),
        ]);
    }
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
    footer_spans.push(Span::raw(" · "));
    if app.preview_author_copy && app.manual_refresh {
        footer_spans.push(toggle(
            "R refresh",
            matches!(app.state, State::Complete | State::Cancelled),
        ));
    } else {
        let ref_label = match app.ref_mode {
            RefMode::All => "r all refs",
            RefMode::Default => "r refs",
            RefMode::None => "r no refs",
        };
        footer_spans.push(toggle(ref_label, app.ref_mode != RefMode::None));
    }
    footer_spans.push(Span::raw(if app.preview_author_copy {
        " · Y copy author"
    } else {
        " · y copy"
    }));
    if app.signature_failures > 0 {
        footer_spans.extend([
            Span::raw(format!(" · s {} ", app.signature_failures)),
            Span::styled("●", color(Color::Red)),
        ]);
    } else if has_verifiable_signatures {
        footer_spans.extend([
            Span::raw(" · s "),
            Span::styled("●", color(Color::Rgb(255, 165, 0))),
            Span::raw(" -> "),
            Span::styled("●", color(Color::Green)),
        ]);
    }
    if !app.changes_focused {
        if app.state == State::Loading {
            footer_spans.push(Span::raw(" · Esc cancel"));
        }
        footer_spans.push(Span::raw(" · q quit"));
    }
    frame.render_widget(Paragraph::new(Line::from(footer_spans)), footer);
    if app.changes_focused {
        frame
            .buffer_mut()
            .set_style(body, Style::default().add_modifier(Modifier::DIM));
    }
}

fn render_changes(frame: &mut Frame<'_>, area: Rect, changes: &Changes, app: &mut App) {
    if !changes.is_visible() || area.height == 0 {
        app.set_changes_bounds(0, 0, area.width as usize, 0);
        return;
    }
    let mut summary = Vec::new();
    for kind in [
        ChangeKind::Added,
        ChangeKind::Modified,
        ChangeKind::Deleted,
        ChangeKind::Renamed,
        ChangeKind::Copied,
        ChangeKind::TypeChanged,
    ] {
        let count = changes.paths.iter().filter(|change| change.kind == kind).count();
        if count == 0 {
            continue;
        }
        if !summary.is_empty() {
            summary.push(Span::raw("  "));
        }
        summary.push(Span::styled(
            format!("{} = {count}", kind.letter()),
            color(change_color(kind)),
        ));
    }
    if !summary.is_empty() {
        summary.push(Span::raw(" · "));
    }
    summary.extend([
        Span::raw(format!("{} files changed · ", changes.paths.len())),
        Span::styled(format!("+{}", changes.lines_added), color(Color::Green)),
        Span::raw(" "),
        Span::styled(format!("-{}", changes.lines_removed), color(Color::Red)),
    ]);
    frame.render_widget(
        Paragraph::new(Line::from(summary)),
        Rect::new(area.x, area.y, area.width, 1),
    );

    let path_capacity = usize::from(area.height.saturating_sub(1));
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
            let selected = app.changes_focused && index == app.changes_selected;
            let path_style = if selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            let mut spans = vec![
                Span::styled(change.kind.letter().to_string(), color(change_color(change.kind))),
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
                spans.extend([
                    Span::raw(" "),
                    Span::styled(format!("+{insertions}"), color(Color::Green)),
                    Span::raw(" "),
                    Span::styled(format!("-{removals}"), color(Color::Red)),
                ]);
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
    app.set_changes_bounds(visible_paths, changes.paths.len(), area.width as usize, horizontal_max);
    let path_area = Rect::new(
        area.x,
        area.y.saturating_add(1),
        area.width,
        u16::try_from(visible_paths).unwrap_or(u16::MAX),
    );
    frame.render_widget(
        Paragraph::new(Text::from(
            lines
                .into_iter()
                .skip(app.changes_offset)
                .take(visible_paths)
                .collect::<Vec<_>>(),
        ))
        .scroll((0, u16::try_from(app.changes_horizontal_offset).unwrap_or(u16::MAX))),
        path_area,
    );
    let hidden = changes
        .paths
        .len()
        .saturating_sub(app.changes_offset.saturating_add(visible_paths));
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
        ChangeKind::Deleted => Color::Red,
        ChangeKind::Renamed | ChangeKind::Copied => Color::Cyan,
        ChangeKind::TypeChanged => Color::Magenta,
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
        SignatureState::Failed => Color::Red,
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
        Color::Red,
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
            (1, Color::Red),
            (2, Color::Green),
            (3, Color::Cyan),
            (4, Color::Red),
            (5, Color::Green),
        ] {
            assert_eq!(terminal.backend().buffer()[(0, y)].fg, color);
        }
        assert!(rendered_line(&terminal, 6).contains("Enter/q/Esc back"));
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

        let footer_text = "1 commits · ↑↓/jk move · h/l pan · [ align · o commit · c changes · d date · e emails · n names · m mailmap · t trailers · r refs · y copy · q quit";
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
        expected[(selected_line.chars().count() as u16 + 1, 0)]
            .set_style(Style::default().fg(Color::Blue).add_modifier(Modifier::REVERSED));
        let commit = footer_text[..footer_text.find("o commit").expect("the commit toggle is present")]
            .chars()
            .count();
        for x in commit..commit + "o commit".len() {
            expected[(x as u16, 1)].set_style(Style::default().add_modifier(Modifier::DIM));
        }
        let email = footer_text[..footer_text.find("e emails").expect("the email toggle is present")]
            .chars()
            .count();
        for x in email..email + "e emails".len() {
            expected[(x as u16, 1)].set_style(Style::default().add_modifier(Modifier::DIM));
        }
        terminal.backend().assert_buffer(&expected);

        app.inline = true;
        let mut inline_terminal = Terminal::new(TestBackend::new(140, 4))?;
        inline_terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert!(
            rendered_line(&inline_terminal, 0).trim().is_empty(),
            "inline mode separates the commits from preceding content"
        );
        assert!(
            rendered_line(&inline_terminal, 2).trim().is_empty(),
            "inline mode separates the commits from the status line"
        );
        assert!(rendered_line(&inline_terminal, 3).starts_with("1 commits"));
        app.inline = false;

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

        app.update(Action::ToggleMailmap);
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
            rendered_line(&terminal, 1).contains("v show hidden"),
            "the footer advertises the configured hidden-history toggle"
        );
        app.show_hidden = true;
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None, None))?;
        assert!(
            rendered_line(&terminal, 1).contains("v hide hidden"),
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
            (SignatureState::Failed, Color::Red),
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
            terminal.backend().buffer()[(62, 1)].symbol(),
            "s",
            "the pane is capped at half width with two columns of horizontal margin"
        );
        assert_eq!(
            terminal.backend().buffer()[(62, 3)].symbol(),
            "b",
            "vertical margin leaves the full commit body intact"
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
        wide_terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                Some(b"subject".as_bstr()),
                None,
            );
        })?;
        assert_eq!(
            wide_terminal.backend().buffer()[(122, 1)].symbol(),
            "s",
            "the pane remains eighty columns wide on a wide screen"
        );
        Ok(())
    }

    #[test]
    fn changing_the_changes_height_keeps_history_alignment_stable() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(11);
        app.extend_commits(
            (1..=8)
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
        app.set_lane(6, "●──────── ");
        let path = crate::app::PathChange {
            kind: ChangeKind::Modified,
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
            rendered_line(&terminal, 0).find("0101010"),
            Some(short),
            "changes pane height does not move aligned history metadata"
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
                    source: None,
                    path: "added".into(),
                    lines: Some((10, 0)),
                },
                crate::app::PathChange {
                    kind: ChangeKind::Modified,
                    source: None,
                    path: "modified".into(),
                    lines: Some((5, 2)),
                },
                crate::app::PathChange {
                    kind: ChangeKind::Deleted,
                    source: None,
                    path: "deleted".into(),
                    lines: Some((0, 7)),
                },
                crate::app::PathChange {
                    kind: ChangeKind::Renamed,
                    source: Some("old".into()),
                    path: "new".into(),
                    lines: Some((3, 3)),
                },
                crate::app::PathChange {
                    kind: ChangeKind::Copied,
                    source: Some("source".into()),
                    path: "copy".into(),
                    lines: Some((0, 0)),
                },
                crate::app::PathChange {
                    kind: ChangeKind::TypeChanged,
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
            terminal.backend().buffer()[(20, 7)].symbol(),
            "─",
            "the changes pane starts at the screen's halfway point"
        );
        assert!(
            terminal.backend().buffer()[(20, 7)].modifier.contains(Modifier::DIM),
            "the inactive changes border is dimmed"
        );
        assert!(
            !terminal.backend().buffer()[(5, 0)].modifier.contains(Modifier::DIM)
                && !terminal.backend().buffer()[(2, 15)].modifier.contains(Modifier::DIM),
            "the focused history and its status use their normal intensity"
        );
        let summary = rendered_line(&terminal, 8);
        assert!(
            summary.contains("A = 1  M = 1  D = 1  R = 1  C = 1  T = 1 · 6 files changed · +42 -17"),
            "the pane starts with nonzero status and line aggregates"
        );
        let added_x = summary.find("A = 1").expect("added aggregate is visible") as u16;
        let deleted_x = summary.find("D = 1").expect("deleted aggregate is visible") as u16;
        assert_eq!(terminal.backend().buffer()[(added_x, 8)].fg, Color::Green);
        assert_eq!(terminal.backend().buffer()[(deleted_x, 8)].fg, Color::Red);
        assert!(
            terminal.backend().buffer()[(added_x, 8)]
                .modifier
                .contains(Modifier::DIM),
            "the inactive summary is dimmed without losing its colors"
        );
        assert!(
            rendered_line(&terminal, 9).contains("A added"),
            "changed paths follow the summary in diff order"
        );
        let inactive_path = rendered_line(&terminal, 9);
        let inactive_x = inactive_path.find("A added").expect("changed path is visible") as u16;
        assert!(
            terminal.backend().buffer()[(inactive_x, 9)]
                .modifier
                .contains(Modifier::DIM)
                && terminal.backend().buffer()[(inactive_x + 2, 9)]
                    .modifier
                    .contains(Modifier::DIM),
            "the inactive change kind and path are dimmed"
        );
        assert!(
            !rendered_line(&terminal, 9).contains("+10"),
            "inactive panes do not display a path selection"
        );
        assert!(
            rendered_line(&terminal, 13).contains("… 2 lines not shown"),
            "the capped pane reports paths that do not fit"
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
            !rendered_line(&terminal, 8).contains("files changed"),
            "repeated history navigation temporarily hides the changes pane"
        );
        assert!(
            app.show_changes && !footer_is_dim(&terminal, "c changes"),
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
            !terminal.backend().buffer()[(20, 7)].modifier.contains(Modifier::DIM),
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
        assert!(rendered_line(&terminal, 15).contains("Tab → changes"));
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
            !terminal.backend().buffer()[(added_x, 8)]
                .modifier
                .contains(Modifier::DIM)
                && !terminal.backend().buffer()[(2, 14)].modifier.contains(Modifier::DIM),
            "the focused summary and status use their normal intensity"
        );
        let selected = rendered_line(&terminal, 10);
        assert!(selected.contains("M modified +5 -2"));
        let path_x = selected.find("modified").expect("selected path is visible") as u16;
        let kind_x = selected.find("M modified").expect("selected kind is visible") as u16;
        let added_x = selected.find("+5").expect("selected additions are visible") as u16;
        let removed_x = selected.find("-2").expect("selected removals are visible") as u16;
        assert!(
            !terminal.backend().buffer()[(kind_x, 10)]
                .modifier
                .contains(Modifier::DIM)
                && !terminal.backend().buffer()[(path_x, 10)]
                    .modifier
                    .contains(Modifier::DIM),
            "focused paths use their normal intensity"
        );
        assert!(
            terminal.backend().buffer()[(path_x, 10)]
                .modifier
                .contains(Modifier::REVERSED),
            "the selected filepath is inverted"
        );
        assert_eq!(terminal.backend().buffer()[(added_x, 10)].fg, Color::Green);
        assert_eq!(terminal.backend().buffer()[(removed_x, 10)].fg, Color::Red);
        assert!(
            !terminal.backend().buffer()[(added_x, 10)]
                .modifier
                .contains(Modifier::REVERSED),
            "the diff-line suffix keeps its normal background"
        );
        assert!(
            !rendered_line(&terminal, 9).contains("+10"),
            "only the selected path displays its line counts"
        );
        assert!(rendered_line(&terminal, 13).contains("… 2 lines not shown"));
        assert!(rendered_line(&terminal, 14).contains("↑↓/jk move · h/l pan"));

        assert!(
            rendered_line(&terminal, 14).contains("Enter diff · c to hide"),
            "the visible changes pane advertises how to hide it"
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
        assert_eq!(app.changes_horizontal_offset, 20);
        assert!(
            rendered_line(&terminal, 12).contains("tail"),
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
            rendered_line(&short_terminal, 5).contains("… 1 line not shown"),
            "the overflow count follows the selected final path when no path row fits"
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
            rendered_line(&terminal, 8).starts_with("  A = 1"),
            "parent context no longer crowds the aggregate summary"
        );
        assert!(
            rendered_line(&terminal, 14)
                .contains("vs parent 1/2 0202020 · p next parent · ↑↓/jk move · h/l pan · Enter diff · c to hide"),
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
        terminal.draw(|frame| {
            super::draw(
                frame,
                &mut app,
                &Decorations::new(),
                &gix::mailmap::Snapshot::default(),
                Some(b"subject".as_bstr()),
                Some(&changes),
            );
        })?;
        assert_eq!(
            terminal.backend().buffer()[(62, 7)].symbol(),
            " ",
            "the right commit pane is rendered over the bottom changes pane"
        );
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
