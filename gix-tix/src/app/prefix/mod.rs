use crate::command_menu::{CommandGroup, CommandId};

use super::App;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// A fully displayed command's position within the wrapped prefix popup.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Item {
    pub id: CommandId,
    pub row: usize,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug)]
pub(super) struct Held {
    group: CommandGroup,
    selected: Option<CommandId>,
    items: Vec<Item>,
}

impl App {
    pub(crate) fn start_held_prefix(&mut self, group: CommandGroup) {
        self.close_shortcut_groups();
        match group {
            CommandGroup::View => self.history_display_expanded = true,
            CommandGroup::Actions => self.actions_expanded = true,
            CommandGroup::Enrich => self.enrich_expanded = true,
            CommandGroup::Information => self.information_expanded = true,
        }
        self.held_prefix = Some(Held {
            group,
            selected: None,
            items: Vec::new(),
        });
    }

    pub(crate) fn cancel_held_prefix(&mut self) -> bool {
        if self.held_prefix.is_none() {
            return false;
        }
        self.close_shortcut_groups();
        true
    }

    pub(crate) fn held_prefix_group(&self) -> Option<CommandGroup> {
        self.held_prefix.as_ref().map(|held| held.group)
    }

    pub(crate) fn held_prefix_selection(&self) -> Option<CommandId> {
        self.held_prefix.as_ref().and_then(|held| held.selected)
    }

    /// Reconcile against the current popup layout, cancelling if the selected command disappeared.
    pub(crate) fn set_held_prefix_layout(&mut self, items: Vec<Item>) -> Option<CommandId> {
        let held = self.held_prefix.as_mut()?;
        let selected = match held.selected {
            Some(selected) => items.iter().find(|item| item.id == selected),
            None => items.first(),
        };
        let Some(selected) = selected.map(|item| item.id) else {
            self.cancel_held_prefix();
            return None;
        };
        held.selected = Some(selected);
        held.items = items;
        Some(selected)
    }

    pub(crate) fn move_held_prefix(&mut self, direction: Direction) {
        let Some(held) = self.held_prefix.as_mut() else {
            return;
        };
        let Some(current) = held.items.iter().find(|item| Some(item.id) == held.selected) else {
            return;
        };
        let next = match direction {
            Direction::Left => held
                .items
                .iter()
                .filter(|item| item.row == current.row && item.start < current.start)
                .max_by_key(|item| item.start),
            Direction::Right => held
                .items
                .iter()
                .filter(|item| item.row == current.row && item.start > current.start)
                .min_by_key(|item| item.start),
            Direction::Up | Direction::Down => {
                let row = if direction == Direction::Up {
                    held.items
                        .iter()
                        .filter(|item| item.row < current.row)
                        .map(|item| item.row)
                        .max()
                } else {
                    held.items
                        .iter()
                        .filter(|item| item.row > current.row)
                        .map(|item| item.row)
                        .min()
                };
                held.items
                    .iter()
                    .filter(|item| Some(item.row) == row)
                    .min_by_key(|item| {
                        (
                            (item.start + item.end).abs_diff(current.start + current.end),
                            item.start,
                        )
                    })
            }
        };
        if let Some(next) = next {
            held.selected = Some(next.id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_stops_at_edges_skips_empty_rows_and_breaks_center_ties_to_the_left() {
        let mut app = App::new(1);
        app.history_display_expanded = true;
        assert!(
            !app.cancel_held_prefix(),
            "cancelling a pending hold preserves an ordinary toggle"
        );
        assert!(app.history_display_expanded, "the toggled prefix stays open");
        app.start_held_prefix(CommandGroup::Actions);
        assert!(
            !app.history_display_expanded,
            "starting a hold closes the previous prefix"
        );
        app.set_held_prefix_layout(vec![
            Item {
                id: CommandId::NewCommit,
                row: 0,
                start: 5,
                end: 10,
            },
            Item {
                id: CommandId::NewEmptyCommit,
                row: 2,
                start: 0,
                end: 5,
            },
            Item {
                id: CommandId::Reword,
                row: 2,
                start: 10,
                end: 15,
            },
        ]);
        for direction in [Direction::Up, Direction::Left, Direction::Right] {
            app.move_held_prefix(direction);
            assert_eq!(
                app.held_prefix_selection(),
                Some(CommandId::NewCommit),
                "navigation does not wrap at an edge"
            );
        }
        app.move_held_prefix(Direction::Down);
        assert_eq!(
            app.held_prefix_selection(),
            Some(CommandId::NewEmptyCommit),
            "skip informational rows and choose the left command on a center tie"
        );
        app.move_held_prefix(Direction::Right);
        assert_eq!(
            app.held_prefix_selection(),
            Some(CommandId::Reword),
            "move within the destination row"
        );
        app.move_held_prefix(Direction::Down);
        assert_eq!(
            app.held_prefix_selection(),
            Some(CommandId::Reword),
            "down stops at the last selectable row"
        );
        app.close_shortcut_groups();
        assert_eq!(
            app.held_prefix_group(),
            None,
            "closing shortcut groups clears the held gesture too"
        );
        assert_eq!(
            app.held_prefix_selection(),
            None,
            "closed groups cannot leave an executable selection"
        );
    }
}
