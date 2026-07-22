use std::path::Path;

use bstr::ByteSlice;
use gix_url::{expand_path, expand_path::ForUser};

#[cfg(windows)]
fn expected_path() -> std::path::PathBuf {
    Path::new(r"C:\UserProfiles\byron\hello\git").into()
}

#[cfg(not(windows))]
fn expected_path() -> std::path::PathBuf {
    Path::new("/home/byron/hello/git").into()
}

#[cfg(windows)]
fn user_home(name: &str) -> std::path::PathBuf {
    Path::new(r"C:\").join("UserProfiles").join(name)
}

#[cfg(not(windows))]
fn user_home(name: &str) -> std::path::PathBuf {
    #[cfg(not(windows))]
    format!("/home/{name}").into()
}

#[test]
fn without_username() -> crate::Result {
    let (user, resolved_path) = expand_path::parse(b"/~/hello/git".as_bstr())?;
    let resolved_path = expand_path::with(user.as_ref(), resolved_path.as_ref(), |user: &ForUser| match user {
        ForUser::Current => Some(user_home("byron")),
        ForUser::Name(name) => Some(format!("/home/{name}").into()),
    })?;
    assert_eq!(resolved_path, expected_path());
    Ok(())
}

#[test]
fn with_username() -> crate::Result {
    let (user, resolved_path) = expand_path::parse(b"/~byron/hello/git".as_bstr())?;
    let resolved_path = expand_path::with(user.as_ref(), resolved_path.as_ref(), |user: &ForUser| match user {
        ForUser::Current => unreachable!("we have a name"),
        ForUser::Name(name) => Some(user_home(name.to_str_lossy().as_ref())),
    })?;
    assert_eq!(resolved_path, expected_path());
    Ok(())
}
