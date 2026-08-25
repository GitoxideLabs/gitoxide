pub(crate) const MAX_VISIBLE_ROWS: usize = 9;

#[derive(Debug)]
pub(crate) struct Item<'a, T> {
    pub(crate) label: &'a str,
    search_prefix: Option<&'a str>,
    scope: Option<char>,
    pub(crate) value: T,
}

impl<'a, T> Item<'a, T> {
    pub(crate) fn new(label: &'a str, value: T) -> Self {
        Item {
            label,
            search_prefix: None,
            scope: None,
            value,
        }
    }

    pub(crate) fn with_search_prefix(label: &'a str, search_prefix: &'a str, scope: char, value: T) -> Self {
        let mut item = Item::new(label, value);
        item.search_prefix = Some(search_prefix);
        item.scope = Some(scope);
        item
    }
}

#[derive(Debug)]
pub(crate) struct Menu<T> {
    open: bool,
    query: String,
    cursor: usize,
    matches: Vec<usize>,
    selection: Option<usize>,
    selected_value: Option<T>,
    window: usize,
    visible_rows: usize,
    last_submitted: Option<T>,
}

impl<T> Default for Menu<T> {
    fn default() -> Self {
        Menu {
            open: false,
            query: String::new(),
            cursor: 0,
            matches: Vec::new(),
            selection: None,
            selected_value: None,
            window: 0,
            visible_rows: MAX_VISIBLE_ROWS,
            last_submitted: None,
        }
    }
}

