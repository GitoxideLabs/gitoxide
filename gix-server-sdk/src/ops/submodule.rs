use bstr::BString;
use gix_hash::ObjectId;

use crate::error::{Result, SdkError};
use crate::RepoHandle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmoduleInfo {
    pub name: BString,
    pub path: BString,
    pub url: Option<String>,
    pub head_commit: Option<ObjectId>,
    pub index_commit: Option<ObjectId>,
    pub is_active: bool,
}

pub fn list_submodules(repo: &RepoHandle) -> Result<Vec<SubmoduleInfo>> {
    let local = repo.to_local();

    let submodules = match local.submodules().map_err(|e| SdkError::Git(Box::new(e)))? {
        Some(iter) => iter,
        None => return Ok(Vec::new()),
    };

    let mut result = Vec::new();
    for submodule in submodules {
        let name = submodule.name().to_owned();
        let path = submodule
            .path()
            .map_err(|e| SdkError::Git(Box::new(e)))?
            .into_owned();
        let url = submodule
            .url()
            .ok()
            .map(|u| u.to_bstring().to_string());
        let head_commit = submodule
            .head_id()
            .map_err(|e| SdkError::Git(Box::new(e)))?;
        let index_commit = submodule
            .index_id()
            .map_err(|e| SdkError::Git(Box::new(e)))?;
        let is_active = submodule.is_active().map_err(|e| SdkError::Git(Box::new(e)))?;

        result.push(SubmoduleInfo {
            name,
            path,
            url,
            head_commit,
            index_commit,
            is_active,
        });
    }

    Ok(result)
}

pub fn get_submodule(repo: &RepoHandle, name: &str) -> Result<SubmoduleInfo> {
    let local = repo.to_local();

    let submodules = match local.submodules().map_err(|e| SdkError::Git(Box::new(e)))? {
        Some(iter) => iter,
        None => return Err(SdkError::Operation(format!("Submodule not found: {}", name))),
    };

    for submodule in submodules {
        if submodule.name() == name.as_bytes() {
            let submodule_name = submodule.name().to_owned();
            let path = submodule
                .path()
                .map_err(|e| SdkError::Git(Box::new(e)))?
                .into_owned();
            let url = submodule
                .url()
                .ok()
                .map(|u| u.to_bstring().to_string());
            let head_commit = submodule
                .head_id()
                .map_err(|e| SdkError::Git(Box::new(e)))?;
            let index_commit = submodule
                .index_id()
                .map_err(|e| SdkError::Git(Box::new(e)))?;
            let is_active = submodule.is_active().map_err(|e| SdkError::Git(Box::new(e)))?;

            return Ok(SubmoduleInfo {
                name: submodule_name,
                path,
                url,
                head_commit,
                index_commit,
                is_active,
            });
        }
    }

    Err(SdkError::Operation(format!("Submodule not found: {}", name)))
}
