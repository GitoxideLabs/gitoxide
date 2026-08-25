use crate::{
    app::{Action, Alignment, App, DateMode, IdMode, NameMode, RefMode},
    history::{DecorationKind, Decorations},
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum CommandId {
    Date,
    Ids,
    Emails,
    Names,
    Mailmap,
    Trailers,
    Refs,
    Hidden,
    Select,
    Reword,
    NewCommit,
    NewEmptyCommit,
    Amend,
    Spill,
    Split,
    Forget,
    Pin,
    Unpin,
    Stash,
    Unstash,
    Rebase,
    RebaseUpdate,
    Push,
    StartReview,
    FinishReview,
    Squash,
    CopyInsert,
    MoveInsert,
    StackInsert,
    ForkCommit,
    Attach,
    Todo,
    Note,
    ChecksPass,
    GitNote,
    VerifySignatures,
    Alignment,
    RefTree,
    CommitMessage,
    Changes,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum CommandGroup {
    View,
    Actions,
    Enrich,
    Information,
}

impl CommandGroup {
    pub(crate) fn label(self) -> &'static str {
        match self {
            CommandGroup::View => "View",
            CommandGroup::Actions => "Actions",
            CommandGroup::Enrich => "Enrich",
            CommandGroup::Information => "Information",
        }
    }

    pub(crate) fn prefix(self) -> char {
        match self {
            CommandGroup::View => 'v',
            CommandGroup::Actions => 'a',
            CommandGroup::Enrich => 'n',
            CommandGroup::Information => '?',
        }
    }

    fn index(self) -> usize {
        match self {
            CommandGroup::View => 0,
            CommandGroup::Actions => 1,
            CommandGroup::Enrich => 2,
            CommandGroup::Information => 3,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Command {
    pub(crate) id: CommandId,
    pub(crate) group: CommandGroup,
    pub(crate) row: usize,
    pub(crate) label: &'static str,
    pub(crate) shortcut: &'static str,
    pub(crate) active: bool,
    pub(crate) action: Action,
}

impl Command {
    pub(crate) fn key(&self) -> char {
        self.shortcut
            .chars()
            .next_back()
            .expect("command shortcuts always contain a leaf key")
    }

    pub(crate) fn search_prefix(&self) -> &'static str {
        match self.group {
            CommandGroup::Actions => "Actions commit",
            CommandGroup::Enrich => "Enrich commit",
            CommandGroup::Information if matches!(self.id, CommandId::CommitMessage | CommandId::Changes) => {
                "Information commit"
            }
            group => group.label(),
        }
    }
}

pub(crate) fn commands(app: &App, decorations: &Decorations, has_verifiable_signatures: bool) -> Vec<Command> {
    let mut out = Vec::with_capacity(36);
    let mut push = |id, group, row, label, shortcut, active, action| {
        out.push(Command {
            id,
            group,
            row,
            label,
            shortcut,
            active,
            action,
        });
    };

    let (date_label, date_active) = match app.date_mode {
        DateMode::Author => ("author date", true),
        DateMode::Committer => ("committer date", true),
        DateMode::None => ("date", false),
    };
    push(
        CommandId::Date,
        CommandGroup::View,
        0,
        date_label,
        "vd",
        date_active,
        Action::ToggleDate,
    );
    let (ids_label, ids_active) = match (app.id_mode, app.effective_id_mode()) {
        (IdMode::Off, IdMode::Change) => ("auto change ids", true),
        (IdMode::Commit, _) => ("commit ids", true),
        (IdMode::Change, _) => ("change ids", true),
        (IdMode::Off, _) => ("ids", false),
    };
    push(
        CommandId::Ids,
        CommandGroup::View,
        0,
        ids_label,
        "vi",
        ids_active,
        Action::CycleIds,
    );
    push(
        CommandId::Emails,
        CommandGroup::View,
        0,
        "emails",
        "vs",
        app.show_emails,
        Action::ToggleEmail,
    );
    let (names_label, names_active) = match app.name_mode {
        NameMode::All => ("names", true),
        NameMode::Author => ("name", true),
        NameMode::None => ("name", false),
    };
    push(
        CommandId::Names,
        CommandGroup::View,
        0,
        names_label,
        "ve",
        names_active,
        Action::ToggleName,
    );
    push(
        CommandId::Mailmap,
        CommandGroup::View,
        0,
        "mailmap",
        "vm",
        app.use_mailmap,
        Action::ToggleMailmap,
    );
    push(
        CommandId::Trailers,
        CommandGroup::View,
        0,
        "trailers",
        "vt",
        app.show_trailers,
        Action::ToggleTrailers,
    );
    let refs_label = match app.ref_mode {
        RefMode::All => "all refs",
        RefMode::Default => "refs",
        RefMode::None => "no refs",
    };
    push(
        CommandId::Refs,
        CommandGroup::View,
        0,
        refs_label,
        "vr",
        app.ref_mode != RefMode::None,
        Action::CycleRefs,
    );
    if app.has_hidden_filter {
        push(
            CommandId::Hidden,
            CommandGroup::View,
            0,
            if app.show_hidden { "hide hidden" } else { "show hidden" },
            "vh",
            app.show_hidden,
            Action::ToggleHidden,
        );
    }
    if app.can_select_entry() {
        push(
            CommandId::Select,
            CommandGroup::View,
            0,
            "select",
            "vc",
            true,
            Action::SelectEntry,
        );
    }

    let selected_is_segment = app.selected_is_segment();
    let actions_visible =
        !selected_is_segment && (app.changes_focus != Some(crate::app::ChangePane::Worktree) || app.can_amend());
    if actions_visible {
        if app.changes_focus.is_none() && app.reword_shortcut_visible() {
            push(
                CommandId::Reword,
                CommandGroup::Actions,
                0,
                "reword",
                "ao",
                true,
                Action::Reword,
            );
        }
        if app.changes_focus.is_none() && app.can_create_commit() {
            push(
                CommandId::NewCommit,
                CommandGroup::Actions,
                0,
                "new",
                "aw",
                true,
                Action::NewCommit,
            );
        }
        if app.changes_focus.is_none() && app.can_create_empty_commit() {
            push(
                CommandId::NewEmptyCommit,
                CommandGroup::Actions,
                0,
                "new-empty",
                "an",
                true,
                Action::NewEmptyCommit,
            );
        }
        if app.can_amend() {
            push(
                CommandId::Amend,
                CommandGroup::Actions,
                0,
                "amend",
                "ae",
                true,
                Action::Amend,
            );
        }
        if app.can_spill() {
            push(
                CommandId::Spill,
                CommandGroup::Actions,
                0,
                "spill",
                "al",
                true,
                Action::Spill,
            );
        }
        if app.can_split() {
            push(
                CommandId::Split,
                CommandGroup::Actions,
                0,
                "split",
                "ap",
                true,
                Action::Split,
            );
        }
        if app.changes_focus.is_none() && app.can_forget() {
            push(
                CommandId::Forget,
                CommandGroup::Actions,
                0,
                "d forget",
                "ad",
                true,
                Action::Forget,
            );
        }
        if app.changes_focus.is_none()
            && let Some(selected) = app.selected.and_then(|index| app.rows.get(index))
        {
            let pinned = decorations.get(&selected.id).is_some_and(|decorations| {
                decorations
                    .iter()
                    .any(|decoration| decoration.kind == DecorationKind::Pin)
            });
            push(
                if pinned { CommandId::Unpin } else { CommandId::Pin },
                CommandGroup::Actions,
                0,
                if pinned { "unpin" } else { "pin" },
                "ai",
                true,
                Action::TogglePin,
            );
        }

        if app.changes_focus.is_none() && app.can_stash() {
            push(
                CommandId::Stash,
                CommandGroup::Actions,
                1,
                "z stash",
                "az",
                true,
                Action::Stash,
            );
        } else if app.changes_focus.is_none() && app.can_unstash() {
            push(
                CommandId::Unstash,
                CommandGroup::Actions,
                1,
                "z unstash",
                "az",
                true,
                Action::Stash,
            );
        }
        if app.changes_focus.is_none() && app.can_rebase() {
            push(
                CommandId::Rebase,
                CommandGroup::Actions,
                1,
                "rebase",
                "ab",
                true,
                Action::Rebase,
            );
        }
        if app.changes_focus.is_none() && app.can_rebase_update() {
            push(
                CommandId::RebaseUpdate,
                CommandGroup::Actions,
                1,
                "rebase-update",
                "au",
                true,
                Action::RebaseUpdate,
            );
        }
        if app.changes_focus.is_none() && app.can_push() {
            push(
                CommandId::Push,
                CommandGroup::Actions,
                1,
                "P push",
                "aP",
                true,
                Action::Push,
            );
        }
        if app.changes_focus.is_none() && app.can_finish_review() {
            push(
                CommandId::FinishReview,
                CommandGroup::Actions,
                1,
                "finish-review",
                "ar",
                true,
                Action::Review,
            );
        } else if app.changes_focus.is_none() && app.can_review() {
            push(
                CommandId::StartReview,
                CommandGroup::Actions,
                1,
                "review",
                "ar",
                true,
                Action::Review,
            );
        }
        if app.changes_focus.is_none() && app.can_squash() {
            push(
                CommandId::Squash,
                CommandGroup::Actions,
                1,
                "squash",
                "as",
                true,
                Action::Squash,
            );
        }
        if app.changes_focus.is_none() && app.can_copy_insert() {
            push(
                CommandId::CopyInsert,
                CommandGroup::Actions,
                1,
                "copy-insert",
                "ay",
                true,
                Action::CopyInsert,
            );
        }
        if app.changes_focus.is_none() && app.can_move_insert() {
            push(
                CommandId::MoveInsert,
                CommandGroup::Actions,
                1,
                "move-insert",
                "am",
                true,
                Action::MoveInsert,
            );
        }
        if app.changes_focus.is_none() && app.can_stack_insert() {
            push(
                CommandId::StackInsert,
                CommandGroup::Actions,
                1,
                "stack-insert",
                "at",
                true,
                Action::StackInsert,
            );
        }
        if app.changes_focus.is_none() && app.can_fork_commit() {
            push(
                CommandId::ForkCommit,
                CommandGroup::Actions,
                1,
                "fork",
                "af",
                true,
                Action::ForkCommit,
            );
        }
        if app.changes_focus.is_none() && app.can_attach() {
            push(
                CommandId::Attach,
                CommandGroup::Actions,
                1,
                "attach",
                "ah",
                true,
                Action::Attach,
            );
        }
    }

    if !selected_is_segment && let Some(row) = app.selected.and_then(|index| app.rows.get(index)) {
        if app.can_reword() {
            push(
                CommandId::Todo,
                CommandGroup::Enrich,
                0,
                "todo",
                "nt",
                app.todo(row.id),
                Action::ToggleTodo,
            );
            push(
                CommandId::Note,
                CommandGroup::Enrich,
                0,
                "note",
                "no",
                app.note(row.id).is_some(),
                Action::EditNote,
            );
        }
        push(
            CommandId::ChecksPass,
            CommandGroup::Enrich,
            0,
            "checks-pass",
            "ne",
            app.checks_pass(row.id),
            Action::ToggleChecksPass,
        );
        push(
            CommandId::GitNote,
            CommandGroup::Enrich,
            0,
            "git note",
            "ng",
            !app.notes(row.id).is_empty(),
            Action::EditGitNote,
        );
    }

    if app.signature_failures > 0 || has_verifiable_signatures {
        push(
            CommandId::VerifySignatures,
            CommandGroup::Information,
            0,
            "verify signatures",
            "?s",
            true,
            Action::VerifySignatures,
        );
    }
    let (alignment_label, alignment_active) = match app.alignment {
        Alignment::Title => ("[ title", true),
        Alignment::Columns => ("[ columns", true),
        Alignment::None => ("[ align", false),
        Alignment::Compressed => ("[ compressed", true),
    };
    push(
        CommandId::Alignment,
        CommandGroup::Information,
        0,
        alignment_label,
        "?[",
        alignment_active,
        Action::ToggleAlign,
    );
    push(
        CommandId::RefTree,
        CommandGroup::Information,
        0,
        "ref-tree",
        "?t",
        false,
        Action::ToggleRefTree,
    );
    if !selected_is_segment {
        push(
            CommandId::CommitMessage,
            CommandGroup::Information,
            0,
            "message",
            "?m",
            app.show_commit,
            Action::ToggleCommit,
        );
        push(
            CommandId::Changes,
            CommandGroup::Information,
            0,
            "changes",
            "?e",
            app.changes_mode.is_some(),
            Action::ToggleChanges,
        );
    }

    let mut positions = [0; 4];
    let mut balanced = out
        .into_iter()
        .map(|command| {
            let group = command.group.index();
            let position = positions[group];
            positions[group] += 1;
            (position, group, command)
        })
        .collect::<Vec<_>>();
    balanced.sort_by_key(|(position, group, _)| (*position, *group));
    balanced.into_iter().map(|(_, _, command)| command).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::{Author, Commit, LoadedCommit, SignatureState, State},
        history::Decoration,
        menu::Menu,
    };
    use gix::{ObjectId, bstr::ByteSlice};

    fn id(n: u8) -> ObjectId {
        let mut bytes = [0; 20];
        bytes[19] = n;
        ObjectId::Sha1(bytes)
    }

    fn row(n: u8, parents: &[u8]) -> LoadedCommit {
        Commit {
            id: id(n),
            parent_ids: parents.iter().copied().map(id).collect(),
            author_time: gix::date::Time::default(),
            committer_time: gix::date::Time::default(),
            author: Box::leak(Box::new(Author {
                name: b"author".as_bstr(),
                email: b"author@example.com".as_bstr(),
            })),
            attributions: 0..0,
            title: format!("commit {n}").into(),
            metadata_loaded: true,
            has_agent_marker: false,
            is_review: false,
            signature: SignatureState::Unsigned,
        }
    }

    fn has(commands: &[Command], id: CommandId) -> bool {
        commands.iter().any(|command| command.id == id)
    }

    #[test]
    fn default_catalog_interleaves_groups_and_preserves_their_internal_order() {
        assert_eq!(
            [
                CommandGroup::View.prefix(),
                CommandGroup::Actions.prefix(),
                CommandGroup::Enrich.prefix(),
                CommandGroup::Information.prefix(),
            ],
            ['v', 'a', 'n', '?'],
            "command scopes use their displayed prefix keys"
        );
        let commands = commands(&App::new(1), &Decorations::default(), true);
        let entries = commands
            .iter()
            .map(|command| (command.group, command.label, command.shortcut, command.key()))
            .collect::<Vec<_>>();

        assert_eq!(
            entries,
            [
                (CommandGroup::View, "author date", "vd", 'd'),
                (CommandGroup::Information, "verify signatures", "?s", 's'),
                (CommandGroup::View, "ids", "vi", 'i'),
                (CommandGroup::Information, "[ title", "?[", '['),
                (CommandGroup::View, "emails", "vs", 's'),
                (CommandGroup::Information, "ref-tree", "?t", 't'),
                (CommandGroup::View, "names", "ve", 'e'),
                (CommandGroup::Information, "message", "?m", 'm'),
                (CommandGroup::View, "mailmap", "vm", 'm'),
                (CommandGroup::Information, "changes", "?e", 'e'),
                (CommandGroup::View, "trailers", "vt", 't'),
                (CommandGroup::View, "refs", "vr", 'r'),
            ],
            "groups alternate without disturbing their popup order"
        );
    }

    #[test]
    fn select_is_available_for_numbered_history() {
        let mut app = App::new(1);
        app.extend_commits(vec![row(1, &[])]);
        let rows = app
            .start_lane_computation()
            .expect("the loaded row starts lane computation");
        let (rows, graph, lane_time) = crate::app::compute_lanes(rows);
        app.finish_lane_computation(rows, graph, lane_time);

        let select = commands(&app, &Decorations::default(), false)
            .into_iter()
            .find(|command| command.id == CommandId::Select)
            .expect("numbered history offers selection by entry number");
        assert_eq!(select.group, CommandGroup::View);
        assert_eq!(select.label, "select");
        assert_eq!(select.shortcut, "vc");
        assert_eq!(select.action, Action::SelectEntry);
    }

    #[test]
    fn contextual_opposites_replace_each_other_without_becoming_the_recalled_command() {
        let mut app = App::new(2);
        app.extend_commits(vec![row(2, &[1]), row(1, &[])]);
        app.state = State::Complete;
        app.set_worktree_head(Some(id(2)), false);
        app.set_head_edit_availability(false, true, false, false, false, false, false);

        let mut decorations = Decorations::default();
        let initial = commands(&app, &decorations, false);
        assert_eq!(
            initial.iter().take(9).map(|command| command.group).collect::<Vec<_>>(),
            [
                CommandGroup::View,
                CommandGroup::Actions,
                CommandGroup::Enrich,
                CommandGroup::Information,
                CommandGroup::View,
                CommandGroup::Actions,
                CommandGroup::Enrich,
                CommandGroup::Information,
                CommandGroup::View,
            ],
            "the initial menu balances all available prefix groups"
        );
        assert!(has(&initial, CommandId::Stash));
        assert!(has(&initial, CommandId::StartReview));
        assert!(has(&initial, CommandId::Pin));
        assert!(!has(&initial, CommandId::Unstash));
        assert!(!has(&initial, CommandId::FinishReview));
        assert!(!has(&initial, CommandId::Unpin));

        let items = crate::command_picker_items(&initial);
        let mut menu = Menu::default();
        menu.open(&items);
        menu.paste("actions", &items);
        assert!(
            !menu.visible_indices().is_empty()
                && menu
                    .visible_indices()
                    .iter()
                    .all(|index| initial[*index].group == CommandGroup::Actions),
            "the production picker searches displayed prefix groups"
        );
        menu.open(&items);
        for character in "stash".chars() {
            menu.insert(character, &items);
        }
        assert_eq!(menu.submit_selected(&items), Some(CommandId::Stash));

        std::sync::Arc::make_mut(&mut app.rows[0]).is_review = true;
        app.set_head_edit_availability(false, false, true, false, true, false, false);
        decorations.insert(
            id(2),
            vec![Decoration {
                name: b"pin".as_bstr().to_owned(),
                kind: DecorationKind::Pin,
            }],
        );
        let changed = commands(&app, &decorations, false);
        assert!(has(&changed, CommandId::Unstash));
        assert!(has(&changed, CommandId::FinishReview));
        assert!(has(&changed, CommandId::Unpin));
        assert!(!has(&changed, CommandId::Stash));
        assert!(!has(&changed, CommandId::StartReview));
        assert!(!has(&changed, CommandId::Pin));

        let changed_items = crate::command_picker_items(&changed);
        menu.open(&changed_items);
        assert_eq!(
            menu.selected_index(),
            None,
            "a contextual opposite does not replace the unavailable recalled command"
        );
    }

    #[test]
    fn commit_query_finds_every_command_applied_to_a_commit() {
        let mut app = App::new(2);
        app.extend_commits(vec![row(2, &[1]), row(1, &[])]);
        app.state = State::Complete;
        app.set_worktree_head(Some(id(2)), false);
        app.set_head_edit_availability(false, true, false, false, false, false, false);

        let commands = commands(&app, &Decorations::default(), false);
        let items = crate::command_picker_items(&commands);
        let expected = commands
            .iter()
            .filter(|command| {
                matches!(command.group, CommandGroup::Actions | CommandGroup::Enrich)
                    || matches!(command.id, CommandId::CommitMessage | CommandId::Changes)
            })
            .map(|command| command.id)
            .collect::<Vec<_>>();
        let mut menu = Menu::default();
        menu.open(&items);
        menu.paste("commit", &items);

        let mut actual = Vec::new();
        while let Some(index) = menu.selected_index() {
            if actual.last() == Some(&commands[index].id) {
                break;
            }
            actual.push(commands[index].id);
            menu.down(&items);
        }
        assert_eq!(actual, expected, "commit aliases retain catalog order");
    }

    #[test]
    fn push_is_a_second_row_action_only_while_no_background_task_runs() {
        let mut app = App::new(1);
        app.state = State::Complete;
        app.set_push_branch(Some("topic".into()));

        let catalog = commands(&app, &Decorations::default(), false);
        let push = catalog
            .iter()
            .find(|command| command.id == CommandId::Push)
            .expect("a remembered branch can be pushed");
        assert_eq!(push.group, CommandGroup::Actions);
        assert_eq!(push.row, 1);
        assert_eq!(push.label, "P push");
        assert_eq!(push.shortcut, "aP");
        assert_eq!(push.action, Action::Push);

        app.start_background_task("pushing topic to origin…");
        assert!(
            !has(&commands(&app, &Decorations::default(), false), CommandId::Push),
            "the single background-task slot hides push while occupied"
        );
    }
}
