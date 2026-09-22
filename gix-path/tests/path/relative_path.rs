use bstr::{BStr, BString};
use gix_error::ExnMessageResult;
use gix_path::RelativePath;

fn assert_validation<T>(result: ExnMessageResult<T>, has_component_source: bool) -> gix_error::Exn<gix_error::Message> {
    let err = result.err().expect("input should be invalid");
    assert_eq!(
        err.downcast_any_ref::<gix_validate::path::component::Error>().is_some(),
        has_component_source
    );
    err
}

#[cfg(not(windows))]
#[test]
fn absolute_paths_return_err() {
    let path_str: &str = "/refs/heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8a: &[u8; 11] = b"/refs/heads";
    let path_u8: &[u8] = &b"/refs/heads"[..];
    let path_bstring: BString = "/refs/heads".into();

    insta::assert_debug_snapshot!(assert_validation(TryInto::<&RelativePath>::try_into(path_str), false), "absolute paths return err", @"A RelativePath is not allowed to be absolute");
    insta::assert_debug_snapshot!(assert_validation(TryInto::<&RelativePath>::try_into(path_bstr), false), "absolute paths return err", @"A RelativePath is not allowed to be absolute");
    insta::assert_debug_snapshot!(assert_validation(TryInto::<&RelativePath>::try_into(path_u8), false), "absolute paths return err", @"A RelativePath is not allowed to be absolute");
    insta::assert_debug_snapshot!(assert_validation(TryInto::<&RelativePath>::try_into(path_u8a), false), "absolute paths return err", @"A RelativePath is not allowed to be absolute");
    insta::assert_debug_snapshot!(assert_validation(TryInto::<&RelativePath>::try_into(&path_bstring), false), "absolute paths return err", @"A RelativePath is not allowed to be absolute");
}

#[cfg(windows)]
#[test]
fn absolute_paths_with_backslashes_return_err() {
    let path_str: &str = r"c:\refs\heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8: &[u8] = &b"c:\\refs\\heads"[..];
    let path_bstring: BString = r"c:\refs\heads".into();

    insta::assert_debug_snapshot!(assert_validation(TryInto::<&RelativePath>::try_into(path_str), false), "Windows absolute paths cannot be relative paths", @"A RelativePath is not allowed to be absolute");
    insta::assert_debug_snapshot!(assert_validation(TryInto::<&RelativePath>::try_into(path_bstr), false), "Windows absolute paths cannot be relative paths", @"A RelativePath is not allowed to be absolute");
    insta::assert_debug_snapshot!(assert_validation(TryInto::<&RelativePath>::try_into(path_u8), false), "Windows absolute paths cannot be relative paths", @"A RelativePath is not allowed to be absolute");
    insta::assert_debug_snapshot!(assert_validation(TryInto::<&RelativePath>::try_into(&path_bstring), false), "Windows absolute paths cannot be relative paths", @"A RelativePath is not allowed to be absolute");
}

