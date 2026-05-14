use std::{
    fs, io,
    path::{Path, PathBuf},
};

use gix_odb::alternate;

pub fn alternate(
    objects_at: impl Into<PathBuf>,
    objects_to: impl Into<PathBuf>,
) -> Result<(PathBuf, PathBuf), io::Error> {
    alternate_with(objects_at, objects_to, None)
}

fn alternate_with(
    objects_at: impl Into<PathBuf>,
    objects_to: impl Into<PathBuf>,
    content_before_to: Option<&str>,
) -> Result<(PathBuf, PathBuf), io::Error> {
    let objects_to = objects_to.into();
    alternate_with_content(
        objects_at,
        objects_to.clone(),
        objects_to.to_str().expect("valid UTF-8").as_bytes().to_owned(),
        content_before_to,
    )
}

fn alternate_with_content(
    objects_at: impl Into<PathBuf>,
    objects_to: impl Into<PathBuf>,
    to_content: Vec<u8>,
    content_before_to: Option<&str>,
) -> Result<(PathBuf, PathBuf), io::Error> {
    let at = objects_at.into();
    let to = objects_to.into();
    let at_info = at.join("info");
    fs::create_dir_all(&at_info)?;
    fs::create_dir_all(&to)?;
    let contents = if let Some(content) = content_before_to {
        let mut c = vec![b'\n'];
        c.extend(content.as_bytes());
        c.extend(to_content);
        c
    } else {
        to_content
    };
    fs::write(at_info.join("alternates"), contents)?;
    Ok((at, to))
}

#[test]
fn circular_alternates_are_detected_with_relative_paths() -> crate::Result {
    let tmp = gix_testtools::tempfile::TempDir::new()?;
    let tmp = tmp.path().join("sub-dir");
    std::fs::create_dir(&tmp)?;
    let (from, _) = alternate(tmp.join("a"), tmp.join("b"))?;
    alternate_with_content(
        tmp.join("b"),
        tmp.join("..").join("a"),
        Path::new("..")
            .join("a")
            .to_str()
            .expect("valid UTF-8")
            .as_bytes()
            .to_owned(),
        None,
    )?;

    match alternate::resolve(from, &std::env::current_dir()?) {
        Err(alternate::Error::Cycle(chain)) => {
            assert_eq!(
                chain
                    .into_iter()
                    .map(|p| p.file_name().expect("non-root").to_str().expect("utf8").to_owned())
                    .collect::<Vec<_>>(),
                vec!["a", "b"]
            );
        }
        res => unreachable!("should be a specific kind of error: {:?}", res),
    }
    Ok(())
}

#[test]
fn single_link_with_comment_before_path_and_ansi_c_escape() -> crate::Result {
    let tmp = gix_testtools::tempfile::TempDir::new()?;
    let non_alternate = tmp.path().join("actual");

    let (from, to) = alternate_with(tmp.path().join("a"), non_alternate, Some("# comment\n"))?;
    let alternates = alternate::resolve(from, &std::env::current_dir()?)?;
    assert_eq!(alternates.len(), 1);
    assert_eq!(alternates[0], to);
    Ok(())
}

#[test]
fn no_alternate_in_first_objects_dir() -> crate::Result {
    let tmp = gix_testtools::tempfile::TempDir::new()?;
    assert!(alternate::resolve(tmp.path().to_owned(), &std::env::current_dir()?)?.is_empty());
    Ok(())
}
