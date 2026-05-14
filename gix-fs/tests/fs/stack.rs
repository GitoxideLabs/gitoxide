#![allow(clippy::join_absolute_paths)]
use std::path::{Path, PathBuf};

use gix_fs::Stack;

#[derive(Debug, Default, Eq, PartialEq)]
struct Record {
    push_dir_count: usize,
    dirs: Vec<PathBuf>,
    push: usize,
}

impl gix_fs::stack::Delegate for Record {
    fn push_directory(&mut self, stack: &Stack) -> std::io::Result<()> {
        self.push_dir_count += 1;
        self.dirs.push(stack.current().into());
        Ok(())
    }

    fn push(&mut self, _is_last_component: bool, _stack: &Stack) -> std::io::Result<()> {
        self.push += 1;
        Ok(())
    }

    fn pop_directory(&mut self) {
        self.dirs.pop();
    }
}

#[derive(Default)]
struct FailOnce {
    directory_to_fail_on: Option<PathBuf>,
    path_to_fail_on: Option<PathBuf>,
    failed: bool,
    directories: Vec<PathBuf>,
    popped_directories: Vec<PathBuf>,
}

impl gix_fs::stack::Delegate for FailOnce {
    fn push_directory(&mut self, stack: &Stack) -> std::io::Result<()> {
        if !self.failed && self.directory_to_fail_on.as_deref() == Some(stack.current_relative()) {
            self.failed = true;
            return Err(std::io::Error::other("failed to push directory"));
        }
        self.directories.push(stack.current_relative().to_owned());
        Ok(())
    }

    fn push(&mut self, _is_last_component: bool, stack: &Stack) -> std::io::Result<()> {
        if !self.failed && self.path_to_fail_on.as_deref() == Some(stack.current_relative()) {
            self.failed = true;
            return Err(std::io::Error::other("failed to push"));
        }
        Ok(())
    }

    fn pop_directory(&mut self) {
        self.popped_directories
            .push(self.directories.pop().expect("directory to pop"));
    }
}

fn p(s: &str) -> &Path {
    s.as_ref()
}