impl<T: Clone + Eq> Menu<T> {
    pub(crate) fn open(&mut self, items: &[Item<'_, T>]) {
        let selected = self.last_submitted.clone();
        self.open_selected(items, selected.as_ref());
    }

    pub(crate) fn open_selected(&mut self, items: &[Item<'_, T>], selected: Option<&T>) {
        self.open = true;
        self.query.clear();
        self.cursor = 0;
        self.matches = (0..items.len()).collect();
        self.selection = selected.and_then(|selected| items.iter().position(|item| &item.value == selected));
        self.selected_value = self.selection.map(|selection| items[selection].value.clone());
        self.window = 0;
        self.keep_selection_visible();
    }

    pub(crate) fn close(&mut self) {
        self.open = false;
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) fn query(&self) -> &str {
        &self.query
    }

    /// The cursor position in Unicode scalar values, not bytes.
    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn visible_indices(&self) -> &[usize] {
        let end = (self.window + self.visible_rows).min(self.matches.len());
        &self.matches[self.window..end]
    }

    pub(crate) fn matching_indices(&self) -> &[usize] {
        &self.matches
    }

    pub(crate) fn set_visible_rows(&mut self, rows: usize) {
        self.visible_rows = rows.min(MAX_VISIBLE_ROWS);
        self.keep_selection_visible();
    }

    pub(crate) fn selected_index(&self) -> Option<usize> {
        self.selection
            .and_then(|selection| self.matches.get(selection).copied())
    }

    pub(crate) fn selected_match(&self) -> Option<usize> {
        self.selection
    }

    pub(crate) fn selected_visible_row(&self) -> Option<usize> {
        self.selection
            .filter(|selection| *selection >= self.window)
            .map(|selection| selection - self.window)
            .filter(|row| *row < self.visible_rows)
    }

    /// Re-filters after the caller replaces the dynamically supplied items.
    pub(crate) fn sync(&mut self, items: &[Item<'_, T>]) {
        let selected_value = self.selected_value.clone();
        self.refilter(items);
        self.selection = selected_value
            .as_ref()
            .and_then(|value| self.matches.iter().position(|index| items[*index].value == *value))
            .or_else(|| (!self.query.is_empty() && !self.matches.is_empty()).then_some(0));
        self.selected_value = self
            .selection
            .map(|selection| items[self.matches[selection]].value.clone());
        if self.selection.is_none() {
            self.window = 0;
        }
        self.keep_selection_visible();
    }

    pub(crate) fn insert(&mut self, ch: char, items: &[Item<'_, T>]) {
        if ch.is_control() {
            return;
        }
        self.query.insert(self.cursor_byte(), ch);
        self.cursor += 1;
        self.after_edit(items);
    }

    pub(crate) fn paste(&mut self, text: &str, items: &[Item<'_, T>]) {
        let text: String = text.chars().filter(|ch| !ch.is_control()).collect();
        if text.is_empty() {
            return;
        }
        self.query.insert_str(self.cursor_byte(), &text);
        self.cursor += text.chars().count();
        self.after_edit(items);
    }

    pub(crate) fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub(crate) fn right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.query.chars().count());
    }

    pub(crate) fn home(&mut self) {
        self.cursor = 0;
    }

    pub(crate) fn end(&mut self) {
        self.cursor = self.query.chars().count();
    }

    pub(crate) fn backspace(&mut self, items: &[Item<'_, T>]) {
        if self.cursor == 0 {
            return;
        }
        let start = byte_at(&self.query, self.cursor - 1);
        let end = byte_at(&self.query, self.cursor);
        self.query.replace_range(start..end, "");
        self.cursor -= 1;
        self.after_edit(items);
    }

    pub(crate) fn delete(&mut self, items: &[Item<'_, T>]) {
        if self.cursor == self.query.chars().count() {
            return;
        }
        let start = self.cursor_byte();
        let end = byte_at(&self.query, self.cursor + 1);
        self.query.replace_range(start..end, "");
        self.after_edit(items);
    }

    pub(crate) fn up(&mut self, items: &[Item<'_, T>]) {
        self.up_by(1, items);
    }

    pub(crate) fn up_by(&mut self, amount: usize, items: &[Item<'_, T>]) {
        self.sync(items);
        self.selection = match self.selection {
            Some(selection) => Some(selection.saturating_sub(amount)),
            None if !self.matches.is_empty() => Some(self.matches.len() - 1),
            None => None,
        };
        self.remember_selection(items);
        self.keep_selection_visible();
    }

    pub(crate) fn down(&mut self, items: &[Item<'_, T>]) {
        self.down_by(1, items);
    }

    pub(crate) fn down_by(&mut self, amount: usize, items: &[Item<'_, T>]) {
        self.sync(items);
        self.selection = match self.selection {
            Some(selection) => Some(
                selection
                    .saturating_add(amount)
                    .min(self.matches.len().saturating_sub(1)),
            ),
            None if !self.matches.is_empty() => Some(0),
            None => None,
        };
        self.remember_selection(items);
        self.keep_selection_visible();
    }

    pub(crate) fn submit_selected(&mut self, items: &[Item<'_, T>]) -> Option<T> {
        self.sync(items);
        let row = self.selected_visible_row()?;
        let item = *self.visible_indices().get(row)?;
        self.submit(&items[item])
    }

    pub(crate) fn submit_digit(&mut self, digit: char, items: &[Item<'_, T>]) -> Option<T> {
        let row = digit.to_digit(10)?.checked_sub(1)? as usize;
        if row >= self.visible_rows {
            return None;
        }
        self.sync(items);
        let item = *self.matches.get(self.window + row)?;
        self.submit(&items[item])
    }

    fn submit(&mut self, item: &Item<'_, T>) -> Option<T> {
        self.last_submitted = Some(item.value.clone());
        self.open = false;
        self.last_submitted.clone()
    }

    fn after_edit(&mut self, items: &[Item<'_, T>]) {
        self.refilter(items);
        self.selection = (!self.query.is_empty() && !self.matches.is_empty()).then_some(0);
        self.remember_selection(items);
        self.window = 0;
    }

    fn refilter(&mut self, items: &[Item<'_, T>]) {
        let scope = scoped_query(&self.query);
        self.matches = items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                let matches = match scope {
                    Some((scope, query)) => item.scope.is_some_and(|item_scope| {
                        item_scope.eq_ignore_ascii_case(&scope) && fuzzy_match(query, item.label)
                    }),
                    None => {
                        fuzzy_match(&self.query, item.label)
                            || item
                                .search_prefix
                                .is_some_and(|prefix| fuzzy_match(&self.query, prefix))
                    }
                };
                matches.then_some(idx)
            })
            .collect();
    }

    fn keep_selection_visible(&mut self) {
        let Some(selection) = self.selection else {
            self.window = self.window.min(self.matches.len().saturating_sub(self.visible_rows));
            return;
        };
        if selection < self.window {
            self.window = selection;
        } else if selection >= self.window + self.visible_rows {
            self.window = selection + 1 - self.visible_rows;
        }
        self.window = self.window.min(self.matches.len().saturating_sub(self.visible_rows));
    }

    fn remember_selection(&mut self, items: &[Item<'_, T>]) {
        self.selected_value = self
            .selection
            .map(|selection| items[self.matches[selection]].value.clone());
    }

    fn cursor_byte(&self) -> usize {
        byte_at(&self.query, self.cursor)
    }
}

fn byte_at(input: &str, char_index: usize) -> usize {
    input
        .char_indices()
        .nth(char_index)
        .map_or(input.len(), |(byte, _)| byte)
}

fn fuzzy_match(query: &str, candidate: &str) -> bool {
    let query = query.to_lowercase();
    let candidate = candidate.to_lowercase();
    let mut candidate = candidate.chars();
    query.chars().all(|needle| candidate.by_ref().any(|hay| hay == needle))
}

fn scoped_query(query: &str) -> Option<(char, &str)> {
    let mut chars = query.chars();
    let scope = chars.next()?;
    (chars.next() == Some(' ')).then_some((scope, chars.as_str()))
}

#[cfg(test)]
mod tests {
    use super::{Item, Menu};

    fn item<T>(label: &str, value: T) -> Item<'_, T> {
        Item::new(label, value)
    }

    #[test]
    fn opening_and_recalling_use_exact_stable_values() {
        let items = [item("Stash", "stash"), item("Unstash", "unstash")];
        let mut menu = Menu::default();

        menu.open(&items);
        assert_eq!(menu.selected_index(), None, "the first open has no default");
        menu.close();
        assert!(!menu.is_open());
        menu.open(&items);
        assert_eq!(menu.submit_digit('2', &items), Some("unstash"));
        assert!(!menu.is_open());

        menu.open(&items);
        assert_eq!(menu.selected_index(), Some(1), "the exact command is recalled");
        let changed = [item("Stash", "stash")];
        menu.open(&changed);
        assert_eq!(menu.selected_index(), None, "an unavailable command is not recalled");
        assert_eq!(menu.last_submitted.as_ref(), Some(&"unstash"));
    }

    #[test]
    fn editing_fuzzy_filters_in_order_and_clearing_removes_the_default() {
        let items = [
            item("Git Commit", 1),
            item("Go To Child", 2),
            item("Commit Info", 3),
            item("Ångström", 4),
        ];
        let mut menu = Menu::default();
        menu.open(&items);

        menu.insert('G', &items);
        menu.insert('c', &items);
        assert_eq!(menu.query(), "Gc");
        assert_eq!(menu.visible_indices(), &[0, 1]);
        assert_eq!(menu.selected_index(), Some(0), "typing selects the first match");

        menu.home();
        menu.delete(&items);
        menu.delete(&items);
        assert_eq!(menu.query(), "");
        assert_eq!(menu.visible_indices(), &[0, 1, 2, 3]);
        assert_eq!(menu.selected_index(), None, "an edited-empty query has no default");

        menu.insert('å', &items);
        menu.insert('S', &items);
        assert_eq!(menu.visible_indices(), &[3], "matching is Unicode case-insensitive");
    }

    #[test]
    fn filtering_matches_search_prefixes() {
        let items = [
            Item::with_search_prefix("Date", "View", 'v', 1),
            Item::with_search_prefix("Reword", "Actions", 'a', 2),
            Item::with_search_prefix("Squash", "Actions", 'a', 3),
        ];
        let mut menu = Menu::default();
        menu.open(&items);
        menu.paste("actions", &items);

        assert_eq!(menu.visible_indices(), &[1, 2]);
    }

    #[test]
    fn a_prefix_and_space_scopes_the_query_until_the_space_is_removed() {
        let items = [
            Item::with_search_prefix("Date", "View", 'v', 1),
            Item::with_search_prefix("Reword", "Actions", 'a', 2),
            Item::with_search_prefix("Squash", "Actions", 'a', 3),
        ];
        let mut menu = Menu::default();
        menu.open(&items);

        menu.paste("A ", &items);
        assert_eq!(menu.query(), "A ", "the literal scoped query remains visible");
        assert_eq!(menu.visible_indices(), &[1, 2], "an empty suffix shows the whole group");
        menu.paste("sq", &items);
        assert_eq!(
            menu.visible_indices(),
            &[2],
            "the suffix fuzzy-filters labels in the group"
        );

        menu.backspace(&items);
        menu.backspace(&items);
        menu.backspace(&items);
        assert_eq!(menu.query(), "A");
        assert_eq!(
            menu.visible_indices(),
            &[0, 1, 2],
            "removing the space restores ordinary label and group search"
        );

        menu.open(&items);
        menu.paste("x ", &items);
        assert!(menu.visible_indices().is_empty(), "an unknown scope has no matches");

        menu.open(&items[..1]);
        menu.paste("a ", &items[..1]);
        assert!(menu.visible_indices().is_empty(), "an unavailable scope has no matches");
    }

    #[test]
    fn unicode_editing_and_paste_never_split_characters() {
        let items = [item("aβ🙂c", ())];
        let mut menu = Menu::default();
        menu.open(&items);

        menu.paste("aβ\r\n\t\u{7}🙂c", &items);
        assert_eq!(menu.query(), "aβ🙂c", "paste ignores control characters");
        assert_eq!(menu.cursor(), 4);
        menu.left();
        menu.left();
        menu.delete(&items);
        assert_eq!(menu.query(), "aβc");
        menu.backspace(&items);
        assert_eq!(menu.query(), "ac");
        assert_eq!(menu.cursor(), 1);
        menu.home();
        menu.left();
        assert_eq!(menu.cursor(), 0, "left clamps at the start");
        menu.end();
        menu.right();
        assert_eq!(menu.cursor(), 2, "right clamps at the end");
    }

    #[test]
    fn navigation_window_digits_and_dynamic_clamping_work_together() {
        let labels: Vec<_> = (0..12).map(|idx| format!("Command {idx}")).collect();
        let items: Vec<_> = labels.iter().enumerate().map(|(idx, label)| item(label, idx)).collect();
        let mut menu = Menu::default();
        menu.open(&items);
        assert_eq!(menu.visible_indices(), &[0, 1, 2, 3, 4, 5, 6, 7, 8]);

        menu.up(&items);
        assert_eq!(menu.selected_index(), Some(11));
        assert_eq!(menu.selected_visible_row(), Some(8));
        assert_eq!(menu.visible_indices(), &[3, 4, 5, 6, 7, 8, 9, 10, 11]);
        menu.down(&items);
        assert_eq!(menu.selected_index(), Some(11), "down clamps at the end");
        assert_eq!(menu.submit_digit('1', &items), Some(3));

        menu.open(&items);
        assert_eq!(menu.selected_index(), Some(3));
        assert_eq!(menu.submit_selected(&items), Some(3));
        menu.open(&items);
        let shorter = &items[..2];
        menu.sync(shorter);
        assert_eq!(menu.selected_index(), None, "a vanished command is never substituted");
        assert_eq!(
            menu.visible_indices(),
            &[0, 1],
            "a vanished selection resets the window"
        );
        assert_eq!(menu.submit_digit('0', shorter), None);

        let mut scrolled = Menu::default();
        scrolled.open(&items);
        scrolled.up(&items);
        scrolled.sync(shorter);
        assert_eq!(scrolled.selected_index(), None);
        assert_eq!(
            scrolled.visible_indices(),
            &[0, 1],
            "a vanished tail selection returns to the start"
        );
    }
}
