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
    RelatedHistory,
    Select,
    Reword,
    NewCommit,
    NewEmptyCommit,
    Amend,
    Spill,
    Split,
    Delete,
    Discard,
    Pin,
    Unpin,
    Stash,
    Unstash,
    Rebase,
    RebaseUpdate,
    #[cfg(feature = "blocking-network-client")]
    Fetch,
    Push,
    StartReview,
    FinishReview,
    Squash,
    CopyInsert,
    MoveInsert,
    StackInsert,
    ForkCommit,
    Attach,
    AutoMerge,
    Remerge,
    RemoveFromAutoMerge,
    RemoveAutoMergeInput,
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

const BINDINGS: &[(CommandId, CommandGroup, &str, Action)] = {
    use CommandGroup::{Actions, Enrich, Information, View};
    use CommandId as Id;
    &[
        (Id::Date, View, "vd", Action::ToggleDate),
        (Id::Ids, View, "vi", Action::CycleIds),
        (Id::Emails, View, "vs", Action::ToggleEmail),
        (Id::Names, View, "ve", Action::ToggleName),
        (Id::Mailmap, View, "vm", Action::ToggleMailmap),
        (Id::Trailers, View, "vt", Action::ToggleTrailers),
        (Id::Refs, View, "vr", Action::CycleRefs),
        (Id::Hidden, View, "vh", Action::ToggleHidden),
        (Id::RelatedHistory, View, "vo", Action::ShowRelatedHistory),
        (Id::Select, View, "vc", Action::SelectEntry),
        (Id::Reword, Actions, "ao", Action::Reword),
        (Id::NewCommit, Actions, "aw", Action::NewCommit),
        (Id::NewEmptyCommit, Actions, "aN", Action::NewEmptyCommit),
        (Id::Amend, Actions, "ae", Action::Amend),
        (Id::Spill, Actions, "al", Action::Spill),
        (Id::Split, Actions, "aS", Action::Split),
        (Id::Delete, Actions, "ad", Action::Delete),
        (Id::Discard, Actions, "ad", Action::Delete),
        (Id::Pin, Actions, "ai", Action::TogglePin),
        (Id::Unpin, Actions, "ai", Action::TogglePin),
        (Id::Stash, Actions, "aT", Action::Stash),
        (Id::Unstash, Actions, "aT", Action::Stash),
        (Id::Rebase, Actions, "ab", Action::Rebase),
        (Id::RebaseUpdate, Actions, "au", Action::RebaseUpdate),
        #[cfg(feature = "blocking-network-client")]
        (Id::Fetch, Actions, "aF", Action::Fetch),
        (Id::Push, Actions, "aP", Action::Push),
        (Id::StartReview, Actions, "ar", Action::Review),
        (Id::FinishReview, Actions, "ar", Action::Review),
        (Id::Squash, Actions, "as", Action::Squash),
        (Id::CopyInsert, Actions, "ay", Action::CopyInsert),
        (Id::MoveInsert, Actions, "am", Action::MoveInsert),
        (Id::StackInsert, Actions, "at", Action::StackInsert),
        (Id::ForkCommit, Actions, "af", Action::ForkCommit),
        (Id::Attach, Actions, "ah", Action::Attach),
        (Id::AutoMerge, Actions, "aM", Action::AutoMerge),
        (Id::Remerge, Actions, "aR", Action::Remerge),
        (Id::RemoveFromAutoMerge, Actions, "ax", Action::RemoveFromAutoMerge),
        (Id::RemoveAutoMergeInput, Actions, "aX", Action::RemoveAutoMergeInput),
        (Id::Todo, Enrich, "nt", Action::ToggleTodo),
        (Id::Note, Enrich, "no", Action::EditNote),
        (Id::ChecksPass, Enrich, "ne", Action::ToggleChecksPass),
        (Id::GitNote, Enrich, "ng", Action::EditGitNote),
        (Id::VerifySignatures, Information, "?s", Action::VerifySignatures),
        (Id::Alignment, Information, "?[", Action::ToggleAlign),
        (Id::RefTree, Information, "?t", Action::ToggleRefTree),
        (Id::CommitMessage, Information, "?m", Action::ToggleCommit),
        (Id::Changes, Information, "?e", Action::ToggleChanges),
    ]
};

