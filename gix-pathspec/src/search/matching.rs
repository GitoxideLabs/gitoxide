use bstr::{BStr, BString, ByteSlice};
use gix_glob::pattern::Case;

use crate::search::MatchKind;
use crate::search::MatchKind::*;
use crate::{
    search::{Match, Spec},
    MagicSignature, Pattern, Search, SearchMode,
};

impl Search {
    /// Return the first [`Match`] of `relative_path`, or `None`.
    /// `is_dir` is `true` if `relative_path` is a directory, or assumed `false` if `None`.
    /// `attributes` is called as `attributes(relative_path, case, is_dir, outcome) -> has_match` to obtain for attributes for `relative_path`, if
    /// the underlying pathspec defined an attribute filter, to be stored in `outcome`, returning true if there was a match.
    /// All attributes of the pathspec have to be present in the defined value for the pathspec to match.
    ///
    /// Note that `relative_path` is expected to be starting at the same root as is assumed for this pattern, see [`Pattern::normalize()`].
    /// Further, empty searches match everything, as if `:` was provided.
    ///
    /// ### Deviation
    ///
    /// The case-sensitivity of the attribute match is controlled by the sensitivity of the pathspec, instead of being based on the
    /// case folding settings of the repository. That way we assure that the matching is consistent.
    /// Higher-level crates should control this default case folding of pathspecs when instantiating them, which is when they can
    /// set it to match the repository setting for more natural behaviour when, for instance, adding files to a repository:
    /// as it stands, on a case-insensitive file system, `touch File && git add file` will not add the file, but also not error.
    pub fn pattern_matching_relative_path(
        &mut self,
        relative_path: &BStr,
        is_dir: bool,
        attributes: &mut dyn FnMut(&BStr, Case, bool, &mut gix_attributes::search::Outcome) -> bool,
    ) -> Option<Match<'_>> {
        static MATCH_ALL_STAND_IN: Pattern = Pattern {
            path: BString::new(Vec::new()),
            signature: MagicSignature::empty(),
            search_mode: SearchMode::ShellGlob,
            attributes: Vec::new(),
            prefix_len: 0,
            nil: true,
        };
        if relative_path.is_empty() {
            return Some(Match {
                pattern: &MATCH_ALL_STAND_IN,
                sequence_number: 0,
                kind: Always,
            });
        }
        let basename_not_important = None;
        if relative_path
            .get(..self.common_prefix_len)
            .map_or(true, |rela_path_prefix| rela_path_prefix != self.common_prefix())
        {
            return None;
        }

        let patterns_len = self.patterns.len();
        let res = self.patterns.iter_mut().find_map(|mapping| {
            let ignore_case = mapping.value.pattern.signature.contains(MagicSignature::ICASE);
            let prefix = mapping.value.pattern.prefix_directory();
            if ignore_case && !prefix.is_empty() {
                let pattern_requirement_is_met = relative_path.get(prefix.len()).map_or_else(|| is_dir, |b| *b == b'/');
                if !pattern_requirement_is_met
                    || relative_path.get(..prefix.len()).map(ByteSlice::as_bstr) != Some(prefix)
                {
                    return None;
                }
            }

            let case = if ignore_case { Case::Fold } else { Case::Sensitive };
            let mut is_match = mapping.value.pattern.always_matches();
            let mut how = Always;
            if !is_match {
                is_match = if mapping.pattern.first_wildcard_pos.is_none() {
                    match_verbatim(mapping, relative_path, is_dir, case, &mut how)
                } else {
                    let wildmatch_mode = match mapping.value.pattern.search_mode {
                        SearchMode::ShellGlob => Some(gix_glob::wildmatch::Mode::empty()),
                        SearchMode::Literal => None,
                        SearchMode::PathAwareGlob => Some(gix_glob::wildmatch::Mode::NO_MATCH_SLASH_LITERAL),
                    };
                    match wildmatch_mode {
                        Some(wildmatch_mode) => {
                            let is_match = mapping.pattern.matches_repo_relative_path(
                                relative_path,
                                basename_not_important,
                                Some(is_dir),
                                case,
                                wildmatch_mode,
                            );
                            if !is_match {
                                match_verbatim(mapping, relative_path, is_dir, case, &mut how)
                            } else {
                                how = mapping.pattern.first_wildcard_pos.map_or(Verbatim, |_| WildcardMatch);
                                true
                            }
                        }
                        None => match_verbatim(mapping, relative_path, is_dir, case, &mut how),
                    }
                }
            }

            if let Some(attrs) = mapping.value.attrs_match.as_mut() {
                if !attributes(relative_path, Case::Sensitive, is_dir, attrs) {
                    // we have attrs, but it didn't match any
                    return None;
                }
                for (actual, expected) in attrs.iter_selected().zip(mapping.value.pattern.attributes.iter()) {
                    if actual.assignment != expected.as_ref() {
                        return None;
                    }
                }
            }

            is_match.then_some(Match {
                pattern: &mapping.value.pattern,
                sequence_number: mapping.sequence_number,
                kind: how,
            })
        });

