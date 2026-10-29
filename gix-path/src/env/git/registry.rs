use std::{ffi::OsString, path::PathBuf};

use winreg::{
    RegKey,
    enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY},
};

const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";

/// Installer registrations also cover custom roots, without guessing directories on each drive.
pub(super) fn installation_bins() -> Vec<PathBuf> {
    let mut locations = Vec::new();
    // Prefer the current user's installations, and 64-bit entries within each hive.
    for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        let root = RegKey::predef(hive);
        for view in [KEY_WOW64_64KEY, KEY_WOW64_32KEY] {
            if let Some(bin) = root
                .open_subkey_with_flags(r"Software\GitForWindows", KEY_READ | view)
                .ok()
                .and_then(|key| key.get_value::<OsString, _>("InstallPath").ok())
                .and_then(absolute)
                .map(|root| root.join("cmd"))
            {
                push_unique(&mut locations, bin);
            }
            let Ok(uninstall) = root.open_subkey_with_flags(UNINSTALL, KEY_READ | view) else {
                continue;
            };
            // Qt's MSYS2 installer uses a generated product UUID as the key name.
            let mut names: Vec<_> = uninstall.enum_keys().filter_map(Result::ok).collect();
            names.sort();
            for name in names {
                let Some(bin) = uninstall
                    .open_subkey_with_flags(name, KEY_READ | view)
                    .ok()
                    .and_then(|key| msys2_bin(|name| key.get_value(name).ok()))
                else {
                    continue;
                };
                push_unique(&mut locations, bin);
            }
        }
    }
    locations
}

fn push_unique(locations: &mut Vec<PathBuf>, bin: PathBuf) {
    if !locations.contains(&bin) {
        locations.push(bin);
    }
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
