use gix::bstr::{BStr, ByteSlice};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph, Wrap},
};

use crate::{
    app::{App, AttributionKind, CommitRow, RefMode, State},
    history::{DecorationKind, Decorations},
};

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    app: &mut App,
    decorations: &Decorations,
    mailmap: &gix::mailmap::Snapshot,
    commit_message: Option<&BStr>,
) {
    let [mut body, footer] = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());
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
    let content = Rect::new(
        body.x.saturating_add(2),
        body.y,
        body.width.saturating_sub(2),
        body.height,
    );
    let rendered_lane_width = visible_rows
        .iter()
        .map(|row| app.lane(row))
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
    let show_author_name = app.show_author_name;
    let show_trailers = app.show_trailers;
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
                    show_trailers,
                    use_mailmap: app.use_mailmap,
                    ref_mode,
                    selected: selected == Some(start + index),
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
        visible_rows
            .iter()
            .zip(&metadata)
            .map(|(row, metadata)| app.lane(row).chars().count().saturating_add(metadata.width()))
            .max()
            .unwrap_or_default()
            .saturating_sub(content.width as usize)
    }
    .min(u16::MAX as usize);
    let horizontal_offset = app.horizontal_offset.min(max_offset);
    let graph_offset = horizontal_offset.min(graph_max_offset);
    let metadata_offset = horizontal_offset.saturating_sub(graph_max_offset);

    let visible_rows = &app.rows[start..end];
    for (index, (row, metadata)) in visible_rows.iter().zip(metadata).enumerate() {
        let lane = app.lane(row);
        let y = body.y.saturating_add(index as u16);
        let selected = app.selected == Some(start + index);
        let metadata_width = metadata.width();
        let style = if selected {
            Style::default().add_modifier(Modifier::REVERSED)
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
            color_graph(frame, row_area, lane, graph_offset, selected);
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
            color_graph(frame, row_area, lane, horizontal_offset, selected);
        }
        if selected && body.width > 0 {
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
    }
    app.set_horizontal_bounds(content.width as usize, max_offset);
    if let Some(area) = commit_pane {
        let message = commit_message.map(|message| message.to_str_lossy()).unwrap_or_default();
        frame.render_widget(Paragraph::new(message).wrap(Wrap { trim: false }), area);
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
    footer_spans.extend([Span::raw(" · "), toggle("] commit", app.show_commit)]);
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
    for (label, enabled) in [
        ("d date", app.show_committer_date),
        ("n name", app.show_author_name),
        ("m mailmap", app.use_mailmap),
        ("t trailers", app.show_trailers),
    ] {
        footer_spans.extend([Span::raw(" · "), toggle(label, enabled)]);
    }
    let ref_label = match app.ref_mode {
        RefMode::All => "r all refs",
        RefMode::Default => "r refs",
        RefMode::None => "r no refs",
    };
    footer_spans.extend([Span::raw(" · "), toggle(ref_label, app.ref_mode != RefMode::None)]);
    footer_spans.extend([Span::raw(" · y copy")]);
    if app.state == State::Loading {
        footer_spans.push(Span::raw(" · Esc cancel"));
    }
    footer_spans.push(Span::raw(" · q quit"));
    frame.render_widget(Paragraph::new(Line::from(footer_spans)), footer);
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
    show_trailers: bool,
    use_mailmap: bool,
    ref_mode: RefMode,
    selected: bool,
}

fn metadata_line<'a>(
    row: &'a CommitRow,
    title: &'a BStr,
    attributions: &'a [crate::app::Attribution],
    decorations: &'a Decorations,
    mailmap: &'a gix::mailmap::Snapshot,
    options: MetadataOptions,
) -> Line<'a> {
    let MetadataOptions {
        show_committer_date,
        show_author_name,
        show_trailers,
        use_mailmap,
        ref_mode,
        selected,
    } = options;
    let id = row.id.to_hex().to_string();
    let id_style = color(Color::Magenta).add_modifier(Modifier::BOLD);
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
        let author = author_name(row.author, mailmap, use_mailmap).to_str_lossy();
        spans.push(Span::styled(
            if row.author.is_bot() {
                format!("[{author}] ")
            } else {
                format!("{author} ")
            },
            color(Color::Green),
        ));
        if show_trailers {
            for (kind, marker) in [
                (AttributionKind::CoAuthor, "Co: "),
                (AttributionKind::Reviewed, "Re: "),
                (AttributionKind::Acked, "Ack: "),
                (AttributionKind::Tested, "Te: "),
                (AttributionKind::SignedOff, "So: "),
            ] {
                let mut actors = attributions.iter().filter(|actor| actor.kind == kind).peekable();
                if actors.peek().is_none() {
                    continue;
                }
                spans.push(Span::styled(marker, color(Color::Green).add_modifier(Modifier::DIM)));
                for (index, actor) in actors.enumerate() {
                    if index != 0 {
                        spans.push(Span::raw(", "));
                    }
                    let name = author_name(actor.author, mailmap, use_mailmap).to_str_lossy();
                    spans.push(Span::styled(
                        if actor.author.is_bot() {
                            format!("[{name}]")
                        } else {
                            name.into_owned()
                        },
                        color(Color::Green),
                    ));
                }
                spans.push(Span::raw(" "));
            }
        }
    }
    spans.push(Span::raw(title.to_str_lossy()));
    Line::from(spans)
}