#[test]
fn dots_in_paths_return_err() {
    let path_str: &str = "./heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8: &[u8] = &b"./heads"[..];
    let path_bstring: BString = "./heads".into();

    insta::assert_debug_snapshot!(assert_validation(TryInto::<&RelativePath>::try_into(path_str), true), "dots in paths return err", @r#"
    Relative path contains an invalid component, "input"="."
    |
    └─ Relative components '.' and '..' are disallowed
    "#);
    insta::assert_debug_snapshot!(assert_validation(TryInto::<&RelativePath>::try_into(path_bstr), true), "dots in paths return err", @r#"
    Relative path contains an invalid component, "input"="."
    |
    └─ Relative components '.' and '..' are disallowed
    "#);
    insta::assert_debug_snapshot!(assert_validation(TryInto::<&RelativePath>::try_into(path_u8), true), "dots in paths return err", @r#"
    Relative path contains an invalid component, "input"="."
    |
    └─ Relative components '.' and '..' are disallowed
    "#);
    insta::assert_debug_snapshot!(assert_validation(TryInto::<&RelativePath>::try_into(&path_bstring), true), "dots in paths return err", @r#"
    Relative path contains an invalid component, "input"="."
    |
    └─ Relative components '.' and '..' are disallowed
    "#);
}

#[test]
fn dots_in_paths_with_backslashes_return_err() {
    let path_str: &str = r".\heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8: &[u8] = path_str.as_bytes();
    let path_bstring: BString = path_str.into();

    insta::allow_duplicates! {
        for err in [
            assert_validation(TryInto::<&RelativePath>::try_into(path_str), true),
            assert_validation(TryInto::<&RelativePath>::try_into(path_bstr), true),
            assert_validation(TryInto::<&RelativePath>::try_into(path_u8), true),
            assert_validation(TryInto::<&RelativePath>::try_into(&path_bstring), true),
        ] {
            #[cfg(windows)]
            insta::assert_debug_snapshot!(err, "Windows treats backslashes as separators", @r#"
            Relative path contains an invalid component, "input"="."
            |
            └─ Relative components '.' and '..' are disallowed
            "#);
            #[cfg(not(windows))]
            insta::assert_debug_snapshot!(err, "backslashes are rejected inside Unix path components", @r#"
            Relative path contains an invalid component, "input"=".\\heads"
            |
            └─ Path separators like / or \ are not allowed
            "#);
        }
    };
}

#[test]
fn double_dots_in_paths_return_err() {
    let path_str: &str = "../heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8: &[u8] = &b"../heads"[..];
    let path_bstring: BString = "../heads".into();

    insta::assert_debug_snapshot!(assert_validation(TryInto::<&RelativePath>::try_into(path_str), true), "double dots in paths return err", @r#"
    Relative path contains an invalid component, "input"=".."
    |
    └─ Relative components '.' and '..' are disallowed
    "#);
    insta::assert_debug_snapshot!(assert_validation(TryInto::<&RelativePath>::try_into(path_bstr), true), "double dots in paths return err", @r#"
    Relative path contains an invalid component, "input"=".."
    |
    └─ Relative components '.' and '..' are disallowed
    "#);
    insta::assert_debug_snapshot!(assert_validation(TryInto::<&RelativePath>::try_into(path_u8), true), "double dots in paths return err", @r#"
    Relative path contains an invalid component, "input"=".."
    |
    └─ Relative components '.' and '..' are disallowed
    "#);
    insta::assert_debug_snapshot!(assert_validation(TryInto::<&RelativePath>::try_into(&path_bstring), true), "double dots in paths return err", @r#"
    Relative path contains an invalid component, "input"=".."
    |
    └─ Relative components '.' and '..' are disallowed
    "#);
}

#[test]
fn double_dots_in_paths_with_backslashes_return_err() {
    let path_str: &str = r"..\heads";
    let path_bstr: &BStr = path_str.into();
    let path_u8: &[u8] = path_str.as_bytes();
    let path_bstring: BString = path_str.into();

    insta::allow_duplicates! {
        for err in [
            assert_validation(TryInto::<&RelativePath>::try_into(path_str), true),
            assert_validation(TryInto::<&RelativePath>::try_into(path_bstr), true),
            assert_validation(TryInto::<&RelativePath>::try_into(path_u8), true),
            assert_validation(TryInto::<&RelativePath>::try_into(&path_bstring), true),
        ] {
            #[cfg(windows)]
            insta::assert_debug_snapshot!(err, "Windows treats backslashes as separators", @r#"
            Relative path contains an invalid component, "input"=".."
            |
            └─ Relative components '.' and '..' are disallowed
            "#);
            #[cfg(not(windows))]
            insta::assert_debug_snapshot!(err, "backslashes are rejected inside Unix path components", @r#"
            Relative path contains an invalid component, "input"="..\\heads"
            |
            └─ Path separators like / or \ are not allowed
            "#);
        }
    };
}
