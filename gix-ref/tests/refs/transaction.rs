mod refedit {
    #[test]
    fn constructors_apply_common_defaults() -> crate::Result {
        use gix_ref::{
            FullName, Target,
            transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog},
        };

        let update_name: FullName = "refs/heads/update".try_into()?;
        let new_id = crate::fixture_hash_kind().null();
        assert_eq!(
            RefEdit::update(
                update_name.clone(),
                new_id,
                PreviousValue::MustNotExist,
                "update message",
            ),
            RefEdit {
                change: Change::Update {
                    log: LogChange {
                        message: "update message".into(),
                        ..Default::default()
                    },
                    expected: PreviousValue::MustNotExist,
                    new: Target::Object(new_id),
                },
                name: update_name,
                deref: false,
            },
            "updates use standard reference-log handling without dereferencing"
        );

        let delete_name: FullName = "refs/heads/delete".try_into()?;
        assert_eq!(
            RefEdit::delete(delete_name.clone(), PreviousValue::MustExist),
            RefEdit {
                change: Change::Delete {
                    expected: PreviousValue::MustExist,
                    log: RefLog::AndReference,
                },
                name: delete_name,
                deref: false,
            },
            "deletions remove the reference and its log without dereferencing"
        );

        let custom_name: FullName = "HEAD".try_into()?;
        let custom_change = Change::Delete {
            expected: PreviousValue::Any,
            log: RefLog::Only,
        };
        assert_eq!(
            RefEdit::new(custom_name.clone(), custom_change.clone()).with_deref(true),
            RefEdit {
                change: custom_change,
                name: custom_name,
                deref: true,
            },
            "general edits retain custom changes and configurable dereferencing"
        );
        Ok(())
    }
}

mod refedit_ext {
    use std::{cell::RefCell, collections::BTreeMap};

    use gix_object::bstr::{BString, ByteSlice};
    use gix_ref::{
        PartialNameRef, Target,
        transaction::{PreviousValue, RefEdit, RefEditsExt},
    };

    #[derive(Default)]
    struct MockStore {
        targets: RefCell<BTreeMap<BString, Target>>,
    }

    impl MockStore {
        fn assert_empty(self) {
            assert_eq!(self.targets.borrow().len(), 0, "all targets should be used");
        }
        fn with(targets: impl IntoIterator<Item = (&'static str, Target)>) -> Self {
            MockStore {
                targets: {
                    let mut h = BTreeMap::new();
                    h.extend(targets.into_iter().map(|(k, v)| (k.as_bytes().as_bstr().to_owned(), v)));
                    RefCell::new(h)
                },
            }
        }
        fn find_existing(&self, name: &PartialNameRef) -> Option<Target> {
            self.targets.borrow_mut().remove(name.as_bstr())
        }
    }

    fn named_edit(name: &str) -> RefEdit {
        RefEdit::delete(name.try_into().expect("valid name"), PreviousValue::Any)
    }

    #[test]
    fn preprocessing_checks_duplicates_after_splits() -> crate::Result {
        let store = MockStore::with(Some(("HEAD", Target::Symbolic("refs/heads/main".try_into()?))));

        let mut edits = vec![
            RefEdit::delete("HEAD".try_into()?, PreviousValue::Any).with_deref(true),
            RefEdit::delete("refs/heads/main".try_into()?, PreviousValue::Any),
        ];

        let err = edits
            .pre_process(&mut |n| store.find_existing(n), &mut |_, e| e)
            .expect_err("duplicate detected");
        assert_eq!(
            err.to_string(),
            "A reference named 'refs/heads/main' has multiple edits"
        );
        Ok(())
    }

    #[test]
    fn reject_duplicates() {
        assert!(
            vec![named_edit("HEAD")].assure_one_name_has_one_edit().is_ok(),
            "there are no duplicates"
        );
        assert!(
            vec![named_edit("refs/foo"), named_edit("HEAD")]
                .assure_one_name_has_one_edit()
                .is_ok(),
            "there are no duplicates"
        );
        assert_eq!(
            vec![named_edit("HEAD"), named_edit("refs/heads/main"), named_edit("HEAD")]
                .assure_one_name_has_one_edit()
                .expect_err("duplicate"),
            "HEAD",
            "a correctly named duplicate"
        );
    }

    mod splitting {
        use std::cell::Cell;

        use gix_ref::{
            PartialNameRef, Target,
            transaction::{LogChange, PreviousValue, RefEdit, RefEditsExt, RefLog},
        };

        use crate::{hex_to_id, transaction::refedit_ext::MockStore};

        fn find<'a>(edits: &'a [RefEdit], name: &str) -> &'a RefEdit {
            edits.iter().find(|e| e.name == name).expect("always available")
        }

