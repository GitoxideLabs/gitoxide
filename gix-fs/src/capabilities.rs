// TODO: tests
use std::path::Path;

use crate::Capabilities;

#[cfg(windows)]
impl Default for Capabilities {
    fn default() -> Self {
        Capabilities {
            precompose_unicode: false,
            ignore_case: true,
            executable_bit: false,
            symlink: false,
        }
    }
}

#[cfg(target_os = "macos")]
impl Default for Capabilities {
    fn default() -> Self {
        Capabilities {
            precompose_unicode: true,
            ignore_case: true,
            executable_bit: true,
            symlink: true,
        }
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
impl Default for Capabilities {
    fn default() -> Self {
        Capabilities {
            precompose_unicode: false,
            ignore_case: false,
            executable_bit: true,
            symlink: true,
        }
    }
}

enum Dir {
    IsGit,
    IsArbitrary,
}

impl Capabilities {
    /// Try to determine all values in this context by probing them in the given `git_dir`, which
    /// should be on the file system the git repository is located on.
    /// `git_dir` is a typical git repository, expected to be populated with the typical files like `config`.
    ///
    /// All errors are ignored and interpreted on top of the default for the platform the binary is compiled for.
    pub fn probe(git_dir: &Path) -> Self {
        Self::probe_inner(git_dir, Dir::IsGit)
    }

    /// Just like [`Self::probe()`], but doesn't expect any git-specific files like `.git/config` to exist.
    ///
    /// Consider using this on worktree directories separately as they may be on a different filesystem
    /// which thus may have different capabilities.
    /// The capabilities of the git-directory is important for loose references, and the worktree directory is
    /// of importance for linked worktrees and their checkouts.
    pub fn probe_dir(dir: &Path) -> Self {
        Self::probe_inner(dir, Dir::IsArbitrary)
    }

    fn probe_inner(dir: &Path, mode: Dir) -> Self {
        let ctx = Capabilities::default();
        Capabilities {
            symlink: Self::probe_symlink(dir).unwrap_or(ctx.symlink),
            ignore_case: Self::probe_ignore_case(dir, mode).unwrap_or(ctx.ignore_case),
            precompose_unicode: Self::probe_precompose_unicode(dir).unwrap_or(ctx.precompose_unicode),
            executable_bit: Self::probe_file_mode(dir).unwrap_or(ctx.executable_bit),
        }
    }

    #[cfg(unix)]
    fn probe_file_mode(root: &Path) -> std::io::Result<bool> {
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};

        // First check that we can create an executable file, then check that we
        // can change the executable bit.
        // The equivalent test by git itself is here:
        // https://github.com/git/git/blob/f0ef5b6d9bcc258e4cbef93839d1b7465d5212b9/setup.c#L2367-L2379
        let rand = gix_utils::rng::usize(..);
        let test_path = root.join(format!("_test_executable_bit{rand}"));
        let res = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o777)
            .open(&test_path)
            .and_then(|file| {
                let old_mode = file.metadata()?.mode();
                let is_executable = old_mode & 0o100 == 0o100;
                let exe_bit_flip_works_in_filesystem = {
                    let toggled_exe_bit = old_mode ^ 0o100;
                    match file.set_permissions(PermissionsExt::from_mode(toggled_exe_bit)) {
                        Ok(()) => toggled_exe_bit == file.metadata()?.mode(),
                        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => false,
                        Err(err) => return Err(err),
                    }
                };
                Ok(is_executable && exe_bit_flip_works_in_filesystem)
            });
        std::fs::remove_file(test_path)?;
        res
    }

    #[cfg(not(unix))]
    fn probe_file_mode(_root: &Path) -> std::io::Result<bool> {
        Ok(false)
    }

    fn probe_ignore_case(git_dir: &Path, mode: Dir) -> std::io::Result<bool> {
        match mode {
            Dir::IsGit => {
                for (probe_name, canonical_name) in [("hEaD", "HEAD"), ("cOnFiG", "config")] {
                    match std::fs::metadata(git_dir.join(probe_name)) {
                        Ok(_) => return Ok(true),
                        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                            if git_dir.join(canonical_name).exists() {
                                return Ok(false);
                            }
                        }
                        Err(err) => return Err(err),
                    }
                }
                Ok(false)
            }
            Dir::IsArbitrary => std::fs::metadata(git_dir.join("cOnFiG")).map(|_| true).or_else(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    if git_dir.join("config").exists() {
                        return Ok(false);
                    }

                    use std::io::ErrorKind;
                    let (mut lower, mut mixed) = (None, None);

                    let res = (|| {
                        let first = git_dir.join("config");
                        let second = git_dir.join("cOnFiG");
                        std::fs::OpenOptions::new().create_new(true).write(true).open(&first)?;
                        lower = Some(first);

                        match std::fs::OpenOptions::new().create_new(true).write(true).open(&second) {
                            Ok(_) => {
                                mixed = Some(second);
                                Ok(false)
                            }
                            Err(err) if err.kind() == ErrorKind::AlreadyExists => Ok(true),
                            Err(err) => Err(err),
                        }
                    })();

                    for file_to_remove in lower.into_iter().chain(mixed) {
                        std::fs::remove_file(file_to_remove).ok();
                    }
                    res
                } else {
                    Err(err)
                }
            }),
        }
    }

    fn probe_precompose_unicode(root: &Path) -> std::io::Result<bool> {
        let rand = gix_utils::rng::usize(..);
        let precomposed = format!("ä{rand}");
        let decomposed = format!("a\u{308}{rand}");

        let precomposed = root.join(precomposed);
        std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&precomposed)?;
        let res = root.join(decomposed).symlink_metadata().map(|_| true);
        std::fs::remove_file(precomposed)?;
        res
    }

    /// Check symlink support by creating and removing a temporary link in `root`.
    /// An unsuccessful creation yields `false`. Errors inspecting or removing the link are returned.
    pub fn probe_symlink(root: &Path) -> std::io::Result<bool> {
        let rand = gix_utils::rng::usize(..);
        let link_path = root.join(format!("__file_link{rand}"));
        if crate::symlink::create("dangling".as_ref(), &link_path).is_err() {
            return Ok(false);
        }

        let res = std::fs::symlink_metadata(&link_path).map(|m| m.file_type().is_symlink());
        crate::symlink::remove(&link_path).or_else(|_| std::fs::remove_file(&link_path))?;
        res
    }
}

#[cfg(target_os = "wasi")]
impl Default for Capabilities {
    fn default() -> Self {
        Capabilities {
            precompose_unicode: false,
            ignore_case: false,
            executable_bit: false,
            symlink: true,
        }
    }
}

#[cfg(not(any(windows, unix, target_os = "wasi")))]
impl Default for Capabilities {
    fn default() -> Self {
        Capabilities {
            precompose_unicode: false,
            ignore_case: false,
            executable_bit: false,
            symlink: false,
        }
    }
}
