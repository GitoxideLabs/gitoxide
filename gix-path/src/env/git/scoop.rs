use std::path::{Path, PathBuf};

/// Resolve Scoop's selected Git shim without launching it or consulting another installation.
pub(super) fn target(executable: &Path) -> Option<PathBuf> {
    if !executable.is_absolute()
        || !executable
            .file_name()?
            .as_encoded_bytes()
            .eq_ignore_ascii_case(b"git.exe")
        || !executable
            .parent()?
            .file_name()?
            .as_encoded_bytes()
            .eq_ignore_ascii_case(b"shims")
    {
        return None;
    }
    let source = std::fs::read_to_string(executable.with_extension("shim")).ok()?;
    let target = target_from_shim(&source)?;
    target.is_file().then(|| target.to_owned())
}

fn target_from_shim(source: &str) -> Option<&Path> {
    // Scoop writes UTF-8 `path = "..."`, followed by `args = ...` for argument-bearing shims.
    // Only a plain forwarding shim establishes where Git will find its runtime and config.
    let mut lines = source
        .trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let value = lines.next()?.strip_prefix("path = \"")?.strip_suffix('"')?;
    if lines.next().is_some() || value.contains('"') {
        return None;
    }
    let path = Path::new(value);
    (path.is_absolute() && path.file_name()?.as_encoded_bytes().eq_ignore_ascii_case(b"git.exe")).then_some(path)
}

#[cfg(test)]
mod tests {
    use super::{target, target_from_shim};
    use std::path::Path;

    #[test]
    fn plain_shim_targets() {
        for source in [
            "path = \"C:/custom scoop/apps/git/current/cmd/git.exe\"",
            "\u{feff}path = \"C:/custom scoop/apps/git/current/cmd/git.exe\"\r\n",
            "
                path = \"C:/custom scoop/apps/git/current/cmd/git.exe\"
            ",
        ] {
            assert_eq!(
                target_from_shim(source),
                Some(Path::new("C:/custom scoop/apps/git/current/cmd/git.exe")),
                "Scoop's plain UTF-8 sidecar identifies the selected installation"
            );
        }
        assert_eq!(
            target_from_shim(r#"path = "C:\custom scoop\apps\git\2.56.0.4\cmd\GIT.EXE""#),
            Some(Path::new(r"C:\custom scoop\apps\git\2.56.0.4\cmd\GIT.EXE")),
            "versioned targets work without Scoop's current junction"
        );
    }

    #[test]
    fn unsupported_shims_fall_back_to_execution() {
        for source in [
            "",
            r#"path = """#,
            r#"path = "relative/cmd/git.exe""#,
            r#"path = "C:relative/cmd/git.exe""#,
            r#"path = "/rooted/cmd/git.exe""#,
            r#"path = "C:/Git/cmd/other.exe""#,
            r#"path = C:/Git/cmd/git.exe"#,
            r#"path = "C:/Git/cmd/git.exe" trailing"#,
            r#"path = "C:/Git"/cmd/git.exe""#,
            "path = \"C:/Git/cmd/git.exe\"
             args = --exec-path=C:/other/libexec/git-core",
            "path = \"C:/Git/cmd/git.exe\"
             path = \"C:/other/cmd/git.exe\"",
            "path = \"C:/Git/cmd/git.exe\"
             unknown = value",
        ] {
            assert_eq!(
                target_from_shim(source),
                None,
                "only an unambiguous forwarding shim can establish Git's location: {source:?}"
            );
        }
    }

    #[test]
    fn selected_shim_requires_an_existing_target() -> gix_error::TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let fixture = std::path::absolute(gix_testtools::scripted_fixture_read_only("windows-scoop-shims.sh")?)?;
        for (installation, version) in [("scoop", "current"), ("no-junction", "2.56.0")] {
            let root = fixture.join(installation);
            assert_eq!(
                target(&root.join("SHIMS/git.exe")),
                Some(root.join("apps/git").join(version).join("cmd/git.exe")),
                "an empty executable cannot be launched, but its sidecar identifies the installation"
            );
        }
        for case in ["args", "stale", "absent"] {
            assert_eq!(
                target(&fixture.join(case).join("shims/git.exe")),
                None,
                "{case} cannot establish the selected Git installation"
            );
        }
        Ok(())
    }

    #[test]
    fn only_absolute_git_shims_are_recognized() {
        for executable in [
            "shims/git.exe",
            "C:shims/git.exe",
            "C:/bin/git.exe",
            "C:/shims/other.exe",
        ] {
            assert_eq!(
                target(Path::new(executable)),
                None,
                "the shortcut is restricted to an absolute selected Git shim"
            );
        }
    }
}
