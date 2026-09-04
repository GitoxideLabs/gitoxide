#![no_main]

use std::{
    collections::{BTreeMap, HashSet},
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};

use gix_object::{Exists as _, Find as _, Write as _};
use libfuzzer_sys::fuzz_target;

#[path = "../../tests/tools/odb.rs"]
mod odb_fixture;

use odb_fixture::{Action, Component, Database, OdbFixture, Pack};

const INSTRUCTION_SIZE: usize = 4;
const MAX_OPERATIONS: usize = 256;
const NUM_HANDLES: usize = 4;
const NUM_LOOSE_OBJECTS: usize = 4;

fuzz_target!(|data: &[u8]| {
    let mut machine = Machine::new().expect("the embedded ODB fixture is valid");
    let (instructions, _) = data.as_chunks::<INSTRUCTION_SIZE>();
    for (index, instruction) in instructions.iter().take(MAX_OPERATIONS).enumerate() {
        let (opcode, outcome) = machine.execute(instruction);
        if trace_enabled() {
            eprintln!("{index:03}: {opcode:?} {instruction:02x?} -> {outcome:?}");
        }
        machine.assert_handle_counts();
    }
});

fn trace_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("GIX_ODB_FUZZ_TRACE").is_some())
}

struct Handle {
    value: gix_odb::Handle,
    store_id: u32,
    stable: bool,
    refresh: RefreshPolicy,
    observed_packed_object: Option<gix_hash::ObjectId>,
    stable_locations: Vec<gix_odb::pack::data::entry::Location>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RefreshPolicy {
    Strict,
    AfterDuration(Duration),
    Never,
}

struct Machine {
    fixture: OdbFixture,
    handles: [Option<Handle>; NUM_HANDLES],
    loose_ids: [Option<gix_hash::ObjectId>; NUM_LOOSE_OBJECTS],
    written_loose_ids: HashSet<gix_hash::ObjectId>,
    now: Arc<Mutex<Instant>>,
    next_store_id: u32,
}

impl Machine {
    fn new() -> odb_fixture::Result<Self> {
        Ok(Machine {
            fixture: OdbFixture::from_embedded_sha1()?,
            handles: std::array::from_fn(|_| None),
            loose_ids: std::array::from_fn(|_| None),
            written_loose_ids: HashSet::new(),
            now: Arc::new(Mutex::new(Instant::now())),
            next_store_id: 0,
        })
    }

