//! Cross-checks `gix_hook::ADVISORY_HOOKS` against git's own canonical hook-name list, vendored
//! in `fixtures/git-hook-names.txt` (see `fixtures/refresh-git-hook-names.sh` for its source).
use std::collections::HashSet;

fn canonical_hook_names() -> HashSet<&'static str> {
    include_str!("fixtures/git-hook-names.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect()
}

#[test]
fn advisory_hooks_are_known_to_git() {
    let canonical = canonical_hook_names();
    for name in gix_hook::ADVISORY_HOOKS {
        assert!(
            canonical.contains(name),
            "{name:?} is not a canonical git hook name per fixtures/git-hook-names.txt - \
             typo, or the name was renamed/removed upstream"
        );
    }
}
