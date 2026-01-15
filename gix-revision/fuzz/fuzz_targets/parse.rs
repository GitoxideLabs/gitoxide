#![no_main]
use gix_revision::spec::parse::{delegate, Delegate};
use libfuzzer_sys::fuzz_target;

use bstr::BStr;
use gix_error::Exn;

fuzz_target!(|data: &[u8]| {
    drop(gix_revision::spec::parse(data.into(), &mut Noop));
});

struct Noop;

impl Delegate for Noop {
    fn done(&mut self) -> Result<(), Exn> {
        Ok(())
    }
}

impl delegate::Kind for Noop {
    fn kind(&mut self, _kind: gix_revision::spec::Kind) -> Result<(), Exn> {
        Ok(())
    }
}

impl delegate::Navigate for Noop {
    fn traverse(&mut self, _kind: delegate::Traversal) -> Result<(), Exn> {
        Ok(())
    }

    fn peel_until(&mut self, _kind: delegate::PeelTo<'_>) -> Result<(), Exn> {
        Ok(())
    }

    fn find(&mut self, _regex: &BStr, _negated: bool) -> Result<(), Exn> {
        Ok(())
    }

    fn index_lookup(&mut self, _path: &BStr, _stage: u8) -> Result<(), Exn> {
        Ok(())
    }
}

impl delegate::Revision for Noop {
    fn find_ref(&mut self, _name: &BStr) -> Result<(), Exn> {
        Ok(())
    }

    fn disambiguate_prefix(
        &mut self,
        _prefix: gix_hash::Prefix,
        _hint: Option<delegate::PrefixHint<'_>>,
    ) -> Result<(), Exn> {
        Ok(())
    }

    fn reflog(&mut self, _query: delegate::ReflogLookup) -> Result<(), Exn> {
        Ok(())
    }

    fn nth_checked_out_branch(&mut self, _branch_no: usize) -> Result<(), Exn> {
        Ok(())
    }

    fn sibling_branch(&mut self, _kind: delegate::SiblingBranch) -> Result<(), Exn> {
        Ok(())
    }
}
