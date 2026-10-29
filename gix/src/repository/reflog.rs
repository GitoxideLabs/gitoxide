//! Expire reflog entries according to age and commit reachability.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    ObjectId, Repository, Result,
    bstr::{BStr, BString, ByteSlice},
    error::{ResultExt, bail, message},
};

/// Settings for [`Repository::expire_reflogs()`].
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Reference clock for relative configuration dates, or the current time.
    pub now: Option<gix_date::Zoned>,
    /// Override total expiry with unsigned seconds since the Unix epoch.
    ///
    /// Entries strictly older than this cutoff expire. Zero means never and
    /// [`u64::MAX`] means all; see [`gix_date::parse_expiry()`].
    pub expire: Option<u64>,
    /// Override expiry of entries whose old or new commit is unreachable.
    /// Uses the same timestamp representation as [`Self::expire`].
    pub expire_unreachable: Option<u64>,
    /// Compute retention under reference locks without rewriting logs.
    pub dry_run: bool,
}

/// Entries processed across all worktree reflogs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// Number of reflog files visited, including empty logs.
    pub logs: usize,
    /// Entries preserved.
    pub retained: usize,
    /// Entries removed, or selected for removal during a dry run.
    pub removed: usize,
}

#[derive(Clone, Copy, Default)]
struct Cutoffs {
    total: u64,
    unreachable: u64,
}

struct Policy {
    defaults: Cutoffs,
    patterns: Vec<(BString, Cutoffs)>,
}

enum Reachability {
    AlwaysExpire,
    Commits(BTreeSet<ObjectId>),
}

impl Repository {
    /// Expire reflogs in the main repository and every registered linked worktree.
    ///
    /// Honor `gc.reflogExpire` (90 days by default), `gc.reflogExpireUnreachable`
    /// (30 days), and the first matching `gc.<pattern>` configuration. An omitted
    /// cutoff in a matching pattern means never. Unconfigured `refs/stash` never
    /// expires, even when global cutoffs are configured. Explicit option cutoffs
    /// override configuration and the stash exception.
    ///
    /// A branch or tag log uses commits reachable from its current target. HEAD
    /// logs use all direct reference tips. Null IDs, missing objects, and objects
    /// that do not peel to commits do not make an entry unreachable. Shallow roots
    /// are respected and replacement objects are ignored. Namespaces do not limit
    /// maintenance. Reachability is read while the log's named reference is locked.
    ///
    /// Retained entries keep their original bytes and object IDs, and reference
    /// values are never updated. Each log is replaced atomically. An error leaves
    /// that log intact; earlier logs may already have been processed. This does not
    /// expire objects or perform Git's optional stale-object repair.
    pub fn expire_reflogs(&self, options: Options, interrupt: &AtomicBool) -> Result<Outcome> {
        check_interrupt(interrupt)?;
        let policy = Policy::from_config(self, options.now.clone().unwrap_or_else(gix_date::Zoned::now))?;
        let mut outcome = Outcome::default();
        for repo in self.worktrees_including_main()? {
            check_interrupt(interrupt)?;
            let mut repo = repo?;
            repo.clear_namespace();
            repo.objects.ignore_replacements = true;
            for name in log_names(&repo.git_dir().join("logs"), interrupt)? {
                check_interrupt(interrupt)?;
                let qualified = if repo.kind() == crate::repository::Kind::LinkedWorkTree {
                    let mut qualified = BString::from("worktrees/");
                    qualified.extend_from_slice(&gix_path::into_bstr(Path::new(
                        repo.git_dir().file_name().expect("worktree name"),
                    ))?);
                    qualified.push(b'/');
                    qualified.extend_from_slice(name.as_bstr());
                    qualified
                } else {
                    name.as_bstr().to_owned()
                };
                let cutoffs = policy.for_name(qualified.as_bstr(), &options);
                let mut reachability = None;
                let retained = repo.refs.reflog_retain(
                    name.as_ref(),
                    gix_ref::file::log::retain::Options {
                        dry_run: options.dry_run,
                    },
                    |line| {
                        check_interrupt(interrupt)?;
                        let timestamp = line.signature.time()?.seconds as u64;
                        if timestamp < cutoffs.total {
                            return Ok(false);
                        }
                        if timestamp >= cutoffs.unreachable {
                            return Ok(true);
                        }
                        if reachability.is_none() {
                            reachability = Some(Reachability::for_ref(&repo, name.as_ref(), cutoffs, interrupt)?);
                        }
                        match reachability.as_ref().expect("initialized above") {
                            Reachability::AlwaysExpire => Ok(false),
                            Reachability::Commits(commits) => {
                                for object_id in [line.previous_oid(), line.new_oid()] {
                                    if let Some(commit_id) = commit_id(&repo, object_id)?
                                        && !commits.contains(&commit_id)
                                    {
                                        return Ok(false);
                                    }
                                }
                                Ok(true)
                            }
                        }
                    },
                )?;
                outcome.logs += 1;
                outcome.retained += retained.retained;
                outcome.removed += retained.removed;
            }
        }
        Ok(outcome)
    }
}

