use std::path::Path;

use crate::{MagicSignature, Pattern, Search, search::Spec};

/// Create a new specification to support matches from `pathspec`, [normalizing](Pattern::normalize()) it with `prefix` and `root`.
fn mapping_from_pattern(
    mut pathspec: Pattern,
    prefix: &Path,
    root: &Path,
    sequence_number: usize,
) -> Result<gix_glob::search::pattern::Mapping<Spec>, crate::normalize::Error> {
    pathspec.normalize(prefix, root)?;
    let mut match_all = pathspec.is_nil();
    let glob = {
        let mut g = gix_glob::Pattern::from_bytes_without_negation(&pathspec.path).unwrap_or_else(|| {
            match_all = true;
            // This pattern is setup to match literally as all-whitespace.
            gix_glob::Pattern {
                text: pathspec.path.clone(),
                mode: gix_glob::pattern::Mode::empty(),
                first_wildcard_pos: None,
            }
        });
        g.mode |= gix_glob::pattern::Mode::ABSOLUTE;
        if pathspec.signature.contains(MagicSignature::MUST_BE_DIR) {
            g.mode |= gix_glob::pattern::Mode::MUST_BE_DIR;
        }
        g
    };

    Ok(gix_glob::search::pattern::Mapping {
        pattern: glob,
        value: Spec {
            attrs_match: {
                (!pathspec.attributes.is_empty()).then(|| {
                    let mut out = gix_attributes::search::Outcome::default();
                    out.initialize_with_selection(
                        &Default::default(),
                        pathspec.attributes.iter().map(|a| a.name.as_str()),
                    );
                    out
                })
            },
            pattern: pathspec,
        },
        sequence_number,
    })
}

fn common_prefix_len(patterns: &[gix_glob::search::pattern::Mapping<Spec>]) -> usize {
    let mut count = 0;
    let len = patterns
        .iter()
        .filter(|p| !p.value.pattern.is_excluded())
        .map(|p| {
            count += 1;
            if p.value.pattern.signature.contains(MagicSignature::ICASE) {
                p.value.pattern.prefix_len
            } else {
                p.pattern.first_wildcard_pos.unwrap_or(p.pattern.text.len())
            }
        })
        .min()
        .unwrap_or_default();

    if len == 0 {
        return 0;
    }

    let mut max_len = len;
    if count < 2 {
        return max_len;
    }

    let mut patterns = patterns
        .iter()
        .filter(|p| !p.value.pattern.is_excluded())
        .map(|p| &p.value.pattern.path);
    let base = &patterns.next().expect("at least two patterns");
    for path in patterns {
        for (idx, (a, b)) in base[..max_len].iter().zip(path[..max_len].iter()).enumerate() {
            if *a != *b {
                max_len = idx;
                break;
            }
        }
    }
    max_len
}

/// Lifecycle
impl Search {
    /// Create a search from ready-made `pathspecs`, and [normalize](Pattern::normalize()) them with `prefix` and `root`.
    /// `root` is the absolute path to the worktree root, if available, or the `git_dir` in case of bare repositories.
    /// If `pathspecs` doesn't yield any pattern, we will match everything automatically. If `prefix` is also provided and not empty,
    /// an artificial pattern will be added to yield all.
    pub fn from_specs(
        pathspecs: impl IntoIterator<Item = Pattern>,
        prefix: Option<&std::path::Path>,
        root: &std::path::Path,
    ) -> Result<Self, crate::normalize::Error> {
        fn inner(
            pathspecs: &mut dyn Iterator<Item = Pattern>,
            prefix: Option<&std::path::Path>,
            root: &std::path::Path,
        ) -> Result<Search, crate::normalize::Error> {
            let prefix = prefix.unwrap_or(std::path::Path::new(""));
            let mut patterns = pathspecs
                .enumerate()
                .map(|(idx, pattern)| mapping_from_pattern(pattern, prefix, root, idx))
                .collect::<Result<Vec<_>, _>>()?;

            if patterns.is_empty() && !prefix.as_os_str().is_empty() {
                patterns.push(mapping_from_pattern(
                    Pattern::from_literal(&[], MagicSignature::MUST_BE_DIR),
                    prefix,
                    root,
                    0,
                )?);
            }

            // Excludes should always happen first so we know a match is authoritative (otherwise we could find a non-excluding match first).
            patterns.sort_by(|a, b| {
                a.value
                    .pattern
                    .is_excluded()
                    .cmp(&b.value.pattern.is_excluded())
                    .reverse()
            });

            let common_prefix_len = common_prefix_len(&patterns);
            Ok(Search {
                all_patterns_are_excluded: patterns.iter().all(|s| s.value.pattern.is_excluded()),
                patterns,
                source: None,
                common_prefix_len,
            })
        }
        inner(&mut pathspecs.into_iter(), prefix, root)
    }

    /// Obtain ownership of the normalized pathspec patterns that were used for the search.
    pub fn into_patterns(self) -> impl Iterator<Item = Pattern> {
        self.patterns.into_iter().map(|p| p.value.pattern)
    }
}
