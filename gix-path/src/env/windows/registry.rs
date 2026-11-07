use std::{ffi::OsString, path::PathBuf};

use winreg::{
    HKEY, RegKey,
    enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY},
};

const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";

/// Installer registrations also cover custom roots, without guessing directories on each drive.
pub(super) fn git_for_windows_bins() -> impl Iterator<Item = PathBuf> {
    registration_bins(|hive, view| {
        RegKey::predef(hive)
            .open_subkey_with_flags(r"Software\GitForWindows", KEY_READ | view)
            .ok()
            .and_then(|key| key.get_value::<OsString, _>("InstallPath").ok())
            .and_then(absolute)
            .map(|root| root.join("cmd"))
    })
}

/// Scan uninstall registrations only after callers exhaust more likely Git installations.
pub(super) fn msys2_bins() -> impl Iterator<Item = PathBuf> {
    registration_bins(|hive, view| {
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
    })
}

fn registration_bins<I: IntoIterator<Item = PathBuf>>(
    mut query: impl FnMut(HKEY, u32) -> I,
) -> impl Iterator<Item = PathBuf> {
    let mut locations = Vec::new();
    // Prefer the current user's installations, and 64-bit entries within each hive.
    [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE]
        .into_iter()
        .flat_map(|hive| {
            [KEY_WOW64_64KEY, KEY_WOW64_32KEY]
                .into_iter()
                .map(move |view| (hive, view))
        })
        .flat_map(move |(hive, view)| query(hive, view))
        .filter(move |bin| {
            if locations.contains(bin) {
                false
            } else {
                locations.push(bin.clone());
                true
            }
        })
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
    fn registry_views_are_lazy_ordered_and_deduplicated() {
        let probes = std::cell::RefCell::new(Vec::new());
        let mut bins = registration_bins(|hive, view| {
            probes.borrow_mut().push((hive, view));
            if hive == HKEY_CURRENT_USER {
                vec![PathBuf::from("C:/user/cmd")]
            } else if view == KEY_WOW64_64KEY {
                vec![PathBuf::from("C:/machine64/cmd"), PathBuf::from("D:/machine64/cmd")]
            } else {
                vec![PathBuf::from("C:/machine32/cmd")]
            }
        });
        assert!(
            probes.borrow().is_empty(),
            "constructing candidates must not read the registry"
        );
        assert_eq!(bins.next(), Some(PathBuf::from("C:/user/cmd")));
        assert_eq!(
            *probes.borrow(),
            [(HKEY_CURRENT_USER, KEY_WOW64_64KEY)],
            "finding the first user registration must not query later views"
        );
        assert_eq!(
            bins.next(),
            Some(PathBuf::from("C:/machine64/cmd")),
            "duplicate user registrations across views are skipped"
        );
        assert_eq!(
            *probes.borrow(),
            [
                (HKEY_CURRENT_USER, KEY_WOW64_64KEY),
                (HKEY_CURRENT_USER, KEY_WOW64_32KEY),
                (HKEY_LOCAL_MACHINE, KEY_WOW64_64KEY),
            ],
            "all user views precede the machine's 64-bit view"
        );
        assert_eq!(bins.next(), Some(PathBuf::from("D:/machine64/cmd")));
        assert_eq!(
            probes.borrow().len(),
            3,
            "consume the current view before querying another"
        );
        assert_eq!(bins.next(), Some(PathBuf::from("C:/machine32/cmd")));
        assert_eq!(
            probes.borrow().last(),
            Some(&(HKEY_LOCAL_MACHINE, KEY_WOW64_32KEY)),
            "the machine's 32-bit view has the lowest precedence"
        );
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
