use bstr::{BStr, BString};
use gix_path::{RelativePath, relative_path::Error};

#[cfg(not(windows))]
#[test]
fn absolute_paths_return_err() {
    let path_str: &str = "/refs/heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8a: &[u8; 11] = b"/refs/heads";
    let path_u8: &[u8] = &b"/refs/heads"[..];
    let path_bstring: BString = "/refs/heads".into();

    assert!(matches!(
        TryInto::<&RelativePath>::try_into(path_str),
        Err(Error::IsAbsolute)
    ));
    assert!(matches!(
        TryInto::<&RelativePath>::try_into(path_bstr),
        Err(Error::IsAbsolute)
    ));
    assert!(matches!(
        TryInto::<&RelativePath>::try_into(path_u8),
        Err(Error::IsAbsolute)
    ));
    assert!(matches!(
        TryInto::<&RelativePath>::try_into(path_u8a),
        Err(Error::IsAbsolute)
    ));
    assert!(matches!(
        TryInto::<&RelativePath>::try_into(&path_bstring),
        Err(Error::IsAbsolute)
    ));
}

#[cfg(windows)]
#[test]
fn absolute_paths_with_backslashes_return_err() {
    let path_str: &str = r"c:\refs\heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8: &[u8] = &b"c:\\refs\\heads"[..];
    let path_bstring: BString = r"c:\refs\heads".into();

    assert!(matches!(
        TryInto::<&RelativePath>::try_into(path_str),
        Err(Error::IsAbsolute)
    ));
    assert!(matches!(
        TryInto::<&RelativePath>::try_into(path_bstr),
        Err(Error::IsAbsolute)
    ));
    assert!(matches!(
        TryInto::<&RelativePath>::try_into(path_u8),
        Err(Error::IsAbsolute)
    ));
    assert!(matches!(
        TryInto::<&RelativePath>::try_into(&path_bstring),
        Err(Error::IsAbsolute)
    ));
}

#[test]
fn dots_in_paths_return_err() {
    let path_str: &str = "./heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8: &[u8] = &b"./heads"[..];
    let path_bstring: BString = "./heads".into();

    assert!(matches!(
        TryInto::<&RelativePath>::try_into(path_str),
        Err(Error::ContainsInvalidComponent(_))
    ));
    assert!(matches!(
        TryInto::<&RelativePath>::try_into(path_bstr),
        Err(Error::ContainsInvalidComponent(_))
    ));
    assert!(matches!(
        TryInto::<&RelativePath>::try_into(path_u8),
        Err(Error::ContainsInvalidComponent(_))
    ));
    assert!(matches!(
        TryInto::<&RelativePath>::try_into(&path_bstring),
        Err(Error::ContainsInvalidComponent(_))
    ));
}

#[test]
fn dots_in_paths_with_backslashes_return_err() {
    let path_str: &str = r".\heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8: &[u8] = &b".\\heads"[..];
    let path_bstring: BString = r".\heads".into();

    assert!(matches!(
        TryInto::<&RelativePath>::try_into(path_str),
        Err(Error::ContainsInvalidComponent(_))
    ));
    assert!(matches!(
        TryInto::<&RelativePath>::try_into(path_bstr),
        Err(Error::ContainsInvalidComponent(_))
    ));
    assert!(matches!(
        TryInto::<&RelativePath>::try_into(path_u8),
        Err(Error::ContainsInvalidComponent(_))
    ));
    assert!(matches!(
        TryInto::<&RelativePath>::try_into(&path_bstring),
        Err(Error::ContainsInvalidComponent(_))
    ));
}

#[test]
fn double_dots_in_paths_return_err() {
    let path_str: &str = "../heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8: &[u8] = &b"../heads"[..];
    let path_bstring: BString = "../heads".into();

    assert!(matches!(
        TryInto::<&RelativePath>::try_into(path_str),
        Err(Error::ContainsInvalidComponent(_))
    ));
    assert!(matches!(
        TryInto::<&RelativePath>::try_into(path_bstr),
        Err(Error::ContainsInvalidComponent(_))
    ));
    assert!(matches!(
        TryInto::<&RelativePath>::try_into(path_u8),
        Err(Error::ContainsInvalidComponent(_))
    ));
    assert!(matches!(
        TryInto::<&RelativePath>::try_into(&path_bstring),
        Err(Error::ContainsInvalidComponent(_))
    ));
}

#[test]
fn double_dots_in_paths_with_backslashes_return_err() {
    let path_str: &str = r"..\heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8: &[u8] = &b"..\\heads"[..];
    let path_bstring: BString = r"..\heads".into();

    assert!(matches!(
        TryInto::<&RelativePath>::try_into(path_str),
        Err(Error::ContainsInvalidComponent(_))
    ));
    assert!(matches!(
        TryInto::<&RelativePath>::try_into(path_bstr),
        Err(Error::ContainsInvalidComponent(_))
    ));
    assert!(matches!(
        TryInto::<&RelativePath>::try_into(path_u8),
        Err(Error::ContainsInvalidComponent(_))
    ));
    assert!(matches!(
        TryInto::<&RelativePath>::try_into(&path_bstring),
        Err(Error::ContainsInvalidComponent(_))
    ));
}