    fn execute(&mut self, instruction: &[u8]) -> (Opcode, Outcome) {
        let [opcode, a, b, c] =
            <[u8; INSTRUCTION_SIZE]>::try_from(instruction).expect("chunks have the instruction size");
        let a = operand(a);
        let b = operand(b);
        let c = operand(c);
        let opcode = decode_opcode(opcode);
        let never_refreshes = opcode.queries_handle().then(|| {
            self.handles[a % NUM_HANDLES]
                .as_ref()
                .filter(|handle| handle.refresh == RefreshPolicy::Never)
                .map(|handle| (handle.store_id, handle.value.store_ref().metrics().num_refreshes))
        });
        let outcome = match opcode {
            Opcode::Open => self.open(a, b, c),
            Opcode::Clone => self.clone_handle(a, b),
            Opcode::Drop => self.drop_handle(a),
            Opcode::Never => self.set_never(a),
            Opcode::SetRefresh => self.set_refresh(a, b, c),
            Opcode::AdvanceClock => self.advance_clock(a, b, c),
            Opcode::MarkStale => self.mark_stale(a),
            Opcode::Stable => self.set_stable(a),
            Opcode::Find => self.find(a, b),
            Opcode::Locate => self.locate(a, b),
            Opcode::Exists => self.exists(a, b),
            Opcode::Missing => self.find_missing(a),
            Opcode::Header => self.header(a, b),
            Opcode::IterateStore => self.iterate_store(a, b),
            Opcode::Prefix => self.prefix(a, b, c),
            Opcode::CountStore => self.count_store(a),
            Opcode::WriteLoose => self.write_loose(a, b, c),
            Opcode::QueryLoose => self.query_loose(a, b, c),
            Opcode::RemoveLoose => self.remove_loose(b),
            Opcode::Publish => self.apply(Action::Publish {
                database: database(a),
                pack: pack(b),
                component: component(c),
            }),
            Opcode::Remove => self.apply(Action::Remove {
                database: database(a),
                pack: pack(b),
                component: component(c),
            }),
            Opcode::Corrupt => self.apply(Action::CorruptIndex {
                database: database(a),
                pack: pack(b),
            }),
            Opcode::WriteMultiIndex => {
                let packs = Pack::ALL
                    .into_iter()
                    .enumerate()
                    .filter_map(|(index, pack)| ((b | 1) & (1 << index) != 0).then_some(pack))
                    .collect::<Vec<_>>();
                self.write_multi_index(database(a), packs)
            }
            Opcode::RemoveMultiIndex => self.apply(Action::RemoveMultiIndex { database: database(a) }),
            Opcode::Alternate => self.apply(Action::SetAlternate { enabled: a & 1 != 0 }),
            Opcode::Checkpoint => self.checkpoint(a),
        };
        if let Some((store_id, before)) = never_refreshes.flatten() {
            let handle = self.handles[a % NUM_HANDLES]
                .as_ref()
                .expect("query opcodes retain their handle");
            assert_eq!(handle.store_id, store_id, "query opcodes retain their store");
            let after = handle.value.store_ref().metrics().num_refreshes;
            assert!(
                after == before || (before == 0 && after == 1),
                "a never-refresh handle only performs the initial disk scan: before={before}, after={after}"
            );
        }
        if outcome == Outcome::Query(Query::Found) && matches!(opcode, Opcode::Find | Opcode::Exists | Opcode::Header) {
            let ids = self.fixture.manifest.object_ids().collect::<Vec<_>>();
            if let Some(handle) = self.handles[a % NUM_HANDLES].as_mut() {
                handle.observed_packed_object = Some(ids[b % ids.len()]);
            }
        }
        if matches!(
            outcome,
            Outcome::Query(Query::Error(QueryError::Refresh)) | Outcome::WriteError(WriteError::Refresh)
        ) {
            self.assert_observed_objects_readable(a);
        }
        self.assert_stable_locations();
        (opcode, outcome)
    }

    fn open(&mut self, handle_index: usize, slots: usize, flags: usize) -> Outcome {
        let slot = handle_index % NUM_HANDLES;
        let store_id = self.next_store_id;
        self.next_store_id = self.next_store_id.wrapping_add(1);
        let slots = if flags & 2 == 0 {
            gix_odb::store::init::Slots::Limit(slots.clamp(1, 16) as u16)
        } else {
            gix_odb::store::init::Slots::Growable {
                initial: slots.min(16) as u16,
            }
        };
        let value = gix_odb::at_opts(
            self.fixture.objects_dir(Database::Primary),
            self.fixture.manifest.object_hash,
            Vec::new(),
            gix_odb::store::init::Options {
                slots,
                use_multi_pack_index: flags & 1 == 0,
                debug: Some(gix_odb::store::init::debug::Options::new(|_| {}).with_clock({
                    let now = Arc::clone(&self.now);
                    move || *now.lock().expect("the deterministic fuzz clock isn't poisoned")
                })),
                ..Default::default()
            },
        )
        .expect("the VM always provides an accessible ODB with valid alternates");
        self.handles[slot] = Some(Handle {
            value,
            store_id,
            stable: false,
            refresh: RefreshPolicy::Strict,
            observed_packed_object: None,
            stable_locations: Vec::new(),
        });
        Outcome::Applied
    }

    fn clone_handle(&mut self, destination: usize, source: usize) -> Outcome {
        let source = source % NUM_HANDLES;
        let destination = destination % NUM_HANDLES;
        let Some(source) = self.handles[source].as_ref() else {
            return Outcome::Skipped(Skip::NoHandle);
        };
        self.handles[destination] = Some(Handle {
            value: source.value.clone(),
            store_id: source.store_id,
            stable: source.stable,
            refresh: source.refresh,
            observed_packed_object: None,
            stable_locations: source.stable_locations.clone(),
        });
        Outcome::Applied
    }

