mod lookup_ref_delta_objects {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use gix_hash::{ObjectId, oid};
    use gix_object::{Data, find::Error};
    use gix_pack::data::{entry::Header, input, input::LookupRefDeltaObjectsIter};

    use crate::hex_to_id;

    const D_A: &[u8] = b"a";
    const D_B: &[u8] = b"bb";
    const D_C: &[u8] = b"ccc";
    const D_D: &[u8] = b"dddd";

    fn base() -> Header {
        Header::Blob
    }
    fn delta_ofs(offset: u64) -> Header {
        Header::OfsDelta { base_distance: offset }
    }
    fn delta_ref(id: ObjectId) -> Header {
        Header::RefDelta { base_id: id }
    }

    fn extract_delta_offset(header: Header) -> u64 {
        match header {
            Header::OfsDelta { base_distance } => base_distance,
            _ => unreachable!("this is supposed to be an offset header, was {:?}", header),
        }
    }

    fn entry(header: Header, data: &'static [u8]) -> input::Entry {
        let obj = gix_object::Data {
            kind: header.as_kind().unwrap_or(gix_object::Kind::Blob),
            object_hash: gix_testtools::object_hash(),
            data,
        };
        let mut entry = input::Entry::from_data_obj(&obj, 0).expect("valid object");
        entry.header = header;
        entry.header_size = header.size(data.len() as u64) as u16;
        entry
    }

    fn compute_offsets(mut entries: Vec<input::Entry>) -> Vec<input::Entry> {
        let mut offset = 0;
        for entry in &mut entries {
            entry.pack_offset = offset;
            offset += entry.bytes_in_pack();
        }
        entries
    }

    fn validate_pack_offsets(entries: &[input::Entry]) {
        let mut offset = 0;
        for (eid, entry) in entries.iter().enumerate() {
            assert_eq!(entry.pack_offset, offset, "invalid pack offset for entry {eid}");
            offset += entry.bytes_in_pack();
        }
    }

    fn into_results_iter(
        entries: Vec<input::Entry>,
    ) -> impl ExactSizeIterator<Item = Result<input::Entry, input::Error>> {
        entries.into_iter().map(Ok)
    }

    struct FindData<'a> {
        calls: &'a AtomicUsize,
        data: Option<&'a [u8]>,
    }

    impl<'a> FindData<'a> {
        fn new(data: impl Into<Option<&'a [u8]>>, calls: &'a AtomicUsize) -> Self {
            FindData {
                data: data.into(),
                calls,
            }
        }
    }

    impl gix_object::Find for FindData<'_> {
        fn try_find<'a>(&self, id: &oid, buf: &'a mut Vec<u8>) -> Result<Option<Data<'a>>, Error> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            if let Some(data) = self.data {
                buf.resize(data.len(), 0);
                buf.copy_from_slice(data);
                Ok(Some(gix_object::Data {
                    kind: gix_object::Kind::Blob,
                    object_hash: id.kind(),
                    data: buf.as_slice(),
                }))
            } else {
                Ok(None)
            }
        }
    }

    #[test]
    fn only_ref_deltas_are_handled() -> crate::Result {
        let input = compute_offsets(vec![entry(base(), D_A), entry(delta_ofs(100), D_B)]);
        let expected = input.clone();
        let actual = LookupRefDeltaObjectsIter::new(into_results_iter(input), gix_object::find::Never)
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(actual, expected, "it won't change the input at all");
        validate_pack_offsets(&actual);
        Ok(())
    }

    #[test]
    fn ref_deltas_have_their_base_injected_if_not_done_before_and_all_future_entries_are_offset() {
        let first_id = hex_to_id("0000000000000000000000000000000000000001");
        let second_id = hex_to_id("0000000000000000000000000000000000000002");
        let first = entry(delta_ref(first_id), D_A);

        let inserted_data = D_D;
        let mut inserted = entry(base(), inserted_data);
        let second = entry(delta_ref(second_id), D_B);
        let third_entry = entry(delta_ofs(second.bytes_in_pack()), D_C);
        let fourth_entry = entry(delta_ofs(third_entry.bytes_in_pack()), D_D);
        let fifth = entry(delta_ref(second_id), D_A);
        let input = compute_offsets(vec![first, second, third_entry, fourth_entry, fifth]);

        let calls = AtomicUsize::default();
        let input_entries = into_results_iter(input);
        let actual_size = input_entries.size_hint();
        let db = FindData::new(inserted_data, &calls);
        let iter = LookupRefDeltaObjectsIter::new(input_entries, &db);
        assert_eq!(
            iter.size_hint(),
            (actual_size.0, actual_size.1.map(|s| s * 2)),
            "size hints are estimated and the upper bound reflects the worst-case scenario for the amount of possible objects"
        );
        let actual = iter.collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(calls.load(Ordering::Relaxed), 2, "there is only two objects to insert");
        assert_eq!(actual.len(), 7, "two object was inserted");

        assert_eq!(&actual[0], &inserted, "first object is inserted one");

        let altered = &actual[1];
        assert_eq!(
            extract_delta_offset(altered.header),
            inserted.bytes_in_pack(),
            "former first entry is now an offset delta pointing at the item before"
        );

        inserted.pack_offset = inserted.bytes_in_pack() + altered.bytes_in_pack();
        assert_eq!(&actual[2], &inserted, "third object is a newly inserted one, too");

        let altered = &actual[3];
        assert_eq!(
            extract_delta_offset(altered.header),
            inserted.bytes_in_pack(),
            "former second entry is now an offset delta pointing at the inserted item before"
        );

        let third = &actual[4];
        assert_eq!(
            extract_delta_offset(third.header),
            altered.bytes_in_pack(),
            "delta offset was adjusted to deal with change in size of predecessor(s)"
        );
        let fourth = &actual[5];
        assert_eq!(
            extract_delta_offset(fourth.header),
            third.bytes_in_pack(),
            "the fourth header base distance was adjusted accordingly"
        );

        let fifth = &actual[6];
        assert_eq!(
            fifth.pack_offset - extract_delta_offset(fifth.header),
            actual[2].pack_offset,
            "the fifth entry points exactly to the second inserted object, as objects are inserted only once"
        );

        validate_pack_offsets(&actual);
    }

    #[test]
    fn lookup_errors_trigger_a_fuse_and_stop_iteration() {
        let input = vec![entry(delta_ref(gix_hash::Kind::Sha1.null()), D_A), entry(base(), D_B)];
        let calls = AtomicUsize::default();
        let db = FindData::new(None, &calls);
        let mut result = LookupRefDeltaObjectsIter::new(into_results_iter(input), &db).collect::<Vec<_>>();
        assert_eq!(calls.load(Ordering::Relaxed), 1, "it tries to lookup the object");
        assert_eq!(result.len(), 1, "the error stops iteration");
        assert!(matches!(
            result.pop().expect("one"),
            Err(input::Error::NotFound {
                object_id
            }) if object_id == gix_hash::Kind::Sha1.null()
        ));
    }

    #[test]
    fn inner_errors_are_passed_on() {
        let input = vec![
            Ok(entry(base(), D_A)),
            Err(input::Error::NotFound {
                object_id: gix_hash::Kind::Sha1.null(),
            }),
            Ok(entry(base(), D_B)),
        ];
        let expected = vec![
            Ok(entry(base(), D_A)),
            Err(input::Error::NotFound {
                object_id: gix_hash::Kind::Sha1.null(),
            }),
            Ok(entry(base(), D_B)),
        ];
        let actual = LookupRefDeltaObjectsIter::new(input.into_iter(), gix_object::find::Never).collect::<Vec<_>>();
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
        }
    }
}
