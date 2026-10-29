use std::{
    collections::HashMap,
    io::{self, Write},
    sync::atomic::{AtomicBool, Ordering},
};

use gix_hashtable::HashSet;
use gix_object::{FindExt, Kind};
use gix_pack::data::output::count::{self, objects::ObjectExpansion};

use crate::{
    ObjectId, Repository, Result,
    error::{ResultExt, bail, cancelled, corruption, ensure, message, validation},
};

impl Repository {
    /// Write a bundle containing `references` and their reachable objects, returning the pack checksum.
    ///
    /// Names are explicit, so callers can choose namespaces and include detached `HEAD` without
    /// including reflogs. Repeated identical names are emitted once; conflicting IDs for one name
    /// are an error. SHA-1 repositories produce version 2 bundles, SHA-256 repositories version 3.
    ///
    /// `excluded_tips` excludes commits and their ancestors, and tags together with their targets.
    /// Directly excluded tree or blob tips are not advertised. Other dependencies are omitted only
    /// when reachable from a declared commit prerequisite, keeping the bundle complete for a receiver
    /// that has those prerequisites. Prerequisites are the actual boundary commits, not necessarily
    /// the excluded tips. Annotated tags pointing into excluded history remain available through an
    /// appropriate prerequisite. An empty resulting reference selection is an error.
    ///
    /// Replacement refs and shallow grafts are ignored: objects retain their original identity and
    /// all required history must be available. In-memory objects are supported without persisting them.
    /// The pack is self-contained with respect to delta bases, as with [`Self::write_pack()`].
    /// Selection and pack preflight errors produce no output; later read, write, and cancellation
    /// failures may leave partial output. Use a temporary file when publishing a bundle.
    pub fn write_bundle<P>(
        &self,
        references: impl IntoIterator<Item = (gix_ref::FullName, ObjectId)>,
        excluded_tips: impl IntoIterator<Item = ObjectId>,
        out: impl Write,
        mut progress: P,
        should_interrupt: &AtomicBool,
    ) -> Result<ObjectId>
    where
        P: gix_utils::progress::NestedProgress,
        P::SubProgress: 'static,
    {
        let check_interrupt = || -> Result<()> {
            if should_interrupt.load(Ordering::Relaxed) {
                bail!(cancelled("Bundle creation interrupted"));
            }
            Ok(())
        };
        let check_hash = |object_id: ObjectId| -> Result<()> {
            ensure!(
                object_id.kind() == self.object_hash(),
                validation("Bundle object IDs must use the repository's hash")
            );
            Ok(())
        };
        check_interrupt()?;
        let mut repo = self.clone();
        repo.make_paths_absolute()?;
        repo.objects.ignore_replacements = true;
        let mut excluded_objects = HashSet::default();
        let mut excluded_commits = Vec::new();
        for object_id in excluded_tips {
            check_hash(object_id)?;
            let (peeled_id, kind) = peel_tags(&repo, object_id, &mut excluded_objects, should_interrupt)?;
            if kind == Kind::Commit {
                excluded_commits.push(peeled_id);
            }
        }
        // Walk exact ancestry rather than using a timestamp cutoff or shallow grafts.
        for commit in gix_traverse::commit::Simple::new(excluded_commits, &repo.objects) {
            check_interrupt()?;
            excluded_objects.insert(commit?.id);
        }

        let mut names = HashMap::new();
        let mut selected = Vec::new();
        let mut included_commits = Vec::new();
        let mut prerequisite_ids = HashSet::default();
        let mut explicit_trees = HashSet::default();
        for (name, object_id) in references {
            check_interrupt()?;
            check_hash(object_id)?;
            if let Some(previous) = names.insert(name.clone(), object_id) {
                ensure!(
                    previous == object_id,
                    validation("A bundle reference has conflicting object IDs")
                );
                continue;
            }
            if excluded_objects.contains(&object_id) {
                continue;
            }
            let (peeled_id, kind) = peel_tags(&repo, object_id, &mut HashSet::default(), should_interrupt)?;
            match kind {
                Kind::Commit if excluded_objects.contains(&peeled_id) => {
                    prerequisite_ids.insert(peeled_id);
                }
                Kind::Commit => included_commits.push(peeled_id),
                Kind::Tree if peeled_id == object_id => {
                    explicit_trees.insert(object_id);
                }
                _ => {}
            }
            selected.push((name, object_id));
        }
        ensure!(
            !selected.is_empty(),
            validation("Cannot create a bundle without references")
        );
        let mut roots: Vec<_> = selected.iter().map(|(_, object_id)| *object_id).collect();
        for commit in gix_traverse::commit::Simple::filtered(included_commits, &repo.objects, |commit_id| {
            !excluded_objects.contains(commit_id)
        }) {
            check_interrupt()?;
            let commit = commit?;
            prerequisite_ids.extend(
                commit
                    .parent_ids
                    .iter()
                    .filter(|id| excluded_objects.contains(*id))
                    .copied(),
            );
            roots.push(commit.id);
        }
        // A prerequisite guarantees its entire closure. Remove redundant ancestor prerequisites
        // while obtaining the commits whose trees may be omitted from the payload.
        let mut prerequisite_commits = Vec::new();
        for commit in gix_traverse::commit::Simple::new(prerequisite_ids.iter().copied(), &repo.objects) {
            check_interrupt()?;
            let commit = commit?;
            for parent_id in commit.parent_ids {
                prerequisite_ids.remove(&parent_id);
            }
            prerequisite_commits.push(commit.id);
        }
        let expansion_progress = progress.add_child("selecting bundle objects");
        let expand = |object_ids: Vec<ObjectId>| {
            count::objects_unthreaded(
                &repo.objects,
                &mut object_ids.into_iter().map(Ok),
                &expansion_progress,
                should_interrupt,
                ObjectExpansion::TreeContents,
            )
            .map(|(counts, _)| counts)
        };
        let guaranteed_objects: HashSet = expand(prerequisite_commits)?
            .into_iter()
            .map(|count| count.id)
            .collect();
        let object_ids = expand(roots)?
            .into_iter()
            .map(|count| count.id)
            .filter(|id| !guaranteed_objects.contains(id) || explicit_trees.contains(id));

        let mut header = Vec::new();
        if self.object_hash() == gix_hash::Kind::Sha1 {
            writeln!(header, "# v2 git bundle").or_error()?;
        } else {
            writeln!(header, "# v3 git bundle\n@object-format={}", self.object_hash()).or_error()?;
        }
        let mut prerequisite_ids: Vec<_> = prerequisite_ids.into_iter().collect();
        prerequisite_ids.sort_unstable();
        for commit_id in prerequisite_ids {
            writeln!(header, "-{commit_id} ").or_error()?;
        }
        for (name, object_id) in selected {
            write!(header, "{object_id} ").or_error()?;
            header.extend_from_slice(name.as_bstr());
            header.push(b'\n');
        }
        header.push(b'\n');
        repo.write_pack(
            object_ids,
            WithHeader {
                out,
                header,
                failed: false,
            },
            progress,
            should_interrupt,
        )
        .map(|outcome| outcome.pack_hash)
        .or_raise(|| message("Could not write bundle pack"))
    }
}

