mod locate {
    use bstr::ByteSlice;
    use gix_object::Kind;
    use gix_odb::pack;

    use crate::{SMALL_PACK_INDEX, fixture_path, hex_to_id};

    fn locate<'a>(hex_id: &str, out: &'a mut Vec<u8>) -> gix_object::Data<'a> {
        let bundle = pack::Bundle::at(fixture_path(SMALL_PACK_INDEX), gix_hash::Kind::Sha1).expect("pack and idx");
        bundle
            .find(
                &hex_to_id(hex_id),
                out,
                &mut gix_zlib::Inflate::default(),
                &mut pack::cache::Never,
            )
            .expect("read success")
            .expect("id present")
            .0
    }

    mod locate_and_verify {
        use gix_odb::pack;

        use crate::{PACKS_AND_INDICES, fixture_path};

        #[test]
        fn all() -> Result<(), Box<dyn std::error::Error>> {
            for (index_path, data_path) in PACKS_AND_INDICES {
                // both paths are equivalent
                pack::Bundle::at(fixture_path(index_path), gix_hash::Kind::Sha1)?;
                let bundle = pack::Bundle::at(fixture_path(data_path), gix_hash::Kind::Sha1)?;

                let mut buf = Vec::new();
                for entry in bundle.index.iter() {
                    let (obj, _location) = bundle
                        .find(
                            &entry.oid,
                            &mut buf,
                            &mut gix_zlib::Inflate::default(),
                            &mut pack::cache::Never,
                        )?
                        .expect("id present");
                    obj.verify_checksum(&entry.oid)?;
                }
            }
            Ok(())
        }
    }

    #[test]
    fn blob() -> Result<(), Box<dyn std::error::Error>> {
        let mut out = Vec::new();
        let obj = locate("bd46bb3f5bb4ca5431770c4fde0735fb89d382f3", &mut out);

        assert_eq!(
            obj.data.as_bstr(),
            b"GitPython is a python library used to interact with Git repositories.\n\nHi there\n".as_bstr()
        );
        assert_eq!(obj.kind, Kind::Blob);
        let object = obj.decode()?;
        assert_eq!(object.kind(), Kind::Blob);
        assert_eq!(object.as_blob().expect("blob").data, obj.data);
        Ok(())
    }

    #[test]
    fn tree() -> Result<(), Box<dyn std::error::Error>> {
        let mut out = Vec::new();
        let obj = locate("e90926b07092bccb7bf7da445fae6ffdfacf3eae", &mut out);

        assert_eq!(obj.kind, Kind::Tree);
        assert_eq!(obj.decode()?.kind(), Kind::Tree);
        Ok(())
    }

    #[test]
    fn commit() -> Result<(), Box<dyn std::error::Error>> {
        let mut out = Vec::new();
        let obj = locate("779c5451ba9fe210ffd1f55db202e55f51acecac", &mut out);

        assert_eq!(obj.kind, Kind::Commit);
        assert_eq!(obj.decode()?.kind(), Kind::Commit);
        Ok(())
    }
}

#[cfg(all(not(feature = "wasm"), feature = "streaming-input"))]
mod write_to_directory {
    use std::{
        fs,
        io::{Cursor, Write},
        path::Path,
        sync::atomic::AtomicBool,
    };

    use gix_features::progress;
    use gix_odb::pack;
    use gix_testtools::tempfile::TempDir;

    use crate::{SMALL_PACK, SMALL_PACK_INDEX, error_chain_contains_message, fixture_path};

    fn expected_outcome() -> Result<pack::bundle::write::Outcome, Box<dyn std::error::Error>> {
        Ok(pack::bundle::write::Outcome {
            index: pack::index::write::Outcome {
                index_version: pack::index::Version::V2,
                index_hash: gix_hash::ObjectId::from_hex(b"544a7204a55f6e9cacccf8f6e191ea8f83575de3")?,
                data_hash: gix_hash::ObjectId::from_hex(b"0f3ea84cd1bba10c2a03d736a460635082833e59")?,
                num_objects: 42,
            },
            pack_version: pack::data::Version::V2,
            index_path: None,
            data_path: None,
            keep_path: None,
            object_hash: gix_hash::Kind::Sha1,
        })
    }

