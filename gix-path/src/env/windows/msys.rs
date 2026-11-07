use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

pub(super) struct Paths {
    pub core: PathBuf,
    pub config: PathBuf,
}

/// Share one conversion of the registered package's defaults, regardless of which is requested first.
pub(super) fn paths() -> Option<&'static Paths> {
    static PATHS: LazyLock<Option<Paths>> = LazyLock::new(|| paths_at(super::msys_bin()?));
    PATHS.as_ref()
}

fn paths_at(bin: &Path) -> Option<Paths> {
    let bin = registered_bin(bin)?;
    let [core, config] = super::native_paths(bin, [Path::new("/usr/lib/git-core"), Path::new("/etc/gitconfig")])?;
    Some(Paths { core, config })
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
    use super::{paths_at, registered_bin};
    use std::path::PathBuf;

    #[test]
    fn package_registration_establishes_the_layout() -> gix_error::TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let fixture = gix_testtools::scripted_fixture_read_only("windows-msys-git.sh")?;
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
        let fixture = gix_testtools::scripted_fixture_read_only("windows-msys-git.sh")?;
        for (installation, core, config) in [
            ("standard", "standard/usr/lib/git-core", "standard/etc/gitconfig"),
            ("mounted", "mounted-lib/git-core", "mounted-etc/gitconfig"),
        ] {
            let bin = fixture.join(installation).join("usr/bin");
            let paths = paths_at(&bin).expect("the registered package's runtime converts both paths");
            assert_eq!(
                paths.core,
                fixture.join(core),
                "the runtime's mount table controls /usr/lib"
            );
            assert_eq!(
                paths.config,
                fixture.join(config),
                "the runtime's mount table controls /etc"
            );
        }
        Ok(())
    }

    #[test]
    fn batched_paths_require_complete_absolute_results() {
        use super::super::parse_native_paths;

        assert_eq!(
            parse_native_paths::<2>(
                b"C:/a directory with spaces /git-core
D:/separate config/gitconfig
"
            ),
            Some([
                PathBuf::from("C:/a directory with spaces /git-core"),
                PathBuf::from("D:/separate config/gitconfig"),
            ]),
            "conversion preserves path order and whitespace"
        );
        for output in [
            &b""[..],
            b"C:/only-one\n",
            b"C:/core
D:/config",
            b"C:/core
D:/config
E:/extra
",
            b"C:/core
relative/config
",
            b"C:/core
\xff
",
        ] {
            assert_eq!(
                parse_native_paths::<2>(output),
                None,
                "incomplete, extra, relative or unrepresentable results cannot establish the two paths"
            );
        }
    }
}
