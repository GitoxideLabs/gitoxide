use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

use gix_hash::ObjectId;
use gix_object::bstr::BString;

use crate::types::{Change, UnblamedHunk};

// Git compares multisets of adjacent, case-folded bytes, normalizing whitespace.
#[derive(Default)]
struct Fingerprint(BTreeMap<u16, usize>);

impl Fingerprint {
    fn new(line: &[u8]) -> Self {
        let mut out = Self::default();
        let mut previous = 0u8;
        for byte in line.iter().copied().chain([0]) {
            let byte = if byte.is_ascii_whitespace() {
                0
            } else {
                byte.to_ascii_lowercase()
            };
            let pair = u16::from(previous) | (u16::from(byte) << 8);
            if pair != 0 {
                *out.0.entry(pair).or_default() += 1;
            }
            previous = byte;
        }
        out
    }

    fn similarity(&self, other: &Self) -> usize {
        other
            .0
            .iter()
            .map(|(pair, count)| self.0.get(pair).copied().unwrap_or_default().min(*count))
            .sum()
    }

    fn subtract(&mut self, other: &Self) {
        self.0.retain(|pair, count| {
            *count = count.saturating_sub(other.0.get(pair).copied().unwrap_or_default());
            *count != 0
        });
    }
}

fn fuzzy_matches(
    old: &mut [Fingerprint],
    new: &[Fingerprint],
    old_range: Range<usize>,
    new_range: Range<usize>,
    matches: &mut [Option<usize>],
) {
    if old_range.is_empty() || new_range.is_empty() {
        return;
    }
    let distance = 10.min(old_range.len() - 1);
    let mut pending = vec![(old_range.clone(), new_range.clone())];
    while let Some((parents, targets)) = pending.pop() {
        let mut strongest = None;
        for target in targets.clone() {
            let closest = ((((target - new_range.start) as u128 * 2 + 1) * old_range.len() as u128)
                / (new_range.len() as u128 * 2)) as usize
                + old_range.start;
            let search = closest.saturating_sub(distance).max(parents.start)..(closest + distance + 1).min(parents.end);
            let (mut best, mut second, mut best_parent) = (0, 0, 0);
            for parent in search {
                let similarity = new[target].similarity(&old[parent]) * (1000 - parent.abs_diff(closest));
                if similarity > best {
                    second = best;
                    best = similarity;
                    best_parent = parent;
                } else if similarity > second {
                    second = similarity;
                }
            }
            if best != 0 {
                let certainty = best * 2 - second;
                if strongest.is_none_or(|(_, _, previous)| certainty > previous) {
                    strongest = Some((best_parent, target, certainty));
                }
            }
        }
        let Some((parent, target, _)) = strongest else { continue };
        matches[target] = Some(parent);
        old[parent].subtract(&new[target]);
        // Preserve ordering, but permit several target lines to consume different parts of one parent line.
        if target + 1 < targets.end {
            pending.push((parent..parents.end, target + 1..targets.end));
        }
        if targets.start < target {
            pending.push((parents.start..parent + 1, targets.start..target));
        }
    }
}

fn line_matches(old: &[u8], new: &[u8], changes: &[Change]) -> Vec<Option<usize>> {
    let mut old: Vec<_> = old
        .split_inclusive(|byte| *byte == b'\n')
        .map(Fingerprint::new)
        .collect();
    let new: Vec<_> = new
        .split_inclusive(|byte| *byte == b'\n')
        .map(Fingerprint::new)
        .collect();
    let mut matches = vec![None; new.len()];
    let mut old_start = 0;
    for change in changes {
        match change {
            Change::Unchanged(range) => old_start += range.len(),
            Change::Deleted(_, deleted) => old_start += *deleted as usize,
            Change::AddedOrReplaced(range, deleted) => {
                let new_range = range.start as usize..range.end as usize;
                fuzzy_matches(
                    &mut old,
                    &new,
                    old_start..old_start + *deleted as usize,
                    new_range.clone(),
                    &mut matches,
                );
                for target in new_range {
                    if matches[target].is_some() {
                        continue;
                    }
                    let mut best = 10;
                    for (parent, fingerprint) in old.iter().enumerate() {
                        let similarity = fingerprint.similarity(&new[target]);
                        if similarity >= best
                            && matches[target].is_none_or(|previous| {
                                similarity > best || parent.abs_diff(target) <= previous.abs_diff(target)
                            })
                        {
                            matches[target] = Some(parent);
                            best = similarity;
                        }
                    }
                }
                old_start += *deleted as usize;
            }
        }
    }
    matches
}

#[expect(clippy::too_many_arguments)]
pub(super) fn pass_blame(
    hunks: Vec<UnblamedHunk>,
    suspect: ObjectId,
    parent: ObjectId,
    source_file_name: Option<BString>,
    old: &[u8],
    new: &[u8],
    changes: &[Change],
    ignored: &mut BTreeSet<u32>,
    unblamable: &mut BTreeSet<u32>,
) -> Vec<UnblamedHunk> {
    let matches = line_matches(old, new, changes);
    let mut out = Vec::new();
    for hunk in hunks {
        let Some(range) = hunk.get_range(&suspect).filter(|_| hunk.suspects.len() == 1) else {
            out.push(hunk);
            continue;
        };
        for (offset, line) in range.clone().enumerate() {
            let final_line = hunk.range_in_blamed_file.start + offset as u32;
            let (commit, source_line, source_file_name) = if let Some(parent_line) = matches[line as usize] {
                ignored.insert(final_line);
                unblamable.remove(&final_line);
                (parent, parent_line as u32, source_file_name.clone())
            } else {
                unblamable.insert(final_line);
                (suspect, line, hunk.source_file_name.clone())
            };
            out.push(UnblamedHunk {
                range_in_blamed_file: final_line..final_line + 1,
                suspects: [(commit, source_line..source_line + 1)].into(),
                source_file_name,
            });
        }
    }
    out
}
