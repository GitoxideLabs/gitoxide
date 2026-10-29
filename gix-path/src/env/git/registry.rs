use std::{ffi::OsString, path::PathBuf};

use winreg::{
    RegKey,
    enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY},
};

const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";

/// Installer registrations also cover custom roots, without guessing directories on each drive.
pub(super) fn installation_bins() -> impl Iterator<Item = PathBuf> {
    let mut locations = Vec::new();
    // Prefer the current user's installations, and 64-bit entries within each hive.
    [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE]
        .into_iter()
        .flat_map(|hive| {
            [KEY_WOW64_64KEY, KEY_WOW64_32KEY].into_iter().flat_map(move |view| {
                registration_bins(
                    move || {
                        RegKey::predef(hive)
                            .open_subkey_with_flags(r"Software\GitForWindows", KEY_READ | view)
                            .ok()
                            .and_then(|key| key.get_value::<OsString, _>("InstallPath").ok())
                            .and_then(absolute)
                            .map(|root| root.join("cmd"))
                    },
                    move || {
                        RegKey::predef(hive)
                            .open_subkey_with_flags(UNINSTALL, KEY_READ | view)
                            .ok()
                            .into_iter()
                            .flat_map(move |uninstall| {
                                // Qt's MSYS2 installer uses a generated product UUID as the key name.
                                let mut names: Vec<_> = uninstall.enum_keys().filter_map(Result::ok).collect();
                                names.sort();
                                names.into_iter().filter_map(move |name| {
                                    uninstall
                                        .open_subkey_with_flags(name, KEY_READ | view)
                                        .ok()
                                        .and_then(|key| msys2_bin(|name| key.get_value(name).ok()))
                                })
                            })
                    },
                )
            })
        })
        .filter(move |bin| {
            if locations.contains(bin) {
                false
            } else {
                locations.push(bin.clone());
                true
            }
        })
}

fn registration_bins<I: IntoIterator<Item = PathBuf>>(
    git_for_windows: impl FnOnce() -> Option<PathBuf>,
    msys2: impl FnOnce() -> I,
) -> impl Iterator<Item = PathBuf> {
    std::iter::once_with(git_for_windows)
        .flatten()
        .chain(std::iter::once_with(msys2).flatten())
}

fn absolute(value: OsString) -> Option<PathBuf> {
    let root = PathBuf::from(value);
    root.is_absolute().then_some(root)
}

fn msys2_bin(value: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    if value("DisplayName")?.as_os_str() != "MSYS2" || value("Publisher")?.as_os_str() != "The MSYS2 Developers" {
        return None;
    }
    absolute(value("InstallLocation")?).map(|root| root.join("usr/bin"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registration_probes_are_deferred_until_needed() {
        let probes = std::cell::RefCell::new(Vec::new());
        let mut bins = registration_bins(
            || {
                probes.borrow_mut().push("git");
                Some(PathBuf::from("C:/Git/cmd"))
            },
            || {
                probes.borrow_mut().push("uninstall");
                ["C:/msys64/usr/bin", "D:/msys64/usr/bin"].into_iter().map(|bin| {
                    probes.borrow_mut().push(bin);
                    PathBuf::from(bin)
                })
            },
        );
        assert!(
            probes.borrow().is_empty(),
            "constructing candidates must not read the registry"
        );
        assert_eq!(bins.next(), Some(PathBuf::from("C:/Git/cmd")));
        assert_eq!(
            *probes.borrow(),
            ["git"],
            "finding Git for Windows must not open the uninstall key"
        );
        assert_eq!(bins.next(), Some(PathBuf::from("C:/msys64/usr/bin")));
        assert_eq!(
            *probes.borrow(),
            ["git", "uninstall", "C:/msys64/usr/bin"],
            "finding the first MSYS2 candidate must not inspect later registrations"
        );
        assert_eq!(bins.next(), Some(PathBuf::from("D:/msys64/usr/bin")));
        assert_eq!(bins.next(), None, "registration providers are only queried once");
    }

    #[test]
    fn msys2_registration_requires_its_identity_and_an_absolute_root() {
        let registration = |name: &str, publisher: &str, location: &str| {
            msys2_bin(|key| match key {
                "DisplayName" => Some(name.into()),
                "Publisher" => Some(publisher.into()),
                "InstallLocation" => Some(location.into()),
                _ => None,
            })
        };
        assert_eq!(
            registration("MSYS2", "The MSYS2 Developers", "D:/custom msys"),
            Some(PathBuf::from("D:/custom msys/usr/bin")),
            "the official installer can register a custom root"
        );
        for (name, publisher) in [("Other", "The MSYS2 Developers"), ("MSYS2", "Other")] {
            assert_eq!(
                registration(name, publisher, "D:/msys"),
                None,
                "unrelated products are ignored"
            );
        }
        for location in ["", "relative", "C:relative", "/rooted"] {
            assert_eq!(
                registration("MSYS2", "The MSYS2 Developers", location),
                None,
                "registry paths must not be resolved relative to the working directory"
            );
        }
        assert_eq!(msys2_bin(|_| None), None, "incomplete registrations are ignored");
    }
}
