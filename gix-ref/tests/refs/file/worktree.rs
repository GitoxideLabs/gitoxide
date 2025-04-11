use std::{cmp::Ordering, path::PathBuf};

use gix_ref::{file::ReferenceExt, Reference};
use gix_testtools::Creation;

fn dir(packed: bool, writable: bool) -> crate::Result<(PathBuf, Option<gix_testtools::tempfile::TempDir>)> {
    let name = "make_worktree_repo.sh";
    let mut args = Vec::new();
    if packed {
        args.push("packed");
    }
    if writable {
        gix_testtools::scripted_fixture_writable_with_args_standalone(name, args, Creation::ExecuteScript)
            .map(|tmp| (tmp.path().to_owned(), tmp.into()))
    } else {
        gix_testtools::scripted_fixture_read_only_with_args_standalone(name, args).map(|p| (p, None))
    }
}

fn main_store(
    packed: bool,
    writable: impl Into<bool>,
) -> crate::Result<(
    gix_ref::file::Store,
    gix_odb::Handle,
    Option<gix_testtools::tempfile::TempDir>,
)> {
    let writable = writable.into();
    let (dir, tmp) = dir(packed, writable)?;
    let git_dir = dir.join("repo").join(".git");
    Ok((
        gix_ref::file::Store::at(git_dir.clone(), Default::default()),
        gix_odb::at(git_dir.join("objects"))?,
        tmp,
    ))
}

fn worktree_store(
    packed: bool,
    worktree_name: &str,
    writable: impl Into<bool>,
) -> crate::Result<(
    gix_ref::file::Store,
    gix_odb::Handle,
    Option<gix_testtools::tempfile::TempDir>,
)> {
    let (dir, tmp) = dir(packed, writable.into())?;
    let (git_dir, _work_tree) = gix_discover::upwards(&dir.join(worktree_name))?
        .0
        .into_repository_and_work_tree_directories();
    let common_dir = git_dir.join("../..");
    Ok((
        gix_ref::file::Store::for_linked_worktree(git_dir, common_dir.clone(), Default::default()),
        gix_odb::at(common_dir.join("objects"))?,
        tmp,
    ))
}

fn into_peel(
    store: &gix_ref::file::Store,
    odb: gix_odb::Handle,
) -> impl Fn(gix_ref::Reference) -> gix_hash::ObjectId + '_ {
    move |mut r: gix_ref::Reference| r.peel_to_id_in_place(store, &odb).unwrap()
}

enum Mode {
    Read,
    Write,
}

impl From<Mode> for bool {
    fn from(v: Mode) -> Self {
        match v {
            Mode::Read => false,
            Mode::Write => true,
        }
    }
}

mod read_only {
    use crate::file::worktree::{assert_reflog, into_peel, main_store, worktree_store, Mode};

    #[test]
    fn linked() -> crate::Result {
        for packed in [false, true] {
            let (store, odb, _tmp) = worktree_store(packed, "w1", Mode::Read)?;
            let peel = into_peel(&store, odb);

            let w1_head_id = peel(store.find("HEAD").unwrap());
            let head_id = peel(store.find("main-worktree/HEAD").unwrap());
            assert_ne!(w1_head_id, head_id, "access to main worktree from linked worktree");
            assert_reflog(&store, store.find("HEAD")?, store.find("worktrees/w1/HEAD")?);
            assert_eq!(
                head_id,
                peel(store.find("main-worktree/refs/bisect/bad").unwrap()),
                "main worktree private branch is accessible and points to its head"
            );
            assert_eq!(
                peel(store.find("refs/bisect/bad").unwrap()),
                w1_head_id,
                "this worktrees bisect branch points to its head"
            );
            assert_eq!(
                peel(store.find("worktrees/w-detached/refs/bisect/bad").unwrap()),
                peel(store.find("worktrees/w-detached/HEAD").unwrap()),
                "the detached worktrees bisect branch points to its head"
            );
            assert_eq!(
                w1_head_id,
                peel(store.find("worktrees/w1/HEAD").unwrap()),
                "access ourselves with worktrees prefix works (HEAD)"
            );
            assert_reflog(&store, store.find("w1")?, store.find("main-worktree/refs/heads/w1")?);
            assert_reflog(&store, store.find("w1")?, store.find("worktrees/w1/refs/heads/w1")?);

            assert_eq!(
                w1_head_id,
                peel(store.find("worktrees/w1/refs/heads/w1").unwrap()),
                "access ourselves with worktrees prefix works (branch)"
            );

            assert_ne!(
                w1_head_id,
                peel(store.find("worktrees/w-detached/HEAD").unwrap()),
                "both point to different ids"
            );
        }
        Ok(())
    }

