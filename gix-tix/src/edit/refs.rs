use anyhow::{Context, Result};
use gix::bstr::{BStr, ByteSlice};
use gix::refs::{
    Category, Target,
    transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog},
};

use crate::history;

#[derive(Clone)]
pub(super) struct MutableRefs {
    names: Vec<gix::refs::FullName>,
    old: Option<gix::ObjectId>,
}

impl MutableRefs {
    pub(super) fn pointing_to(repo: &gix::Repository, id: gix::ObjectId) -> Result<Self> {
        let mut names = Vec::new();
        for reference in repo.references()?.all()? {
            let reference = match reference {
                Ok(reference) => reference,
                Err(err) if is_missing_ref(&*err) => continue,
                Err(err) => anyhow::bail!("could not inspect references pointing to commit: {err}"),
            };
            if !matches!(
                reference.name().category(),
                Some(Category::Tag | Category::RemoteBranch)
            ) && reference.try_id().is_some_and(|target| target.as_ref() == id)
            {
                names.push(reference.name().to_owned());
            }
        }
        if let Some(head) = repo.try_find_reference("HEAD")?
            && head.try_id().is_some_and(|target| target.as_ref() == id)
        {
            names.push(head.name().to_owned());
        }
        Ok(Self { names, old: Some(id) })
    }

    pub(super) fn unborn(repo: &gix::Repository) -> Result<Self> {
        let head = repo.head().context("could not read unborn HEAD")?;
        if !head.is_unborn() {
            anyhow::bail!("an unborn HEAD is required");
        }
        let name = head
            .referent_name()
            .context("an unborn HEAD must point to a branch")?
            .to_owned();
        Ok(Self {
            names: vec![name],
            old: None,
        })
    }

    pub(super) fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    pub(super) fn contains(&self, name: &gix::refs::FullNameRef) -> bool {
        self.names.iter().any(|candidate| candidate.as_ref() == name)
    }

    pub(super) fn validate(&self, repo: &gix::Repository) -> Result<()> {
        for name in &self.names {
            let actual = repo
                .try_find_reference(name)?
                .and_then(|reference| reference.try_id().map(gix::Id::detach));
            if actual != self.old {
                anyhow::bail!("a reference changed while editing");
            }
        }
        Ok(())
    }

    pub(super) fn ensure_not_checked_out_elsewhere(&self, repo: &gix::Repository) -> Result<()> {
        if history::worktree_checkouts(repo).iter().any(|checkout| {
            !checkout.is_current
                && checkout
                    .reference
                    .as_ref()
                    .is_some_and(|name| self.contains(name.as_ref()))
        }) {
            anyhow::bail!("an affected branch is checked out in another worktree");
        }
        Ok(())
    }

    pub(super) fn update(
        &self,
        repo: &gix::Repository,
        new: gix::ObjectId,
        message: &BStr,
        committer: Option<gix::actor::SignatureRef<'_>>,
    ) -> Result<()> {
        repo.edit_references_as(
            self.names.iter().cloned().map(|name| RefEdit {
                change: Change::Update {
                    log: LogChange {
                        mode: RefLog::AndReference,
                        force_create_reflog: false,
                        message: message.to_owned(),
                    },
                    expected: self.old.map_or(PreviousValue::MustNotExist, |old| {
                        PreviousValue::MustExistAndMatch(Target::Object(old))
                    }),
                    new: Target::Object(new),
                },
                name,
                deref: false,
            }),
            committer,
        )
        .context("could not update references")?;
        Ok(())
    }

    pub(super) fn delete(
        &self,
        repo: &gix::Repository,
        _message: &BStr,
        committer: Option<gix::actor::SignatureRef<'_>>,
    ) -> Result<()> {
        let old = self.old.context("cannot delete an unborn reference")?;
        repo.edit_references_as(
            self.names.iter().cloned().map(|name| RefEdit {
                change: Change::Delete {
                    expected: PreviousValue::MustExistAndMatch(Target::Object(old)),
                    log: RefLog::AndReference,
                },
                name,
                deref: false,
            }),
            committer,
        )
        .context("could not delete references")?;
        Ok(())
    }

    pub(super) fn rollback(&self, repo: &gix::Repository, current: gix::ObjectId) -> Result<()> {
        let current_refs = Self {
            names: self.names.clone(),
            old: Some(current),
        };
        let committer = repo.committer().transpose()?;
        match self.old {
            Some(old) => current_refs.update(repo, old, b"tix edit rollback".as_bstr(), committer),
            None => current_refs.delete(repo, b"tix edit rollback".as_bstr(), committer),
        }
    }

    pub(super) fn rollback_deleted(&self, repo: &gix::Repository) -> Result<()> {
        let old = self.old.context("an unborn reference was not deleted")?;
        let deleted = Self {
            names: self.names.clone(),
            old: None,
        };
        let committer = repo.committer().transpose()?;
        deleted.update(repo, old, b"tix edit rollback".as_bstr(), committer)
    }
}

fn is_missing_ref(mut err: &(dyn std::error::Error + 'static)) -> bool {
    loop {
        if err
            .downcast_ref::<std::io::Error>()
            .is_some_and(|err| err.kind() == std::io::ErrorKind::NotFound)
        {
            return true;
        }
        let Some(source) = err.source() else { return false };
        err = source;
    }
}