        #[test]
        fn non_symbolic_refs_are_ignored_or_if_the_deref_flag_is_not_set() -> crate::Result {
            let store = MockStore::with(Some((
                "refs/heads/anything-but-not-symbolic",
                Target::Object(gix_hash::Kind::Sha1.null()),
            )));
            let mut edits = vec![
                RefEdit::delete(
                    "SYMBOLIC_PROBABLY_BUT_DEREF_IS_FALSE_SO_IGNORED".try_into()?,
                    PreviousValue::Any,
                ),
                RefEdit::delete("refs/heads/anything-but-not-symbolic".try_into()?, PreviousValue::Any)
                    .with_deref(true),
                RefEdit::delete(
                    "refs/heads/does-not-exist-and-deref-is-ignored".try_into()?,
                    PreviousValue::Any,
                )
                .with_deref(true),
            ];

            edits.extend_with_splits_of_symbolic_refs(&mut |n| store.find_existing(n), &mut |_, _| {
                panic!("should not be called")
            })?;
            assert_eq!(edits.len(), 3, "no edit was added");
            assert!(
                !find(&edits, "refs/heads/anything-but-not-symbolic").deref,
                "the algorithm corrects these flags"
            );
            assert!(
                !find(&edits, "refs/heads/does-not-exist-and-deref-is-ignored").deref,
                "non-existing refs also disable the deref flag"
            );
            store.assert_empty();
            Ok(())
        }
        #[test]
        fn empty_inputs_are_ok() -> crate::Result {
            let store = MockStore::default();
            Vec::<RefEdit>::new()
                .extend_with_splits_of_symbolic_refs(&mut |n| store.find_existing(n), &mut |_, e| e)
                .map_err(Into::into)
        }

        #[test]
        fn symbolic_refs_cycles_are_handled_gracefully() -> crate::Result {
            #[derive(Default)]
            struct Cycler {
                next_item: Cell<bool>,
            }
            impl Cycler {
                fn find_existing(&self, _name: &PartialNameRef) -> Option<Target> {
                    let item: bool = self.next_item.get();
                    self.next_item.set(!item);
                    Some(Target::Symbolic(
                        if item { "heads/refs/next" } else { "heads/refs/previous" }
                            .try_into()
                            .expect("static refs are valid"),
                    ))
                }
            }

            let mut edits = vec![
                RefEdit::delete("refs/heads/delete-symbolic-1".try_into()?, PreviousValue::Any).with_deref(true),
                RefEdit::update_with_log(
                    "refs/heads/update-symbolic-1".try_into()?,
                    gix_hash::Kind::Sha1.null(),
                    PreviousValue::MustNotExist,
                    LogChange {
                        mode: RefLog::AndReference,
                        force_create_reflog: true,
                        message: "the log message".into(),
                    },
                )
                .with_deref(true),
            ];

            let store = Cycler::default();
            let err = edits
                .extend_with_splits_of_symbolic_refs(&mut |n| store.find_existing(n), &mut |_, e| e)
                .expect_err("cycle detected");
            assert_eq!(
                err.to_string(),
                "Could not follow all splits after 5 rounds, assuming reference cycle"
            );
            Ok(())
        }

        #[test]
        fn symbolic_refs_are_split_into_referents_handling_the_reflog_and_previous_values_recursively() -> crate::Result
        {
            let store = MockStore::with(vec![
                (
                    "refs/heads/delete-symbolic-1",
                    Target::Symbolic("refs/heads/delete-symbolic-2".try_into()?),
                ),
                (
                    "refs/heads/delete-symbolic-2",
                    Target::Symbolic("refs/heads/delete-symbolic-3".try_into()?),
                ),
                (
                    "refs/heads/delete-symbolic-3",
                    Target::Object(hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")),
                ),
                (
                    "refs/heads/update-symbolic-1",
                    Target::Symbolic("refs/heads/update-symbolic-2".try_into()?),
                ),
                (
                    "refs/heads/update-symbolic-2",
                    Target::Symbolic("refs/heads/update-symbolic-3".try_into()?),
                ),
                (
                    "refs/heads/update-symbolic-3",
                    Target::Object(hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")),
                ),
            ]);
            let log = LogChange {
                mode: RefLog::AndReference,
                force_create_reflog: true,
                message: "the log message".into(),
            };
            let log_only = {
                let mut l = log.clone();
                l.mode = RefLog::Only;
                l
            };
            let mut edits = vec![
                RefEdit::delete("refs/heads/delete-symbolic-1".try_into()?, PreviousValue::Any).with_deref(true),
                RefEdit::update_with_log(
                    "refs/heads/update-symbolic-1".try_into()?,
                    gix_hash::Kind::Sha1.null(),
                    PreviousValue::MustNotExist,
                    log.clone(),
                )
                .with_deref(true),
            ];

            let mut indices = Vec::new();
            edits.extend_with_splits_of_symbolic_refs(&mut |n| store.find_existing(n), &mut |idx, e| {
                indices.push(idx);
                e
            })?;
            assert_eq!(
                indices,
                vec![0, 1, 2, 3],
                "the parent index is passed each time there is a split"
            );

            assert_eq!(
                edits,
                vec![
                    RefEdit::delete_with_log(
                        "refs/heads/delete-symbolic-1".try_into()?,
                        PreviousValue::Any,
                        RefLog::Only,
                    ),
                    RefEdit::update_with_log(
                        "refs/heads/update-symbolic-1".try_into()?,
                        gix_hash::Kind::Sha1.null(),
                        PreviousValue::Any,
                        log_only.clone(),
                    ),
                    RefEdit::delete_with_log(
                        "refs/heads/delete-symbolic-2".try_into()?,
                        PreviousValue::Any,
                        RefLog::Only,
                    ),
                    RefEdit::update_with_log(
                        "refs/heads/update-symbolic-2".try_into()?,
                        gix_hash::Kind::Sha1.null(),
                        PreviousValue::Any,
                        log_only,
                    ),
                    RefEdit::delete("refs/heads/delete-symbolic-3".try_into()?, PreviousValue::Any),
                    RefEdit::update_with_log(
                        "refs/heads/update-symbolic-3".try_into()?,
                        gix_hash::Kind::Sha1.null(),
                        PreviousValue::MustNotExist,
                        log,
                    ),
                ]
            );
            Ok(())
        }
    }
}