    fn drop_handle(&mut self, handle_index: usize) -> Outcome {
        if self.handles[handle_index % NUM_HANDLES].take().is_some() {
            Outcome::Applied
        } else {
            Outcome::Skipped(Skip::NoHandle)
        }
    }

    fn set_never(&mut self, handle_index: usize) -> Outcome {
        if let Some(handle) = self.handles[handle_index % NUM_HANDLES].as_mut() {
            handle.value.refresh_never();
            handle.refresh = RefreshPolicy::Never;
            Outcome::RefreshConfigured(RefreshPolicy::Never)
        } else {
            Outcome::Skipped(Skip::NoHandle)
        }
    }

    fn set_refresh(&mut self, handle_index: usize, selector: usize, window: usize) -> Outcome {
        let Some(handle) = self.handles[handle_index % NUM_HANDLES].as_mut() else {
            return Outcome::Skipped(Skip::NoHandle);
        };
        let policy = match selector % 3 {
            0 => {
                handle.value.refresh = gix_odb::store::RefreshMode::AfterAllIndicesLoaded;
                RefreshPolicy::Strict
            }
            1 => {
                let window = Duration::from_millis(window as u64);
                handle.value.refresh = gix_odb::store::RefreshMode::AfterDuration(window);
                RefreshPolicy::AfterDuration(window)
            }
            _ => {
                handle.value.refresh_never();
                RefreshPolicy::Never
            }
        };
        handle.refresh = policy;
        Outcome::RefreshConfigured(policy)
    }

    fn advance_clock(&self, a: usize, b: usize, c: usize) -> Outcome {
        let elapsed = Duration::from_millis(a as u64 + ((b as u64) << 4) + ((c as u64) << 8));
        let mut now = self.now.lock().expect("the deterministic fuzz clock isn't poisoned");
        *now = now.checked_add(elapsed).expect("fuzz clock advances stay bounded");
        Outcome::ClockAdvanced(elapsed)
    }

    fn mark_stale(&self, handle_index: usize) -> Outcome {
        let Some(handle) = self.handles[handle_index % NUM_HANDLES].as_ref() else {
            return Outcome::Skipped(Skip::NoHandle);
        };
        handle.value.store_ref().mark_disk_state_stale();
        Outcome::Applied
    }

    fn set_stable(&mut self, handle_index: usize) -> Outcome {
        if let Some(handle) = self.handles[handle_index % NUM_HANDLES].as_mut() {
            handle.value.prevent_pack_unload();
            handle.stable = true;
            Outcome::Applied
        } else {
            Outcome::Skipped(Skip::NoHandle)
        }
    }

    fn find(&self, handle_index: usize, object_index: usize) -> Outcome {
        let Some(handle) = self.handles[handle_index % NUM_HANDLES].as_ref() else {
            return Outcome::Skipped(Skip::NoHandle);
        };
        let ids = self.fixture.manifest.object_ids().collect::<Vec<_>>();
        validate_find(&handle.value, ids[object_index % ids.len()])
    }

    fn locate(&mut self, handle_index: usize, object_index: usize) -> Outcome {
        let Some(handle) = self.handles[handle_index % NUM_HANDLES].as_mut() else {
            return Outcome::Skipped(Skip::NoHandle);
        };
        if !handle.stable {
            return Outcome::Skipped(Skip::NotStable);
        }
        let ids = self.fixture.manifest.object_ids().collect::<Vec<_>>();
        let mut buffer = Vec::new();
        let Some(location) =
            gix_odb::pack::Find::location_by_oid(&handle.value, &ids[object_index % ids.len()], &mut buffer)
        else {
            return Outcome::Query(Query::Missing);
        };
        assert!(
            gix_odb::pack::Find::entry_by_location(&handle.value, &location).is_some(),
            "a newly acquired stable pack location is readable"
        );
        if !handle.stable_locations.contains(&location) {
            handle.stable_locations.push(location);
        }
        Outcome::Query(Query::Found)
    }

    fn exists(&self, handle_index: usize, object_index: usize) -> Outcome {
        let Some(handle) = self.handles[handle_index % NUM_HANDLES].as_ref() else {
            return Outcome::Skipped(Skip::NoHandle);
        };
        let ids = self.fixture.manifest.object_ids().collect::<Vec<_>>();
        Outcome::Query(if handle.value.exists(&ids[object_index % ids.len()]) {
            Query::Found
        } else {
            Query::Missing
        })
    }

