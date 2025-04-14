use gix_actor::Signature;
use gix_object::bstr::ByteSlice;
use gix_testtools::tempfile::TempDir;

use super::*;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Convert a hexadecimal hash into its corresponding `ObjectId` or _panic_.
fn hex_to_id(hex: &str) -> gix_hash::ObjectId {
    gix_hash::ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex")
}

fn empty_store(writemode: WriteReflog) -> Result<(TempDir, file::Store)> {
    let dir = TempDir::new()?;
    let store = file::Store::at(
        dir.path().into(),
        crate::store::init::Options {
            write_reflog: writemode,
            ..Default::default()
        },
    );
    Ok((dir, store))
}

fn reflog_lines(store: &file::Store, name: &str, buf: &mut Vec<u8>) -> Result<Vec<crate::log::Line>> {
    store
        .reflog_iter(name, buf)?
        .expect("existing reflog")
        .map(|l| l.map(crate::log::Line::from))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

const WRITE_MODES: &[WriteReflog] = &[WriteReflog::Normal, WriteReflog::Disable, WriteReflog::Always];

#[test]
fn should_autocreate_is_unaffected_by_writemode() -> Result {
    let (_keep, store) = empty_store(WriteReflog::Disable)?;
    for should_create_name in &["HEAD", "refs/heads/main", "refs/remotes/any", "refs/notes/any"] {
        assert!(store.should_autocreate_reflog(Path::new(should_create_name)));
    }
    for should_not_create_name in &["FETCH_HEAD", "SOMETHING", "refs/special/this", "refs/tags/0.1.0"] {
        assert!(!store.should_autocreate_reflog(Path::new(should_not_create_name)));
    }
    Ok(())
}

#[test]
fn missing_reflog_creates_it_even_if_similarly_named_empty_dir_exists_and_append_log_lines() -> Result {
    for mode in WRITE_MODES {
        let (_keep, store) = empty_store(*mode)?;
        let full_name_str = "refs/heads/main";
        let full_name: &FullNameRef = full_name_str.try_into()?;
        let new = hex_to_id("28ce6a8b26aa170e1de65536fe8abe1832bd3242");
        let committer = Signature {
            name: "committer".into(),
            email: "committer@example.com".into(),
            time: "1234 +0800".into(),
        };
        store.reflog_create_or_append(
            full_name,
            None,
            &new,
            committer.to_ref().into(),
            b"the message".as_bstr(),
            false,
        )?;

        let mut buf = Vec::new();
        match mode {
            WriteReflog::Normal | WriteReflog::Always => {
                assert_eq!(
                    reflog_lines(&store, full_name_str, &mut buf)?,
                    vec![crate::log::Line {
                        previous_oid: gix_hash::Kind::Sha1.null(),
                        new_oid: new,
                        signature: committer.clone(),
                        message: "the message".into()
                    }]
                );
                let previous = hex_to_id("0000000000000000000000111111111111111111");
                store.reflog_create_or_append(
                    full_name,
                    Some(previous),
                    &new,
                    committer.to_ref().into(),
                    b"next message".as_bstr(),
                    false,
                )?;

                let lines = reflog_lines(&store, full_name_str, &mut buf)?;
                assert_eq!(lines.len(), 2, "now there is another line");
                assert_eq!(
                    lines.last().expect("non-empty"),
                    &crate::log::Line {
                        previous_oid: previous,
                        new_oid: new,
                        signature: committer.clone(),
                        message: "next message".into()
                    }
                );
            }
            WriteReflog::Disable => {
                assert!(
                    store.reflog_iter(full_name, &mut buf)?.is_none(),
                    "there is no logs in disabled mode"
                );
            }
        }

        // create onto existing directory
        let full_name_str = "refs/heads/other";
        let full_name: &FullNameRef = full_name_str.try_into()?;
        let reflog_path = store.reflog_path(full_name_str.try_into().expect("valid"));
        let directory_in_place_of_reflog = reflog_path.join("empty-a").join("empty-b");
        std::fs::create_dir_all(directory_in_place_of_reflog)?;

        store.reflog_create_or_append(
            full_name,
            None,
            &new,
            committer.to_ref().into(),
            b"more complicated reflog creation".as_bstr(),
            false,
        )?;

        match mode {
            WriteReflog::Normal | WriteReflog::Always => {
                assert_eq!(
                    reflog_lines(&store, full_name_str, &mut buf)?.len(),
                    1,
                    "reflog was written despite directory"
                );
                assert!(
                    reflog_path.is_file(),
                    "the empty directory was replaced with the reflog file"
                );
            }
            WriteReflog::Disable => {
                assert!(
                    store.reflog_iter(full_name_str, &mut buf)?.is_none(),
                    "reflog still doesn't exist"
                );
                assert!(
                    store.reflog_iter_rev(full_name_str, &mut buf)?.is_none(),
                    "reflog still doesn't exist"
                );
                assert!(reflog_path.is_dir(), "reflog directory wasn't touched");
            }
        }
    }
    Ok(())
}
