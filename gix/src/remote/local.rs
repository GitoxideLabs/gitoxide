use crate::{
    Progress, Remote, Repository, Result,
    bstr::{BString, ByteSlice},
    error::{bail, message},
    remote::{Direction, fetch, ref_map},
};

impl Remote<'_> {
    /// Map advertised fetch refs, reading local repositories directly without launching Git.
    ///
    /// Network URLs use [`crate::remote::Connection::ref_map()`]. Local repositories obey
    /// `uploadpack.hideRefs` and `transfer.hideRefs`, advertise peeled tags and symbolic refs,
    /// and report unborn `HEAD`. Per-operation `GIT_NAMESPACE` selects the remote namespace;
    /// `^`-prefixed hideRefs rules match physical names before stripping that namespace.
    /// URL rewrites and protocol permissions match [`Self::connect()`]. A configured upload-pack
    /// command uses the process transport instead of native repository access.
    pub fn ref_map(
        &self,
        progress: impl Progress,
        options: ref_map::Options,
    ) -> Result<(fetch::RefMap, gix_protocol::Handshake)> {
        match open(self)? {
            Some(source) => ref_map(self, &source, options),
            None => self.connect(Direction::Fetch)?.ref_map(progress, options),
        }
    }
}

pub(super) fn open(remote: &Remote<'_>) -> Result<Option<Repository>> {
    let (url, _) = remote.sanitized_url_and_version(Direction::Fetch)?;
    if url.scheme != gix_url::Scheme::File || remote.service_command(Direction::Fetch)?.is_some() {
        return Ok(None);
    }
    let mut options = remote.repo.options.clone().without_repository_environment_overrides();
    // Client overrides must not become server configuration.
    options.api_config_overrides.clear();
    options.cli_config_overrides.clear();
    let mut repo = crate::open_opts(gix_path::from_bstr(url.path)?.into_owned(), options)?;
    repo.make_paths_absolute()?;
    apply_namespace(remote, &mut repo)?;
    repo.objects.ignore_replacements = true;
    Ok(Some(repo))
}

pub(super) fn apply_namespace(remote: &Remote<'_>, repo: &mut Repository) -> Result<()> {
    let namespace = remote
        .command_environment
        .iter()
        .rev()
        .find(|(key, _)| key == "GIT_NAMESPACE")
        .map(|(_, value)| value.clone())
        .or_else(|| {
            remote
                .repo
                .options
                .permissions
                .env
                .git_prefix
                .check_opt("GIT_NAMESPACE")
                .and_then(std::env::var_os)
        });
    if let Some(namespace) = namespace {
        if namespace.is_empty() {
            repo.clear_namespace();
        } else {
            repo.set_namespace(gix_path::os_string_into_bstring(namespace)?.as_bstr())?;
        }
    }
    Ok(())
}

pub(super) fn ref_map(
    remote: &Remote<'_>,
    source: &Repository,
    mut options: ref_map::Options,
) -> Result<(fetch::RefMap, gix_protocol::Handshake)> {
    if !options.handshake_parameters.is_empty() {
        bail!(message("Handshake parameters are not supported for native local fetch").unsupported());
    }
    let hidden = hidden_refs(source)?;
    let mut refs = Vec::new();
    if let Some(head) = source.try_find_reference("HEAD")?
        && !is_hidden(&hidden, b"HEAD", source.namespace())
    {
        advertise(head, true, &mut refs)?;
    }
    for reference in source.references()?.all()? {
        let reference = reference?;
        if !is_hidden(&hidden, reference.name().as_bstr(), source.namespace()) {
            advertise(reference, false, &mut refs)?;
        }
    }
    let capabilities = gix_transport::client::Capabilities::from_bytes(
        format!("\0object-format={}", source.object_hash()).as_bytes(),
    )?
    .0;
    if let Some(spec) = remote.fetch_tags.to_refspec().map(|spec| spec.to_owned())
        && !options.extra_refspecs.contains(&spec)
    {
        options.extra_refspecs.push(spec);
    }
    let map = fetch::RefMap::from_refs(
        refs.clone(),
        &capabilities,
        fetch::refmap::init::Context {
            fetch_refspecs: remote.fetch_specs.clone(),
            extra_refspecs: options.extra_refspecs,
        },
    )?;
    Ok((
        map,
        gix_protocol::Handshake {
            server_protocol_version: gix_transport::Protocol::V1,
            refs: Some(refs),
            v1_shallow_updates: None,
            capabilities,
        },
    ))
}

fn advertise(
    mut reference: crate::Reference<'_>,
    allow_unborn: bool,
    refs: &mut Vec<gix_protocol::handshake::Ref>,
) -> Result<()> {
    use gix_protocol::handshake::Ref;
    let full_ref_name = reference.name().as_bstr().to_owned();
    let target = reference.target().try_name().map(|name| name.as_bstr().to_owned());
    let object_id = match reference.follow_to_object() {
        Ok(object_id) => object_id.detach(),
        Err(err) if err.is_not_found() && target.is_some() => {
            if allow_unborn {
                refs.push(Ref::Unborn {
                    full_ref_name,
                    target: target.expect("symbolic reference"),
                });
            }
            return Ok(());
        }
        Err(err) => return Err(err),
    };
    let peeled_id = reference.peel_to_id()?.detach();
    let tag = (object_id != peeled_id).then_some(object_id);
    refs.push(match target {
        Some(target) => Ref::Symbolic {
            full_ref_name,
            target,
            tag,
            object: peeled_id,
        },
        None => match tag {
            Some(tag) => Ref::Peeled {
                full_ref_name,
                tag,
                object: peeled_id,
            },
            None => Ref::Direct {
                full_ref_name,
                object: object_id,
            },
        },
    });
    Ok(())
}

fn hidden_refs(source: &Repository) -> Result<Vec<BString>> {
    let config = source.config_snapshot();
    let mut hidden = Vec::new();
    for section in config.plumbing().sections() {
        if section.header().subsection_name().is_none()
            && (section.header().name().eq_ignore_ascii_case(b"uploadpack")
                || section.header().name().eq_ignore_ascii_case(b"transfer"))
        {
            let values = section.values("hideRefs");
            if values.len()
                != section
                    .value_names()
                    .filter(|name| name.eq_ignore_ascii_case("hideRefs"))
                    .count()
            {
                bail!(message("hideRefs configuration requires an explicit value").validation());
            }
            hidden.extend(values);
        }
    }
    Ok(hidden)
}

fn is_hidden(hidden: &[BString], name: &[u8], namespace: Option<&gix_ref::Namespace>) -> bool {
    let mut full_name = namespace
        .map(|namespace| namespace.as_bstr().to_owned())
        .unwrap_or_default();
    full_name.extend_from_slice(name);
    for rule in hidden.iter().rev() {
        let rule = rule.trim_end_with(|byte| byte == '/');
        let (negated, rule) = rule.strip_prefix(b"!").map_or((false, rule), |rule| (true, rule));
        let (rule, name) = match rule.strip_prefix(b"^") {
            Some(rule) => (rule, full_name.as_slice()),
            None => (rule, name),
        };
        if name
            .strip_prefix(rule)
            .is_some_and(|suffix| suffix.is_empty() || suffix.starts_with(b"/"))
        {
            return !negated;
        }
    }
    false
}