    fn find_missing(&self, handle_index: usize) -> Outcome {
        let Some(handle) = self.handles[handle_index % NUM_HANDLES].as_ref() else {
            return Outcome::Skipped(Skip::NoHandle);
        };
        validate_find(&handle.value, self.fixture.manifest.missing_id())
    }

    fn header(&self, handle_index: usize, object_index: usize) -> Outcome {
        let Some(handle) = self.handles[handle_index % NUM_HANDLES].as_ref() else {
            return Outcome::Skipped(Skip::NoHandle);
        };
        let ids = self.fixture.manifest.object_ids().collect::<Vec<_>>();
        let id = ids[object_index % ids.len()];
        let mut buffer = Vec::new();
        match (
            gix_odb::Header::try_header(&handle.value, &id),
            handle.value.try_find(&id, &mut buffer),
        ) {
            (Ok(Some(header)), Ok(Some(object))) => {
                assert_eq!(header.kind(), object.kind);
                assert_eq!(header.size(), object.data.len() as u64);
                Outcome::Query(Query::Found)
            }
            (Ok(None), Ok(None)) => Outcome::Query(Query::Missing),
            (Err(err), _) => query_error(err),
            (_, Err(err)) => query_error(err),
            (_, _) if !self.fixture.is_valid() => Outcome::Query(Query::Stale),
            (header, object) => {
                panic!("header and object lookup disagree: header={header:?}, object={object:?}")
            }
        }
    }

    fn iterate_store(&self, handle_index: usize, ordering: usize) -> Outcome {
        let Some(handle) = self.handles[handle_index % NUM_HANDLES].as_ref() else {
            return Outcome::Skipped(Skip::NoHandle);
        };
        let Ok(iter) = handle.value.iter() else {
            return Outcome::Query(Query::Error(QueryError::Refresh));
        };
        let iter = iter.with_ordering(if ordering & 1 == 0 {
            gix_odb::store::iter::Ordering::PackLexicographicalThenLooseLexicographical
        } else {
            gix_odb::store::iter::Ordering::PackAscendingOffsetThenLooseLexicographical
        });
        let packed_ids = self.fixture.manifest.object_ids().collect::<HashSet<_>>();
        let mut count = 0;
        for id in iter.take(16) {
            let Ok(id) = id else {
                return Outcome::Query(Query::Error(QueryError::LooseTraversal));
            };
            assert!(
                packed_ids.contains(&id) || self.written_loose_ids.contains(&id),
                "store iteration only yields IDs generated by the fixture, got {id}"
            );
            count += 1;
        }
        Outcome::Iterated(count)
    }

    fn prefix(&self, handle_index: usize, object_index: usize, input: usize) -> Outcome {
        let Some(handle) = self.handles[handle_index % NUM_HANDLES].as_ref() else {
            return Outcome::Skipped(Skip::NoHandle);
        };
        let ids = self.fixture.manifest.object_ids().collect::<Vec<_>>();
        let id = ids[object_index % ids.len()];
        let hex_len = 4 + input % (id.kind().len_in_hex() - 3);
        match input % 5 {
            0 => {
                let candidate =
                    gix_odb::store::prefix::disambiguate::Candidate::new(id, hex_len).expect("bounded prefix length");
                match handle.value.disambiguate_prefix(candidate) {
                    Ok(Some(_)) => Outcome::Query(Query::Found),
                    Ok(None) => Outcome::Query(Query::Missing),
                    Err(err) => disambiguate_error(err),
                }
            }
            1 => validate_prefix_lookup(
                &handle.value,
                gix_hash::Prefix::new(&id, hex_len).expect("bounded prefix length"),
                false,
            ),
            2 => validate_prefix_lookup(
                &handle.value,
                gix_hash::Prefix::new(&id, hex_len).expect("bounded prefix length"),
                true,
            ),
            3 => validate_prefix_lookup(
                &handle.value,
                gix_hash::Prefix::from_hex(&self.fixture.manifest.ambiguous_prefix)
                    .expect("the manifest contains a valid prefix"),
                true,
            ),
            _ => validate_prefix_lookup(
                &handle.value,
                gix_hash::Prefix::from(self.fixture.manifest.missing_id()),
                true,
            ),
        }
    }