    #[test]
    fn main() -> crate::Result {
        for packed in [false, true] {
            let (store, odb, _tmp) = main_store(packed, Mode::Read)?;
            let peel = into_peel(&store, odb);

            let head_id = peel(store.find("HEAD").unwrap());
            assert_eq!(
                head_id,
                peel(store.find("main-worktree/HEAD").unwrap()),
                "main-worktree prefix in pseudorefs from main worktree just works"
            );
            assert_reflog(&store, store.find("HEAD")?, store.find("main-worktree/HEAD")?);
            assert_eq!(
                peel(store.find("main").unwrap()),
                peel(store.find("main-worktree/refs/heads/main").unwrap()),
                "main-worktree prefix in pseudorefs from main worktree just works"
            );
            assert_reflog(
                &store,
                store.find("main")?,
                store.find("main-worktree/refs/heads/main")?,
            );
            assert_eq!(
                peel(store.find("refs/bisect/bad").unwrap()),
                head_id,
                "bisect is worktree-private"
            );

            let w1_main_id = peel(store.find("w1").unwrap());
            assert_ne!(w1_main_id, head_id, "w1 is checked out at previous commit");

            let w1_head_id = peel(store.find("worktrees/w1/HEAD").unwrap());
            assert_eq!(w1_head_id, w1_main_id, "worktree head points to the branch");
            assert_eq!(
                peel(store.find("worktrees/w1/refs/bisect/bad").unwrap()),
                w1_main_id,
                "linked worktree bisect points to its head"
            );
            assert_eq!(
                w1_head_id,
                peel(store.find("worktrees/w1/refs/heads/w1").unwrap()),
                "worktree branch can be accessed with refs notation too (git doesn't do this right now, but it's documented)"
            );
            let wd_head_id = peel(store.find("worktrees/w-detached/HEAD").unwrap());
            assert_ne!(wd_head_id, w1_main_id, "both worktrees are in different locations");
            assert_eq!(
                peel(store.find("worktrees/w-detached/refs/bisect/bad").unwrap()),
                wd_head_id,
                "detached worktree bisect is at the same location as its HEAD"
            );
            assert_ne!(
                w1_head_id, head_id,
                "access from main to worktree with respective prefix"
            );
        }
        Ok(())
    }
}

mod writable {
    use gix_lock::acquire::Fail;
    use gix_ref::{
        bstr::ByteSlice,
        file::{transaction::PackedRefs, Store},
        transaction::{Change, LogChange, PreviousValue, RefEdit},
        FullName, FullNameRef, Target,
    };

    use crate::{
        file::{
            transaction::prepare_and_commit::committer,
            worktree::{main_store, worktree_store, Mode},
            EmptyCommit,
        },
        hex_to_id,
    };

    fn change_with_id(id: gix_hash::ObjectId) -> Change {
        Change::Update {
            log: LogChange::default(),
            expected: PreviousValue::MustNotExist,
            new: Target::Object(id),
        }
    }

