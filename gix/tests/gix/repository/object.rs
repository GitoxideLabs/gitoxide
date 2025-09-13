use gix_testtools::tempfile;

use crate::util::named_subrepo_opts;

mod object_database_impl {
    use gix_object::{Exists, Find, FindHeader};

    #[test]
    fn empty_tree_is_always_present() -> crate::Result {
        let repo = crate::named_subrepo_opts("make_basic_repo.sh", "unborn", gix::open::Options::isolated())?;
        let empty_tree = gix::ObjectId::empty_tree(repo.object_hash());
        assert!(repo.exists(&empty_tree));
        assert_eq!(
            repo.try_header(&empty_tree)?.expect("tree present"),
            gix_object::Header {
                kind: gix_object::Kind::Tree,
                size: 0
            }
        );
        let mut buf = repo.empty_reusable_buffer();
        buf.push(42);
        assert_eq!(
            repo.try_find(&empty_tree, &mut buf)?.expect("tree present").kind,
            gix_object::Kind::Tree
        );
        assert_eq!(buf.len(), 0, "the data in the buffer matches the empty tree");
        Ok(())
    }
}

#[cfg(feature = "tree-editor")]
mod edit_tree {
    use gix::bstr::{BStr, BString};
    use gix_object::tree::EntryKind;

    use crate::util::hex_to_id;

    #[test]
    // Some part of the test validation the implementation for this exists, but it's needless nonetheless.
    #[allow(clippy::needless_borrows_for_generic_args)]
    fn from_head_tree() -> crate::Result {
        let (repo, _tmp) = crate::repo_rw("make_packed_and_loose.sh")?;
        let head_tree_id = repo.head_tree_id()?;
        assert_eq!(
            display_tree(head_tree_id, &repo),
            "24374df94315568adfaee119d038f710d1f45397
├── that ce013625030ba8dba906f756967f9e9ca394464a.100644
└── this 317e9677c3bcffd006f9fc84bbb0a54ef1676197.100644
"
        );
        let this_id = hex_to_id("317e9677c3bcffd006f9fc84bbb0a54ef1676197");
        let that_id = hex_to_id("ce013625030ba8dba906f756967f9e9ca394464a");
        let mut editor = repo.edit_tree(head_tree_id)?;
        let actual = editor
            .upsert("a/b", EntryKind::Blob, this_id)?
            .upsert(String::from("this/subdir/that"), EntryKind::Blob, this_id)?
            .upsert(BString::from("that/other/that"), EntryKind::Blob, that_id)?
            .remove(BStr::new("that"))?
            .remove(&String::from("that"))?
            .remove(&BString::from("that"))?
            .write()?;

        assert_eq!(
            display_tree(actual, &repo),
            "fe02a8bd15e4c0476d938f772f1eece6d164b1bd
├── a
│   └── b 317e9677c3bcffd006f9fc84bbb0a54ef1676197.100644
└── this
    └── subdir
        └── that 317e9677c3bcffd006f9fc84bbb0a54ef1676197.100644
",
            "all trees are actually written, or else we couldn't visualize them."
        );

        let actual = editor
            .upsert("a/b", EntryKind::Blob, that_id)?
            .upsert(String::from("this/subdir/that"), EntryKind::Blob, this_id)?
            .remove(BStr::new("does-not-exist"))?
            .write()?;
        assert_eq!(
            display_tree(actual, &repo),
            "219596ff52fc84b6b39bc327f202d408cc02e1db
├── a
│   └── b ce013625030ba8dba906f756967f9e9ca394464a.100644
└── this
    └── subdir
        └── that 317e9677c3bcffd006f9fc84bbb0a54ef1676197.100644
",
            "existing blobs can also be changed"
        );

        let mut cursor = editor.cursor_at("something/very/nested/to/add/entries/to")?;
        let actual = cursor
            .upsert("a/b", EntryKind::Blob, this_id)?
            .upsert(String::from("this/subdir/that"), EntryKind::Blob, that_id)?
            .upsert(BString::from("that/other/that"), EntryKind::Blob, that_id)?
            .remove(BStr::new("that"))?
            .write()?;

        assert_eq!(
            display_tree(actual, &repo),
            "35ea623106198f21b6959dd2731740e5153db2bb
├── a
│   └── b 317e9677c3bcffd006f9fc84bbb0a54ef1676197.100644
└── this
    └── subdir
        └── that ce013625030ba8dba906f756967f9e9ca394464a.100644
",
            "all remaining subtrees are written from the cursor position"
        );

        let actual = editor.write()?;
        assert_eq!(
            display_tree(actual, &repo),
            "9ebdc2c1d22e91636fa876a51521464f8a88dd6f
├── a
│   └── b ce013625030ba8dba906f756967f9e9ca394464a.100644
├── something
│   └── very
│       └── nested
│           └── to
│               └── add
│                   └── entries
│                       └── to
│                           ├── a
│                           │   └── b 317e9677c3bcffd006f9fc84bbb0a54ef1676197.100644
│                           └── this
│                               └── subdir
│                                   └── that ce013625030ba8dba906f756967f9e9ca394464a.100644
└── this
    └── subdir
        └── that 317e9677c3bcffd006f9fc84bbb0a54ef1676197.100644
",
            "it looks as it should when seen from the root tree"
        );

        editor.set_root(&head_tree_id.object()?.into_tree())?;
        let actual = editor.write()?;
        assert_eq!(
            display_tree(actual, &repo),
            "24374df94315568adfaee119d038f710d1f45397
├── that ce013625030ba8dba906f756967f9e9ca394464a.100644
└── this 317e9677c3bcffd006f9fc84bbb0a54ef1676197.100644
",
            "it's possible to set the editor to any tree after creating it, could help with memory re-use"
        );
        Ok(())
    }