pub(crate) fn shortcut_action(group: CommandGroup, key: char) -> Option<Action> {
    BINDINGS
        .iter()
        .find(|(_, candidate, shortcut, _)| *candidate == group && shortcut.ends_with(key))
        .map(|(_, _, _, action)| action.clone())
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
            CommandGroup::Actions if self.id == CommandId::Discard => "Actions worktree",
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
    let mut push = |id, row, label, active| {
        let (_, group, shortcut, action) = BINDINGS
            .iter()
            .find(|(candidate, ..)| *candidate == id)
            .expect("every command has a binding");
        out.push(Command {
            id,
            group: *group,
            row,
            label,
            shortcut,
            active,
            action: action.clone(),
        });
    };

    let (date_label, date_active) = match app.date_mode {
        DateMode::Author => ("author date", true),
        DateMode::Committer => ("committer date", true),
        DateMode::None => ("date", false),
    };
    push(CommandId::Date, 0, date_label, date_active);
    let (ids_label, ids_active) = match (app.id_mode, app.effective_id_mode()) {
        (IdMode::Off, IdMode::Change) => ("auto change ids", true),
        (IdMode::Commit, _) => ("commit ids", true),
        (IdMode::Change, _) => ("change ids", true),
        (IdMode::Off, _) => ("ids", false),
    };
    push(CommandId::Ids, 0, ids_label, ids_active);
    push(CommandId::Emails, 0, "emails", app.show_emails);
    let (names_label, names_active) = match app.name_mode {
        NameMode::All => ("names", true),
        NameMode::Author => ("name", true),
        NameMode::None => ("name", false),
    };
    push(CommandId::Names, 0, names_label, names_active);
    push(CommandId::Mailmap, 0, "mailmap", app.use_mailmap);
    push(CommandId::Trailers, 0, "trailers", app.show_trailers);
    let refs_label = match app.ref_mode {
        RefMode::All => "all refs",
        RefMode::Default => "refs",
        RefMode::None => "no refs",
    };
    push(CommandId::Refs, 0, refs_label, app.ref_mode != RefMode::None);
    if app.has_hidden_filter {
        push(
            CommandId::Hidden,
            0,
            if app.show_hidden { "hide hidden" } else { "show hidden" },
            app.show_hidden,
        );
    }
    if app.can_select_entry() {
        push(CommandId::Select, 0, "select", true);
    }
    if app.related_history_commit().is_some() {
        push(CommandId::RelatedHistory, 0, "show related history", true);
    }

    let selected_is_segment = app.selected_is_segment();
    if app.actions_visible() {
        if app.can_discard() {
            push(CommandId::Discard, 0, "discard", true);
        }
        for (id, available, label) in [
            (
                CommandId::AutoMerge,
                app.can_auto_merge(),
                if app
                    .related_history_commit()
                    .and_then(|id| decorations.get(&id))
                    .is_some_and(|names| {
                        names
                            .iter()
                            .any(|name| name.kind == crate::history::DecorationKind::Head)
                    })
                {
                    "AutoMerge"
                } else {
                    "AutoMerge into HEAD"
                },
            ),
            (CommandId::Remerge, app.can_remerge(), "Remerge"),
            (
                CommandId::RemoveFromAutoMerge,
                app.can_remove_from_auto_merge(),
                "exclude from AutoMerge",
            ),
            (
                CommandId::RemoveAutoMergeInput,
                app.can_remove_auto_merge_input(),
                "eXclude input",
            ),
        ] {
            if available {
                push(id, 1, label, true);
            }
        }
        if app.changes_focus.is_none() && app.reword_shortcut_visible() {
            push(CommandId::Reword, 0, "reword", true);
        }
        if app.changes_focus.is_none() && app.can_create_commit() {
            push(CommandId::NewCommit, 0, "new", true);
        }
        if app.changes_focus.is_none() && app.can_create_empty_commit() {
            push(CommandId::NewEmptyCommit, 0, "New-empty", true);
        }
        if app.can_amend() {
            push(CommandId::Amend, 0, "amend", true);
        }
        if app.can_spill() {
            push(CommandId::Spill, 0, "spill", true);
        }
        if app.can_split() {
            push(CommandId::Split, 0, "Split", true);
        }
        if app.changes_focus.is_none() && app.can_delete() {
            push(CommandId::Delete, 0, "delete", true);
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
                0,
                if pinned { "unpin" } else { "pin" },
                true,
            );
        }

        if app.changes_focus.is_none() && app.can_stash() {
            push(CommandId::Stash, 1, "sTash", true);
        } else if app.changes_focus.is_none() && app.can_unstash() {
            push(CommandId::Unstash, 1, "unsTash", true);
        }
        if app.changes_focus.is_none() && app.can_rebase() {
            push(CommandId::Rebase, 1, "rebase", true);
        }
        if app.changes_focus.is_none() && app.can_rebase_update() {
            push(CommandId::RebaseUpdate, 1, "rebase-update", true);
        }
        #[cfg(feature = "blocking-network-client")]
        if app.changes_focus.is_none() && app.can_fetch() {
            push(CommandId::Fetch, 1, "Fetch", true);
        }
        if app.changes_focus.is_none() && app.can_push() {
            push(CommandId::Push, 1, "Push", true);
        }
        if app.changes_focus.is_none() && app.can_finish_review() {
            push(CommandId::FinishReview, 1, "finish-review", true);
        } else if app.changes_focus.is_none() && app.can_review() {
            push(CommandId::StartReview, 1, "review", true);
        }
        if app.changes_focus.is_none() && app.can_squash() {
            push(CommandId::Squash, 1, "squash", true);
        }
        if app.changes_focus.is_none() && app.can_copy_insert() {
            push(CommandId::CopyInsert, 1, "copy-insert", true);
        }
        if app.changes_focus.is_none() && app.can_move_insert() {
            push(CommandId::MoveInsert, 1, "move-insert", true);
        }
        if app.changes_focus.is_none() && app.can_stack_insert() {
            push(CommandId::StackInsert, 1, "stack-insert", true);
        }
        if app.changes_focus.is_none() && app.can_fork_commit() {
            push(CommandId::ForkCommit, 1, "fork", true);
        }
        if app.changes_focus.is_none() && app.can_attach() {
            push(CommandId::Attach, 1, "attach", true);
        }
    }

    if !selected_is_segment && let Some(row) = app.selected.and_then(|index| app.rows.get(index)) {
        if app.can_enrich() {
            push(CommandId::Todo, 0, "todo", app.todo(row.id));
            push(CommandId::Note, 0, "note", app.note(row.id).is_some());
        }
        push(CommandId::ChecksPass, 0, "checks-pass", app.checks_pass(row.id));
        push(CommandId::GitNote, 0, "git note", !app.notes(row.id).is_empty());
    }

    if app.signature_failures > 0 || has_verifiable_signatures {
        push(CommandId::VerifySignatures, 0, "verify signatures", true);
    }
    let (alignment_label, alignment_active) = match app.alignment {
        Alignment::Title => ("[ title", true),
        Alignment::Columns => ("[ columns", true),
        Alignment::None => ("[ align", false),
        Alignment::Compressed => ("[ compressed", true),
    };
    push(CommandId::Alignment, 0, alignment_label, alignment_active);
    push(CommandId::RefTree, 0, "ref-tree", false);
    if !selected_is_segment {
        push(CommandId::CommitMessage, 0, "message", app.show_commit);
        push(CommandId::Changes, 0, "changes", app.changes_mode.is_some());
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
    fn worktree_discard_is_searchable_and_independent_of_history_selection() {
        use crate::app::{ChangePane, ChangesLayout, Effect};

        let mut app = App::new(2);
        app.set_changes_bounds(ChangePane::Worktree, 2, 2, None, 20, 0);
        app.set_changes_layout(ChangesLayout::SideBySide, false, true);
        app.changes_focus = Some(ChangePane::Worktree);
        app.worktree_changes.selected = 1;
        assert!(
            app.can_discard(),
            "discard is available even before history finishes loading"
        );
        app.update(Action::ToggleActions);
        assert!(app.actions_expanded, "the worktree actions prefix can be opened");
        let catalog = commands(&app, &Decorations::default(), false);
        assert!(!has(&catalog, CommandId::Delete), "history deletion stays hidden");
        let items = crate::command_picker_items(&catalog);
        let mut menu = Menu::default();
        for query in ["discard", "a dscrd", "worktree"] {
            menu.open(&items);
            menu.paste(query, &items);
            assert_eq!(menu.submit_selected(&items), Some(CommandId::Discard));
        }
        let discard = catalog
            .iter()
            .find(|command| command.id == CommandId::Discard)
            .expect("discard is listed");
        assert_eq!(discard.shortcut, "ad");
        assert_eq!(app.update(discard.action.clone()), vec![Effect::Discard(1)]);
        assert!(!app.actions_expanded, "repeating d cannot discard the next path");

        app.extend_commits(vec![row(2, &[1]), row(1, &[])]);
        app.state = State::Complete;
        app.set_worktree_head(Some(id(2)), false);
        app.select_commit(id(1));
        app.changes_focus = Some(ChangePane::Worktree);
        assert!(!app.can_amend(), "an older history selection cannot amend HEAD");
        assert!(has(&commands(&app, &Decorations::default(), false), CommandId::Discard));
        let shortcut = shortcut_action(CommandGroup::Actions, 'd').expect("a d is bound");
        assert_eq!(app.update(shortcut.clone()), vec![Effect::Discard(1)]);

        app.changes_focus = Some(ChangePane::Tree);
        assert!(!has(
            &commands(&app, &Decorations::default(), false),
            CommandId::Discard
        ));
        assert!(
            app.update(shortcut.clone()).is_empty(),
            "tree focus does not discard or delete"
        );
        app.changes_focus = None;
        assert_eq!(
            app.update(shortcut),
            vec![Effect::Delete(id(1))],
            "history retains delete"
        );
        app.changes_focus = Some(ChangePane::Worktree);
        app.set_changes_layout(ChangesLayout::SideBySide, false, false);
        assert!(!app.can_discard(), "a clean worktree has nothing to discard");
        app.set_worktree_changes_available(false);
        assert!(!app.can_discard(), "bare repositories cannot discard worktree files");
    }

    #[test]
    fn auto_merge_actions_follow_selection_and_keep_enrichment_available() -> gix_testtools::Result {
        use crate::edit::auto_merge::{Change, Definition, Input, InputSource, Selection};
        let mut app = App::new(6);
        app.extend_commits(vec![
            row(6, &[4]),
            row(5, &[1]),
            row(4, &[2, 3]),
            row(3, &[1]),
            row(2, &[1]),
            row(1, &[]),
        ]);
        app.state = State::Complete;
        app.set_worktree_head(Some(id(4)), false);
        app.select_commit(id(4));
        app.set_head_edit_availability(true, true, false, false, false, true, true);
        let definition = Definition {
            inputs: vec![
                Input {
                    source: InputSource::Reference("refs/heads/A".try_into()?),
                    commit_id: id(2),
                    muted: false,
                },
                Input {
                    source: InputSource::Change(id(3).into()),
                    commit_id: id(3),
                    muted: false,
                },
            ],
        };
        let mut graph = crate::history::HistoryGraph::from_test_commits(&[
            (id(1), vec![]),
            (id(2), vec![id(1)]),
            (id(3), vec![id(1)]),
            (id(4), vec![id(2), id(3)]),
            (id(5), vec![id(1)]),
            (id(6), vec![id(4)]),
        ]);
        graph.auto_merges.insert(id(4), definition);
        let decorations = Decorations::from([
            (
                id(4),
                vec![Decoration {
                    name: "HEAD".into(),
                    kind: DecorationKind::Head,
                }],
            ),
            (
                id(2),
                vec![Decoration {
                    name: "A".into(),
                    kind: DecorationKind::Local,
                }],
            ),
            (
                id(3),
                vec![Decoration {
                    name: "C".into(),
                    kind: DecorationKind::Local,
                }],
            ),
        ]);
        app.set_auto_merges(&graph, &decorations, &[]);
        app.set_known_merge_descendants(graph.commits_with_merge_descendants());
        let catalog = commands(&app, &decorations, false);
        for available in [
            CommandId::AutoMerge,
            CommandId::Remerge,
            CommandId::RemoveAutoMergeInput,
            CommandId::Todo,
            CommandId::Note,
        ] {
            assert!(
                has(&catalog, available),
                "AutoMerge actions and enrichments are available at its HEAD"
            );
        }
        for unavailable in [
            CommandId::Reword,
            CommandId::Amend,
            CommandId::Spill,
            CommandId::Split,
            CommandId::Squash,
        ] {
            assert!(
                !has(&catalog, unavailable),
                "generated content cannot be edited directly"
            );
        }
        app.select_commit(id(2));
        let catalog = commands(&app, &decorations, false);
        assert!(has(&catalog, CommandId::RemoveFromAutoMerge));
        assert!(
            has(&catalog, CommandId::Reword),
            "an input is editable despite its AutoMerge descendant"
        );
        assert!(
            !has(&catalog, CommandId::AutoMerge),
            "the input is already reachable from HEAD"
        );
        app.select_commit(id(3));
        assert!(app.can_remove_from_auto_merge(), "unnamed change inputs offer removal");
        assert!(!app.can_auto_merge(), "all HEAD parents are already included");
        app.select_commit(id(5));
        let catalog = commands(&app, &decorations, false);
        assert_eq!(
            catalog
                .iter()
                .find(|command| command.id == CommandId::AutoMerge)
                .map(|command| command.label),
            Some("AutoMerge into HEAD"),
            "an unrelated selected commit can be added to HEAD"
        );
        app.select_commit(id(6));
        assert!(!app.can_auto_merge(), "an AutoMerge descendant would introduce a cycle");
        graph.auto_merges.clear();
        app.set_auto_merges(&graph, &decorations, &[]);
        assert!(app.can_auto_merge(), "a descendant of an ordinary HEAD can be merged");
        app.select_commit(id(4));
        assert!(app.can_auto_merge(), "an unnamed ordinary HEAD can create an AutoMerge");
        let options = vec![
            Selection {
                merge_commit_id: id(4),
                change: Change::Remove(InputSource::Reference("refs/heads/A".try_into()?)),
                label: "A · first merge".into(),
            },
            Selection {
                merge_commit_id: id(5),
                change: Change::Remove(InputSource::Reference("refs/heads/A".try_into()?)),
                label: "A · second merge".into(),
            },
        ];
        assert_eq!(
            app.open_auto_merge_picker(options.clone(), " Remove from AutoMerge "),
            None
        );
        let items: Vec<_> = options
            .iter()
            .map(|option| crate::menu::Item::new(&option.label, option.clone()))
            .collect();
        app.auto_merge_picker.paste("scnd", &items);
        assert_eq!(
            app.auto_merge_picker.submit_selected(&items),
            Some(options[1].clone()),
            "the shared picker fuzzy-matches a membership"
        );
        Ok(())
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

        let related = commands(&app, &Decorations::default(), false)
            .into_iter()
            .find(|command| command.id == CommandId::RelatedHistory)
            .expect("completed history offers related targets");
        assert_eq!(related.group, CommandGroup::View);
        assert_eq!(related.shortcut, "vo");
        assert_eq!(related.action, Action::ShowRelatedHistory);
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
    fn network_actions_use_the_second_row_and_the_single_background_slot() {
        let mut app = App::new(1);
        app.state = State::Complete;
        app.set_active_branch(Some("topic".into()));
        #[cfg(feature = "blocking-network-client")]
        app.set_fetch_remote(Some("origin".into()));

        let catalog = commands(&app, &Decorations::default(), false);
        let push = catalog
            .iter()
            .find(|command| command.id == CommandId::Push)
            .expect("a remembered branch can be pushed");
        assert_eq!(push.group, CommandGroup::Actions);
        assert_eq!(push.row, 1);
        assert_eq!(push.label, "Push");
        assert_eq!(push.shortcut, "aP");
        assert_eq!(push.action, Action::Push);
        #[cfg(feature = "blocking-network-client")]
        {
            let fetch = catalog
                .iter()
                .find(|command| command.id == CommandId::Fetch)
                .expect("an active branch can be fetched");
            assert_eq!(fetch.group, CommandGroup::Actions);
            assert_eq!(fetch.row, 1);
            assert_eq!(fetch.label, "Fetch");
            assert_eq!(fetch.shortcut, "aF");
            assert_eq!(fetch.action, Action::Fetch);
        }

        app.start_background_task("pushing topic to origin…");
        assert!(
            !has(&commands(&app, &Decorations::default(), false), CommandId::Push),
            "the single background-task slot hides push while occupied"
        );
        #[cfg(feature = "blocking-network-client")]
        assert!(
            !has(&commands(&app, &Decorations::default(), false), CommandId::Fetch),
            "the single background-task slot hides fetch while occupied"
        );
    }
}