    #[test]
    fn main() -> crate::Result {
        let new_id_main_str = "11111111111111111162102c6a483440bfda2a03";
        let new_id_main = hex_to_id(new_id_main_str);
        let new_id_linked_str = "22222222222222222262102c6a483440bfda2a03";
        let new_id_linked = hex_to_id(new_id_linked_str);

        for packed in [false, true] {
            let (store, _odb, _tmp) = main_store(packed, Mode::Write)?;
            let mut t = store.transaction();
            if packed {
                t = t.packed_refs(PackedRefs::DeletionsAndNonSymbolicUpdates(Box::new(EmptyCommit)));
            }

            let edits = t
                .prepare(
                    vec![
                        RefEdit {
                            change: change_with_id(new_id_main),
                            name: "main-worktree/refs/heads/new".try_into()?,
                            deref: false,
                        },
                        RefEdit {
                            change: change_with_id(new_id_linked),
                            name: "worktrees/w1/refs/worktree/private".try_into()?,
                            deref: false,
                        },
                        RefEdit {
                            change: change_with_id(new_id_linked),
                            name: "worktrees/w1/refs/bisect/good".try_into()?,
                            deref: false,
                        },
                        RefEdit {
                            change: change_with_id(new_id_main),
                            name: "refs/bisect/good".try_into()?,
                            deref: false,
                        },
                        RefEdit {
                            change: change_with_id(new_id_linked),
                            name: "worktrees/w1/refs/heads/shared".try_into()?,
                            deref: false,
                        },
                    ],
                    Fail::Immediately,
                    Fail::Immediately,
                )?
                .commit(committer().to_ref())
                .expect("successful commit as even similar resolved names live in different base locations");

            assert_eq!(
                store
                    .iter()?
                    .all()?
                    .map(Result::unwrap)
                    .map(|r| (r.name.to_string(), r.target.to_string()))
                    .collect::<Vec<_>>(),
                [
                    ("refs/bisect/bad", "9556057aee5abb06912922e9f26c46386a816822"),
                    ("refs/bisect/good", new_id_main_str),
                    ("refs/heads/main", "9556057aee5abb06912922e9f26c46386a816822"),
                    ("refs/heads/new", new_id_main_str),
                    ("refs/heads/shared", new_id_linked_str),
                    ("refs/heads/w1", "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7"),
                    ("refs/stacks/common", "134385f6d781b7e97062102c6a483440bfda2a03"),
                    ("refs/stacks/w1", "17d78c64cef6c33a10a604573fd2c429e477fd63"),
                    ("refs/stacks/wtdetached", "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7"),
                    ("refs/tags/dt1", "d3ba65e5e3be5cdd7210da9998307a4762999cc5"),
                    ("refs/tags/t1", "9556057aee5abb06912922e9f26c46386a816822")
                ]
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect::<Vec<_>>(),
                "we traverse only refs of the main worktree"
            );
            assert_eq!(
                store
                    .iter()?
                    .prefixed(b"refs/stacks/".as_bstr())?
                    .map(Result::unwrap)
                    .map(|r| (r.name.to_string(), r.target.to_string()))
                    .collect::<Vec<_>>(),
                [
                    ("refs/stacks/common", "134385f6d781b7e97062102c6a483440bfda2a03"),
                    ("refs/stacks/w1", "17d78c64cef6c33a10a604573fd2c429e477fd63"),
                    ("refs/stacks/wtdetached", "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7"),
                ]
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect::<Vec<_>>(),
            );

            for name in ["refs/stacks/common", "refs/stacks/w1", "refs/stacks/wtdetached"] {
                assert_eq!(
                    store.find(name)?.name.category_and_short_name(),
                    None,
                    "uncategorized but present as they are shared"
                );
            }

            let mut buf = Vec::new();
            let unprefixed_ref_name = "refs/heads/new";
            let unprefixed_shared_name: FullName = "refs/heads/shared".try_into()?;

            {
                let reference = store.find(unprefixed_ref_name)?;
                assert_eq!(
                    reflog_for_name(&store, reference.name.as_ref(), &mut buf),
                    vec![new_id_main.to_string()]
                );
                assert_eq!(
                    reference.target.id(),
                    new_id_main,
                    "prefixed refs are written into the correct place"
                );
            }

            {
                let reference = store.find(edits[1].name.as_ref())?;
                assert_eq!(
                    reference.target.id(),
                    new_id_linked,
                    "private worktree refs are written into the correct place"
                );
                assert_eq!(
                    reflog_for_name(&store, reference.name.as_ref(), &mut buf),
                    vec![new_id_linked.to_string()]
                );
            }

            {
                let reference = store.find(edits[2].name.as_ref())?;
                assert_eq!(
                    reference.target.id(),
                    new_id_linked,
                    "worktree-private bisect information is in the correct place"
                );
                assert!(
                    !store.reflog_exists(reference.name.as_ref())?,
                    "private special branches don't have a reflog written"
                );
            }

            {
                let reference = store.find(edits[3].name.as_ref())?;
                assert_eq!(
                    reference.target.id(),
                    new_id_main,
                    "main-worktree private bisect information is in the correct place"
                );
                assert!(
                    !store.reflog_exists(reference.name.as_ref())?,
                    "private special branches don't have a reflog written"
                );
            }

            {
                let reference = store.find(edits[4].name.as_ref())?;
                assert_eq!(
                    reference.target.id(),
                    new_id_linked,
                    "normal refs with worktrees syntax are shared and in the common dir"
                );
                assert_eq!(
                    store.find(unprefixed_shared_name.as_ref())?.target.id(),
                    new_id_linked,
                    "the unprefixed name finds the same ref"
                );
                assert_eq!(
                    reflog_for_name(&store, reference.name.as_ref(), &mut buf),
                    vec![new_id_linked.to_string()],
                    "they have a reflog as one would expect"
                );
            }

            if packed {
                let packed_refs = store.cached_packed_buffer()?.expect("packed refs file present");
                assert_eq!(
                    packed_refs.find(unprefixed_ref_name)?.object(),
                    new_id_main,
                    "ref can be found without prefix"
                );
                assert_eq!(
                    packed_refs.find(edits[0].name.as_ref())?.object(),
                    new_id_main,
                    "ref can be found with prefix"
                );
                for edit in edits.iter().skip(1).take(3) {
                    assert!(
                        packed_refs.try_find(edit.name.as_ref())?.is_none(),
                        "worktree private refs are never packed"
                    );
                }
                assert_eq!(
                    packed_refs.find(edits[4].name.as_ref())?.object(),
                    new_id_linked,
                    "shared worktree refs accessed by prefix are packed"
                );
                assert_eq!(
                    packed_refs.find(unprefixed_shared_name.as_ref())?.object(),
                    new_id_linked,
                    "shared worktree refs accessed without prefix are just the same"
                );
            }

            assert!(matches!(
                store.transaction().prepare(
                    vec![
                        RefEdit {
                            change: change_with_id(new_id_main),
                            name: "main-worktree/refs/heads/foo".try_into()?,
                            deref: false,
                        },
                        RefEdit {
                            change: change_with_id(new_id_main),
                            name: "refs/heads/foo".try_into()?,
                            deref: false,
                        },
                    ],
                    Fail::Immediately,
                    Fail::Immediately,
                ),
                Err(gix_ref::file::transaction::prepare::Error::LockAcquire { .. })
            ), "prefixed refs resolve to the same name and will fail to be locked (so we don't check for this when doing dupe checking)");

            assert!(matches!(
                store.transaction().prepare(
                    vec![
                        RefEdit {
                            change: change_with_id(new_id_main),
                            name: "refs/heads/new-shared".try_into()?,
                            deref: false,
                        },
                        RefEdit {
                            change: change_with_id(new_id_main),
                            name: "worktrees/w1/refs/heads/new-shared".try_into()?,
                            deref: false,
                        },
                    ],
                    Fail::Immediately,
                    Fail::Immediately,
                ),
                Err(gix_ref::file::transaction::prepare::Error::LockAcquire { .. })
            ));
        }

        Ok(())
    }

