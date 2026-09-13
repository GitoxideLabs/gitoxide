use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::{
    app::{Action, App, prefix::Direction},
    command_menu::{CommandGroup, CommandId},
};

const HOLD_DELAY: Duration = Duration::from_millis(300);

#[derive(Default)]
pub(crate) struct State {
    gesture: Option<Gesture>,
    // Releases still belong to a gesture after Enter, Escape, or a shortcut ended it.
    pressed: HashSet<CommandGroup>,
    ignore_enter: bool,
    ignore_escape: bool,
}

#[derive(Clone, Copy)]
enum Gesture {
    Pending { group: CommandGroup, started: Instant },
    Held(CommandGroup),
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Outcome {
    Pass,
    Handled,
    Submit(CommandId),
    Action(Action),
}

impl State {
    pub(crate) fn timeout(&self, now: Instant) -> Option<Duration> {
        let Gesture::Pending { started, .. } = self.gesture? else {
            return None;
        };
        Some(HOLD_DELAY.saturating_sub(now.saturating_duration_since(started)))
    }

    /// Promote before drawing so only a command that was actually shown can be submitted.
    pub(crate) fn promote(&mut self, app: &mut App, now: Instant) -> bool {
        let Some(Gesture::Pending { group, started }) = self.gesture else {
            return false;
        };
        if now.saturating_duration_since(started) < HOLD_DELAY {
            return false;
        }
        self.gesture = Some(Gesture::Held(group));
        app.start_held_prefix(group);
        true
    }

    pub(crate) fn cancel(&mut self, app: &mut App) -> bool {
        self.pressed.clear();
        self.end(app)
    }

    fn end(&mut self, app: &mut App) -> bool {
        let had_gesture = self.gesture.take().is_some();
        app.cancel_held_prefix() || had_gesture
    }

    /// `enabled` requires both key-release reporting and ownership of the normal application input.
    pub(crate) fn handle(&mut self, event: &Event, app: &mut App, now: Instant, enabled: bool) -> Outcome {
        if !enabled {
            self.cancel(app);
        }
        if matches!(event, Event::FocusLost) {
            self.pressed.clear();
            self.ignore_enter = false;
            self.ignore_escape = false;
        }
        if let Event::Key(key) = event {
            let ignored = match key.code {
                KeyCode::Enter => Some(&mut self.ignore_enter),
                KeyCode::Esc => Some(&mut self.ignore_escape),
                _ => None,
            };
            if let Some(ignored) = ignored {
                let consume = *ignored;
                if key.kind == KeyEventKind::Release {
                    *ignored = false;
                }
                if consume {
                    return Outcome::Handled;
                }
            }
        }
        if !enabled {
            return Outcome::Pass;
        }
        if let Some(Gesture::Held(group)) = self.gesture
            && app.held_prefix_group() != Some(group)
        {
            self.gesture = None;
        }
        let key = match event {
            Event::Key(key) if !matches!(key.code, KeyCode::Modifier(_)) => key,
            Event::FocusLost => {
                self.cancel(app);
                return Outcome::Pass;
            }
            Event::Mouse(_) | Event::Paste(_) => {
                self.end(app);
                return Outcome::Pass;
            }
            _ => return Outcome::Pass,
        };
        if key.kind == KeyEventKind::Release {
            let Some(group) = released_group(key.code) else {
                return Outcome::Pass;
            };
            let was_pressed = self.pressed.remove(&group);
            match self.gesture {
                Some(Gesture::Pending { group: pending, .. }) if pending == group => {
                    self.gesture = None;
                    return Outcome::Handled;
                }
                Some(Gesture::Held(held)) if held == group => {
                    let selected = app.held_prefix_selection();
                    self.end(app);
                    return selected.map_or(Outcome::Handled, Outcome::Submit);
                }
                _ => {}
            }
            return if was_pressed { Outcome::Handled } else { Outcome::Pass };
        }
        let prefix = prefix_group(*key);
        // Native Windows reports repeated key-down events as Press, rather than Repeat.
        if (key.kind == KeyEventKind::Repeat && prefix.is_some())
            || released_group(key.code).is_some_and(|group| self.pressed.contains(&group))
        {
            return Outcome::Handled;
        }
        if key.kind == KeyEventKind::Press
            && let Some(group) = prefix
        {
            self.end(app);
            self.pressed.insert(group);
            self.gesture = Some(Gesture::Pending { group, started: now });
            return Outcome::Pass;
        }
        match self.gesture {
            Some(Gesture::Pending { .. }) => {
                self.gesture = None;
                Outcome::Pass
            }
            Some(Gesture::Held(_)) => {
                if let Some(direction) = direction(*key) {
                    app.move_held_prefix(direction);
                    return Outcome::Handled;
                }
                match key.code {
                    KeyCode::Enter | KeyCode::Esc if key.kind != KeyEventKind::Press => Outcome::Handled,
                    KeyCode::Enter => {
                        self.ignore_enter = true;
                        let selected = app.held_prefix_selection();
                        self.end(app);
                        selected.map_or(Outcome::Handled, Outcome::Submit)
                    }
                    KeyCode::Esc => {
                        self.ignore_escape = true;
                        self.end(app);
                        Outcome::Handled
                    }
                    _ => {
                        let action = crate::app_action(*key, app);
                        self.end(app);
                        action.map_or(Outcome::Pass, Outcome::Action)
                    }
                }
            }
            None => Outcome::Pass,
        }
    }
}

fn prefix_group(key: KeyEvent) -> Option<CommandGroup> {
    if key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
        return None;
    }
    match key.code {
        KeyCode::Char('a') if !key.modifiers.contains(KeyModifiers::SHIFT) => Some(CommandGroup::Actions),
        KeyCode::Char('v') if !key.modifiers.contains(KeyModifiers::SHIFT) => Some(CommandGroup::View),
        KeyCode::Char('n') if !key.modifiers.contains(KeyModifiers::SHIFT) => Some(CommandGroup::Enrich),
        KeyCode::Char('?') => Some(CommandGroup::Information),
        KeyCode::Char('/') if key.modifiers.contains(KeyModifiers::SHIFT) => Some(CommandGroup::Information),
        _ => None,
    }
}