    fn count_store(&self, handle_index: usize) -> Outcome {
        let Some(handle) = self.handles[handle_index % NUM_HANDLES].as_ref() else {
            return Outcome::Skipped(Skip::NoHandle);
        };
        match handle.value.packed_object_count() {
            Ok(count) => Outcome::Counted(count),
            Err(_) => Outcome::Query(Query::Error(QueryError::Refresh)),
        }
    }

    fn write_loose(&mut self, handle_index: usize, loose_index: usize, content: usize) -> Outcome {
        let Some(handle) = self.handles[handle_index % NUM_HANDLES].as_ref() else {
            return Outcome::Skipped(Skip::NoHandle);
        };
        let payload = [loose_index as u8, content as u8];
        match handle.value.write_buf(gix_object::Kind::Blob, &payload) {
            Ok(id) => {
                self.loose_ids[loose_index % NUM_LOOSE_OBJECTS] = Some(id);
                self.written_loose_ids.insert(id);
                Outcome::Applied
            }
            Err(err) => write_error(err),
        }
    }

    fn query_loose(&self, handle_index: usize, loose_index: usize, input: usize) -> Outcome {
        let Some(handle) = self.handles[handle_index % NUM_HANDLES].as_ref() else {
            return Outcome::Skipped(Skip::NoHandle);
        };
        let Some(id) = self.loose_ids[loose_index % NUM_LOOSE_OBJECTS] else {
            return Outcome::Skipped(Skip::NoLooseObject);
        };
        match input % 3 {
            0 => validate_find(&handle.value, id),
            1 => {
                let mut buffer = Vec::new();
                match (
                    gix_odb::Header::try_header(&handle.value, &id),
                    handle.value.try_find(&id, &mut buffer),
                ) {
                    (Ok(Some(header)), Ok(Some(object))) => {
                        assert_eq!(header.kind(), object.kind, "loose header and object kinds agree");
                        assert_eq!(
                            header.size(),
                            object.data.len() as u64,
                            "loose header and object sizes agree"
                        );
                        Outcome::Query(Query::Found)
                    }
                    (Ok(None), Ok(None)) => Outcome::Query(Query::Missing),
                    (Err(err), _) => query_error(err),
                    (_, Err(err)) => query_error(err),
                    (header, object) => {
                        panic!("loose header and object lookup disagree: header={header:?}, object={object:?}")
                    }
                }
            }
            _ => {
                let hex_len = 4 + input % (id.kind().len_in_hex() - 3);
                let prefix = gix_hash::Prefix::new(&id, hex_len).expect("bounded prefix length");
                match handle.value.lookup_prefix(prefix, None) {
                    Ok(Some(Ok(_))) => Outcome::Query(Query::Found),
                    Ok(Some(Err(()))) => Outcome::Query(Query::Ambiguous),
                    Ok(None) => Outcome::Query(Query::Missing),
                    Err(err) => lookup_error(err),
                }
            }
        }
    }

    fn remove_loose(&mut self, loose_index: usize) -> Outcome {
        let Some(id) = self.loose_ids[loose_index % NUM_LOOSE_OBJECTS] else {
            return Outcome::Skipped(Skip::NoLooseObject);
        };
        self.fixture
            .remove_loose_object(Database::Primary, &id)
            .expect("filesystem-backed loose object removal succeeds");
        Outcome::Applied
    }

