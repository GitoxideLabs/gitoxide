use std::io;

use crate::{
    Repository, Result,
    bstr::{BString, ByteSlice},
    error::{ResultExt, bail, message},
    push::UpdateOutcome,
};

#[derive(Clone, Copy)]
enum Action {
    Ignore,
    Warn,
    Refuse,
    UpdateInstead,
}

pub(super) struct Policy {
    pub advertise_atomic: bool,
    pub advertise_push_options: bool,
    pub deny_non_fast_forwards: bool,
    pub allow_shallow_updates: bool,
    deny_deletes: bool,
    deny_current: Action,
    deny_delete_current: Action,
    hidden: Vec<BString>,
}

impl Policy {
    pub fn new(repo: &Repository) -> Result<Self> {
        let config = repo.config_snapshot();
        let mut hidden = Vec::new();
        for section in config.plumbing().sections() {
            if section.header().subsection_name().is_none()
                && (section.header().name().eq_ignore_ascii_case(b"receive")
                    || section.header().name().eq_ignore_ascii_case(b"transfer"))
            {
                let values = section.values("hideRefs");
                if values.len()
                    != section
                        .value_names()
                        .filter(|name| name.eq_ignore_ascii_case("hideRefs"))
                        .count()
                {
                    bail!(message("hideRefs configuration requires an explicit value").validation());
                }
                hidden.extend(values);
            }
        }
        Ok(Self {
            advertise_atomic: config.try_boolean("receive.advertiseAtomic")?.unwrap_or(true),
            advertise_push_options: config.try_boolean("receive.advertisePushOptions")?.unwrap_or(false),
            deny_non_fast_forwards: config.try_boolean("receive.denyNonFastForwards")?.unwrap_or(false),
            allow_shallow_updates: config.try_boolean("receive.shallowUpdate")?.unwrap_or(false),
            deny_deletes: config.try_boolean("receive.denyDeletes")?.unwrap_or(false),
            deny_current: action(repo, "receive.denyCurrentBranch")?,
            deny_delete_current: action(repo, "receive.denyDeleteCurrent")?,
            hidden,
        })
    }

    pub fn hidden(&self, name: &[u8]) -> bool {
        for rule in self.hidden.iter().rev() {
            let rule = rule.trim_end_with(|byte| byte == '/');
            let (negated, rule) = rule.strip_prefix(b"!").map_or((false, rule), |rule| (true, rule));
            let rule = rule.strip_prefix(b"^").unwrap_or(rule);
            if name
                .strip_prefix(rule)
                .is_some_and(|suffix| suffix.is_empty() || suffix.starts_with(b"/"))
            {
                return !negated;
            }
        }
        false
    }

    pub fn ensure_supported(&self, repo: &Repository) -> Result<()> {
        if repo.refs.namespace.is_some() {
            bail!(message("Native local receive does not yet support namespaces").unsupported());
        }
        let config = repo.config_snapshot();
        if config.string("receive.procReceiveRefs").is_some()
            || config
                .try_boolean("receive.fsckObjects")?
                .or(config.try_boolean("transfer.fsckObjects")?)
                .unwrap_or(false)
        {
            bail!(message("Native local receive cannot yet enforce this advanced receive policy").unsupported());
        }
        Ok(())
    }

    pub fn reject_before_transfer(
        &self,
        repo: &Repository,
        update: &UpdateOutcome,
        progress: &mut impl FnMut(bool, &[u8]) -> io::Result<()>,
    ) -> Result<Option<BString>> {
        let name = update.update.destination.as_bstr();
        if self.hidden(name) {
            return Ok(Some(
                if update.update.new_id.is_some() {
                    "deny updating a hidden ref"
                } else {
                    "deny deleting a hidden ref"
                }
                .into(),
            ));
        }
        if self.deny_deletes && update.update.new_id.is_none() && name.starts_with_str("refs/heads/") {
            return Ok(Some("deletion prohibited".into()));
        }
        for worktree in repo.worktrees_including_main()? {
            let worktree = worktree?;
            if worktree.head_name()?.is_none_or(|head| head.as_bstr() != name) {
                continue;
            }
            if !worktree.is_bare() {
                match self.deny_current {
                    Action::Refuse => return Ok(Some("branch is currently checked out".into())),
                    Action::UpdateInstead => return Ok(Some("updating checked-out worktrees is not supported".into())),
                    Action::Warn => progress(false, b"warning: updating the current branch\n").or_error()?,
                    Action::Ignore => {}
                }
            }
            if update.update.new_id.is_none() {
                match self.deny_delete_current {
                    Action::Refuse | Action::UpdateInstead => {
                        return Ok(Some("deletion of the current branch prohibited".into()));
                    }
                    Action::Warn => progress(false, b"warning: deleting the current branch\n").or_error()?,
                    Action::Ignore => {}
                }
            }
        }
        Ok(None)
    }
}

fn action(repo: &Repository, key: &str) -> Result<Action> {
    let config = repo.config_snapshot();
    let Some(value) = config.string(key) else {
        return Ok(Action::Refuse);
    };
    Ok(match value.to_ascii_lowercase().as_slice() {
        b"ignore" => Action::Ignore,
        b"warn" => Action::Warn,
        b"refuse" => Action::Refuse,
        b"updateinstead" => Action::UpdateInstead,
        _ => {
            if config.try_boolean(key)?.unwrap_or(true) {
                Action::Refuse
            } else {
                Action::Ignore
            }
        }
    })
}
