use std::path::{Component, Path, PathBuf};

use bstr::{BStr, BString, ByteSlice, ByteVec};

use crate::{MagicSignature, Pattern, SearchMode, normalize};

/// Access
impl Pattern {
    /// Returns `true` if this seems to be a pathspec that indicates that 'there is no pathspec'.
    ///
    /// Note that such a spec is `:`.
    pub fn is_nil(&self) -> bool {
        self.nil
    }

    /// Return the prefix-portion of the `path` of this spec, which is a *directory*.
    /// It can be empty if there is no prefix.
    ///
    /// A prefix is effectively the CWD seen as relative to the working tree, and it's assumed to
    /// match case-sensitively. This makes it useful for skipping over large portions of input by
    /// directly comparing them.
    pub fn prefix_directory(&self) -> &BStr {
        self.path[..self.prefix_len].as_bstr()
    }

    /// Return the path of this spec, typically used for matching.
    pub fn path(&self) -> &BStr {
        self.path.as_ref()
    }
}

/// Mutation
impl Pattern {
    /// Normalize the pattern's path by assuring it's relative to the root of the working tree, and contains
    /// no relative path components. Further, it assures that `/` are used as path separator.
    ///
    /// If `self.path` is a relative path, it will be put in front of the pattern path if `self.signature` isn't indicating `TOP` already.
    /// If `self.path` is an absolute path, we will use `root` to make it worktree relative if possible.
    ///
    /// `prefix` can be empty, we will still normalize this pathspec to resolve relative path components, and
    /// it is assumed not to contain any relative path components, e.g. '', 'a', 'a/b' are valid.
    /// `root` is the absolute path to the root of either the worktree or the repository's `git_dir`.
    pub fn normalize(&mut self, prefix: &Path, root: &Path) -> Result<&mut Self, normalize::Error> {
        fn prefix_components_to_subtract(path: &Path) -> usize {
            let parent_component_end_bound = path.components().enumerate().fold(None::<usize>, |acc, (idx, c)| {
                matches!(c, Component::ParentDir).then_some(idx + 1).or(acc)
            });
            let count = path
                .components()
                .take(parent_component_end_bound.unwrap_or(0))
                .map(|c| match c {
                    Component::ParentDir => 1_isize,
                    Component::Normal(_) => -1,
                    _ => 0,
                })
                .sum::<isize>();
            if count > 0 { count as usize } else { Default::default() }
        }

        let mut path = gix_path::from_bstr(self.path.as_bstr());
        let mut num_prefix_components = 0;
        let mut was_absolute = false;
        if gix_path::is_absolute(path.as_ref()) {
            was_absolute = true;
            let rela_path = match path.strip_prefix(root) {
                Ok(path) => path,
                Err(_) => {
                    return Err(normalize::Error::AbsolutePathOutsideOfWorktree {
                        path: path.into_owned(),
                        worktree_path: root.into(),
                    });
                }
            };
            path = rela_path.to_owned().into();
        } else if !prefix.as_os_str().is_empty() && !self.signature.contains(MagicSignature::TOP) {
            debug_assert_eq!(
                prefix
                    .components()
                    .filter(|c| matches!(c, Component::Normal(_)))
                    .count(),
                prefix.components().count(),
                "BUG: prefixes must not have relative path components, or calculations here will be wrong so pattern won't match"
            );
            num_prefix_components = prefix
                .components()
                .count()
                .saturating_sub(prefix_components_to_subtract(path.as_ref()));
            path = prefix.join(path).into();
        }

        let assure_path_cannot_break_out_upwards = Path::new("");
        let path = match gix_path::normalize(path.as_ref().into(), assure_path_cannot_break_out_upwards) {
            Some(path) => {
                if was_absolute {
                    num_prefix_components = path.components().count().saturating_sub(
                        if self.signature.contains(MagicSignature::MUST_BE_DIR) {
                            0
                        } else {
                            1
                        },
                    );
                }
                path
            }
            None => {
                return Err(normalize::Error::OutsideOfWorktree {
                    path: path.into_owned(),
                });
            }
        };

        self.path = if path == Path::new(".") {
            self.nil = true;
            BString::from(".")
        } else {
            let cleaned = PathBuf::from_iter(path.components().filter(|c| !matches!(c, Component::CurDir)));
            let mut out = gix_path::to_unix_separators_on_windows(gix_path::into_bstr(cleaned)).into_owned();
            self.prefix_len = {
                if self.signature.contains(MagicSignature::MUST_BE_DIR) {
                    out.push(b'/');
                }
                let len = out
                    .find_iter(b"/")
                    .take(num_prefix_components)
                    .last()
                    .unwrap_or_default();
                if self.signature.contains(MagicSignature::MUST_BE_DIR) {
                    out.pop();
                }
                len
            };
            out
        };

        Ok(self)
    }
}

/// Access
impl Pattern {
    /// Return `true` if this pathspec is negated, which means it will exclude an item from the result set instead of including it.
    pub fn is_excluded(&self) -> bool {
        self.signature.contains(MagicSignature::EXCLUDE)
    }

    /// Returns `true` is this pattern is supposed to always match, as it's either empty or designated `nil`.
    /// Note that technically the pattern might still be excluded.
    pub fn always_matches(&self) -> bool {
        self.is_nil() || self.path.is_empty()
    }

    /// Translate ourselves to a long display format, that when parsed back will yield the same pattern.
    ///
    /// Note that the
    pub fn to_bstring(&self) -> BString {
        if self.is_nil() {
            ":".into()
        } else {
            let mut buf: BString = ":(".into();
            if self.signature.contains(MagicSignature::TOP) {
                buf.push_str("top,");
            }
            if self.signature.contains(MagicSignature::EXCLUDE) {
                buf.push_str("exclude,");
            }
            if self.signature.contains(MagicSignature::ICASE) {
                buf.push_str("icase,");
            }
            match self.search_mode {
                SearchMode::ShellGlob => {}
                SearchMode::Literal => buf.push_str("literal,"),
                SearchMode::PathAwareGlob => buf.push_str("glob,"),
            }
            if self.attributes.is_empty() {
                if buf.last() == Some(&b',') {
                    buf.pop();
                }
            } else {
                buf.push_str("attr:");
                for attr in &self.attributes {
                    let attr = attr.as_ref().to_string().replace(',', r"\,");
                    buf.push_str(&attr);
                    buf.push(b' ');
                }
                buf.pop(); // trailing ' '
            }
            buf.push(b')');
            buf.extend_from_slice(&self.path);
            if self.signature.contains(MagicSignature::MUST_BE_DIR) {
                buf.push(b'/');
            }
            buf
        }
    }
}

impl std::fmt::Display for Pattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_bstring().fmt(f)
    }
}
