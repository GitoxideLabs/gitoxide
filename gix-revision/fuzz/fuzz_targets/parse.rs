#![no_main]
use gix_revision::spec::parse::{delegate, Delegate};
use libfuzzer_sys::fuzz_target;

use bstr::BStr;
use gix_error::ExnResult;

fuzz_target!(|data: &[u8]| {
    drop(gix_revision::spec::parse(data.into(), &mut Noop));
});

struct Noop;

impl Delegate for Noop {
    fn done(&mut self) -> ExnResult {
        Ok(())
    }
}

impl delegate::Kind for Noop {
    fn kind(&mut self, _kind: gix_revision::spec::Kind) -> ExnResult {
        Ok(())
    }
}

impl delegate::Navigate for Noop {
    fn traverse(&mut self, _kind: delegate::Traversal) -> ExnResult {
        Ok(())
    }

    fn peel_until(&mut self, _kind: delegate::PeelTo<'_>) -> ExnResult {
        Ok(())
    }

    fn find(&mut self, _regex: &BStr, _negated: bool) -> ExnResult {
        Ok(())
    }

    fn index_lookup(&mut self, _path: &BStr, _stage: u8) -> ExnResult {
        Ok(())
    }
}

impl delegate::Revision for Noop {
    fn find_ref(&mut self, _name: &BStr) -> ExnResult {
        Ok(())
    }

    fn disambiguate_prefix(
        &mut self,
        _prefix: gix_hash::Prefix,
        _hint: Option<delegate::PrefixHint<'_>>,
    ) -> ExnResult {
        Ok(())
    }

    fn reflog(&mut self, _query: delegate::ReflogLookup) -> ExnResult {
        Ok(())
    }

    fn nth_checked_out_branch(&mut self, _branch_no: usize) -> ExnResult {
        Ok(())
    }

    fn sibling_branch(&mut self, _kind: delegate::SiblingBranch) -> ExnResult {
        Ok(())
    }
}