    fn reflog_for_name(store: &Store, name: &FullNameRef, buf: &mut Vec<u8>) -> Vec<String> {
        store
            .reflog_iter(name, buf)
            .unwrap()
            .unwrap_or_else(|| panic!("we expect to write reflogs for {}", name.as_bstr()))
            .map(Result::unwrap)
            .map(|e| e.new_oid.to_owned().to_string())
            .collect::<Vec<_>>()
    }

    #[test]
    fn linked() -> crate::Result {
        let new_id_str = "134385f6d781b7e97062102c6a483440bfda2a03";
        let new_id = hex_to_id(new_id_str);
        let new_id_main_str = "22222222222222227062102c6a483440bfda2a03";
        let new_id_main = hex_to_id(new_id_main_str);
        for packed in [false, true] {
            let (store, _odb, _tmp) = worktree_store(packed, "w1", Mode::Write)?;

            for conflicting_name in ["main-worktree/refs/heads/shared", "worktrees/w1/refs/heads/shared"] {
                assert!(matches!(
                    store.transaction().prepare(
                        vec![
                            RefEdit {
                                change: change_with_id(new_id),
                                name: conflicting_name.try_into()?,
                                deref: false,
                            },
                            RefEdit {
                                change: change_with_id(new_id),
                                name: "refs/heads/shared".try_into()?,
                                deref: false,
                            },
                        ],
                        Fail::Immediately,
                        Fail::Immediately,
                    ),
                    Err(gix_ref::file::transaction::prepare::Error::LockAcquire { .. })
                ), "prefixed refs resolve to the same name and will fail to be locked (so we don't check for this when doing dupe checking)");
            }

            let mut t = store.transaction();
            if packed {
                t = t.packed_refs(PackedRefs::DeletionsAndNonSymbolicUpdates(Box::new(EmptyCommit)));
            }

            let edits = t
                .prepare(
                    vec![
                        RefEdit {
                            change: change_with_id(new_id_main),
                            name: "main-worktree/refs/heads/new".try_into()?,
                            deref: false,
                        },
                        RefEdit {
                            change: change_with_id(new_id_main),
                            name: "main-worktree/refs/bisect/good".try_into()?,
                            deref: false,
                        },
                        RefEdit {
                            change: change_with_id(new_id),
                            name: "refs/bisect/good".try_into()?,
                            deref: false,
                        },
                        RefEdit {
                            change: change_with_id(new_id),
                            name: "refs/worktree/private".try_into()?,
                            deref: false,
                        },
                        RefEdit {
                            change: change_with_id(new_id),
                            name: "refs/heads/shared".try_into()?,
                            deref: false,
                        },
                    ],
                    Fail::Immediately,
                    Fail::Immediately,
                )?
                .commit(committer().to_ref())
                .expect("successful commit as even similar resolved names live in different base locations");

            assert_eq!(
                store
                    .iter()?
                    .all()?
                    .map(Result::unwrap)
                    .map(|r| (r.name.to_string(), r.target.to_string()))
                    .collect::<Vec<_>>(),
                [
                    ("refs/bisect/bad", "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7"),
                    ("refs/bisect/good", new_id_str),
                    ("refs/heads/main", "9556057aee5abb06912922e9f26c46386a816822"),
                    ("refs/heads/new", new_id_main_str),
                    ("refs/heads/shared", new_id_str),
                    ("refs/heads/w1", "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7"),
                    ("refs/stacks/common", "134385f6d781b7e97062102c6a483440bfda2a03"),
                    ("refs/stacks/w1", "17d78c64cef6c33a10a604573fd2c429e477fd63"),
                    ("refs/stacks/wtdetached", "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7"),
                    ("refs/tags/dt1", "d3ba65e5e3be5cdd7210da9998307a4762999cc5"),
                    ("refs/tags/t1", "9556057aee5abb06912922e9f26c46386a816822"),
                    ("refs/worktree/private", new_id_str)
                ]
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect::<Vec<_>>(),
                "we traverse only refs of the main worktree"
            );
            assert_eq!(
                store
                    .iter()?
                    .prefixed(b"refs/stacks/".as_bstr())?
                    .map(Result::unwrap)
                    .map(|r| (r.name.to_string(), r.target.to_string()))
                    .collect::<Vec<_>>(),
                [
                    ("refs/stacks/common", "134385f6d781b7e97062102c6a483440bfda2a03"),
                    ("refs/stacks/w1", "17d78c64cef6c33a10a604573fd2c429e477fd63"),
                    ("refs/stacks/wtdetached", "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7"),
                ]
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect::<Vec<_>>(),
            );

            for name in ["refs/stacks/common", "refs/stacks/w1", "refs/stacks/wtdetached"] {
                assert_eq!(
                    store.find(name)?.name.category_and_short_name(),
                    None,
                    "uncategorized but present as they are shared"
                );
            }

            let mut buf = Vec::new();

            {
                let unprefixed_name = "refs/heads/new";
                let reference = store.find(unprefixed_name)?;
                assert_eq!(reference.target.id(), new_id_main);
                assert_eq!(
                    reflog_for_name(&store, reference.name.as_ref(), &mut buf),
                    vec![new_id_main.to_string()]
                );
            }

            {
                let reference = store.find(edits[1].name.as_ref())?;
                assert_eq!(reference.target.id(), new_id_main);
                assert!(
                    !store.reflog_exists(edits[1].name.as_ref())?,
                    "special refs do not write reflogs"
                );
            }

            {
                let reference = store.find(edits[2].name.as_ref())?;
                assert_eq!(reference.target.id(), new_id);
                assert!(
                    !store.reflog_exists(edits[2].name.as_ref())?,
                    "special worktree refs do not write reflogs"
                );
            }

            {
                let reference = store.find(edits[3].name.as_ref())?;
                assert_eq!(reference.target.id(), new_id);
                assert_eq!(
                    reflog_for_name(&store, reference.name.as_ref(), &mut buf),
                    vec![new_id.to_string()],
                    "private worktree refs do have a changelog"
                );
            }

            {
                let reference = store.find(edits[4].name.as_ref())?;
                assert_eq!(reference.target.id(), new_id);
                assert_eq!(
                    reflog_for_name(&store, reference.name.as_ref(), &mut buf),
                    vec![new_id.to_string()],
                    "shared worktree references have reflogs"
                );
            }

            if packed {
                let packed_refs = store.cached_packed_buffer()?.expect("packed refs file present");
                assert_eq!(
                    packed_refs.find(edits[0].name.as_ref())?.object(),
                    new_id_main,
                    "shared refs are packed"
                );

                for edit in edits.iter().skip(1).take(3) {
                    assert!(
                        packed_refs.try_find(edit.name.as_ref())?.is_none(),
                        "private refs like these are never packed"
                    );
                }

                assert_eq!(
                    packed_refs.find(edits[4].name.as_ref())?.object(),
                    new_id,
                    "shared refs are packed"
                );
            }
        }

        Ok(())
    }
}

fn assert_reflog(store: &gix_ref::file::Store, a: Reference, b: Reference) {
    let mut arl = a.log_iter(store);
    let arl = arl.all().unwrap();
    let mut brl = b.log_iter(store);
    let brl = brl.all().unwrap();
    match (arl, brl) {
        (Some(arl), Some(brl)) => {
            assert_eq!(
                arl.map(Result::unwrap).cmp(brl.map(Result::unwrap)),
                Ordering::Equal,
                "{} and {} should have equal reflogs",
                a.name,
                b.name
            );
        }
        (None, None) => {}
        (arl, brl) => panic!("{} != {} ({} != {})", arl.is_some(), brl.is_some(), a.name, b.name),
    }
}