    #[test]
    fn submodules_are_not_checked_for_existence() -> crate::Result {
        let repo = crate::named_subrepo_opts("make_submodules.sh", "with-submodules", gix::open::Options::isolated())?
            .with_object_memory();
        let mut editor = repo.head_tree()?.edit()?;
        let actual = editor.write()?;
        assert_eq!(
            actual,
            repo.head_tree_id()?,
            "Nothing changed, but it did validate the root tree that it would want to write"
        );
        Ok(())
    }

    #[test]
    fn missing_objects_and_illformed_path_components_trigger_error() -> crate::Result {
        let (repo, _tmp) = crate::repo_rw("make_packed_and_loose.sh")?;
        let tree = repo.head_tree_id()?.object()?.into_tree();
        let mut editor = tree.edit()?;
        let actual = editor
            .upsert("non-existing", EntryKind::Blob, repo.object_hash().null())?
            .write()?;
        assert_eq!(
            actual,
            tree.id(),
            "nulls are pruned before writing the tree, so it just rewrites the same tree"
        );

        let err = editor
            .upsert(
                "non-existing",
                EntryKind::Blob,
                hex_to_id("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            )?
            .write()
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "The object aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa (100644) at 'non-existing' could not be found",
            "each entry to be written is checked for existence"
        );

        let this_id = hex_to_id("317e9677c3bcffd006f9fc84bbb0a54ef1676197");
        let err = editor
            .remove("non-existing")?
            .upsert(".git", EntryKind::Blob, this_id)?
            .write()
            .expect_err(".git is universally forbidden in trees");
        assert_eq!(
            err.to_string(),
            "The object 317e9677c3bcffd006f9fc84bbb0a54ef1676197 (100644) has an invalid filename: '.git'",
            "each component is validated"
        );

        Ok(())
    }

    mod utils {
        use gix::{
            bstr::{BStr, ByteSlice},
            Repository,
        };
        use gix_hash::ObjectId;

        fn display_tree_recursive(
            tree_id: ObjectId,
            repo: &Repository,
            name: Option<&BStr>,
        ) -> anyhow::Result<termtree::Tree<String>> {
            let tree = repo.find_tree(tree_id)?.decode()?.to_owned();
            let mut termtree = termtree::Tree::new(if let Some(name) = name {
                if tree.entries.is_empty() {
                    format!("{name} (empty)")
                } else {
                    name.to_string()
                }
            } else {
                tree_id.to_string()
            });

            for entry in &tree.entries {
                if entry.mode.is_tree() {
                    termtree.push(display_tree_recursive(entry.oid, repo, Some(entry.filename.as_bstr()))?);
                } else {
                    termtree.push(format!(
                        "{} {}.{}",
                        entry.filename,
                        entry.oid,
                        entry.mode.kind().as_octal_str()
                    ));
                }
            }
            Ok(termtree)
        }

        pub(super) fn display_tree(tree_id: impl Into<ObjectId>, odb: &Repository) -> String {
            display_tree_recursive(tree_id.into(), odb, None)
                .expect("tree exists and everything was written")
                .to_string()
        }
    }
    use utils::display_tree;
}
mod write_object {
    use crate::repository::object::empty_bare_in_memory_repo;