    fn checkpoint(&self, handle_index: usize) -> Outcome {
        if !self.fixture.is_valid() {
            return Outcome::Skipped(Skip::FixtureInTransition);
        }
        let Some(handle) = self.handles[handle_index % NUM_HANDLES].as_ref() else {
            return Outcome::Skipped(Skip::NoHandle);
        };
        if handle.stable {
            return if handle.stable_locations.is_empty() {
                Outcome::Skipped(Skip::NoStableLocation)
            } else {
                Outcome::Compared(handle.stable_locations.len())
            };
        }
        if handle.refresh != RefreshPolicy::Strict {
            return Outcome::Skipped(Skip::RefreshNotStrict);
        }

        let store_id = handle.store_id;
        let fresh = gix_odb::at(
            self.fixture.objects_dir(Database::Primary),
            self.fixture.manifest.object_hash,
        )
        .expect("a valid fixture opens");
        let missing = self.fixture.manifest.missing_id();
        let ids = self.fixture.manifest.object_ids().collect::<Vec<_>>();
        let mut compared = 0;
        for handle in
            self.handles.iter().flatten().filter(|handle| {
                handle.store_id == store_id && !handle.stable && handle.refresh == RefreshPolicy::Strict
            })
        {
            let mut buffer = Vec::new();
            if let Err(err) = handle.value.try_find(&missing, &mut buffer) {
                return query_error(err);
            }
            for id in &ids {
                let current = match handle.value.try_find(id, &mut buffer) {
                    Ok(current) => current,
                    Err(err) => return query_error(err),
                };
                let current = current.is_some();
                let expected = fresh
                    .try_find(id, &mut buffer)
                    .expect("reference lookup succeeds")
                    .is_some();
                assert_eq!(
                    current, expected,
                    "strict handles sharing a store agree with a freshly opened store for {id}"
                );
                compared += 1;
            }
        }
        Outcome::Compared(compared)
    }

    fn apply(&mut self, action: Action) -> Outcome {
        self.fixture
            .apply(action)
            .expect("filesystem-backed fixture mutations succeed");
        Outcome::Applied
    }

    fn write_multi_index(&mut self, database: Database, packs: Vec<Pack>) -> Outcome {
        let can_write = self.fixture.can_write_multi_index(database, &packs);
        let result = self.fixture.apply(Action::WriteMultiIndex { database, packs });
        match (can_write, result) {
            (true, Ok(())) => Outcome::Applied,
            (false, Err(_)) => Outcome::Rejected,
            (true, Err(err)) => panic!("a MIDX with readable indices can be written: {err}"),
            (false, Ok(())) => panic!("a MIDX cannot be written from missing or malformed indices"),
        }
    }

    fn assert_handle_counts(&self) {
        let mut expected = BTreeMap::<u32, usize>::new();
        for handle in self.handles.iter().flatten() {
            *expected.entry(handle.store_id).or_default() += 1;
        }
        for handle in self.handles.iter().flatten() {
            assert_eq!(
                handle.value.store_ref().metrics().num_handles,
                expected[&handle.store_id],
                "store metrics count all live handles"
            );
        }
    }

    fn assert_observed_objects_readable(&self, handle_index: usize) {
        if !self.fixture.is_valid() {
            return;
        }
        let selected_index = handle_index % NUM_HANDLES;
        let Some(selected) = self.handles[selected_index].as_ref() else {
            return;
        };
        let reachable = self.fixture.reachable_ids();
        for (index, handle) in self.handles.iter().enumerate().filter_map(|(index, handle)| {
            handle
                .as_ref()
                .filter(|handle| {
                    handle.store_id == selected.store_id
                        && (index == selected_index || (!handle.stable && handle.refresh != RefreshPolicy::Never))
                })
                .map(|handle| (index, handle))
        }) {
            if let Some(id) = handle.observed_packed_object.filter(|id| reachable.contains(id)) {
                assert_eq!(
                    validate_find(&handle.value, id),
                    Outcome::Query(Query::Found),
                    "a failed refresh through handle {selected_index} preserves {id} observed by shared-store handle {index}"
                );
            }
        }
    }

