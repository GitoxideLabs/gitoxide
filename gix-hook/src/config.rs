use crate::is_known_event;
use bstr::{BString, ByteSlice};
use gix_error::{OptionExt, Result, ResultExt, ensure, validation};
use std::collections::BTreeMap;

#[derive(Default)]
struct ConfiguredHook {
    command: Option<BString>,
    events: BTreeMap<BString, usize>,
    disabled: bool,
    has_parallel: bool,
}

/// Resolve enabled `hook.<name>.event` commands for `event` in last-registration order.
///
/// Only sections accepted by `section_filter` are inspected. An empty event clears that hook's
/// earlier registrations; the last command and enabled values win. Every enabled registration must
/// have a command, even for other events. Known event names cannot be used as configured hook names.
///
/// Returns `(friendly name, command)` pairs, or `None` for an event-wide disable. The latter also
/// disables the traditional hook when passed to [`crate::run_event_serially()`]. Parallel settings
/// are parsed to distinguish friendly names but do not change serial execution.
pub fn configured_commands(
    config: &gix_config::File,
    mut section_filter: impl FnMut(&gix_config::file::Metadata) -> bool,
    event: &str,
) -> Result<Option<Vec<(BString, BString)>>> {
    crate::validate_name(event)?;
    let mut hooks = BTreeMap::<BString, ConfiguredHook>::new();
    let mut position = 0;
    for section in config.sections() {
        if !section.header().name().eq_ignore_ascii_case(b"hook") || !section_filter(section.meta()) {
            continue;
        }
        let Some(name) = section.header().subsection_name() else {
            continue;
        };
        for (key, value) in section.body().into_iter_implicit() {
            let names_a_hook = key.eq_ignore_ascii_case("command")
                || key.eq_ignore_ascii_case("parallel")
                || (key.eq_ignore_ascii_case("event") && value.as_ref().is_some_and(|value| !value.is_empty()));
            if names_a_hook {
                ensure!(
                    !is_known_event(name),
                    validation(format!("Hook name {name:?} collides with a known event name"))
                );
            }
            if key.eq_ignore_ascii_case("event") {
                let value = value.ok_or_raise(|| validation(format!("hook.{name}.event requires a value")))?;
                let hook = hooks.entry(name.to_owned()).or_default();
                if value.is_empty() {
                    hook.events.clear();
                } else {
                    hook.events.insert(value, position);
                    position += 1;
                }
            } else if key.eq_ignore_ascii_case("command") {
                let value = value.ok_or_raise(|| validation(format!("hook.{name}.command requires a value")))?;
                hooks.entry(name.to_owned()).or_default().command = Some(value);
            } else if key.eq_ignore_ascii_case("enabled") || key.eq_ignore_ascii_case("parallel") {
                let enabled = match value {
                    Some(value) => {
                        gix_config::Boolean::try_from(value.as_bstr())
                            .or_raise(|| validation(format!("hook.{name}.{key} must be a boolean")))?
                            .0
                    }
                    None => true,
                };
                let hook = hooks.entry(name.to_owned()).or_default();
                if key.eq_ignore_ascii_case("enabled") {
                    hook.disabled = !enabled;
                } else {
                    // Parallel execution is deliberately disabled, but this key identifies the friendly-name namespace.
                    hook.has_parallel = true;
                }
            }
        }
    }
    let event_disabled = hooks
        .get(event.as_bytes().as_bstr())
        .is_some_and(|hook| hook.disabled && hook.command.is_none() && hook.events.is_empty() && !hook.has_parallel);
    let mut commands = Vec::new();
    for (name, hook) in hooks {
        if hook.disabled || hook.events.is_empty() {
            continue;
        }
        let command = hook.command.ok_or_raise(|| {
            validation(format!(
                "hook.{name}.command must be configured while events are registered"
            ))
        })?;
        if let Some(position) = hook.events.get(event.as_bytes().as_bstr()) {
            commands.push((*position, name, command));
        }
    }
    if event_disabled {
        return Ok(None);
    }
    commands.sort_by_key(|(position, _, _)| *position);
    Ok(Some(
        commands.into_iter().map(|(_, name, command)| (name, command)).collect(),
    ))
}
