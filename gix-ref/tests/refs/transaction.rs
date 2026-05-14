mod refedit_ext {
    use std::{cell::RefCell, collections::BTreeMap};

    use gix_object::bstr::{BString, ByteSlice};
    use gix_ref::{
        PartialNameRef, Target,
        transaction::{Change, PreviousValue, RefEdit, RefEditsExt, RefLog},
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
        RefEdit {
            change: Change::Delete {
                expected: PreviousValue::Any,
                log: RefLog::AndReference,
            },
            name: name.try_into().expect("valid name"),
            deref: false,
        }
    }

    #[test]
    fn preprocessing_checks_duplicates_after_splits() -> crate::Result {
        let store = MockStore::with(Some(("HEAD", Target::Symbolic("refs/heads/main".try_into()?))));

        let mut edits = vec![
            RefEdit {
                change: Change::Delete {
                    expected: PreviousValue::Any,
                    log: RefLog::AndReference,
                },
                name: "HEAD".try_into()?,
                deref: true,
            },
            RefEdit {
                change: Change::Delete {
                    expected: PreviousValue::Any,
                    log: RefLog::AndReference,
                },
                name: "refs/heads/main".try_into()?,
                deref: false,
            },
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
            FullNameRef, PartialNameRef, Target,
            transaction::{Change, LogChange, PreviousValue, RefEdit, RefEditsExt, RefLog},
        };

        use crate::{hex_to_id, transaction::refedit_ext::MockStore};

        fn find<'a>(edits: &'a [RefEdit], name: &str) -> &'a RefEdit {
            let name: &FullNameRef = name.try_into().unwrap();
            edits
                .iter()
                .find(|e| e.name.as_bstr() == name.as_bstr())
                .expect("always available")
        }

        #[test]
        fn non_symbolic_refs_are_ignored_or_if_the_deref_flag_is_not_set() -> crate::Result {
            let store = MockStore::with(Some((
                "refs/heads/anything-but-not-symbolic",
                Target::Object(gix_hash::Kind::Sha1.null()),
            )));
            let mut edits = vec![
                RefEdit {
                    change: Change::Delete {
                        expected: PreviousValue::Any,
                        log: RefLog::AndReference,
                    },
                    name: "SYMBOLIC_PROBABLY_BUT_DEREF_IS_FALSE_SO_IGNORED".try_into()?,
                    deref: false,
                },
                RefEdit {
                    change: Change::Delete {
                        expected: PreviousValue::Any,
                        log: RefLog::AndReference,
                    },
                    name: "refs/heads/anything-but-not-symbolic".try_into()?,
                    deref: true,
                },
                RefEdit {
                    change: Change::Delete {
                        expected: PreviousValue::Any,
                        log: RefLog::AndReference,
                    },
                    name: "refs/heads/does-not-exist-and-deref-is-ignored".try_into()?,
                    deref: true,
                },
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
                RefEdit {
                    change: Change::Delete {
                        expected: PreviousValue::Any,
                        log: RefLog::AndReference,
                    },
                    name: "refs/heads/delete-symbolic-1".try_into()?,
                    deref: true,
                },
                RefEdit {
                    change: Change::Update {
                        expected: PreviousValue::MustNotExist,
                        log: LogChange {
                            mode: RefLog::AndReference,
                            force_create_reflog: true,
                            message: "the log message".into(),
                        },
                        new: Target::Object(gix_hash::Kind::Sha1.null()),
                    },
                    name: "refs/heads/update-symbolic-1".try_into()?,
                    deref: true,
                },
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
                RefEdit {
                    change: Change::Delete {
                        expected: PreviousValue::Any,
                        log: RefLog::AndReference,
                    },
                    name: "refs/heads/delete-symbolic-1".try_into()?,
                    deref: true,
                },
                RefEdit {
                    change: Change::Update {
                        expected: PreviousValue::MustNotExist,
                        log: log.clone(),
                        new: Target::Object(gix_hash::Kind::Sha1.null()),
                    },
                    name: "refs/heads/update-symbolic-1".try_into()?,
                    deref: true,
                },
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
                    RefEdit {
                        change: Change::Delete {
                            expected: PreviousValue::Any,
                            log: RefLog::Only,
                        },
                        name: "refs/heads/delete-symbolic-1".try_into()?,
                        deref: false,
                    },
                    RefEdit {
                        change: Change::Update {
                            expected: PreviousValue::Any,
                            log: log_only.clone(),
                            new: Target::Object(gix_hash::Kind::Sha1.null()),
                        },
                        name: "refs/heads/update-symbolic-1".try_into()?,
                        deref: false,
                    },
                    RefEdit {
                        change: Change::Delete {
                            expected: PreviousValue::Any,
                            log: RefLog::Only,
                        },
                        name: "refs/heads/delete-symbolic-2".try_into()?,
                        deref: false,
                    },
                    RefEdit {
                        change: Change::Update {
                            expected: PreviousValue::Any,
                            log: log_only,
                            new: Target::Object(gix_hash::Kind::Sha1.null()),
                        },
                        name: "refs/heads/update-symbolic-2".try_into()?,
                        deref: false,
                    },
                    RefEdit {
                        change: Change::Delete {
                            expected: PreviousValue::Any,
                            log: RefLog::AndReference,
                        },
                        name: "refs/heads/delete-symbolic-3".try_into()?,
                        deref: false,
                    },
                    RefEdit {
                        change: Change::Update {
                            expected: PreviousValue::MustNotExist,
                            log,
                            new: Target::Object(gix_hash::Kind::Sha1.null()),
                        },
                        name: "refs/heads/update-symbolic-3".try_into()?,
                        deref: false,
                    },
                ]
            );
            Ok(())
        }
    }
}