fn author_name<'a>(author: &'a crate::app::Author, mailmap: &'a gix::mailmap::Snapshot, use_mailmap: bool) -> &'a BStr {
    if use_mailmap {
        mailmap
            .try_resolve_ref(gix::actor::SignatureRef {
                name: author.name,
                email: author.email,
                time: "",
            })
            .and_then(|resolved| resolved.name)
            .unwrap_or(author.name)
    } else {
        author.name
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

fn color_graph(frame: &mut Frame<'_>, area: Rect, graph: &str, offset: usize, selected: bool) {
    for (x, symbol) in graph.chars().skip(offset).take(area.width as usize).enumerate() {
        if symbol.is_whitespace() {
            continue;
        }
        let mut style = if symbol == '●' {
            Style::default().fg(Color::Blue)
        } else {
            graph_style(offset.saturating_add(x) / 2)
        };
        if selected {
            style = style.add_modifier(Modifier::REVERSED);
        }
        frame.buffer_mut()[(area.x + x as u16, area.y)].set_style(style);
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
                lane: 0..0,
                committer_time: gix::date::Time::default(),
                author: author(b"Codex", b"codex@openai.com"),
                attributions: 0..6,
                title: "subject".into(),
            }],
            attributions: vec![
                Attribution {
                    kind: AttributionKind::CoAuthor,
                    author: author(b"Human", b"human@example.com"),
                },
                Attribution {
                    kind: AttributionKind::CoAuthor,
                    author: author(b"Claude", b"noreply@anthropic.com"),
                },
                Attribution {
                    kind: AttributionKind::Reviewed,
                    author: author(b"Reviewer", b"reviewer@example.com"),
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
            row.contains(
                "[Codex] Co: Mapped Human, [Claude] Re: Reviewer Ack: Acknowledger Te: Tester So: Signer subject"
            ),
            "same-kind trailers share one marker, use mailmap, and render bots with bracketed names"
        );
        let buffer = terminal.backend().buffer();
        let style_at = |needle: &str| {
            let x = row.find(needle).expect("rendered metadata contains the named actor") as u16;
            buffer[(x, 0)].fg
        };
        assert_eq!(style_at("[Codex]"), Color::Green, "bot authors use the agent color");
        assert_eq!(style_at("Co:"), Color::Green, "attribution markers use the agent color");
        let marker_x = row.find("Co:").expect("rendered metadata contains a trailer marker") as u16;
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
        assert!(!row.contains("Codex"), "n hides the primary actor");
        assert!(
            !row.contains("Reviewer"),
            "n hides trailer actors while trailers are enabled"
        );
        app.update(Action::ToggleName);
        app.update(Action::ToggleMailmap);
        terminal.draw(|frame| super::draw(frame, &mut app, &Decorations::new(), &mailmap, None))?;
        assert!(
            rendered_row(&terminal).contains("Co: Human, [Claude]"),
            "m restores original trailer actor names"
        );
        Ok(())
    }

    #[test]
    fn renders_rows_decorations_selection_and_footer() -> Result<(), Box<dyn std::error::Error>> {
        let id = gix::ObjectId::Sha1([1; 20]);
        let mut app = App::new(2);
        app.extend_commits(vec![Commit {
            id,
            parent_ids: Default::default(),
            lane: 0..0,
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
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
        let mut terminal = Terminal::new(TestBackend::new(140, 2))?;

        terminal.draw(|frame| super::draw(frame, &mut app, &decorations, &mailmap, None))?;

        let footer_text = "1 commits · ↑↓/jk move · h/l pan · [ align · ] commit · d date · n name · m mailmap · t trailers · r refs · y copy · q quit";
        let selected_line = "> ● 0101010 (HEAD) 1970-01-01 mapped author subject";
        let mut expected = Buffer::with_lines([format!("{selected_line:<140}"), format!("{footer_text:<140}")]);
        for x in 0..11 {
            expected[(x, 0)].set_style(Style::default().add_modifier(Modifier::REVERSED));
        }
        expected[(2, 0)].set_style(Style::default().fg(Color::Blue).add_modifier(Modifier::REVERSED));
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
            .set_style(Style::default().add_modifier(Modifier::REVERSED));
        let commit = footer_text[..footer_text.find("] commit").expect("the commit toggle is present")]
            .chars()
            .count();
        for x in commit..commit + "] commit".len() {
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
        assert!(!row.contains("author"), "n hides the author name");
        assert!(!row.contains("refs/patches"), "special refs are hidden until requested");
        assert!(row.contains("subject"), "the commit subject remains visible");
        assert!(footer_is_dim(&terminal, "d date"), "disabled date is dimmed");
        assert!(footer_is_dim(&terminal, "n name"), "disabled name is dimmed");

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
            lane: 0..0,
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
        }]);
        let mut terminal = Terminal::new(TestBackend::new(120, 6))?;

        terminal.draw(|frame| draw(frame, &mut app, &Decorations::new()))?;
        assert!(footer_is_dim(&terminal, "] commit"), "the closed commit pane is dimmed");

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
            !footer_is_dim(&terminal, "] commit"),
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
    fn renders_only_the_visible_rows() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new(2);
        app.extend_commits(
            (1..=3)
                .map(|n| Commit {
                    id: gix::ObjectId::Sha1([n; 20]),
                    parent_ids: Default::default(),
                    lane: 0..0,
                    committer_time: gix::date::Time::default(),
                    author: author(b"author", b"author@example.com"),
                    attributions: 0..0,
                    title: format!("subject {n}").into(),
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
        Ok(())
    }

    #[test]
    fn uses_the_tig_palette_without_coloring_the_selection() -> Result<(), Box<dyn std::error::Error>> {
        let id = gix::ObjectId::Sha1([1; 20]);
        let commit = Commit {
            id,
            parent_ids: Default::default(),
            lane: 0..0,
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: "subject".into(),
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
                show_trailers: true,
                use_mailmap: false,
                ref_mode: RefMode::All,
                selected: false,
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
            lane: 0..0,
            committer_time: gix::date::Time::default(),
            author: author(b"author", b"author@example.com"),
            attributions: 0..0,
            title: format!("{} subject-tail", "a".repeat(50)).into(),
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