impl Policy {
    fn from_config(repo: &Repository, now: gix_date::Zoned) -> Result<Self> {
        let now_seconds = now.timestamp().as_second() as u64;
        let mut defaults = Cutoffs {
            total: now_seconds.saturating_sub(90 * 24 * 60 * 60),
            unreachable: now_seconds.saturating_sub(30 * 24 * 60 * 60),
        };
        let mut patterns = Vec::<(BString, Cutoffs)>::new();
        let mut positions = BTreeMap::new();
        for section in repo.config.resolved.sections_by_name("gc").into_iter().flatten() {
            for (key, total) in [("reflogExpire", true), ("reflogExpireUnreachable", false)] {
                if let Some(value) = section.value_implicit(key) {
                    let value = value.ok_or_else(|| message!("gc.{key} requires an expiry date").validation_error())?;
                    let cutoff = gix_date::parse_expiry(value.to_str().or_error()?, Some(now.clone()))
                        .or_raise(|| message!("Invalid gc.{key} expiry date"))?;
                    let cutoffs = if let Some(pattern) = section.header().subsection_name() {
                        let position = *positions.entry(pattern.to_owned()).or_insert_with(|| {
                            let position = patterns.len();
                            patterns.push((pattern.to_owned(), Cutoffs::default()));
                            position
                        });
                        &mut patterns[position].1
                    } else {
                        &mut defaults
                    };
                    if total {
                        cutoffs.total = cutoff;
                    } else {
                        cutoffs.unreachable = cutoff;
                    }
                }
            }
        }
        Ok(Self { defaults, patterns })
    }

    fn for_name(&self, name: &BStr, options: &Options) -> Cutoffs {
        let mut cutoffs = self
            .patterns
            .iter()
            .find_map(|(pattern, cutoffs)| {
                gix_glob::wildmatch(pattern.as_bstr(), name, gix_glob::wildmatch::Mode::empty()).then_some(*cutoffs)
            })
            .unwrap_or_else(|| {
                if name == "refs/stash" {
                    Cutoffs::default()
                } else {
                    self.defaults
                }
            });
        if let Some(value) = options.expire {
            cutoffs.total = value;
        }
        if let Some(value) = options.expire_unreachable {
            cutoffs.unreachable = value;
        }
        cutoffs
    }
}

impl Reachability {
    fn for_ref(
        repo: &Repository,
        name: &gix_ref::FullNameRef,
        cutoffs: Cutoffs,
        interrupt: &AtomicBool,
    ) -> Result<Self> {
        if cutoffs.unreachable <= cutoffs.total {
            return Ok(Self::AlwaysExpire);
        }
        let mut tips = Vec::new();
        if name.as_bstr() == "HEAD" {
            for reference in repo.references()?.all()? {
                check_interrupt(interrupt)?;
                if let Some(object_id) = reference?.try_id()
                    && let Some(commit_id) = commit_id(repo, object_id.detach())?
                {
                    tips.push(commit_id);
                }
            }
        } else {
            let Some(mut reference) = repo.try_find_reference(name.as_bstr())? else {
                return Ok(Self::AlwaysExpire);
            };
            let object_id = reference.follow_to_object()?.detach();
            let Some(commit_id) = commit_id(repo, object_id)? else {
                return Ok(Self::AlwaysExpire);
            };
            tips.push(commit_id);
        }
        let mut commits = BTreeSet::new();
        for commit in repo.rev_walk(tips).all()? {
            check_interrupt(interrupt)?;
            commits.insert(commit?.id);
        }
        Ok(Self::Commits(commits))
    }
}

fn commit_id(repo: &Repository, object_id: ObjectId) -> Result<Option<ObjectId>> {
    if object_id.is_null() {
        return Ok(None);
    }
    if object_id.kind() != repo.object_hash() {
        bail!(message("Wrong object format in reflog").validation());
    }
    let Some(object) = repo.try_find_object(object_id)? else {
        return Ok(None);
    };
    let object = object.peel_tags_to_end()?;
    Ok((object.kind == gix_object::Kind::Commit).then_some(object.id))
}

fn log_names(root: &Path, interrupt: &AtomicBool) -> Result<BTreeSet<gix_ref::FullName>> {
    let mut names = BTreeSet::<gix_ref::FullName>::new();
    let mut pending = vec![root.to_owned()];
    while let Some(directory) = pending.pop() {
        check_interrupt(interrupt)?;
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
            Err(err) => {
                return Err(err).or_raise(|| message!("Could not enumerate reflogs in {}", directory.display()));
            }
        };
        for entry in entries {
            check_interrupt(interrupt)?;
            let entry = entry.or_error()?;
            let kind = entry.file_type().or_error()?;
            let path = entry.path();
            if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file() {
                if path.extension().is_some_and(|ext| ext == "lock") {
                    continue;
                }
                let name = gix_path::to_unix_separators_on_windows(gix_path::into_bstr(
                    path.strip_prefix(root).expect("within root"),
                )?);
                names.insert(gix_ref::FullName::try_from(name.as_bstr()).or_error()?);
            } else {
                bail!(message!("Expected a regular reflog at {}", path.display()).validation());
            }
        }
    }
    Ok(names)
}

fn check_interrupt(interrupt: &AtomicBool) -> Result<()> {
    if interrupt.load(Ordering::Relaxed) {
        bail!(message("Reflog expiry interrupted").cancelled());
    }
    Ok(())
}
