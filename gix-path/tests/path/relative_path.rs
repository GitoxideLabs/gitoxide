use bstr::{BStr, BString};
use gix_path::{RelativePath, relative_path::Error};

fn assert_validation<T>(result: Result<T, Error>, expected_message: &str, has_component_source: bool) {
    let err = result.err().expect("input should be invalid");
    assert_eq!(err.message, expected_message);
    assert_eq!(
        err.downcast_any_ref::<gix_validate::path::component::Error>().is_some(),
        has_component_source
    );
}

#[cfg(not(windows))]
#[test]
fn absolute_paths_return_err() {
    let path_str: &str = "/refs/heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8a: &[u8; 11] = b"/refs/heads";
    let path_u8: &[u8] = &b"/refs/heads"[..];
    let path_bstring: BString = "/refs/heads".into();

    let message = "A RelativePath is not allowed to be absolute";
    assert_validation(TryInto::<&RelativePath>::try_into(path_str), message, false);
    assert_validation(TryInto::<&RelativePath>::try_into(path_bstr), message, false);
    assert_validation(TryInto::<&RelativePath>::try_into(path_u8), message, false);
    assert_validation(TryInto::<&RelativePath>::try_into(path_u8a), message, false);
    assert_validation(TryInto::<&RelativePath>::try_into(&path_bstring), message, false);
}

#[cfg(windows)]
#[test]
fn absolute_paths_with_backslashes_return_err() {
    let path_str: &str = r"c:\refs\heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8: &[u8] = &b"c:\\refs\\heads"[..];
    let path_bstring: BString = r"c:\refs\heads".into();

    let message = "A RelativePath is not allowed to be absolute";
    assert_validation(TryInto::<&RelativePath>::try_into(path_str), message, false);
    assert_validation(TryInto::<&RelativePath>::try_into(path_bstr), message, false);
    assert_validation(TryInto::<&RelativePath>::try_into(path_u8), message, false);
    assert_validation(TryInto::<&RelativePath>::try_into(&path_bstring), message, false);
}

#[test]
fn dots_in_paths_return_err() {
    let path_str: &str = "./heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8: &[u8] = &b"./heads"[..];
    let path_bstring: BString = "./heads".into();

    let message = "Relative path contains an invalid component";
    assert_validation(TryInto::<&RelativePath>::try_into(path_str), message, true);
    assert_validation(TryInto::<&RelativePath>::try_into(path_bstr), message, true);
    assert_validation(TryInto::<&RelativePath>::try_into(path_u8), message, true);
    assert_validation(TryInto::<&RelativePath>::try_into(&path_bstring), message, true);
}

#[test]
fn dots_in_paths_with_backslashes_return_err() {
    let path_str: &str = r".\heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8: &[u8] = &b".\\heads"[..];
    let path_bstring: BString = r".\heads".into();

    let message = "Relative path contains an invalid component";
    assert_validation(TryInto::<&RelativePath>::try_into(path_str), message, true);
    assert_validation(TryInto::<&RelativePath>::try_into(path_bstr), message, true);
    assert_validation(TryInto::<&RelativePath>::try_into(path_u8), message, true);
    assert_validation(TryInto::<&RelativePath>::try_into(&path_bstring), message, true);
}

#[test]
fn double_dots_in_paths_return_err() {
    let path_str: &str = "../heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8: &[u8] = &b"../heads"[..];
    let path_bstring: BString = "../heads".into();

    let message = "Relative path contains an invalid component";
    assert_validation(TryInto::<&RelativePath>::try_into(path_str), message, true);
    assert_validation(TryInto::<&RelativePath>::try_into(path_bstr), message, true);
    assert_validation(TryInto::<&RelativePath>::try_into(path_u8), message, true);
    assert_validation(TryInto::<&RelativePath>::try_into(&path_bstring), message, true);
}

#[test]
fn double_dots_in_paths_with_backslashes_return_err() {
    let path_str: &str = r"..\heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8: &[u8] = &b"..\\heads"[..];
    let path_bstring: BString = r"..\heads".into();

    let message = "Relative path contains an invalid component";
    assert_validation(TryInto::<&RelativePath>::try_into(path_str), message, true);
    assert_validation(TryInto::<&RelativePath>::try_into(path_bstr), message, true);
    assert_validation(TryInto::<&RelativePath>::try_into(path_u8), message, true);
    assert_validation(TryInto::<&RelativePath>::try_into(&path_bstring), message, true);
}
