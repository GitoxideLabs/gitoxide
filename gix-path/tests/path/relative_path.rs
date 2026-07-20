use bstr::{BStr, BString};
use gix_path::RelativePath;

#[cfg(not(windows))]
#[test]
fn absolute_paths_return_err() {
    let path_str: &str = "/refs/heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8a: &[u8; 11] = b"/refs/heads";
    let path_u8: &[u8] = &b"/refs/heads"[..];
    let path_bstring: BString = "/refs/heads".into();

    assert!(
        TryInto::<&RelativePath>::try_into(path_str)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("A RelativePath is not allowed to be absolute"),
        "absolute paths are rejected"
    );
    assert!(
        TryInto::<&RelativePath>::try_into(path_bstr)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("A RelativePath is not allowed to be absolute"),
        "absolute paths are rejected"
    );
    assert!(
        TryInto::<&RelativePath>::try_into(path_u8)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("A RelativePath is not allowed to be absolute"),
        "absolute paths are rejected"
    );
    assert!(
        TryInto::<&RelativePath>::try_into(path_u8a)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("A RelativePath is not allowed to be absolute"),
        "absolute paths are rejected"
    );
    assert!(
        TryInto::<&RelativePath>::try_into(&path_bstring)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("A RelativePath is not allowed to be absolute"),
        "absolute paths are rejected"
    );
}

#[cfg(windows)]
#[test]
fn absolute_paths_with_backslashes_return_err() {
    let path_str: &str = r"c:\refs\heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8: &[u8] = &b"c:\\refs\\heads"[..];
    let path_bstring: BString = r"c:\refs\heads".into();

    assert!(
        TryInto::<&RelativePath>::try_into(path_str)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("A RelativePath is not allowed to be absolute"),
        "absolute paths are rejected"
    );
    assert!(
        TryInto::<&RelativePath>::try_into(path_bstr)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("A RelativePath is not allowed to be absolute"),
        "absolute paths are rejected"
    );
    assert!(
        TryInto::<&RelativePath>::try_into(path_u8)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("A RelativePath is not allowed to be absolute"),
        "absolute paths are rejected"
    );
    assert!(
        TryInto::<&RelativePath>::try_into(&path_bstring)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("A RelativePath is not allowed to be absolute"),
        "absolute paths are rejected"
    );
}

#[test]
fn dots_in_paths_return_err() {
    let path_str: &str = "./heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8: &[u8] = &b"./heads"[..];
    let path_bstring: BString = "./heads".into();

    assert!(
        TryInto::<&RelativePath>::try_into(path_str)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("The path contains an invalid component"),
        "invalid components are rejected"
    );
    assert!(
        TryInto::<&RelativePath>::try_into(path_bstr)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("The path contains an invalid component"),
        "invalid components are rejected"
    );
    assert!(
        TryInto::<&RelativePath>::try_into(path_u8)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("The path contains an invalid component"),
        "invalid components are rejected"
    );
    assert!(
        TryInto::<&RelativePath>::try_into(&path_bstring)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("The path contains an invalid component"),
        "invalid components are rejected"
    );
}

#[test]
fn dots_in_paths_with_backslashes_return_err() {
    let path_str: &str = r".\heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8: &[u8] = &b".\\heads"[..];
    let path_bstring: BString = r".\heads".into();

    assert!(
        TryInto::<&RelativePath>::try_into(path_str)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("The path contains an invalid component"),
        "invalid components are rejected"
    );
    assert!(
        TryInto::<&RelativePath>::try_into(path_bstr)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("The path contains an invalid component"),
        "invalid components are rejected"
    );
    assert!(
        TryInto::<&RelativePath>::try_into(path_u8)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("The path contains an invalid component"),
        "invalid components are rejected"
    );
    assert!(
        TryInto::<&RelativePath>::try_into(&path_bstring)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("The path contains an invalid component"),
        "invalid components are rejected"
    );
}

#[test]
fn double_dots_in_paths_return_err() {
    let path_str: &str = "../heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8: &[u8] = &b"../heads"[..];
    let path_bstring: BString = "../heads".into();

    assert!(
        TryInto::<&RelativePath>::try_into(path_str)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("The path contains an invalid component"),
        "invalid components are rejected"
    );
    assert!(
        TryInto::<&RelativePath>::try_into(path_bstr)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("The path contains an invalid component"),
        "invalid components are rejected"
    );
    assert!(
        TryInto::<&RelativePath>::try_into(path_u8)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("The path contains an invalid component"),
        "invalid components are rejected"
    );
    assert!(
        TryInto::<&RelativePath>::try_into(&path_bstring)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("The path contains an invalid component"),
        "invalid components are rejected"
    );
}

#[test]
fn double_dots_in_paths_with_backslashes_return_err() {
    let path_str: &str = r"..\heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8: &[u8] = &b"..\\heads"[..];
    let path_bstring: BString = r"..\heads".into();

    assert!(
        TryInto::<&RelativePath>::try_into(path_str)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("The path contains an invalid component"),
        "invalid components are rejected"
    );
    assert!(
        TryInto::<&RelativePath>::try_into(path_bstr)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("The path contains an invalid component"),
        "invalid components are rejected"
    );
    assert!(
        TryInto::<&RelativePath>::try_into(path_u8)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("The path contains an invalid component"),
        "invalid components are rejected"
    );
    assert!(
        TryInto::<&RelativePath>::try_into(&path_bstring)
            .err()
            .map(|err| err.to_string())
            .expect("conversion must fail")
            .starts_with("The path contains an invalid component"),
        "invalid components are rejected"
    );
}