    #[test]
    fn without_providing_one() -> Result<(), Box<dyn std::error::Error>> {
        let res = write_pack(None::<&Path>, SMALL_PACK)?;
        assert_eq!(res, expected_outcome()?);
        assert_eq!(
            res.index.index_hash,
            pack::index::File::at(fixture_path(SMALL_PACK_INDEX), gix_hash::Kind::Sha1)?.index_checksum()
        );
        assert!(res.to_bundle().is_none());
        Ok(())
    }

    #[test]
    fn given_a_directory() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let mut res = write_pack(Some(&dir), SMALL_PACK)?;
        let (index_path, data_path, keep_path) = (res.index_path.take(), res.data_path.take(), res.keep_path.take());
        assert_eq!(res, expected_outcome()?);
        let mut sorted_entries = fs::read_dir(&dir)?.filter_map(Result::ok).collect::<Vec<_>>();
        sorted_entries.sort_by_key(fs::DirEntry::file_name);
        assert_eq!(
            sorted_entries.len(),
            3,
            "we want a pack and the corresponding index and the keep file"
        );

        let pack_hash = res.index.data_hash.to_hex();
        assert_eq!(file_name(&sorted_entries[0]), format!("pack-{pack_hash}.idx"));
        assert_eq!(Some(sorted_entries[0].path()), index_path);
        assert_eq!(file_name(&sorted_entries[1]), format!("pack-{pack_hash}.keep"));
        assert_eq!(Some(sorted_entries[1].path()), keep_path);
        assert_eq!(file_name(&sorted_entries[2]), format!("pack-{pack_hash}.pack"));
        assert_eq!(Some(sorted_entries[2].path()), data_path);