        if res.is_none() && self.all_patterns_are_excluded {
            Some(Match {
                pattern: &MATCH_ALL_STAND_IN,
                sequence_number: patterns_len,
                kind: Always,
            })
        } else {
            res
        }
    }

    /// As opposed to [`Self::pattern_matching_relative_path()`], this method will return `true` for a possibly partial `relative_path`
    /// if this pathspec *could* match by looking at the shortest shared prefix only.
    ///
    /// This is useful if `relative_path` is a directory leading up to the item that is going to be matched in full later.
    /// Note that it should not end with `/` to indicate it's a directory, rather, use `is_dir` to indicate this.
    /// TODO
    /// Returns `false` if this pathspec has no chance of ever matching `relative_path`.
    pub fn can_match_relative_path(&self, relative_path: &BStr, is_dir: bool) -> bool {
        if self.patterns.is_empty() || relative_path.is_empty() {
            return true;
        }
        let common_prefix_len = self.common_prefix_len.min(relative_path.len());
        if relative_path.get(..common_prefix_len).map_or(true, |rela_path_prefix| {
            rela_path_prefix != self.common_prefix()[..common_prefix_len]
        }) {
            return false;
        }
        for mapping in &self.patterns {
            let pattern = &mapping.value.pattern;
            if mapping.pattern.first_wildcard_pos == Some(0) && !pattern.is_excluded() {
                return true;
            }
            let max_usable_pattern_len = mapping.pattern.first_wildcard_pos.unwrap_or_else(|| pattern.path.len());
            let common_len = max_usable_pattern_len.min(relative_path.len());

            let ignore_case = pattern.signature.contains(MagicSignature::ICASE);
            let mut is_match = pattern.always_matches();
            if !is_match && common_len != 0 {
                let pattern_path = pattern.path[..common_len].as_bstr();
                let longest_possible_relative_path = &relative_path[..common_len];
                is_match = if ignore_case {
                    pattern_path.eq_ignore_ascii_case(longest_possible_relative_path)
                } else {
                    pattern_path == longest_possible_relative_path
                };

                if is_match {
                    is_match = if common_len < max_usable_pattern_len {
                        pattern.path.get(common_len) == Some(&b'/')
                    } else if relative_path.len() > max_usable_pattern_len
                        && mapping.pattern.first_wildcard_pos.is_none()
                    {
                        relative_path.get(common_len) == Some(&b'/')
                    } else {
                        is_match
                    };
                    if is_dir {
                        is_match = if pattern.signature.contains(MagicSignature::MUST_BE_DIR) {
                            matches!(pattern.path.get(common_len), None | Some(&b'/'))
                        } else {
                            relative_path.get(common_len) == Some(&b'/')
                        };
                    }
                }
            }
            if is_match && (!pattern.is_excluded() || pattern.always_matches()) {
                return !pattern.is_excluded();
            }
        }

        self.all_patterns_are_excluded
    }

    /// Returns `true` if `relative_path` matches the prefix of this pathspec.
    ///
    /// For example, the relative path `d` matches `d/`, `d*/`, `d/` and `d/*`, but not `d/d/*` or `dir`.
    /// When `leading` is `true`, then `d` matches `d/d` as well. Thus, `relative_path` must may be
    /// partially included in `pathspec`, otherwise it has to be fully included.
    pub fn directory_matches_prefix(&self, relative_path: &BStr, leading: bool) -> bool {
        if self.patterns.is_empty() || relative_path.is_empty() {
            return true;
        }
        let common_prefix_len = self.common_prefix_len.min(relative_path.len());
        if relative_path.get(..common_prefix_len).map_or(true, |rela_path_prefix| {
            rela_path_prefix != self.common_prefix()[..common_prefix_len]
        }) {
            return false;
        }
        for mapping in &self.patterns {
            let pattern = &mapping.value.pattern;
            if mapping.pattern.first_wildcard_pos.is_some() && pattern.is_excluded() {
                return true;
            }
            let mut rightmost_idx = mapping.pattern.first_wildcard_pos.map_or_else(
                || pattern.path.len(),
                |idx| pattern.path[..idx].rfind_byte(b'/').unwrap_or(idx),
            );
            let ignore_case = pattern.signature.contains(MagicSignature::ICASE);
            let mut is_match = pattern.always_matches();
            if !is_match {
                let plen = relative_path.len();
                if leading && rightmost_idx > plen {
                    if let Some(idx) = pattern.path[..plen]
                        .rfind_byte(b'/')
                        .or_else(|| pattern.path[plen..].find_byte(b'/').map(|idx| idx + plen))
                    {
                        rightmost_idx = idx;
                    }
                }
                if let Some(relative_path) = relative_path.get(..rightmost_idx) {
                    let pattern_path = pattern.path[..rightmost_idx].as_bstr();
                    is_match = if ignore_case {
                        pattern_path.eq_ignore_ascii_case(relative_path)
                    } else {
                        pattern_path == relative_path
                    };
                }
            }
            if is_match && (!pattern.is_excluded() || pattern.always_matches()) {
                return !pattern.is_excluded();
            }
        }

        self.all_patterns_are_excluded
    }
}

fn match_verbatim(
    mapping: &gix_glob::search::pattern::Mapping<Spec>,
    relative_path: &BStr,
    is_dir: bool,
    case: Case,
    how: &mut MatchKind,
) -> bool {
    let pattern_len = mapping.value.pattern.path.len();
    let mut relative_path_ends_with_slash_at_pattern_len = false;
    let (match_is_allowed, probably_how) = relative_path.get(pattern_len).map_or_else(
        || (relative_path.len() == pattern_len, Verbatim),
        |b| {
            relative_path_ends_with_slash_at_pattern_len = *b == b'/';
            (relative_path_ends_with_slash_at_pattern_len, Prefix)
        },
    );
    *how = probably_how;
    let pattern_requirement_is_met = !mapping.pattern.mode.contains(gix_glob::pattern::Mode::MUST_BE_DIR)
        || (relative_path_ends_with_slash_at_pattern_len || is_dir);

    if match_is_allowed && pattern_requirement_is_met {
        let dir_or_file = &relative_path[..mapping.value.pattern.path.len()];
        match case {
            Case::Sensitive => mapping.value.pattern.path == dir_or_file,
            Case::Fold => mapping.value.pattern.path.eq_ignore_ascii_case(dir_or_file),
        }
    } else {
        false
    }
}
