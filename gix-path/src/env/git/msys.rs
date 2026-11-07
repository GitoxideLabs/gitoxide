use std::{path::Path, sync::LazyLock};

/// Recognize the selected MSYS2 Git package before relying on its compiled Unix paths.
pub(super) fn bin() -> Option<&'static Path> {
    static BIN: LazyLock<Option<&'static Path>> = LazyLock::new(|| registered_bin(super::msys_bin()?));
    *BIN
}

fn registered_bin(bin: &Path) -> Option<&Path> {
    let usr = bin.parent()?;
    if !usr.join("lib/git-core/git.exe").is_file() {
        return None;
    }
    // A custom pacman database or an incomplete package falls back to querying Git.
    let mut packages = std::fs::read_dir(usr.parent()?.join("var/lib/pacman/local"))
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .and_then(|name| name.strip_prefix("git-")?.as_bytes().first().copied())
                .is_some_and(|first| first.is_ascii_digit())
                && entry.file_type().is_ok_and(|kind| kind.is_dir())
        });
    let package = packages.next()?.path();
    if packages.next().is_some() {
        return None;
    }
    let desc = std::fs::read_to_string(package.join("desc")).ok()?;
    let mut fields = desc.lines();
    fields.find(|line| *line == "%NAME%")?;
    if fields.next()? != "git" {
        return None;
    }
    let files = std::fs::read_to_string(package.join("files")).ok()?;
    ["usr/bin/git.exe", "usr/lib/git-core/git.exe"]
        .into_iter()
        .all(|file| files.lines().any(|line| line == file))
        .then_some(bin)
}

#[cfg(test)]
mod tests {
    use super::registered_bin;
    use std::path::Path;

    #[test]
    fn package_registration_establishes_the_layout() -> gix_error::TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let fixture = std::path::absolute(gix_testtools::scripted_fixture_read_only("windows-msys-git.sh")?)?;
        for installation in ["standard", "missing-config", "mounted"] {
            let bin = fixture.join(installation).join("usr/bin");
            assert_eq!(
                registered_bin(&bin),
                Some(bin.as_path()),
                "the MSYS Git package owns both its launcher and core binary"
            );
        }
        for installation in ["unregistered", "wrong-package", "incomplete", "duplicate", "no-core"] {
            assert_eq!(
                registered_bin(&fixture.join(installation).join("usr/bin")),
                None,
                "{installation} does not establish an unambiguous installed MSYS Git package"
            );
        }
        Ok(())
    }

    #[test]
    fn unix_paths_follow_the_selected_runtimes_mounts() -> gix_error::TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let fixture = std::path::absolute(gix_testtools::scripted_fixture_read_only("windows-msys-git.sh")?)?;
        for (installation, core, config) in [
            ("standard", "standard/usr/lib/git-core", "standard/etc/gitconfig"),
            ("mounted", "mounted-lib/git-core", "mounted-etc/gitconfig"),
        ] {
            let bin = fixture.join(installation).join("usr/bin");
            for (unix_path, expected) in [("/usr/lib/git-core", core), ("/etc/gitconfig", config)] {
                assert_eq!(
                    super::super::native_path_with(Path::new(unix_path), || Some(&bin)),
                    Some(fixture.join(expected)),
                    "the selected runtime's mount table controls {unix_path}"
                );
            }
        }
        Ok(())
    }
}
