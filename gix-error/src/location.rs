use std::{fmt, panic::Location};

pub(crate) fn write(f: &mut fmt::Formatter<'_>, location: &Location<'_>) -> fmt::Result {
    if cfg!(feature = "error-print-location") {
        f.write_str(", at ")?;
        write_path(f, location.file())?;
        write!(f, ":{}", location.line())?;
    }
    Ok(())
}

fn write_path(f: &mut impl fmt::Write, file: &str) -> fmt::Result {
    let (package, source) = local_path(file);
    f.write_str(package)?;
    for (index, component) in source.split('\\').enumerate() {
        if index != 0 {
            f.write_char('/')?;
        }
        f.write_str(component)?;
    }
    Ok(())
}

/// Location has no Cargo package metadata. Recognize conventional source directories;
/// exact package names for arbitrary layouts require the build owner to use --remap-path-prefix.
fn local_path(file: &str) -> (&str, &str) {
    let registry_path = file
        .rsplit_once("/registry/src/")
        .or_else(|| file.rsplit_once(r"\registry\src\"))
        .and_then(|(_, registry)| registry.split_once(['/', '\\']))
        .map(|(_, package)| package);
    let file = registry_path.unwrap_or(file);
    let mut package = "";
    let mut offset = 0;
    for component in file.split(['/', '\\']) {
        if offset == 0 && matches!(component, "src" | "tests" | "examples" | "benches") {
            return ("", file);
        }
        if matches!(component, "src" | "tests" | "examples" | "benches")
            && !matches!(package, "" | "." | ".." | "registry")
        {
            if registry_path.is_some() {
                package = without_version(package);
            }
            return (package, &file[offset - 1..]);
        }
        package = component;
        offset += component.len() + 1;
    }
    ("", file.rsplit(['/', '\\']).next().unwrap_or(file))
}

fn without_version(package: &str) -> &str {
    package
        .match_indices('-')
        .find_map(|(offset, _)| {
            let version = package[offset + 1..].split(['-', '+']).next()?;
            let mut numbers = version.split('.');
            (numbers.clone().count() == 3
                && numbers.all(|number| !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())))
            .then_some(&package[..offset])
        })
        .unwrap_or(package)
}

#[cfg(test)]
mod tests {
    #[test]
    fn paths_are_local_to_conventional_package_roots() {
        for (input, expected) in [
            ("gix-error/src/lib.rs", "gix-error/src/lib.rs"),
            ("/home/user/gitoxide/gix-error/src/lib.rs", "gix-error/src/lib.rs"),
            (
                "/home/user/gitoxide/gix-error/tests/error/main.rs",
                "gix-error/tests/error/main.rs",
            ),
            ("/checkout/crate/examples/main.rs", "crate/examples/main.rs"),
            ("/checkout/crate/benches/main.rs", "crate/benches/main.rs"),
            ("/checkout/crate/src/nested/src/main.rs", "crate/src/nested/src/main.rs"),
            (
                "/home/user/.cargo/registry/src/index.crates.io-hash/gix-url-0.39.0/src/parse.rs",
                "gix-url/src/parse.rs",
            ),
            (
                "/home/user/src/cargo/registry/src/index.crates.io-hash/gix-url-0.39.0/src/parse.rs",
                "gix-url/src/parse.rs",
            ),
            (
                "/cargo/registry/src/index.crates.io-hash/crate-2-name-1.2.3-alpha.1+build/src/lib.rs",
                "crate-2-name/src/lib.rs",
            ),
            (
                r"C:\Users\user\.cargo\registry\src\index.crates.io-hash\gix-url-0.39.0\src\parse.rs",
                "gix-url/src/parse.rs",
            ),
            (
                r"C:\checkout\gix-error\tests\error\main.rs",
                "gix-error/tests/error/main.rs",
            ),
            (r"gix-error\tests/error\main.rs", "gix-error/tests/error/main.rs"),
            (r"src\nested/main.rs", "src/nested/main.rs"),
            ("/checkout/crate-1.2.3/src/main.rs", "crate-1.2.3/src/main.rs"),
            ("src/main.rs", "src/main.rs"),
            ("../src/main.rs", "main.rs"),
            ("/src/main.rs", "main.rs"),
            ("/custom/layout/main.rs", "main.rs"),
            (r"C:\custom\layout\main.rs", "main.rs"),
            ("main.rs", "main.rs"),
            ("", ""),
        ] {
            let mut actual = String::new();
            super::write_path(&mut actual, input).expect("writing a source path to a String cannot fail");
            assert_eq!(actual, expected, "source path: {input}");
        }
    }
}
