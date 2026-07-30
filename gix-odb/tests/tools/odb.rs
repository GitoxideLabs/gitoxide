#![allow(dead_code)] // Each source-including consumer intentionally uses a different part of this shared vocabulary.

use std::{
    collections::HashSet,
    fs,
    io::{self, Cursor},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

use filetime::FileTime;
use gix_hash::ObjectId;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

const SHA1_FIXTURE_ARCHIVE: &[u8] = include_bytes!("../fixtures/generated-archives/make_odb_scenarios.tar");
const MTIME_EPOCH: i64 = 1_700_000_000;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Database {
    Primary,
    Alternate,
}

impl Database {
    const ALL: [Database; 2] = [Database::Primary, Database::Alternate];

    fn directory(self) -> &'static str {
        match self {
            Database::Primary => "primary",
            Database::Alternate => "alternate",
        }
    }

    fn index(self) -> usize {
        match self {
            Database::Primary => 0,
            Database::Alternate => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Pack {
    A,
    B,
    C,
}

impl Pack {
    pub const ALL: [Pack; 3] = [Pack::A, Pack::B, Pack::C];

    fn index(self) -> usize {
        match self {
            Pack::A => 0,
            Pack::B => 1,
            Pack::C => 2,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Pack::A => "a",
            Pack::B => "b",
            Pack::C => "c",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Component {
    Pack,
    Index,
    ReverseIndex,
    Bitmap,
    Promisor,
    Mtimes,
    Keep,
}

impl Component {
    pub const AUXILIARY: [Component; 5] = [
        Component::ReverseIndex,
        Component::Bitmap,
        Component::Promisor,
        Component::Mtimes,
        Component::Keep,
    ];

    fn extension(self) -> &'static str {
        match self {
            Component::Pack => "pack",
            Component::Index => "idx",
            Component::ReverseIndex => "rev",
            Component::Bitmap => "bitmap",
            Component::Promisor => "promisor",
            Component::Mtimes => "mtimes",
            Component::Keep => "keep",
        }
    }
}

#[derive(Clone, Debug)]
pub enum Action {
    Publish {
        database: Database,
        pack: Pack,
        component: Component,
    },
    Remove {
        database: Database,
        pack: Pack,
        component: Component,
    },
    CorruptIndex {
        database: Database,
        pack: Pack,
    },
    WriteMultiIndex {
        database: Database,
        packs: Vec<Pack>,
    },
    RemoveMultiIndex {
        database: Database,
    },
    SetAlternate {
        enabled: bool,
    },
}

#[derive(Clone, Debug)]
pub struct PackInfo {
    pub name: String,
    pub object_ids: Vec<ObjectId>,
}

#[derive(Clone, Debug)]
pub struct Manifest {
    pub object_hash: gix_hash::Kind,
    packs: Vec<PackInfo>,
    pub ambiguous_prefix: String,
    pub ambiguous_ids: [ObjectId; 2],
}

impl Manifest {
    pub fn pack(&self, pack: Pack) -> &PackInfo {
        &self.packs[pack.index()]
    }

    pub fn object_ids(&self) -> impl Iterator<Item = ObjectId> + '_ {
        self.packs.iter().flat_map(|pack| pack.object_ids.iter().copied())
    }

    pub fn missing_id(&self) -> ObjectId {
        ObjectId::null(self.object_hash)
    }
}

pub struct OdbFixture {
    root: gix_testtools::tempfile::TempDir,
    pub manifest: Manifest,
    clock: u32,
    corrupt_indices: HashSet<(Database, Pack)>,
    corrupt_packs: HashSet<(Database, Pack)>,
    multi_index_packs: [Option<Vec<Pack>>; 2],
}

impl OdbFixture {
    pub fn from_script() -> Result<Self> {
        let source = gix_testtools::scripted_fixture_read_only_needs_archive("make_odb_scenarios.sh")?;
        Self::from_template(source)
    }

    pub fn from_embedded_sha1() -> Result<Self> {
        let source = gix_testtools::tempfile::TempDir::new()?;
        tar::Archive::new(Cursor::new(SHA1_FIXTURE_ARCHIVE)).unpack(source.path())?;
        Self::from_template(source.path())
    }

    fn from_template(source: impl AsRef<Path>) -> Result<Self> {
        let root = gix_testtools::tempfile::TempDir::new()?;
        gix_testtools::copy_recursively_into_existing_dir(source, root.path())?;
        let manifest = parse_manifest(&fs::read_to_string(root.path().join("manifest"))?)?;
        Ok(OdbFixture {
            root,
            manifest,
            clock: 0,
            corrupt_indices: HashSet::new(),
            corrupt_packs: HashSet::new(),
            multi_index_packs: [None, None],
        })
    }

    pub fn objects_dir(&self, database: Database) -> PathBuf {
        self.root.path().join(database.directory()).join("objects")
    }

    pub fn component_path(&self, database: Database, pack: Pack, component: Component) -> PathBuf {
        self.objects_dir(database).join("pack").join(format!(
            "{}.{}",
            self.manifest.pack(pack).name,
            component.extension()
        ))
    }

    pub fn multi_index_path(&self, database: Database) -> PathBuf {
        self.objects_dir(database).join("pack/multi-pack-index")
    }

    pub fn apply(&mut self, action: Action) -> Result<()> {
        match action {
            Action::Publish {
                database,
                pack,
                component,
            } => self.publish(database, pack, component),
            Action::Remove {
                database,
                pack,
                component,
            } => self.remove(database, pack, component),
            Action::CorruptIndex { database, pack } => self.corrupt_index(database, pack),
            Action::WriteMultiIndex { database, packs } => self.write_multi_index(database, &packs),
            Action::RemoveMultiIndex { database } => self.remove_multi_index(database),
            Action::SetAlternate { enabled } => self.set_alternate(enabled),
        }
    }

    pub fn install_pack(&mut self, database: Database, pack: Pack) -> Result<()> {
        for component in [Component::Pack, Component::ReverseIndex, Component::Index] {
            self.publish(database, pack, component)?;
        }
        Ok(())
    }

    pub fn remove_pack(&mut self, database: Database, pack: Pack) -> Result<()> {
        for component in [
            Component::Index,
            Component::Pack,
            Component::ReverseIndex,
            Component::Bitmap,
            Component::Promisor,
            Component::Mtimes,
            Component::Keep,
        ] {
            self.remove(database, pack, component)?;
        }
        Ok(())
    }

    pub fn publish(&mut self, database: Database, pack: Pack, component: Component) -> Result<()> {
        let target = self.component_path(database, pack, component);
        let source = self.root.path().join("catalog").join(pack.label()).join(format!(
            "{}.{}",
            self.manifest.pack(pack).name,
            component.extension()
        ));
        self.atomic_replace(&target, |staged| {
            if source.is_file() {
                fs::copy(&source, staged)?;
            } else {
                fs::File::create(staged)?;
            }
            Ok(())
        })?;
        if component == Component::Index {
            self.corrupt_indices.remove(&(database, pack));
        } else if component == Component::Pack {
            self.corrupt_packs.remove(&(database, pack));
        }
        Ok(())
    }

    pub fn remove(&mut self, database: Database, pack: Pack, component: Component) -> Result<()> {
        remove_if_exists(&self.component_path(database, pack, component))?;
        if component == Component::Index {
            self.corrupt_indices.remove(&(database, pack));
        } else if component == Component::Pack {
            self.corrupt_packs.remove(&(database, pack));
        }
        Ok(())
    }

    pub fn corrupt_index(&mut self, database: Database, pack: Pack) -> Result<()> {
        let target = self.component_path(database, pack, Component::Index);
        self.atomic_replace(&target, |staged| {
            fs::write(staged, b"not a pack index")?;
            Ok(())
        })?;
        self.corrupt_indices.insert((database, pack));
        Ok(())
    }

    pub fn corrupt_pack(&mut self, database: Database, pack: Pack) -> Result<()> {
        let target = self.component_path(database, pack, Component::Pack);
        self.atomic_replace(&target, |staged| {
            fs::write(staged, b"not a pack")?;
            Ok(())
        })?;
        self.corrupt_packs.insert((database, pack));
        Ok(())
    }

    pub fn write_multi_index(&mut self, database: Database, packs: &[Pack]) -> Result<()> {
        let index_paths = packs
            .iter()
            .map(|pack| self.component_path(database, *pack, Component::Index))
            .collect::<Vec<_>>();
        let target = self.multi_index_path(database);
        let object_hash = self.manifest.object_hash;
        self.atomic_replace(&target, |staged| {
            let mut out = fs::OpenOptions::new().write(true).create_new(true).open(staged)?;
            gix_odb::pack::multi_index::write_from_index_paths(
                index_paths,
                &mut out,
                &mut gix_features::progress::Discard,
                &AtomicBool::default(),
                gix_odb::pack::multi_index::write::Options { object_hash },
            )?;
            Ok(())
        })?;
        self.multi_index_packs[database.index()] = Some(packs.to_vec());
        Ok(())
    }

    pub fn remove_multi_index(&mut self, database: Database) -> Result<()> {
        remove_if_exists(&self.multi_index_path(database))?;
        self.multi_index_packs[database.index()] = None;
        Ok(())
    }

    pub fn set_alternate(&mut self, enabled: bool) -> Result<()> {
        let target = self.objects_dir(Database::Primary).join("info/alternates");
        if enabled {
            let alternate = self.objects_dir(Database::Alternate);
            self.atomic_replace(&target, |staged| {
                fs::write(staged, format!("{}\n", alternate.display()))?;
                Ok(())
            })
        } else {
            remove_if_exists(&target)
        }
    }

    pub fn is_valid(&self) -> bool {
        if !self.corrupt_indices.is_empty() || !self.corrupt_packs.is_empty() {
            return false;
        }
        Database::ALL.into_iter().all(|database| {
            Pack::ALL.into_iter().all(|pack| {
                self.component_path(database, pack, Component::Pack).is_file()
                    == self.component_path(database, pack, Component::Index).is_file()
            }) && self.multi_index_packs[database.index()].as_ref().is_none_or(|packs| {
                packs.iter().all(|pack| {
                    self.component_path(database, *pack, Component::Pack).is_file()
                        && self.component_path(database, *pack, Component::Index).is_file()
                })
            })
        })
    }

    pub fn reachable_ids(&self) -> Vec<ObjectId> {
        let alternate_enabled = self.objects_dir(Database::Primary).join("info/alternates").is_file();
        Pack::ALL
            .into_iter()
            .flat_map(|pack| {
                [Database::Primary, Database::Alternate]
                    .into_iter()
                    .filter(move |database| *database == Database::Primary || alternate_enabled)
                    .filter(move |database| {
                        self.component_path(*database, pack, Component::Pack).is_file()
                            && self.component_path(*database, pack, Component::Index).is_file()
                            && !self.corrupt_packs.contains(&(*database, pack))
                    })
                    .flat_map(move |_| self.manifest.pack(pack).object_ids.iter().copied())
            })
            .collect()
    }

    fn atomic_replace(&mut self, target: &Path, write: impl FnOnce(&Path) -> Result<()>) -> Result<()> {
        self.clock = self.clock.wrapping_add(1);
        let staged = target.with_extension(format!(
            "{}.tmp-{}",
            target.extension().and_then(|ext| ext.to_str()).unwrap_or("file"),
            self.clock
        ));
        remove_if_exists(&staged)?;
        if let Err(err) = write(&staged).and_then(|()| {
            filetime::set_file_mtime(
                &staged,
                FileTime::from_unix_time(MTIME_EPOCH + i64::from(self.clock), 0),
            )
            .map_err(Into::into)
        }) {
            remove_if_exists(&staged)?;
            return Err(err);
        }
        match fs::rename(&staged, target) {
            Ok(()) => Ok(()),
            Err(err)
                if matches!(
                    err.kind(),
                    io::ErrorKind::AlreadyExists | io::ErrorKind::PermissionDenied
                ) =>
            {
                remove_if_exists(target)?;
                fs::rename(staged, target)?;
                Ok(())
            }
            Err(err) => Err(err.into()),
        }
    }
}

fn remove_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

fn parse_manifest(input: &str) -> Result<Manifest> {
    let mut object_hash = None;
    let mut packs: [Option<PackInfo>; 3] = [None, None, None];
    let mut ambiguous_prefix = None;
    let mut ambiguous_ids = None;

    for line in input.lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        match fields.as_slice() {
            ["hash", "sha1"] => object_hash = Some(gix_hash::Kind::Sha1),
            ["hash", "sha256"] => object_hash = Some(gix_hash::Kind::Sha256),
            ["pack", label, name] => {
                let pack = pack_from_label(label)?;
                packs[pack.index()] = Some(PackInfo {
                    name: (*name).to_owned(),
                    object_ids: Vec::new(),
                });
            }
            ["object", label, oid] => {
                let pack = pack_from_label(label)?;
                packs[pack.index()]
                    .as_mut()
                    .ok_or_else(|| format!("pack {label} must precede its objects"))?
                    .object_ids
                    .push(ObjectId::from_hex(oid.as_bytes())?);
            }
            ["ambiguous", prefix, one, two] => {
                ambiguous_prefix = Some((*prefix).to_owned());
                ambiguous_ids = Some([ObjectId::from_hex(one.as_bytes())?, ObjectId::from_hex(two.as_bytes())?]);
            }
            _ => return Err(format!("invalid ODB fixture manifest line: {line}").into()),
        }
    }

    Ok(Manifest {
        object_hash: object_hash.ok_or("manifest has no hash kind")?,
        packs: packs
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or("manifest does not define all packs")?,
        ambiguous_prefix: ambiguous_prefix.ok_or("manifest has no ambiguous prefix")?,
        ambiguous_ids: ambiguous_ids.ok_or("manifest has no ambiguous ids")?,
    })
}

fn pack_from_label(label: &str) -> Result<Pack> {
    match label {
        "a" => Ok(Pack::A),
        "b" => Ok(Pack::B),
        "c" => Ok(Pack::C),
        _ => Err(format!("unknown pack label: {label}").into()),
    }
}
