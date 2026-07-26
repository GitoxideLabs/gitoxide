use gix::bstr::{BStr, BString, ByteSlice};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Clear, Paragraph, Wrap},
};

use crate::{
    app::{App, AttributionKind, CommitRow, CopyKind, NameMode, RefMode, SignatureState, State},
    history::{DecorationKind, Decorations},
};

const NOTE_COLOR: Color = Color::LightMagenta;

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    app: &mut App,
    decorations: &Decorations,
    mailmap: &gix::mailmap::Snapshot,
    commit_message: Option<&BStr>,
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
    let commit_pane = app.show_commit.then(|| {
        let width = 80.min(body.width / 2);
        let [commits, message] = Layout::horizontal([Constraint::Min(0), Constraint::Length(width)]).areas(body);
        body = commits;
        message.inner(Margin {
            horizontal: 2,
            vertical: 1,
        })
    });
    app.viewport_rows = body.height as usize;
    app.ensure_visible();
    let start = app.offset.min(app.rows.len());
    let end = start.saturating_add(app.viewport_rows).min(app.rows.len());
    let visible_rows = &app.rows[start..end];
    let has_verifiable_signatures = visible_rows
        .iter()
        .any(|row| matches!(row.signature, SignatureState::Unverified | SignatureState::Verifying));
    let lanes = app.render_lanes(start..end);
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
                    use_mailmap: app.use_mailmap && !preview_author_copy && copy_feedback != Some(CopyKind::Author),
                    ref_mode,
                    selected: selected == Some(start + index),
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
    let metadata_max_offset = if align_metadata {
        metadata
            .iter()
            .map(Line::width)
            .max()
            .unwrap_or_default()
            .saturating_sub((content.width as usize).saturating_sub(align_width))
    } else {
        0
    };
    let max_offset = if align_metadata {
        graph_max_offset.saturating_add(metadata_max_offset)
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
    let metadata_offset = horizontal_offset.saturating_sub(graph_max_offset);

    for (index, metadata) in metadata.into_iter().enumerate() {
        let lane = lanes.lane(index);
        let y = body.y.saturating_add(index as u16);
        let selected = app.selected == Some(start + index);
        let metadata_width = metadata.width();
        let signature_color = signature_color(visible_rows[index].signature);
        let style = if selected {
            color(signature_color).add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
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
                selected,
                visible_rows[index].signature,
            );
            let aligned = Rect::new(
                content.x.saturating_add(align_width as u16),
                y,
                content.width.saturating_sub(align_width as u16),
                1,
            );
            frame.render_widget(Clear, aligned);
            frame.render_widget(Paragraph::new(metadata).scroll((0, metadata_offset as u16)), aligned);
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
                selected,
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
                align_width.saturating_add(metadata_width.saturating_sub(metadata_offset))
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
    }
    app.set_horizontal_bounds(content.width as usize, max_offset);
    if let (Some(area), Some(message)) = (commit_pane, commit_message) {
        render_commit_message(frame, area, message);
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
    footer_spans.extend([Span::raw(" · "), toggle("[ align", app.align_metadata)]);
    footer_spans.extend([Span::raw(" · "), toggle("o commit", app.show_commit)]);
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
    let ref_label = match app.ref_mode {
        RefMode::All => "r all refs",
        RefMode::Default => "r refs",
        RefMode::None => "r no refs",
    };
    footer_spans.extend([Span::raw(" · "), toggle(ref_label, app.ref_mode != RefMode::None)]);
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
    if app.state == State::Loading {
        footer_spans.push(Span::raw(" · Esc cancel"));
    }
    footer_spans.push(Span::raw(" · q quit"));
    frame.render_widget(Paragraph::new(Line::from(footer_spans)), footer);
}

fn render_commit_message(frame: &mut Frame<'_>, area: Rect, message: &BStr) {
    let parsed = gix::objs::commit::MessageRef::from_bytes(message);
    let Some(body) = parsed.body() else {
        frame.render_widget(
            Paragraph::new(commit_text(parsed.title, None)).wrap(Wrap { trim: false }),
            area,
        );
        return;
    };
    let mut body_message = BString::default();
    let mut trailers = Vec::new();
    for block in body.message_blocks() {
        body_message.extend_from_slice(block.message);
        trailers.extend(block.trailers());
    }
    if trailers.is_empty() || area.width < 3 {
        frame.render_widget(
            Paragraph::new(commit_text(parsed.title, parsed.body)).wrap(Wrap { trim: false }),
            area,
        );
        return;
    }
    let key_width = trailers
        .iter()
        .map(|trailer| Line::raw(trailer.token.to_str_lossy()).width())
        .max()
        .unwrap_or_default();
    if key_width > area.width.saturating_sub(3) as usize {
        frame.render_widget(
            Paragraph::new(commit_text(parsed.title, parsed.body)).wrap(Wrap { trim: false }),
            area,
        );
        return;
    }
    let key_width = key_width as u16;

    let body_message = body_message.trim_end().as_bstr();
    let text = commit_text(parsed.title, (!body_message.is_empty()).then_some(body_message));
    let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
    let mut y = area
        .y
        .saturating_add(u16::try_from(paragraph.line_count(area.width)).unwrap_or(u16::MAX))
        .saturating_add(1);
    frame.render_widget(paragraph, area);

    let value_x = area.x.saturating_add(key_width).saturating_add(2);
    let value_width = area.right().saturating_sub(value_x);
    for trailer in trailers {
        if y >= area.bottom() {
            break;
        }
        let value = Paragraph::new(trailer.value.to_str_lossy()).wrap(Wrap { trim: false });
        let height = u16::try_from(value.line_count(value_width))
            .unwrap_or(u16::MAX)
            .max(1)
            .min(area.bottom().saturating_sub(y));
        frame.render_widget(
            Paragraph::new(format!("{}:", trailer.token.to_str_lossy()))
                .style(color(Color::Green))
                .right_aligned(),
            Rect::new(area.x, y, key_width.saturating_add(1), 1),
        );
        frame.render_widget(value, Rect::new(value_x, y, value_width, height));
        y = y.saturating_add(height);
    }
}

fn commit_text<'a>(title: &'a BStr, body: Option<&'a BStr>) -> Text<'a> {
    let mut text = Text::raw(title.to_str_lossy());
    for line in &mut text.lines {
        line.style = Style::default().add_modifier(Modifier::BOLD);
    }
    if let Some(body) = body.filter(|body| !body.is_empty()) {
        text.lines.push(Line::default());
        text.lines.extend(Text::raw(body.to_str_lossy()).lines);
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
    selected: bool,
    signature: SignatureState,
) {
    for (x, symbol) in graph.chars().skip(offset).take(area.width as usize).enumerate() {
        if symbol.is_whitespace() {
            continue;
        }
        let style = if selected {
            color(signature_color(signature)).add_modifier(Modifier::REVERSED)
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
        super::draw(frame, app, decorations, &gix::mailmap::Snapshot::default(), None);
    }

    fn complete(app: &mut App) {
        let rows = app
            .start_lane_computation()
            .expect("a loading app starts lane computation");
        let (rows, lanes, lane_time) = crate::app::compute_lanes(rows);
        app.finish_lane_computation(rows, lanes, lane_time);
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
        terminal.draw(|frame| super::draw(frame, &mut app, &Decorations::new(), &mailmap, None))?;

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
        terminal.draw(|frame| super::draw(frame, &mut app, &Decorations::new(), &mailmap, None))?;
        let row = rendered_row(&terminal);
        assert!(row.contains("Codex"), "the first n keeps the primary actor");
        assert!(
            !row.contains("Mapped Human"),
            "the first n hides trailer actors while trailers are enabled"
        );
        app.update(Action::ToggleName);
        terminal.draw(|frame| super::draw(frame, &mut app, &Decorations::new(), &mailmap, None))?;
        let row = rendered_row(&terminal);
        assert!(!row.contains("Codex"), "the second n hides the primary actor");
        assert!(
            !row.contains("Mapped Human"),
            "the second n keeps trailer actors hidden"
        );
        app.update(Action::ToggleName);
        app.update(Action::ToggleMailmap);
        terminal.draw(|frame| super::draw(frame, &mut app, &Decorations::new(), &mailmap, None))?;
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
        let mut terminal = Terminal::new(TestBackend::new(150, 2))?;

        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None))?;

        let footer_text = "1 commits · ↑↓/jk move · h/l pan · [ align · o commit · d date · e emails · n names · m mailmap · t trailers · r refs · y copy · q quit";
        let selected_line = "> ● 0101010 (HEAD) 1970-01-01 mapped author subject";
        let mut expected = Buffer::with_lines([format!("{selected_line:<150}"), format!("{footer_text:<150}")]);
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
        inline_terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None))?;
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
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None))?;
        assert!(
            rendered_row(&terminal).contains(" author subject"),
            "m restores the original author name"
        );
        assert!(footer_is_dim(&terminal, "m mailmap"), "disabled mailmap is dimmed");
        app.update(Action::ToggleMailmap);

        app.update(Action::ToggleDate);
        app.update(Action::ToggleName);
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None))?;
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
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None))?;
        assert!(
            rendered_row(&terminal).contains("author"),
            "the second n restores the author name"
        );
        assert!(
            !footer_is_dim(&terminal, "n name"),
            "the restored name mode is not dimmed"
        );

        app.update(Action::ToggleRefs);
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None))?;
        assert!(!rendered_row(&terminal).contains("HEAD"), "no refs hides regular refs");
        assert!(
            !rendered_row(&terminal).contains("refs/patches"),
            "no refs hides special refs"
        );
        assert!(footer_is_dim(&terminal, "r no refs"), "no refs is dimmed");

        app.update(Action::ToggleRefs);
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None))?;
        assert!(rendered_row(&terminal).contains("HEAD"), "all refs shows regular refs");
        assert!(
            rendered_row(&terminal).contains("refs/patches"),
            "all refs shows special refs"
        );
        assert!(!footer_is_dim(&terminal, "r all refs"), "all refs is not dimmed");

        app.update(Action::ToggleRefs);
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None))?;
        assert!(rendered_row(&terminal).contains("HEAD"), "refs shows regular refs");
        assert!(
            !rendered_row(&terminal).contains("refs/patches"),
            "refs hides special refs"
        );
        assert!(!footer_is_dim(&terminal, "r refs"), "refs is not dimmed");

        app.has_hidden_filter = true;
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None))?;
        assert!(
            rendered_line(&terminal, 1).contains("v show hidden"),
            "the footer advertises the configured hidden-history toggle"
        );
        app.show_hidden = true;
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None))?;
        assert!(
            rendered_line(&terminal, 1).contains("v hide hidden"),
            "the footer reflects the unfiltered view"
        );

        app.update(Action::PreviewAuthorCopy(true));
        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None))?;
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
                color_graph(frame, Rect::new(0, y as u16, 2, 1), "●─", 0, true, *state);
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
    fn aligns_commit_trailers_and_wraps_only_in_the_value_column() -> Result<(), Box<dyn std::error::Error>> {
        let mut terminal = Terminal::new(TestBackend::new(40, 8))?;
        let message = b"subject\n\nbody\n\nShort: one two three four five six seven\nCo-authored-by: Alice".as_bstr();

        terminal.draw(|frame| render_commit_message(frame, frame.area(), message))?;

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
            render_commit_message(frame, frame.area(), b"plain subject\n\nplain body".as_bstr());
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
        terminal.draw(|frame| render_commit_message(frame, frame.area(), message))?;
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
        assert_eq!(app.selected, Some(2), "drawing preserves the global selection");
        assert_eq!(app.offset, 1, "drawing preserves the global offset");

        app.show_selection_tail = false;
        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        let buffer = terminal.backend().buffer();
        assert!(
            buffer[(0, 1)].modifier.contains(Modifier::REVERSED),
            "the final frame keeps the left selection marker"
        );
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
        assert!(
            rendered_row(&terminal).contains("subject-tail"),
            "l reveals clipped aligned metadata after graph panning is exhausted"
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
