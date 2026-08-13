#![cfg(unix)]

use gix_lock::acquire::Fail;
use gix_ref::{
    Target, file,
    transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog},
};

/// Preparing a transaction must retain only a `gix_lock::Marker` per edit, not an open file descriptor.
#[test]
fn large_transactions_hold_a_constant_number_of_file_descriptors() -> gix_testtools::Result {
    let limit = libc::rlimit {
        rlim_cur: 16,
        rlim_max: 16,
    };
    if unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &limit) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }

    let dir = gix_testtools::tempfile::TempDir::new()?;
    let object_hash = gix_testtools::object_hash();
    let store = file::Store::at(
        dir.path().into(),
        gix_ref::store::init::Options {
            object_hash,
            ..Default::default()
        },
    );
    let edits = (0..20).map(|i| RefEdit {
        change: Change::Update {
            log: LogChange {
                mode: RefLog::AndReference,
                force_create_reflog: true,
                message: "log peeled".into(),
            },
            expected: PreviousValue::MustNotExist,
            new: Target::Object(object_hash.empty_blob()),
        },
        name: format!("refs/heads/fd-{i:02}").try_into().expect("valid ref name"),
        deref: false,
    });

    let applied = store
        .transaction()
        .prepare(edits, Fail::Immediately, Fail::Immediately)?
        .commit(
            gix_actor::Signature {
                name: "committer".into(),
                email: "committer@example.com".into(),
                time: gix_date::parse_header("1234 +0800").expect("valid timestamp"),
            }
            .to_ref(&mut gix_date::parse::TimeBuf::default()),
        )?;

    assert_eq!(applied.len(), 20, "all refs were created despite the low fd limit");
    Ok(())
}
