use gix_hash::ObjectId;

use crate::error::{Result, SdkError};
use crate::pool::RepoHandle;
use crate::types::{ObjectData, ObjectInfo, ObjectKind};

pub fn get_object(repo: &RepoHandle, id: ObjectId) -> Result<ObjectData> {
    let local = repo.to_local();
    let object = local.find_object(id).map_err(|e| {
        if e.to_string().contains("not found") {
            SdkError::ObjectNotFound(id)
        } else {
            SdkError::Git(Box::new(e))
        }
    })?;

    Ok(ObjectData {
        id,
        kind: object.kind.into(),
        data: object.data.to_vec(),
    })
}

pub fn get_object_header(repo: &RepoHandle, id: ObjectId) -> Result<ObjectInfo> {
    let local = repo.to_local();
    let header = local.find_header(id).map_err(|e| {
        if e.to_string().contains("not found") {
            SdkError::ObjectNotFound(id)
        } else {
            SdkError::Git(Box::new(e))
        }
    })?;

    let (kind, size) = match header {
        gix_odb::find::Header::Loose { kind, size } => (kind, size as usize),
        gix_odb::find::Header::Packed(packed) => (packed.kind, packed.object_size as usize),
    };

    Ok(ObjectInfo {
        id,
        kind: kind.into(),
        size,
    })
}

pub fn object_exists(repo: &RepoHandle, id: &ObjectId) -> bool {
    let local = repo.to_local();
    local.has_object(id)
}

pub fn resolve_revision(repo: &RepoHandle, spec: &str) -> Result<ObjectId> {
    let local = repo.to_local();
    let id = local
        .rev_parse_single(spec)
        .map_err(|e| SdkError::InvalidRevision(format!("{}: {}", spec, e)))?;

    Ok(id.detach())
}

pub fn get_blob(repo: &RepoHandle, id: ObjectId) -> Result<Vec<u8>> {
    let local = repo.to_local();
    let object = local.find_object(id).map_err(|e| {
        if e.to_string().contains("not found") {
            SdkError::ObjectNotFound(id)
        } else {
            SdkError::Git(Box::new(e))
        }
    })?;

    if object.kind != gix_object::Kind::Blob {
        return Err(SdkError::InvalidObjectType {
            expected: "blob".to_string(),
            actual: ObjectKind::from(object.kind).to_string(),
        });
    }

    Ok(object.data.to_vec())
}

pub fn get_blob_size(repo: &RepoHandle, id: ObjectId) -> Result<usize> {
    let local = repo.to_local();
    let header = local.find_header(id).map_err(|e| {
        if e.to_string().contains("not found") {
            SdkError::ObjectNotFound(id)
        } else {
            SdkError::Git(Box::new(e))
        }
    })?;

    let (kind, size) = match header {
        gix_odb::find::Header::Loose { kind, size } => (kind, size as usize),
        gix_odb::find::Header::Packed(packed) => (packed.kind, packed.object_size as usize),
    };

    if kind != gix_object::Kind::Blob {
        return Err(SdkError::InvalidObjectType {
            expected: "blob".to_string(),
            actual: ObjectKind::from(kind).to_string(),
        });
    }

    Ok(size)
}