fn released_group(code: KeyCode) -> Option<CommandGroup> {
    match code {
        KeyCode::Char('a' | 'A') => Some(CommandGroup::Actions),
        KeyCode::Char('v' | 'V') => Some(CommandGroup::View),
        KeyCode::Char('n' | 'N') => Some(CommandGroup::Enrich),
        KeyCode::Char('?' | '/') => Some(CommandGroup::Information),
        _ => None,
    }
}

fn direction(key: KeyEvent) -> Option<Direction> {
    if key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
        return None;
    }
    match key.code {
        KeyCode::Left | KeyCode::Char('h' | 'H') => Some(Direction::Left),
        KeyCode::Right | KeyCode::Char('l' | 'L') => Some(Direction::Right),
        KeyCode::Up | KeyCode::Char('k' | 'K') => Some(Direction::Up),
        KeyCode::Down | KeyCode::Char('j' | 'J') => Some(Direction::Down),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::prefix::Item;

    fn key(code: KeyCode, kind: KeyEventKind) -> Event {
        Event::Key(KeyEvent::new_with_kind(code, KeyModifiers::NONE, kind))
    }

    fn press(input: &mut State, app: &mut App, group: CommandGroup, now: Instant) {
        let key = KeyEvent::new(KeyCode::Char(group.prefix()), KeyModifiers::NONE);
        assert_eq!(input.handle(&Event::Key(key), app, now, true), Outcome::Pass);
        app.update(crate::app_action(key, app).expect("prefixes retain their existing toggle binding"));
    }

    fn show(app: &mut App, commands: &[CommandId]) {
        app.set_held_prefix_layout(
            commands
                .iter()
                .enumerate()
                .map(|(index, id)| Item {
                    id: *id,
                    row: index / 2,
                    start: index % 2 * 20,
                    end: index % 2 * 20 + 10,
                })
                .collect(),
        );
    }

    fn hold(input: &mut State, app: &mut App, group: CommandGroup, commands: &[CommandId], now: Instant) {
        press(input, app, group, now);
        assert!(input.promote(app, now + HOLD_DELAY), "the deadline needs no autorepeat");
        show(app, commands);
    }

    #[test]
    fn tap_preserves_the_toggle_and_unsupported_input_never_arms() {
        let now = Instant::now();
        let mut input = State::default();
        let mut app = App::new(5);
        press(&mut input, &mut app, CommandGroup::View, now);
        assert!(
            app.history_display_expanded,
            "the press opens the existing toggle immediately"
        );
        assert_eq!(input.timeout(now), Some(HOLD_DELAY));
        assert_eq!(
            input.handle(
                &key(KeyCode::Char('v'), KeyEventKind::Release),
                &mut app,
                now + HOLD_DELAY + Duration::from_millis(1),
                true
            ),
            Outcome::Handled,
            "servicing a queued release late must not turn a tap into a hold"
        );
        assert!(app.history_display_expanded, "a short release leaves the toggle open");
        assert_eq!(input.timeout(now), None);
        assert!(!input.promote(&mut app, now + HOLD_DELAY));
        press(&mut input, &mut app, CommandGroup::View, now);
        assert!(!app.history_display_expanded, "the next short press toggles it off");
        input.cancel(&mut app);
        for kind in [KeyEventKind::Press, KeyEventKind::Repeat, KeyEventKind::Release] {
            assert_eq!(
                input.handle(&key(KeyCode::Char('v'), kind), &mut app, now, false),
                Outcome::Pass,
                "unsupported terminals and other input owners retain their key handling"
            );
            assert_eq!(input.timeout(now), None);
        }
    }

    #[test]
    fn every_prefix_promotes_at_the_deadline_and_submits_only_once() {
        for (group, command) in [
            (CommandGroup::Actions, CommandId::Reword),
            (CommandGroup::View, CommandId::Date),
            (CommandGroup::Enrich, CommandId::Todo),
            (CommandGroup::Information, CommandId::Alignment),
        ] {
            let now = Instant::now();
            let mut input = State::default();
            let mut app = App::new(5);
            press(&mut input, &mut app, group, now);
            let before = now + HOLD_DELAY.saturating_sub(Duration::from_millis(1));
            assert!(!input.promote(&mut app, before), "a short press remains a toggle");
            assert_eq!(input.timeout(before), Some(Duration::from_millis(1)));
            for kind in [KeyEventKind::Repeat, KeyEventKind::Press] {
                assert_eq!(
                    input.handle(&key(KeyCode::Char(group.prefix()), kind), &mut app, before, true),
                    Outcome::Handled,
                    "autorepeat, including native Windows key-downs, cannot reset the deadline"
                );
            }
            assert!(input.promote(&mut app, now + HOLD_DELAY));
            assert_eq!(input.timeout(now + HOLD_DELAY), None);
            assert_eq!(app.held_prefix_group(), Some(group));
            assert_eq!(app.held_prefix_selection(), None, "undrawn commands cannot execute");
            show(&mut app, &[command]);
            let release = key(KeyCode::Char(group.prefix()), KeyEventKind::Release);
            assert_eq!(
                input.handle(&release, &mut app, now + HOLD_DELAY, true),
                Outcome::Submit(command)
            );
            assert_eq!(
                app.held_prefix_group(),
                None,
                "submission closes the held group before dispatch"
            );
            assert_eq!(input.handle(&release, &mut app, now + HOLD_DELAY, true), Outcome::Pass);
        }
    }

    #[test]
    fn quick_shortcuts_disarm_holding_and_modifier_events_do_not() {
        let now = Instant::now();
        let mut input = State::default();
        let mut app = App::new(5);
        press(&mut input, &mut app, CommandGroup::Actions, now);
        let modifier = Event::Key(KeyEvent::new(
            KeyCode::Modifier(crossterm::event::ModifierKeyCode::LeftShift),
            KeyModifiers::SHIFT,
        ));
        assert_eq!(input.handle(&modifier, &mut app, now, true), Outcome::Pass);
        assert_eq!(
            input.timeout(now),
            Some(HOLD_DELAY),
            "a modifier alone keeps the gesture pending"
        );
        let shortcut = KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE);
        assert_eq!(input.handle(&Event::Key(shortcut), &mut app, now, true), Outcome::Pass);
        assert_eq!(crate::app_action(shortcut, &app), Some(Action::Reword));
        assert!(
            !input.promote(&mut app, now + HOLD_DELAY),
            "quick shortcuts cannot leave a hold armed"
        );
        assert_eq!(
            input.handle(&key(KeyCode::Char('a'), KeyEventKind::Repeat), &mut app, now, true),
            Outcome::Handled
        );
        assert_eq!(
            input.handle(&key(KeyCode::Char('a'), KeyEventKind::Release), &mut app, now, true),
            Outcome::Handled
        );
    }

    #[test]
    fn held_navigation_overrides_shortcuts_but_other_actions_keep_their_binding() {
        let now = Instant::now();
        let mut input = State::default();
        let mut app = App::new(5);
        hold(
            &mut input,
            &mut app,
            CommandGroup::Actions,
            &[
                CommandId::Reword,
                CommandId::NewCommit,
                CommandId::Amend,
                CommandId::Spill,
            ],
            now,
        );
        for (code, expected) in [
            (KeyCode::Char('l'), CommandId::NewCommit),
            (KeyCode::Char('J'), CommandId::Spill),
            (KeyCode::Left, CommandId::Amend),
            (KeyCode::Char('K'), CommandId::Reword),
        ] {
            assert_eq!(
                input.handle(&key(code, KeyEventKind::Press), &mut app, now, true),
                Outcome::Handled
            );
            assert_eq!(
                app.held_prefix_selection(),
                Some(expected),
                "held keys navigate the displayed verbs"
            );
        }
        assert_eq!(
            input.handle(&key(KeyCode::Char('o'), KeyEventKind::Press), &mut app, now, true),
            Outcome::Action(Action::Reword),
            "direct shortcuts are decoded before the held group closes"
        );
        assert_eq!(app.held_prefix_group(), None);
        assert_eq!(
            input.handle(&key(KeyCode::Char('a'), KeyEventKind::Release), &mut app, now, true),
            Outcome::Handled
        );
        assert_eq!(
            direction(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL)),
            None
        );
        assert_eq!(direction(KeyEvent::new(KeyCode::Left, KeyModifiers::ALT)), None);
    }

    #[test]
    fn information_release_matches_after_shift_is_released() {
        let now = Instant::now();
        let mut input = State::default();
        let mut app = App::new(5);
        let prefix = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::SHIFT);
        assert_eq!(input.handle(&Event::Key(prefix), &mut app, now, true), Outcome::Pass);
        app.update(crate::app_action(prefix, &app).expect("Shift-/ opens Information"));
        assert!(input.promote(&mut app, now + HOLD_DELAY));
        show(&mut app, &[CommandId::Alignment, CommandId::RefTree]);
        let navigation = Event::Key(KeyEvent::new(KeyCode::Char('L'), KeyModifiers::SHIFT));
        assert_eq!(input.handle(&navigation, &mut app, now, true), Outcome::Handled);
        assert_eq!(app.held_prefix_selection(), Some(CommandId::RefTree));
        for kind in [KeyEventKind::Repeat, KeyEventKind::Press] {
            assert_eq!(
                input.handle(&key(KeyCode::Char('/'), kind), &mut app, now, true),
                Outcome::Handled,
                "dropping Shift cannot turn a held Information prefix into repeated search keys"
            );
        }
        assert_eq!(
            input.handle(&key(KeyCode::Char('/'), KeyEventKind::Release), &mut app, now, true),
            Outcome::Submit(CommandId::RefTree)
        );
    }

    #[test]
    fn submit_and_cancel_swallow_their_remaining_key_cycle_across_input_handoffs() {
        for closing in [KeyCode::Enter, KeyCode::Esc] {
            for release_prefix_first in [false, true] {
                let now = Instant::now();
                let mut input = State::default();
                let mut app = App::new(5);
                hold(&mut input, &mut app, CommandGroup::View, &[CommandId::Date], now);
                assert_eq!(
                    input.handle(&key(closing, KeyEventKind::Press), &mut app, now, true),
                    if closing == KeyCode::Enter {
                        Outcome::Submit(CommandId::Date)
                    } else {
                        Outcome::Handled
                    }
                );
                let release = key(KeyCode::Char('v'), KeyEventKind::Release);
                for kind in [KeyEventKind::Repeat, KeyEventKind::Press] {
                    assert_eq!(
                        input.handle(&key(KeyCode::Char('v'), kind), &mut app, now, true),
                        Outcome::Handled,
                        "the ended gesture cannot restart while its prefix is still pressed"
                    );
                }
                if release_prefix_first {
                    assert_eq!(input.handle(&release, &mut app, now, true), Outcome::Handled);
                }
                for kind in [KeyEventKind::Press, KeyEventKind::Repeat, KeyEventKind::Release] {
                    assert_eq!(
                        input.handle(&key(closing, kind), &mut app, now, false),
                        Outcome::Handled,
                        "the closing key cannot act on an editor or overlay entered by its press"
                    );
                }
                if !release_prefix_first {
                    assert_eq!(
                        input.handle(&release, &mut app, now, true),
                        Outcome::Pass,
                        "input handoff forgets the prefix key in case its release happens outside tix"
                    );
                }
                assert_eq!(
                    input.handle(&key(closing, KeyEventKind::Press), &mut app, now, false),
                    Outcome::Pass,
                    "a later deliberate press belongs to the new input owner"
                );
                assert_eq!(app.held_prefix_group(), None);
            }
        }
    }

    #[test]
    fn ownership_changes_allow_a_new_prefix_press_when_the_old_release_was_missed() {
        let now = Instant::now();
        for event in [None, Some(Event::FocusLost), Some(Event::FocusGained)] {
            let mut input = State::default();
            let mut app = App::new(5);
            hold(&mut input, &mut app, CommandGroup::View, &[CommandId::Date], now);
            match event {
                Some(event) => {
                    input.handle(&event, &mut app, now, false);
                }
                None => {
                    input.cancel(&mut app);
                }
            }
            press(&mut input, &mut app, CommandGroup::View, now + HOLD_DELAY);
            assert_eq!(input.timeout(now + HOLD_DELAY), Some(HOLD_DELAY));
        }
    }

    #[test]
    fn focus_loss_handoff_and_missing_display_never_execute_the_selection() {
        let now = Instant::now();
        for cancel in 0..5 {
            let mut input = State::default();
            let mut app = App::new(5);
            hold(&mut input, &mut app, CommandGroup::View, &[CommandId::Date], now);
            match cancel {
                0 => {
                    input.handle(&Event::FocusLost, &mut app, now, true);
                }
                1 => {
                    input.handle(&Event::FocusGained, &mut app, now, false);
                }
                2 => {
                    input.cancel(&mut app);
                }
                3 => {
                    show(&mut app, &[]);
                }
                4 => {
                    show(&mut app, &[CommandId::Ids]);
                }
                _ => unreachable!("all cancellation cases are covered"),
            }
            assert!(
                !matches!(
                    input.handle(&key(KeyCode::Char('v'), KeyEventKind::Release), &mut app, now, true),
                    Outcome::Submit(_)
                ),
                "cancelled or unavailable selections never execute"
            );
            assert_eq!(app.held_prefix_group(), None);
        }
        let mut input = State::default();
        let mut app = App::new(5);
        press(&mut input, &mut app, CommandGroup::View, now);
        assert!(input.promote(&mut app, now + HOLD_DELAY));
        assert_eq!(
            input.handle(&key(KeyCode::Char('v'), KeyEventKind::Release), &mut app, now, true),
            Outcome::Handled,
            "a popup that has not been rendered cannot execute"
        );
    }
}