    fn assert_stable_locations(&self) {
        for (index, handle) in self.handles.iter().enumerate().filter_map(|(index, handle)| {
            handle
                .as_ref()
                .filter(|handle| handle.stable)
                .map(|handle| (index, handle))
        }) {
            for location in &handle.stable_locations {
                assert!(
                    gix_odb::pack::Find::entry_by_location(&handle.value, location).is_some(),
                    "stable handle {index} retains pack location {location:?}"
                );
            }
        }
    }
}

fn validate_find(handle: &gix_odb::Handle, id: gix_hash::ObjectId) -> Outcome {
    let mut buffer = Vec::new();
    match handle.try_find(&id, &mut buffer) {
        Ok(Some(object)) => {
            assert_eq!(
                gix_object::compute_hash(id.kind(), object.kind, object.data).expect("fixture hash kind is enabled"),
                id,
                "successful lookups return the requested object"
            );
            Outcome::Query(Query::Found)
        }
        Ok(None) => Outcome::Query(Query::Missing),
        Err(err) => query_error(err),
    }
}

fn query_error(err: gix_object::find::Error) -> Outcome {
    let err = err
        .downcast_ref::<gix_odb::store::find::Error>()
        .expect("dynamic handles report their concrete lookup error");
    Outcome::Query(Query::Error(match err {
        gix_odb::store::find::Error::LoadIndex(_) => QueryError::Refresh,
        gix_odb::store::find::Error::Loose(_) => QueryError::LooseObject,
        gix_odb::store::find::Error::Pack(_)
        | gix_odb::store::find::Error::LoadPack(_)
        | gix_odb::store::find::Error::EntryType(_)
        | gix_odb::store::find::Error::DeltaBaseRecursionLimit { .. }
        | gix_odb::store::find::Error::DeltaBaseMissing { .. }
        | gix_odb::store::find::Error::DeltaBaseLookup { .. } => QueryError::PackedObject,
    }))
}

fn disambiguate_error(err: gix_odb::store::prefix::disambiguate::Error) -> Outcome {
    match err {
        gix_odb::store::prefix::disambiguate::Error::Contains(err) => Outcome::Query(Query::Error(match err {
            gix_odb::store::find::Error::LoadIndex(_) => QueryError::Refresh,
            gix_odb::store::find::Error::Loose(_) => QueryError::LooseObject,
            _ => QueryError::PackedObject,
        })),
        gix_odb::store::prefix::disambiguate::Error::Lookup(err) => lookup_error(err),
    }
}

fn validate_prefix_lookup(handle: &gix_odb::Handle, prefix: gix_hash::Prefix, collect: bool) -> Outcome {
    let mut candidates = HashSet::new();
    let result = handle.lookup_prefix(prefix, collect.then_some(&mut candidates));
    assert!(
        candidates
            .iter()
            .all(|candidate| prefix.cmp_oid(candidate) == std::cmp::Ordering::Equal),
        "all returned candidates match the requested prefix"
    );
    match result {
        Ok(Some(Ok(id))) => {
            assert_eq!(
                prefix.cmp_oid(&id),
                std::cmp::Ordering::Equal,
                "the unique result matches the requested prefix"
            );
            if collect {
                assert_eq!(
                    candidates,
                    HashSet::from([id]),
                    "the candidate set has the unique result"
                );
            }
            Outcome::Query(Query::Found)
        }
        Ok(Some(Err(()))) => {
            if collect {
                assert!(candidates.len() > 1, "ambiguity has at least two candidates");
            }
            Outcome::Query(Query::Ambiguous)
        }
        Ok(None) => {
            assert!(candidates.is_empty(), "a missing prefix has no candidates");
            Outcome::Query(Query::Missing)
        }
        Err(err) => lookup_error(err),
    }
}

fn lookup_error(err: gix_odb::store::prefix::lookup::Error) -> Outcome {
    Outcome::Query(Query::Error(match err {
        gix_odb::store::prefix::lookup::Error::LooseWalkDir(_) => QueryError::LooseTraversal,
        gix_odb::store::prefix::lookup::Error::LoadIndex(_) => QueryError::Refresh,
    }))
}

fn write_error(err: gix_object::write::Error) -> Outcome {
    let kind = if err.is::<gix_odb::store::load_index::Error>() || err.is::<Box<gix_odb::store::load_index::Error>>() {
        WriteError::Refresh
    } else if err.is::<gix_odb::loose::write::Error>() {
        WriteError::LooseObject
    } else if err.is::<std::io::Error>() {
        WriteError::Io
    } else {
        panic!("dynamic handles report a known write error, got {err}")
    };
    Outcome::WriteError(kind)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Outcome {
    Applied,
    Rejected,
    RefreshConfigured(RefreshPolicy),
    ClockAdvanced(Duration),
    Skipped(Skip),
    Query(Query),
    Counted(u64),
    Iterated(usize),
    Compared(usize),
    WriteError(WriteError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Skip {
    NoHandle,
    NoLooseObject,
    NotStable,
    NoStableLocation,
    FixtureInTransition,
    RefreshNotStrict,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Query {
    Found,
    Missing,
    Ambiguous,
    Stale,
    Error(QueryError),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QueryError {
    Refresh,
    LooseTraversal,
    LooseObject,
    PackedObject,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WriteError {
    Refresh,
    LooseObject,
    Io,
}

#[derive(Clone, Copy, Debug)]
enum Opcode {
    Open,
    Clone,
    Drop,
    Never,
    SetRefresh,
    AdvanceClock,
    MarkStale,
    Stable,
    Find,
    Locate,
    Exists,
    Missing,
    Header,
    IterateStore,
    Prefix,
    CountStore,
    WriteLoose,
    QueryLoose,
    RemoveLoose,
    Publish,
    Remove,
    Corrupt,
    WriteMultiIndex,
    RemoveMultiIndex,
    Alternate,
    Checkpoint,
}

impl Opcode {
    fn queries_handle(self) -> bool {
        matches!(
            self,
            Opcode::Find
                | Opcode::Locate
                | Opcode::Exists
                | Opcode::Missing
                | Opcode::Header
                | Opcode::Prefix
                | Opcode::WriteLoose
                | Opcode::QueryLoose
                | Opcode::Checkpoint
        )
    }
}

fn decode_opcode(byte: u8) -> Opcode {
    match byte {
        b'O' => Opcode::Open,
        b'C' => Opcode::Clone,
        b'D' => Opcode::Drop,
        b'N' => Opcode::Never,
        b'R' => Opcode::SetRefresh,
        b'T' => Opcode::AdvanceClock,
        b'!' => Opcode::MarkStale,
        b'S' => Opcode::Stable,
        b'F' => Opcode::Find,
        b'G' => Opcode::Locate,
        b'E' => Opcode::Exists,
        b'?' => Opcode::Missing,
        b'H' => Opcode::Header,
        b'I' => Opcode::IterateStore,
        b'P' => Opcode::Prefix,
        b'#' => Opcode::CountStore,
        b'W' => Opcode::WriteLoose,
        b'Q' => Opcode::QueryLoose,
        b'r' => Opcode::RemoveLoose,
        b'+' => Opcode::Publish,
        b'-' => Opcode::Remove,
        b'X' => Opcode::Corrupt,
        b'M' => Opcode::WriteMultiIndex,
        b'm' => Opcode::RemoveMultiIndex,
        b'L' => Opcode::Alternate,
        b'K' => Opcode::Checkpoint,
        byte => match byte % 23 {
            0 => Opcode::Open,
            1 => Opcode::Clone,
            2 => Opcode::Drop,
            3 => Opcode::Never,
            4 => Opcode::Stable,
            5 => Opcode::Find,
            6 => Opcode::Missing,
            7 => Opcode::Header,
            8 => Opcode::IterateStore,
            9 => Opcode::Prefix,
            10 => Opcode::CountStore,
            11 => Opcode::WriteLoose,
            12 => Opcode::QueryLoose,
            13 => Opcode::RemoveLoose,
            14 => Opcode::Publish,
            15 => Opcode::Remove,
            16 => Opcode::Corrupt,
            17 => Opcode::WriteMultiIndex,
            18 => Opcode::RemoveMultiIndex,
            19 => Opcode::Alternate,
            20 => Opcode::Checkpoint,
            21 => Opcode::Exists,
            _ => Opcode::Locate,
        },
    }
}

fn operand(byte: u8) -> usize {
    if byte.is_ascii_digit() {
        usize::from(byte - b'0')
    } else {
        usize::from(byte)
    }
}

fn database(value: usize) -> Database {
    if value & 1 == 0 {
        Database::Primary
    } else {
        Database::Alternate
    }
}

fn pack(value: usize) -> Pack {
    Pack::ALL[value % Pack::ALL.len()]
}

fn component(value: usize) -> Component {
    [
        Component::Pack,
        Component::Index,
        Component::ReverseIndex,
        Component::Bitmap,
        Component::Promisor,
        Component::Mtimes,
        Component::Keep,
    ][value % 7]
}
