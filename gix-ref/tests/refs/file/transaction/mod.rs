pub(crate) mod prepare_and_commit {
    use gix_hash::ObjectId;
    use gix_object::bstr::BString;
    use gix_ref::{
        file,
        transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog},
        Target,
    };

    use crate::hex_to_id;

    fn reflog_lines(store: &file::Store, name: &str) -> crate::Result<Vec<gix_ref::log::Line>> {
        let mut buf = Vec::new();
        let res = store
            .reflog_iter(name, &mut buf)?
            .expect("existing reflog")
            .map(|l| l.map(gix_ref::log::Line::from))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(res)
    }

    pub(crate) fn empty_store() -> crate::Result<(gix_testtools::tempfile::TempDir, file::Store)> {
        let dir = gix_testtools::tempfile::TempDir::new().unwrap();
        let store = file::Store::at(dir.path().into(), Default::default());
        Ok((dir, store))
    }

    pub(crate) fn committer() -> gix_actor::Signature {
        gix_actor::Signature {
            name: "committer".into(),
            email: "committer@example.com".into(),
            time: "1234 +0800".into(),
        }
    }

    fn log_line(previous: ObjectId, new: ObjectId, message: impl Into<BString>) -> gix_ref::log::Line {
        gix_ref::log::Line {
            previous_oid: previous,
            new_oid: new,
            signature: committer(),
            message: message.into(),
        }
    }

    pub(crate) fn create_at(name: &str) -> RefEdit {
        RefEdit {
            change: Change::Update {
                log: LogChange {
                    mode: RefLog::AndReference,
                    force_create_reflog: true,
                    message: "log peeled".into(),
                },
                expected: PreviousValue::MustNotExist,
                new: Target::Object(hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")),
            },
            name: name.try_into().expect("valid"),
            deref: false,
        }
    }

    fn create_symbolic_at(name: &str, symbolic_target: &str) -> RefEdit {
        RefEdit {
            change: Change::Update {
                log: LogChange::default(),
                expected: PreviousValue::MustNotExist,
                new: Target::Symbolic(symbolic_target.try_into().expect("valid target name")),
            },
            name: name.try_into().expect("valid"),
            deref: false,
        }
    }

    fn delete_at(name: &str) -> RefEdit {
        RefEdit {
            change: Change::Delete {
                expected: PreviousValue::Any,
                log: RefLog::AndReference,
            },
            name: name.try_into().expect("valid name"),
            deref: false,
        }
    }

    mod create_or_update;

    mod delete;
}