fn peel_tags(
    repo: &Repository,
    mut object_id: ObjectId,
    seen: &mut HashSet,
    should_interrupt: &AtomicBool,
) -> Result<(ObjectId, Kind)> {
    let mut buf = Vec::new();
    let mut tags = HashSet::default();
    loop {
        if should_interrupt.load(Ordering::Relaxed) {
            bail!(cancelled("Bundle creation interrupted"));
        }
        seen.insert(object_id);
        let object = repo.objects.find(&object_id, &mut buf)?;
        if object.kind != Kind::Tag {
            return Ok((object_id, object.kind));
        }
        ensure!(tags.insert(object_id), corruption("Cycle in bundle tag targets"));
        object_id = gix_object::TagRefIter::from_bytes(object.data, object.object_hash).target_id()?;
    }
}

/// Delay the bundle header until pack preflight has succeeded.
struct WithHeader<W> {
    out: W,
    header: Vec<u8>,
    failed: bool,
}

impl<W: Write> Write for WithHeader<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.failed {
            return Err(io::Error::other("Bundle header write previously failed"));
        }
        self.failed = true;
        self.out.write_all(&self.header)?;
        self.header.clear();
        self.failed = false;
        self.out.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_writes_and_flush_do_not_emit_the_header() -> crate::error::TestResult {
        let mut writer = WithHeader {
            out: Vec::new(),
            header: b"header".to_vec(),
            failed: false,
        };
        assert_eq!(writer.write(b"")?, 0, "empty writes make no progress");
        writer.flush()?;
        assert!(writer.out.is_empty(), "only pack bytes trigger the bundle header");
        writer.write_all(b"pack")?;
        assert_eq!(
            writer.out, b"headerpack",
            "the first pack write emits the header exactly once"
        );
        Ok(())
    }
}