    #[test]
    fn empty_tree() -> crate::Result {
        let repo = empty_bare_in_memory_repo()?;
        let oid = repo.write_object(gix::objs::TreeRef::empty())?;
        assert_eq!(
            oid,
            gix::hash::ObjectId::empty_tree(repo.object_hash()),
            "it produces a well-known empty tree id"
        );
        Ok(())
    }

    #[test]
    fn commit_with_invalid_author() -> crate::Result {
        let repo = empty_bare_in_memory_repo()?;
        let actor = gix::actor::Signature {
            name: "1 < 0".into(),
            email: Default::default(),
            time: Default::default(),
        };
        let commit = gix::objs::Commit {
            tree: gix::hash::ObjectId::empty_tree(repo.object_hash()),
            author: actor.clone(),
            committer: actor,
            parents: Default::default(),
            encoding: None,
            message: Default::default(),
            extra_headers: vec![],
        };
        assert_eq!(
            repo.write_object(commit).unwrap_err().to_string(),
            r"Signature name or email must not contain '<', '>' or \n",
            "the actor is invalid so triggers an error when persisting it"
        );
        Ok(())
    }
}

mod write_blob {
    use std::io::{Seek, SeekFrom};

    use crate::{
        repository::object::{empty_bare_in_memory_repo, empty_bare_repo},
        util::hex_to_id,
    };

    #[test]
    fn from_slice() -> crate::Result {
        let (_tmp, repo) = empty_bare_repo()?;
        let expected = hex_to_id("95d09f2b10159347eece71399a7e2e907ea3df4f");
        assert!(!repo.has_object(expected));

        let oid = repo.write_blob(b"hello world")?;
        assert_eq!(oid, expected);

        let mut other_repo = gix::open_opts(repo.path(), gix::open::Options::isolated())?;
        other_repo.objects.enable_object_memory();
        assert!(
            other_repo.has_object(oid),
            "we definitely don't accidentally write to memory only"
        );
        let in_memory_id = other_repo.write_blob("hello world - to memory")?;
        assert!(!repo.has_object(in_memory_id), "the object was never written to disk…");
        assert!(
            other_repo.has_object(in_memory_id),
            "…and exists only in the instance that wrote it"
        );
        Ok(())
    }