/// Just to learn the specialities of `Path::join()`, which boils down to `Path::push(component)`.
#[test]
#[cfg(windows)]
fn path_join_handling() {
    let looks_absolute = p("/absolute");
    assert!(
        looks_absolute.is_relative(),
        "on Windows, 'absolute' Linux paths are relative (and relative to the current drive)"
    );
    let bs_looks_absolute = p(r"\absolute");
    assert!(
        bs_looks_absolute.is_relative(),
        "on Windows, strange single-backslash paths are relative (and relative to the current drive)"
    );
    assert_eq!(
        p("relative").join(looks_absolute),
        looks_absolute,
        "relative + unix-absolute = unix-absolute - the relative path without a drive is replaced"
    );
    assert_eq!(
        p("relative").join(bs_looks_absolute),
        bs_looks_absolute,
        "relative + unix-absolute = unix-absolute - the relative path without a drive is replaced - backslashes aren't special here"
    );

    assert_eq!(
        p("c:").join("relative"),
        p("c:relative"),
        "drive + relative = relative to the drive-specific current directory"
    );
    assert_eq!(
        p(r"c:\").join("relative"),
        p(r"c:\relative"),
        "absolute + relative = joined result"
    );

    assert_eq!(
        p(r"\\?\base").join(looks_absolute),
        p(r"\\?\base\absolute"),
        "absolute1 + unix-absolute2 = joined result with backslash"
    );
    assert_eq!(
        p(r"\\.\base").join(looks_absolute),
        p(r"\\.\base\absolute"),
        "absolute1 + absolute2 = joined result with backslash (device namespace)"
    );
    assert_eq!(
        p(r"\\?\base").join(bs_looks_absolute),
        p(r"\\?\base\absolute"),
        "absolute1 + absolute2 = joined result"
    );
    assert_eq!(
        p(r"\\.\base").join(bs_looks_absolute),
        p(r"\\.\base\absolute"),
        "absolute1 + absolute2 = joined result (device namespace)"
    );

    assert_eq!(p("/").join("C:"), p("C:"), "unix-absolute + win-drive = win-drive");
    assert_eq!(
        p("d:/").join("C:"),
        p("C:"),
        "d-drive + c-drive-relative = c-drive-relative - C: is relative but not on D:"
    );
    assert_eq!(
        p(r"d:\").join(r"C:\"),
        p(r"C:\"),
        "d-drive-with-bs + c-drive-with-bs = c-drive-with-bs - nothing special happens with backslashes"
    );
    assert_eq!(
        p(r"c:\").join(r"\\.\"),
        p(r"\\.\"),
        "c-drive-with-bs + device-namespace-unc = device-namespace-unc"
    );
    assert_eq!(
        p("/").join("C:/"),
        p(r"C:\"),
        "unix-absolute + win-drive = win-drive, strangely enough it changed the trailing slash to backslash, so better not have trailing slashes"
    );
    assert_eq!(p("/").join(r"C:\"), p(r"C:\"), "unix-absolute + win-drive = win-drive");
    assert_eq!(
        p(r"\\.").join("C:"),
        p("C:"),
        r"device-namespace-unc + win-drive-relative = win-drive-relative - C: as a relative path is not the C: device, so this is not \\.\C:"
    );
    assert_eq!(p("relative").join("C:"), p("C:"), "relative + win-drive = win-drive");

    assert_eq!(
        p("/").join(r"\\localhost"),
        p(r"\localhost"),
        "unix-absolute + win-absolute-unc-host = strangely, single-backslashed host"
    );
    assert_eq!(
        p("relative").join(r"\\localhost"),
        p(r"\\localhost"),
        "relative + win-absolute-unc-host = win-absolute-unc-host"
    );
}

/// Just to learn the specialities of `Path::join()`, which boils down to `Path::push(component)`.
#[test]
#[cfg(not(windows))]
fn path_join_handling() {
    assert_eq!(
        p("relative").join("/absolute"),
        p("/absolute"),
        "relative + absolute = absolute"
    );

    assert_eq!(
        p("/").join("relative"),
        p("/relative"),
        "absolute + relative = joined result"
    );

    assert_eq!(
        p("/").join("/absolute"),
        p("/absolute"),
        "absolute1 + absolute2 = absolute2"
    );

    assert_eq!(p("/").join("C:"), p("/C:"), "absolute + win-drive = joined result");
    assert_eq!(p("/").join("C:/"), p("/C:/"), "absolute + win-absolute = joined result");
    assert_eq!(
        p("/").join(r"C:\"),
        p(r"/C:\"),
        "absolute + win-absolute = joined result"
    );
    assert_eq!(
        p("relative").join("C:"),
        p("relative/C:"),
        "relative + win-drive = joined result"
    );

    assert_eq!(
        p("/").join(r"\\localhost"),
        p(r"/\\localhost"),
        "absolute + win-absolute-unc-host = joined result"
    );
    assert_eq!(
        p("relative").join(r"\\localhost"),
        p(r"relative/\\localhost"),
        "relative + win-absolute-unc-host = joined result"
    );
}

#[test]
fn empty_paths_are_noop_if_no_path_was_pushed_before() {
    let root = PathBuf::from(".");
    let mut s = Stack::new(root.clone());

    let mut r = Record::default();
    s.make_relative_path_current("", &mut r).unwrap();
    assert_eq!(
        s.current_relative().to_string_lossy(),
        "",
        "it's fine to push an empty path to get a value for the stack root, once"
    );
}

#[test]
fn relative_components_are_invalid() {
    let root = PathBuf::from(".");
    let mut s = Stack::new(root.clone());

    let mut r = Record::default();
    let err = s.make_relative_path_current(p("a/.."), &mut r).unwrap_err();
    assert_eq!(
        err.to_string(),
        format!(
            "Input path {input:?} contains relative or absolute components",
            input = "a/.."
        )
    );

    s.make_relative_path_current(p("a/./b"), &mut r)
        .expect("dot is ignored");
    assert_eq!(
        r,
        Record {
            push_dir_count: 2,
            dirs: vec![".".into(), "./a".into()],
            push: 2,
        },
        "The `a` directory is pushed, and the leaf, for a total of 2 pushes"
    );
    assert_eq!(
        s.current().to_string_lossy(),
        if cfg!(windows) { r".\a\b" } else { "./a/b" },
        "dot is silently ignored"
    );
    s.make_relative_path_current(p("a//b/"), &mut r)
        .expect("multiple-slashes are ignored");
    assert_eq!(
        r,
        Record {
            push_dir_count: 2,
            dirs: vec![".".into(), "./a".into()],
            push: 2,
        },
        "nothing changed"
    );
}

#[test]
fn absolute_paths_are_invalid() -> crate::Result {
    let root = PathBuf::from(".");
    let mut s = Stack::new(root.clone());

    let mut r = Record::default();
    let err = s.make_relative_path_current(p("/"), &mut r).unwrap_err();
    assert_eq!(
        err.to_string(),
        r#"Input path "/" contains relative or absolute components"#,
        "a leading slash is always considered absolute"
    );
    s.make_relative_path_current("/", &mut r)?;
    assert_eq!(
        s.current(),
        s.root(),
        "as string this is a no-op as it's just split by /"
    );

    let err = s.make_relative_path_current("../breakout", &mut r).unwrap_err();
    assert_eq!(
        err.to_string(),
        r#"Input path "../breakout" contains relative or absolute components"#,
        "otherwise breakout attempts are detected"
    );
    s.make_relative_path_current(p("a/"), &mut r)?;
    assert_eq!(
        s.current(),
        p("./a/"),
        "trailing slashes aren't a problem at this stage, as they cannot cause a 'breakout'"
    );
    s.make_relative_path_current(p(r"b\"), &mut r)?;
    assert_eq!(
        s.current(),
        p(r"./b\"),
        "trailing backslashes are fine both on Windows and Unix - on Unix it's part of the filename"
    );

    #[cfg(windows)]
    {
        let err = s.make_relative_path_current(Path::new(r"\"), &mut r).unwrap_err();
        assert_eq!(
            err.to_string(),
            r#"Input path "\" contains relative or absolute components"#,
            "on Windows, backslashes are considered absolute and replace the base if it is relative, \
            hence they are forbidden."
        );

        let err = s.make_relative_path_current(Path::new("c:"), &mut r).unwrap_err();
        assert_eq!(
            err.to_string(),
            r#"Input path "c:" contains relative or absolute components"#,
            "on Windows, drive-letters without trailing backslash or slash are also absolute (even though they ought to be relative)"
        );
        let err = s.make_relative_path_current(Path::new(r"c:\"), &mut r).unwrap_err();
        assert_eq!(
            err.to_string(),
            r#"Input path "c:\" contains relative or absolute components"#,
            "on Windows, drive-letters are absolute, which is expected"
        );

        s.make_relative_path_current(Path::new("֍:"), &mut r)?;
        assert_eq!(
            s.current().to_string_lossy(),
            ".\\֍:",
            "on Windows, almost any unicode character will do as virtual drive-letter actually with `subst`, \
            but we just turn it into a presumably invalid path which is fine, i.e. we get a joined path"
        );
        let err = s
            .make_relative_path_current(Path::new(r"\\localhost\hello"), &mut r)
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            r#"Input path "\\localhost\hello" contains relative or absolute components"#,
            "there is UNC paths as well"
        );

        let err = s
            .make_relative_path_current(Path::new(r#"\\?\C:"#), &mut r)
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            r#"Input path "\\?\C:" contains relative or absolute components"#,
            "there is UNC paths as well, sometimes they look different"
        );
    }
    Ok(())
}

#[test]
fn delegate_calls_are_consistent() -> crate::Result {
    let root = PathBuf::from(".");
    let mut s = Stack::new(root.clone());

    assert_eq!(s.current(), root);
    assert_eq!(s.current_relative(), p(""));

    let mut r = Record::default();
    s.make_relative_path_current("a/b", &mut r)?;
    let mut dirs = vec![root.clone(), root.join("a")];
    assert_eq!(
        r,
        Record {
            push_dir_count: 2,
            dirs: dirs.clone(),
            push: 2,
        },
        "it pushes the root-directory first, then the intermediate one"
    );

    s.make_relative_path_current("a/b2", &mut r)?;
    assert_eq!(
        r,
        Record {
            push_dir_count: 2,
            dirs: dirs.clone(),
            push: 3,
        },
        "dirs remain the same as b2 is a leaf/file, hence the new `push`"
    );

    s.make_relative_path_current("c/d/e", &mut r)?;
    dirs.pop();
    dirs.extend([root.join("c"), root.join("c").join("d")]);
    assert_eq!(
        r,
        Record {
            push_dir_count: 4,
            dirs: dirs.clone(),
            push: 6,
        },
        "each directory is pushed individually, after popping 'a' which isn't included anymore"
    );

    dirs.push(root.join("c").join("d").join("x"));
    s.make_relative_path_current("c/d/x/z", &mut r)?;
    assert_eq!(
        r,
        Record {
            push_dir_count: 5,
            dirs: dirs.clone(),
            push: 8,
        },
        "a new path component is added, hence `push_dir + 1`, but two components are added in total"
    );

    dirs.drain(1..).count();
    s.make_relative_path_current("f", &mut r)?;
    assert_eq!(s.current_relative(), p("f"));
    assert_eq!(
        r,
        Record {
            push_dir_count: 5,
            dirs: dirs.clone(),
            push: 9,
        },
        "Now we only keep the root, as `f` is a leaf, hence `push + 1`"
    );

    dirs.push(root.join("x"));
    s.make_relative_path_current("x/z", &mut r)?;
    assert_eq!(
        r,
        Record {
            push_dir_count: 6,
            dirs: dirs.clone(),
            push: 11,
        },
        "a new directory is pushed, or two new components total, hence `push + 2`"
    );

    dirs.push(root.join("x").join("z"));
    s.make_relative_path_current("x/z/a", &mut r)?;
    assert_eq!(
        r,
        Record {
            push_dir_count: 7,
            dirs: dirs.clone(),
            push: 13,
        },
        "and another sub-directory is added after revalidating the cached leaf as a directory"
    );

    dirs.push(root.join("x").join("z").join("a"));
    dirs.push(root.join("x").join("z").join("a").join("b"));
    s.make_relative_path_current("x/z/a/b/c", &mut r)?;
    assert_eq!(
        r,
        Record {
            push_dir_count: 9,
            dirs: dirs.clone(),
            push: 16,
        },
        "and more subdirectories, two at once this time, after revalidating the cached leaf as a directory."
    );

    dirs.drain(1 /*root*/ + 1 /*x*/ + 1 /*x/z*/ ..).count();
    s.make_relative_path_current("x/z", &mut r)?;
    assert_eq!(
        r,
        Record {
            push_dir_count: 9,
            dirs: dirs.clone(),
            push: 16,
        },
        "this only pops components, and as x/z/a/ was previously a directory, x/z is still a directory"
    );
    assert_eq!(
        dirs.last(),
        Some(&PathBuf::from("./x/z")),
        "the stack is state so keeps thinking it's a directory which is consistent. Git does it differently though."
    );

    let err = s.make_relative_path_current(p(""), &mut r).unwrap_err();
    assert_eq!(
        err.to_string(),
        "empty inputs are not allowed",
        "this is to protect us from double-counting the root path next time a component is pushed, \
        and besides that really shouldn't happen"
    );

    s.make_relative_path_current("leaf", &mut r)?;
    dirs.drain(1..).count();
    assert_eq!(
        r,
        Record {
            push_dir_count: 9,
            dirs: dirs.clone(),
            push: 17,
        },
        "reset as much as possible, with just a leaf-component and the root directory"
    );

    s.make_relative_path_current(p("a//b"), &mut r)?;
    dirs.push(root.join("a"));
    assert_eq!(
        r,
        Record {
            push_dir_count: 10,
            dirs: dirs.clone(),
            push: 19,
        },
        "double-slashes are automatically cleaned, even though they shouldn't happen, it's not forbidden"
    );

    #[cfg(not(windows))]
    {
        s.make_relative_path_current(r"\/b", &mut r)?;
        dirs.pop();
        dirs.push(root.join(r"\"));
        assert_eq!(
            r,
            Record {
                push_dir_count: 11,
                dirs: dirs.clone(),
                push: 21,
            },
            "a backslash is a normal character outside of Windows, so it's fine to have it as component"
        );

        s.make_relative_path_current(r"\", &mut r)?;
        assert_eq!(
            r,
            Record {
                push_dir_count: 11,
                dirs: dirs.clone(),
                push: 21,
            },
        );
        assert_eq!(
            s.current().to_string_lossy(),
            r"./\",
            r"a backslash can also be a valid leaf component - here we only popped the 'b', leaving the \ 'directory'"
        );

        s.make_relative_path_current(r"\\", &mut r)?;
        dirs.pop();
        assert_eq!(
            r,
            Record {
                push_dir_count: 11,
                dirs: dirs.clone(),
                push: 22,
            },
        );
        assert_eq!(
            s.current().to_string_lossy(),
            r"./\\",
            "the backslash can also be an ordinary leaf, without the need for it to be a directory"
        );
    }

    #[cfg(windows)]
    {
        s.make_relative_path_current(Path::new(r"c\/d"), &mut r)?;
        dirs.pop();
        dirs.push(root.join("c"));
        assert_eq!(
            r,
            Record {
                push_dir_count: 11,
                dirs: dirs.clone(),
                push: 21,
            },
        );
        assert_eq!(
            s.current().to_string_lossy(),
            r".\c\d",
            "the backslash is a path-separator, and so is the `/`, which is turned into backslash"
        );
    }

    Ok(())
}

#[test]
fn failed_leaf_to_directory_transition_restores_leaf_state() -> crate::Result {
    let mut s = Stack::new(PathBuf::from("."));
    let mut r = FailOnce {
        directory_to_fail_on: Some(PathBuf::from("x/z")),
        ..Default::default()
    };

    s.make_relative_path_current("x/z", &mut r)?;
    let err = s.make_relative_path_current("x/z/a", &mut r).unwrap_err();
    assert_eq!(err.to_string(), "failed to push directory");
    assert_eq!(
        s.current_relative(),
        p("x/z"),
        "the failed directory transition must restore the current path"
    );
    let popped_directories = r.popped_directories.len();

    s.make_relative_path_current("x/q", &mut r)?;
    assert_eq!(s.current_relative(), p("x/q"));
    assert_eq!(
        r.popped_directories.len(),
        popped_directories,
        "the failed directory transition must not leave directory state to pop"
    );
    Ok(())
}

#[test]
fn failed_child_push_after_leaf_to_directory_transition_restores_directory_state() -> crate::Result {
    for (path_to_fail_on, directory_to_fail_on, expected_error) in [
        (Some("x/z/a"), None, "failed to push"),
        (None, Some("x/z/a"), "failed to push directory"),
    ] {
        let mut s = Stack::new(PathBuf::from("."));
        let mut r = FailOnce {
            path_to_fail_on: path_to_fail_on.map(PathBuf::from),
            directory_to_fail_on: directory_to_fail_on.map(PathBuf::from),
            ..Default::default()
        };

        s.make_relative_path_current("x/z", &mut r)?;
        let err = s.make_relative_path_current("x/z/a/b", &mut r).unwrap_err();
        assert_eq!(err.to_string(), expected_error);
        assert_eq!(
            s.current_relative(),
            p("x/z"),
            "the failed child push must restore the current relative path"
        );
        let popped_directories = r.popped_directories.len();

        s.make_relative_path_current("x/q", &mut r)?;
        assert_eq!(s.current_relative(), p("x/q"));
        assert_eq!(
            r.popped_directories[popped_directories],
            PathBuf::from("x/z"),
            "the successful leaf-to-directory transition remains the next directory state to pop"
        );
    }
    Ok(())
}