        res.index_path = index_path;
        assert!(res.to_bundle().transpose()?.is_some());
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn immutable_bundle_permissions_match_git_after_umask() -> gix_testtools::Result {
        use std::{os::unix::fs::PermissionsExt, process::Command};

        static SHOULD_INTERRUPT: AtomicBool = AtomicBool::new(false);
        for mask in [0o022, 0o077] {
            if !gix_testtools::run_with_umask(mask)? {
                continue;
            }
            for (setting, permissions) in [("false", 0), ("group", 0o660), ("all", 0o664), ("0640", -0o640)] {
                let dir = TempDir::new()?;
                gix_testtools::git(
                    dir.path(),
                    &format!("-c core.sharedRepository={setting} init --bare --object-format=sha1 git"),
                )?;
                let git_dir = dir.path().join("git");
                // Receiving a pack must recreate a missing pack directory with the repository's sharing policy.
                fs::remove_dir(git_dir.join("objects/pack"))?;
                let output = Command::new(gix_path::env::exe_invocation())
                    .current_dir(&git_dir)
                    .args(["index-pack", "--stdin", "--keep=permissions baseline"])
                    .env_remove("GIT_DIR")
                    .env_remove("GIT_WORK_TREE")
                    .env_remove("GIT_COMMON_DIR")
                    .env_remove("GIT_OBJECT_DIRECTORY")
                    .env("GIT_CONFIG_NOSYSTEM", "1")
                    .env("GIT_CONFIG_GLOBAL", "/dev/null")
                    .stdin(fs::File::open(fixture_path(SMALL_PACK))?)
                    .output()?;
                assert!(
                    output.status.success(),
                    "Git can index the baseline pack: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                for eager in [false, true] {
                    let actual_dir = dir.path().join(format!("actual-{eager}"));
                    let options = pack::bundle::write::Options {
                        thread_limit: Some(1),
                        shared_repository_permissions: permissions,
                        ..Default::default()
                    };
                    let input = fs::read(fixture_path(SMALL_PACK))?;
                    let outcome = if eager {
                        pack::Bundle::write_to_directory_eagerly(
                            Box::new(Cursor::new(input)),
                            None,
                            Some(actual_dir.clone()),
                            &mut progress::Discard,
                            &SHOULD_INTERRUPT,
                            None::<gix_object::find::Never>,
                            gix_hash::Kind::Sha1,
                            options,
                        )?
                    } else {
                        pack::Bundle::write_to_directory(
                            &mut Cursor::new(input),
                            Some(&actual_dir),
                            &mut progress::Discard,
                            &SHOULD_INTERRUPT,
                            None::<gix_object::find::Never>,
                            gix_hash::Kind::Sha1,
                            options,
                        )?
                    };
                    let actual = outcome.data_path.as_ref().expect("the pack contains objects");
                    let expected = git_dir
                        .join("objects/pack")
                        .join(actual.file_name().expect("pack filename"));
                    assert_eq!(
                        fs::metadata(&actual_dir)?.permissions().mode() & 0o7777,
                        fs::metadata(git_dir.join("objects/pack"))?.permissions().mode() & 0o7777,
                        "shared={setting}, umask={mask:o}, eager={eager}: newly created pack directories follow Git"
                    );
                    for extension in ["pack", "idx", "keep"] {
                        assert_eq!(
                            fs::metadata(actual.with_extension(extension))?.permissions().mode() & 0o7777,
                            fs::metadata(expected.with_extension(extension))?.permissions().mode() & 0o7777,
                            "shared={setting}, umask={mask:o}, eager={eager}: {extension} permissions follow Git"
                        );
                    }
                    assert!(
                        outcome.to_bundle().transpose()?.is_some(),
                        "the published pack and index can be opened"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn an_existing_keep_marker_is_preserved_and_not_owned_by_the_receiver() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let original = write_pack(Some(&dir), SMALL_PACK)?;
        let keep_path = original.keep_path.expect("new packs are protected");
        fs::remove_file(original.data_path.expect("nonempty pack"))?;
        fs::remove_file(original.index_path.expect("index was written"))?;
        fs::write(&keep_path, b"another operation")?;
        let received = write_pack(Some(&dir), SMALL_PACK)?;
        assert_eq!(
            received.keep_path, None,
            "the caller must not remove another operation's marker"
        );
        assert_eq!(
            fs::read(&keep_path)?,
            b"another operation",
            "existing keep markers are not truncated"
        );
        assert!(
            received.to_bundle().transpose()?.is_some(),
            "receiving the pack still succeeds"
        );
        Ok(())
    }

    /// A forward reference is a `REF_DELTA` stored before the object named as its base.
    /// Unlike `OFS_DELTA`, its object ID can name an object at any position in the pack.
    ///
    /// Git normally writes bases first, but sends thin packs which omit bases the receiver
    /// already has. `index-pack --fix-thin` makes such packs self-contained by appending
    /// those bases, leaving the original deltas as forward references.
    #[test]
    fn in_pack_ref_deltas_with_forward_references() -> Result<(), Box<dyn std::error::Error>> {
        for object_hash in [gix_hash::Kind::Sha1, gix_hash::Kind::Sha256] {
            for objects in [
                &[b"A".as_slice(), b"B".as_slice()][..],
                &[b"A".as_slice(), b"B".as_slice(), b"C".as_slice()][..],
            ] {
                let pack_data = ref_delta_pack(object_hash, objects, pack::data::Version::V2)?;
                for lookup in [None, Some(gix_object::find::Never)] {
                    let dir = TempDir::new()?;
                    let mut input = Cursor::new(pack_data.clone());
                    let outcome = pack::Bundle::write_to_directory(
                        &mut input,
                        Some(dir.as_ref()),
                        &mut progress::Discard,
                        &AtomicBool::new(false),
                        lookup,
                        object_hash,
                        pack::bundle::write::Options {
                            thread_limit: None,
                            iteration_mode: pack::data::input::Mode::Verify,
                            index_version: pack::index::Version::V2,
                            alloc_limit_bytes: None,
                            compression: gix_zlib::Compression::BEST_SPEED,
                            shared_repository_permissions: 0,
                        },
                    )?;
                    assert_eq!(
                        outcome.index.num_objects as usize,
                        objects.len(),
                        "all in-pack objects are indexed"
                    );

                    let bundle = outcome
                        .to_bundle()
                        .transpose()?
                        .expect("writing to a directory creates a bundle");
                    let mut buf = Vec::new();
                    for expected in objects {
                        let id = gix_object::compute_hash(object_hash, gix_object::Kind::Blob, expected)?;
                        let object = bundle
                            .find(
                                &id,
                                &mut buf,
                                &mut gix_zlib::Inflate::default(),
                                &mut pack::cache::Never,
                            )?
                            .expect("object is indexed")
                            .0;
                        assert_eq!(object.kind, gix_object::Kind::Blob);
                        assert_eq!(object.data, *expected, "the ref-delta is fully resolved");
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn version_3_with_thin_pack_lookup() -> Result<(), Box<dyn std::error::Error>> {
        let object_hash = gix_hash::Kind::Sha1;
        let pack_data = ref_delta_pack(
            object_hash,
            &[b"A".as_slice(), b"B".as_slice()],
            pack::data::Version::V3,
        )?;
        let outcome = pack::Bundle::write_to_directory(
            &mut Cursor::new(pack_data),
            Some(TempDir::new()?.as_ref()),
            &mut progress::Discard,
            &AtomicBool::new(false),
            Some(gix_object::find::Never),
            object_hash,
            pack::bundle::write::Options {
                iteration_mode: pack::data::input::Mode::Verify,
                ..Default::default()
            },
        )?;
        assert_eq!(
            outcome.pack_version,
            pack::data::Version::V3,
            "the rewritten pack preserves its supported input version"
        );
        Ok(())
    }

    #[test]
    fn unresolved_ref_delta_base_is_reported() -> Result<(), Box<dyn std::error::Error>> {
        let object_hash = gix_hash::Kind::Sha1;
        let base_id = object_hash.null();
        let delta = [0, 0];
        let mut pack_data = pack::data::header::encode(pack::data::Version::V2, 1).to_vec();
        pack::data::entry::Header::RefDelta { base_id }.write_to(delta.len() as u64, &mut pack_data)?;
        pack_data.extend(deflate(&delta)?);
        let mut hasher = gix_hash::hasher(object_hash);
        hasher.update(&pack_data);
        pack_data.extend_from_slice(hasher.try_finalize()?.as_slice());

        let err = pack::Bundle::write_to_directory(
            &mut Cursor::new(pack_data),
            None,
            &mut progress::Discard,
            &AtomicBool::new(false),
            None::<gix_object::find::Never>,
            object_hash,
            pack::bundle::write::Options {
                thread_limit: None,
                iteration_mode: pack::data::input::Mode::Verify,
                index_version: pack::index::Version::V2,
                alloc_limit_bytes: None,
                compression: gix_zlib::Compression::BEST_SPEED,
                shared_repository_permissions: 0,
            },
        )
        .expect_err("a ref-delta without an in-pack or external base cannot be indexed");
        let expected = format!("The ref-delta base object {base_id} could not be found");
        assert!(
            error_chain_contains_message(&err, &expected),
            "the missing base id is retained in the error chain"
        );
        Ok(())
    }

    #[test]
    fn respects_alloc_limit_bytes() -> Result<(), Box<dyn std::error::Error>> {
        let pack_file = fs::File::open(fixture_path(SMALL_PACK))?;
        static SHOULD_INTERRUPT: AtomicBool = AtomicBool::new(false);

        let prevent_allocation = Some(0);
        let err = pack::Bundle::write_to_directory_eagerly(
            Box::new(pack_file),
            None,
            None::<&Path>,
            &mut progress::Discard,
            &SHOULD_INTERRUPT,
            None::<gix_object::find::Never>,
            gix_hash::Kind::Sha1,
            pack::bundle::write::Options {
                thread_limit: None,
                iteration_mode: pack::data::input::Mode::Verify,
                index_version: pack::index::Version::V2,
                alloc_limit_bytes: prevent_allocation,
                compression: gix_zlib::Compression::BEST_SPEED,
                shared_repository_permissions: 0,
            },
        )
        .expect_err("a zero allocation limit rejects non-empty delta-tree storage");

        assert!(
            error_chain_contains_message(&err, "The pack delta tree is too large to fit in memory"),
            "bundle writing must forward its allocation limit to index writing"
        );
        Ok(())
    }

    fn file_name(entry: &fs::DirEntry) -> String {
        entry.path().file_name().unwrap().to_str().unwrap().to_owned()
    }

    fn write_pack(
        directory: Option<impl AsRef<Path>>,
        pack_file: &str,
    ) -> Result<pack::bundle::write::Outcome, Box<dyn std::error::Error>> {
        let pack_file = fs::File::open(fixture_path(pack_file))?;
        static SHOULD_INTERRUPT: AtomicBool = AtomicBool::new(false);
        pack::Bundle::write_to_directory_eagerly(
            Box::new(pack_file),
            None,
            directory,
            &mut progress::Discard,
            &SHOULD_INTERRUPT,
            None::<gix_object::find::Never>,
            gix_hash::Kind::Sha1,
            pack::bundle::write::Options {
                thread_limit: None,
                iteration_mode: pack::data::input::Mode::Verify,
                index_version: pack::index::Version::V2,
                alloc_limit_bytes: None,
                compression: gix_zlib::Compression::BEST_SPEED,
                shared_repository_permissions: 0,
            },
        )
        .map_err(Into::into)
    }

    /// Build a complete pack whose one-byte blobs form a forward `REF_DELTA` chain.
    /// `objects` lists the base first, but entries are written in reverse dependency order:
    ///
    /// ```text
    /// objects = [A, B, C]
    ///
    /// increasing pack offset ────────────────────────────────────────────────►
    /// [REF_DELTA base=oid(B), yields C] → [REF_DELTA base=oid(A), yields B] → [BLOB A]
    /// ```
    ///
    /// Each arrow points to the entry needed as the base, so every delta refers forward.
    /// With `[A, B]`, the first entry is omitted, leaving `B → A`.
    fn ref_delta_pack(
        object_hash: gix_hash::Kind,
        objects: &[&'static [u8]],
        version: pack::data::Version,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut pack_data = pack::data::header::encode(version, objects.len() as u32).to_vec();
        for pair in objects.windows(2).rev() {
            let (base, resolved) = (pair[0], pair[1]);
            let base_id = gix_object::compute_hash(object_hash, gix_object::Kind::Blob, base)?;
            let delta = [1, 1, 1, resolved[0]];
            pack::data::entry::Header::RefDelta { base_id }.write_to(delta.len() as u64, &mut pack_data)?;
            pack_data.extend(deflate(&delta)?);
        }
        let base = objects[0];
        pack::data::entry::Header::Blob.write_to(base.len() as u64, &mut pack_data)?;
        pack_data.extend(deflate(base)?);

        let mut hasher = gix_hash::hasher(object_hash);
        hasher.update(&pack_data);
        pack_data.extend_from_slice(hasher.try_finalize()?.as_slice());
        Ok(pack_data)
    }

    fn deflate(input: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut out = gix_zlib::stream::deflate::Write::new(Vec::new(), gix_zlib::Compression::BEST_SPEED);
        out.write_all(input)?;
        out.flush()?;
        Ok(out.into_inner())
    }
}