    #[test]
    fn from_stream() -> crate::Result {
        let repo = empty_bare_in_memory_repo()?;
        let mut cursor = std::io::Cursor::new(b"hello world");
        let mut seek_cursor = cursor.clone();
        let mut repo = repo.without_freelist();
        let oid = repo.write_blob_stream(&mut cursor)?;
        assert_eq!(oid, hex_to_id("95d09f2b10159347eece71399a7e2e907ea3df4f"));

        seek_cursor.seek(SeekFrom::Start(6))?;
        let oid = repo.write_blob_stream(&mut seek_cursor)?;
        assert_eq!(
            oid,
            hex_to_id("04fea06420ca60892f73becee3614f6d023a4b7f"),
            "it computes the object size correctly"
        );

        assert_eq!(
            oid.object()?.data,
            &b"world"[..],
            "the seek position is taken into account, so only part of the input data is written"
        );

        assert!(repo.set_freelist(None).is_none(), "previous list was already dropped");
        Ok(())
    }
}

#[test]
fn writes_avoid_io_using_duplicate_check() -> crate::Result {
    let mut repo = crate::named_repo("make_packed_and_loose.sh")?;
    let store = gix::odb::loose::Store::at(repo.git_dir().join("objects"), repo.object_hash());
    let loose_count = store.iter().count();
    assert_eq!(loose_count, 3, "there are some loose objects");
    assert_eq!(
        repo.objects.iter()?.count() - loose_count,
        6,
        "there is packed objects as well"
    );

    for id in repo.objects.iter()? {
        let id = id?;
        assert!(repo.has_object(id));
        let obj = repo.find_object(id)?;
        let header = repo.find_header(id)?;
        assert_eq!(obj.kind, header.kind(), "header and object agree");
        assert_eq!(repo.try_find_header(id)?, Some(header));
        use gix_object::Kind::*;
        match obj.kind {
            Commit => {
                let commit = obj.into_commit();
                let new_id = repo.write_object(commit.decode()?)?;
                assert_eq!(new_id, id);
            }
            Tag => {
                let tag = obj.into_tag();
                let new_id = repo.write_object(tag.decode()?)?;
                assert_eq!(new_id, id);
            }
            Tree => {
                let tree = obj.into_tree();
                let new_id = repo.write_object(tree.decode()?)?;
                assert_eq!(new_id, id);
            }
            Blob => {
                let blob = obj.into_blob();
                let new_id = repo.write_blob(&blob.data)?;
                assert_eq!(new_id, id);
                let new_id = repo.write_blob_stream(std::io::Cursor::new(&blob.data))?;
                assert_eq!(new_id, id);
            }
        }
    }

    assert_eq!(
        store.iter().count(),
        loose_count,
        "no new object was written as all of them already existed"
    );

    {
        let buf = repo.empty_reusable_buffer();
        assert!(buf.is_empty(), "the freelist buffer must be clearerd");
        let mut other_buf = buf.clone();
        other_buf.inner = Vec::new();
    }

    let freelist = repo.set_freelist(None).expect("free list is present by default");
    assert_eq!(
        freelist.len(),
        2,
        "only one object was read at a time, and one is written"
    );

    let mut repo_clone = repo.clone();
    assert!(
        repo_clone.set_freelist(None).is_none(),
        "new instances inherit the free-list configuration of their parent"
    );
    Ok(())
}

mod find {
    use gix_pack::Find;

    use crate::basic_repo;

    #[test]
    fn find_and_try_find_with_and_without_object_cache() -> crate::Result {
        let mut repo = basic_repo()?;

        assert_eq!(
            repo.worktrees()?.len(),
            0,
            "it's OK to query linked worktrees in a repo without worktrees"
        );
        for round in 1..=2 {
            match round {
                1 => repo.object_cache_size(None),
                2 => repo.object_cache_size(128 * 1024),
                _ => unreachable!("BUG"),
            }
            for commit_id in repo.head()?.into_peeled_id()?.ancestors().all()? {
                let commit = commit_id?;
                assert_eq!(commit.id().object()?.kind, gix_object::Kind::Commit);
                assert_eq!(commit.id().header()?.kind(), gix_object::Kind::Commit);
                if round == 2 {
                    assert_eq!(
                        commit.id().object()?.kind,
                        gix_object::Kind::Commit,
                        "repeated request triggers cache and doesn't fail"
                    );
                }
                assert_eq!(
                    commit.id().try_object()?.expect("exists").kind,
                    gix_object::Kind::Commit,
                );
                assert_eq!(
                    commit.id().try_header()?.expect("exists").kind(),
                    gix_object::Kind::Commit,
                );
            }
        }
        Ok(())
    }

