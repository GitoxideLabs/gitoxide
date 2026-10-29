use std::{collections::BTreeSet, io, sync::atomic::AtomicBool};

use gix_fsck::objects::{Diagnostic, MessageId, Options, Severity, SpecialBlobs};

use crate::{
    ObjectId, Repository, Result,
    bstr::ByteSlice,
    error::{ResultExt, bail, message},
    push,
};

pub(super) struct Fsck {
    options: Options,
    special: SpecialBlobs,
    rejected: bool,
}

impl Fsck {
    pub fn new(
        repo: &Repository,
        receive_objects: bool,
        progress: &mut impl FnMut(bool, &[u8]) -> io::Result<()>,
    ) -> Result<Option<Self>> {
        let config = repo.config_snapshot();
        let receive = config.try_boolean("receive.fsckObjects")?;
        let transfer = config.try_boolean("transfer.fsckObjects")?;
        let enabled = receive.or(transfer).unwrap_or(false) && receive_objects;
        let mut options = Options::strict();
        for section in config.plumbing().sections().filter(|section| {
            section.header().name().eq_ignore_ascii_case(b"receive")
                && section.header().subsection_name().is_some_and(|name| name == "fsck")
        }) {
            let names: BTreeSet<_> = section.value_names().map(|name| name.to_ascii_lowercase()).collect();
            for name in names {
                let values = section.values(&name);
                if values.len()
                    != section
                        .value_names()
                        .filter(|key| key.eq_ignore_ascii_case(&name))
                        .count()
                {
                    bail!(message!("receive.fsck.{name} requires an explicit value").validation());
                }
                for value in values {
                    if name.eq_ignore_ascii_case("skipList") {
                        read_skip_list(repo, value.into(), enabled.then_some(&mut options))?;
                        continue;
                    }
                    let Ok(id) = name.parse::<MessageId>() else {
                        progress(true, format!("warning: skipping unknown msg id '{name}'\n").as_bytes()).or_error()?;
                        continue;
                    };
                    let severity = value
                        .to_str()
                        .or_raise(|| message("Invalid fsck severity"))?
                        .parse::<Severity>()?;
                    if enabled {
                        options.set_severity(id, severity)?;
                    }
                }
            }
        }
        Ok(enabled.then_some(Self {
            options,
            special: SpecialBlobs::default(),
            rejected: false,
        }))
    }

    pub fn validate(
        &mut self,
        object_id: ObjectId,
        object: gix_object::Data<'_>,
        progress: &mut impl FnMut(bool, &[u8]) -> io::Result<()>,
    ) -> Result<()> {
        self.report(gix_fsck::objects::validate(object_id, object, &self.options)?, progress)?;
        if object.kind == gix_object::Kind::Tree {
            let diagnostics = self.special.observe_tree(object_id, object.data, &self.options)?;
            self.report(diagnostics, progress)?;
        }
        Ok(())
    }

    pub fn finish(
        &mut self,
        objects: &impl gix_object::Find,
        interrupt: &AtomicBool,
        progress: &mut impl FnMut(bool, &[u8]) -> io::Result<()>,
    ) -> Result<bool> {
        push::plan::check_interrupt(interrupt)?;
        let diagnostics = self.special.validate(objects, &self.options)?;
        self.report(diagnostics, progress)?;
        Ok(!self.rejected)
    }

    fn report(
        &mut self,
        diagnostics: Vec<Diagnostic>,
        progress: &mut impl FnMut(bool, &[u8]) -> io::Result<()>,
    ) -> Result<()> {
        for diagnostic in diagnostics {
            let severity = match diagnostic.severity {
                Severity::Ignore => continue,
                Severity::Warning => "warning",
                Severity::Error => {
                    self.rejected = true;
                    "error"
                }
            };
            progress(
                true,
                format!(
                    "{severity}: object {}: {}: {}\n",
                    diagnostic.object_id,
                    diagnostic.id.as_str(),
                    diagnostic.id.description()
                )
                .as_bytes(),
            )
            .or_error()?;
        }
        Ok(())
    }
}

fn read_skip_list(repo: &Repository, path: gix_config::Path, options: Option<&mut Options>) -> Result<()> {
    let install_dir = crate::path::install_dir().ok();
    let home = crate::config::cache::access::home_dir(repo.options.permissions.env);
    let optional = path.is_optional;
    let path = path.interpolate(crate::config::cache::interpolate_context(
        install_dir.as_deref(),
        home.as_deref(),
    ))?;
    let path = repo.current_dir().join(repo.git_dir()).join(path);
    let Some(options) = options else {
        return Ok(());
    };
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if optional && err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(err).or_raise(|| message!("Could not read receive fsck skip list {}", path.display()));
        }
    };
    options.extend_skip_list(&bytes, repo.object_hash())
}
