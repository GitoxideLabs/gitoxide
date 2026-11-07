use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

use super::git::{self, ConfigPaths};

pub(super) const EXE_NAME: &str = "git";
pub(super) const NULL_DEVICE: &str = "/dev/null";

pub(super) fn exe_invocation() -> &'static Path {
    Path::new(EXE_NAME)
}

pub(super) fn alternative_executable() -> Option<&'static Path> {
    None
}

pub(super) fn native_path(path: &Path) -> Option<PathBuf> {
    Some(path.to_owned())
}

pub(super) fn core_dir() -> Option<PathBuf> {
    git::core_dir_from_executable()
}

pub(super) fn config_paths_without_git() -> Option<ConfigPaths> {
    None
}

pub(super) fn system_prefix() -> Option<&'static Path> {
    Some(Path::new("/"))
}

pub(super) fn system_config() -> Option<&'static Path> {
    Some(Path::new("/etc/gitconfig"))
}

pub(super) fn shell() -> super::Shell {
    super::Shell {
        program: "/bin/sh".into(),
        needs_posix_mode: false,
    }
}

pub(super) fn associated_program(_name: &str) -> Option<PathBuf> {
    None
}

pub(super) fn command(executable: impl AsRef<OsStr>) -> Command {
    Command::new(executable)
}

pub(super) fn config_query_cwd() -> PathBuf {
    "/".into()
}