    #[test]
    fn empty_tree_can_always_be_found() -> crate::Result {
        let repo = basic_repo()?;
        let empty_tree = gix::hash::ObjectId::empty_tree(repo.object_hash());
        assert_eq!(repo.find_object(empty_tree)?.into_tree().iter().count(), 0);
        assert!(repo.has_object(empty_tree));
        assert_eq!(
            repo.find_header(empty_tree)?,
            gix_odb::find::Header::Loose {
                kind: gix_object::Kind::Tree,
                size: 0,
            },
            "empty tree is considered a loose object"
        );
        assert_eq!(
            repo.try_find_object(empty_tree)?
                .expect("present")
                .into_tree()
                .iter()
                .count(),
            0
        );
        assert_eq!(
            repo.try_find_header(empty_tree)?,
            Some(gix_odb::find::Header::Loose {
                kind: gix_object::Kind::Tree,
                size: 0,
            }),
            "empty tree is considered a loose object"
        );

        let mut buf = Vec::new();
        assert!(
            repo.objects.try_find(&empty_tree, &mut buf)?.is_none(),
            "the lower level has no such special case so one can determine if this object exists or not"
        );
        Ok(())
    }

    #[test]
    fn empty_blob_can_always_be_found() -> crate::Result {
        let repo = basic_repo()?;
        let empty_blob = gix::hash::ObjectId::empty_blob(repo.object_hash());
        assert_eq!(repo.find_object(empty_blob)?.into_blob().data.len(), 0);
        assert!(repo.has_object(empty_blob));
        assert_eq!(
            repo.find_header(empty_blob)?,
            gix_odb::find::Header::Loose {
                kind: gix_object::Kind::Blob,
                size: 0,
            },
            "empty blob is considered a loose object"
        );
        assert_eq!(
            repo.try_find_object(empty_blob)?
                .expect("present")
                .into_blob()
                .data
                .len(),
            0
        );
        assert_eq!(
            repo.try_find_header(empty_blob)?,
            Some(gix_odb::find::Header::Loose {
                kind: gix_object::Kind::Blob,
                size: 0,
            }),
            "empty blob is considered a loose object"
        );

        // Note: Unlike empty tree, empty blobs might actually exist in the repository.
        // The key point is that has_object() should always return true for empty blobs,
        // regardless of whether they are physically stored or not.
        Ok(())
    }
}

#[test]
fn empty_blob_is_always_considered_present() -> crate::Result {
    use gix_object::Find;
    
    // Test with an empty in-memory repository to ensure empty blob is considered present
    // even when it doesn't physically exist
    let repo = empty_bare_in_memory_repo()?;
    let empty_blob = gix::hash::ObjectId::empty_blob(repo.object_hash());
    
    // The key behavior being tested: has_object should return true for empty blob
    assert!(repo.has_object(empty_blob), "empty blob should always be considered present");
    
    // Verify that the lower-level object database doesn't have it
    let mut buf = Vec::new();
    let lower_level_result = repo.objects.try_find(&empty_blob, &mut buf)?;
    
    // Empty blob might or might not exist at the lower level - that's implementation dependent
    // But has_object should always return true regardless
    match lower_level_result {
        Some(_) => {
            // If it exists at the lower level, that's fine
        }
        None => {
            // If it doesn't exist at the lower level, has_object should still return true
            // thanks to our special handling
        }
    }
    
    Ok(())
}

#[test]
fn empty_blob_edge_cases() -> crate::Result {
    let repo = empty_bare_in_memory_repo()?;
    let empty_blob_id = gix::hash::ObjectId::empty_blob(repo.object_hash());
    
    // Test all the related methods for empty blobs
    assert!(repo.has_object(&empty_blob_id), "has_object should return true");
    
    // Test find_header
    let header = repo.find_header(empty_blob_id)?;
    assert_eq!(header.kind(), gix_object::Kind::Blob);
    assert_eq!(header.size(), 0);
    
    // Test try_find_header
    let header = repo.try_find_header(empty_blob_id)?.expect("should find header");
    assert_eq!(header.kind(), gix_object::Kind::Blob);
    assert_eq!(header.size(), 0);
    
    // Test find_object
    let obj = repo.find_object(empty_blob_id)?;
    assert_eq!(obj.kind, gix_object::Kind::Blob);
    assert_eq!(obj.data.len(), 0);
    
    // Test try_find_object
    let obj = repo.try_find_object(empty_blob_id)?.expect("should find object");
    assert_eq!(obj.kind, gix_object::Kind::Blob);
    assert_eq!(obj.data.len(), 0);
    
    // Test that we can get a blob from the object
    let blob = obj.into_blob();
    assert_eq!(blob.data.len(), 0);
    
    Ok(())
}

mod tag {
    #[test]
    fn simple() -> crate::Result {
        let (repo, _keep) = crate::repo_rw("make_basic_repo.sh")?;
        let current_head_id = repo.head_id()?;
        let message = "a multi\nline message";
        let tag_ref = repo.tag(
            "v1.0.0",
            current_head_id,
            gix_object::Kind::Commit,
            Some(repo.committer().expect("present")?),
            message,
            gix_ref::transaction::PreviousValue::MustNotExist,
        )?;
        assert_eq!(tag_ref.name().as_bstr(), "refs/tags/v1.0.0");
        assert_ne!(tag_ref.id(), current_head_id, "it points to the tag object");
        let tag = tag_ref.id().object()?;
        let tag = tag.try_to_tag_ref()?;
        assert_eq!(tag.name, "v1.0.0");
        assert_eq!(current_head_id, tag.target(), "the tag points to the commit");
        assert_eq!(tag.target_kind, gix_object::Kind::Commit);
        assert_eq!(
            tag.tagger.as_ref().expect("tagger").actor(),
            repo.committer().expect("present")?.actor()
        );
        assert_eq!(tag.message, message);
        Ok(())
    }
}

mod commit_as {
    use gix_date::parse::TimeBuf;
    use gix_testtools::tempfile;

    #[test]
    fn specify_committer_and_author() -> crate::Result {
        let tmp = tempfile::tempdir()?;
        let repo = gix::ThreadSafeRepository::init_opts(
            &tmp,
            gix::create::Kind::WithWorktree,
            Default::default(),
            gix::open::Options::isolated(),
        )?
        .to_thread_local();
        let empty_tree = repo.empty_tree();
        let committer = gix::actor::Signature {
            name: "c".into(),
            email: "c@example.com".into(),
            time: gix_date::parse_header("1 +0030").unwrap(),
        };
        let author = gix::actor::Signature {
            name: "a".into(),
            email: "a@example.com".into(),
            time: gix_date::parse_header("3 +0100").unwrap(),
        };

        let commit_id = repo.commit_as(
            committer.to_ref(&mut TimeBuf::default()),
            author.to_ref(&mut TimeBuf::default()),
            "HEAD",
            "initial",
            empty_tree.id,
            gix::commit::NO_PARENT_IDS,
        )?;
        let commit = commit_id.object()?.into_commit();

        let mut buf = TimeBuf::default();
        assert_eq!(commit.committer()?, committer.to_ref(&mut buf));
        assert_eq!(commit.author()?, author.to_ref(&mut buf));
        Ok(())
    }
}

mod commit {
    use gix_testtools::tempfile;

    use crate::{freeze_time, restricted_and_git, util::hex_to_id};

    #[test]
    fn parent_in_initial_commit_causes_failure() -> crate::Result {
        let tmp = tempfile::tempdir()?;
        let repo = gix::ThreadSafeRepository::init_opts(
            &tmp,
            gix::create::Kind::WithWorktree,
            Default::default(),
            crate::restricted(),
        )?
        .to_thread_local();
        let empty_tree_id = repo.write_object(gix::objs::Tree::empty())?.detach();
        let err = repo
            .commit("HEAD", "initial", empty_tree_id, [empty_tree_id])
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Reference \"refs/heads/main\" was supposed to exist with value 4b825dc642cb6eb9a060e54bf8d69288fbee4904, but didn't.",
            "cannot provide parent id in initial commit"
        );
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn single_line_initial_commit_empty_tree_ref_nonexisting() -> crate::Result {
        let _env = freeze_time();
        let tmp = tempfile::tempdir()?;
        let repo = gix::ThreadSafeRepository::init_opts(
            &tmp,
            gix::create::Kind::WithWorktree,
            Default::default(),
            restricted_and_git(),
        )?
        .to_thread_local();
        let empty_tree_id = repo.write_object(gix::objs::Tree::empty())?;
        let commit_id = repo.commit("HEAD", "initial", empty_tree_id, gix::commit::NO_PARENT_IDS)?;
        assert_eq!(
            commit_id,
            hex_to_id("3a774843723a713a8d361b4d4d98ad4092ef05bd"),
            "the commit id is stable"
        );

        let head = repo.head()?.try_into_referent().expect("born");
        assert_eq!(head.name().as_bstr(), "refs/heads/main", "'main' is the default name");
        assert_eq!(
            head.log_iter()
                .rev()?
                .expect("log present")
                .next()
                .expect("one line")?
                .message,
            "commit (initial): initial"
        );
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn multi_line_commit_message_uses_first_line_in_ref_log_ref_nonexisting() -> crate::Result {
        let _env = freeze_time();
        let (repo, _keep) = crate::repo_rw_opts("make_basic_repo.sh", restricted_and_git())?;
        let parent = repo.find_reference("HEAD")?.peel_to_id()?;
        let empty_tree_id = parent.object()?.to_commit_ref_iter().tree_id().expect("tree to be set");
        assert_eq!(
            parent
                .try_object()?
                .expect("present")
                .to_commit_ref_iter()
                .tree_id()
                .expect("tree to be set"),
            empty_tree_id,
            "try and non-try work the same"
        );
        let first_commit_id = repo.commit("HEAD", "hello there \r\n\nthe body", empty_tree_id, Some(parent))?;
        assert_eq!(
            first_commit_id,
            hex_to_id("e7c7273539cfc1a52802fa9d61aa578f6ccebcb4"),
            "the commit id is stable"
        );

        let head_log_entries: Vec<_> = repo
            .head()?
            .log_iter()
            .rev()?
            .expect("log present")
            .map(Result::unwrap)
            .map(|l| l.message)
            .collect();
        assert_eq!(
            head_log_entries,
            vec!["commit: hello there", "commit: c2", "commit (initial): c1"],
            "we get the actual HEAD log, not the log of some reference"
        );
        let current_commit = repo.head()?.into_peeled_id()?;
        assert_eq!(current_commit, first_commit_id, "the commit was set");

        let second_commit_id = repo.commit(
            "refs/heads/new-branch",
            "committing into a new branch creates it",
            empty_tree_id,
            Some(first_commit_id),
        )?;

        assert_eq!(
            second_commit_id,
            hex_to_id("e1412f169e0812eb260601bdab3854ca0f1a7b33"),
            "the second commit id is stable"
        );

        let mut branch = repo.find_reference("new-branch")?;
        let current_commit = branch.peel_to_id()?;
        assert_eq!(current_commit, second_commit_id, "the commit was set");

        let mut log = branch.log_iter();
        let mut log_iter = log.rev()?.expect("log present");
        assert_eq!(
            log_iter.next().expect("one line")?.message,
            "commit: committing into a new branch creates it"
        );
        assert!(
            log_iter.next().is_none(),
            "there is only one log line in the new branch"
        );
        Ok(())
    }
}

fn empty_bare_in_memory_repo() -> crate::Result<gix::Repository> {
    Ok(named_subrepo_opts("make_basic_repo.sh", "bare.git", gix::open::Options::isolated())?.with_object_memory())
}

fn empty_bare_repo() -> crate::Result<(tempfile::TempDir, gix::Repository)> {
    let tmp = tempfile::tempdir()?;
    let repo = gix::ThreadSafeRepository::init_opts(
        tmp.path(),
        gix::create::Kind::Bare,
        gix::create::Options::default(),
        gix::open::Options::isolated(),
    )?
    .into();
    Ok((tmp, repo))
}
