use gix_error::{bail, message, ErrorExt, Exn};
use gix_object::bstr::{BStr, BString};
use gix_revision::{
    spec,
    spec::parse::{delegate, Delegate},
};

#[derive(Default, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct Options {
    reject_kind: bool,
    reject_prefix: bool,
    no_internal_assertions: bool,
}

#[derive(Default, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct Recorder {
    // anchors
    find_ref: [Option<BString>; 2],
    prefix: [Option<gix_hash::Prefix>; 2],
    prefix_hint: [Option<PrefixHintOwned>; 2],
    current_branch_reflog_entry: [Option<String>; 2],
    nth_checked_out_branch: [Option<usize>; 2],
    sibling_branch: [Option<String>; 2],
    index_lookups: Vec<(BString, u8)>,

    // navigation
    traversal: Vec<delegate::Traversal>,
    peel_to: Vec<PeelToOwned>,
    patterns: Vec<(BString, bool)>,

    // range
    kind: Option<spec::Kind>,

    order: Vec<Call>,
    calls: usize,
    opts: Options,
    done: bool,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Call {
    FindRef,
    DisambiguatePrefix,
    Reflog,
    NthCheckedOutBranch,
    SiblingBranch,
    Traverse,
    PeelUntil,
    Find,
    IndexLookup,
    Kind,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum PeelToOwned {
    ObjectKind(gix_object::Kind),
    ExistingObject,
    RecursiveTagObject,
    Path(BString),
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum PrefixHintOwned {
    MustBeCommit,
    DescribeAnchor { ref_name: BString, generation: usize },
}

impl Recorder {
    fn with(options: Options) -> Self {
        Recorder {
            opts: options,
            ..Default::default()
        }
    }

    fn get_ref(&self, idx: usize) -> &BStr {
        self.find_ref[idx].as_ref().map(AsRef::as_ref).unwrap()
    }

    fn called(&mut self, f: Call) {
        self.calls += 1;
        self.order.push(f);
    }
}

fn set_val<T: std::fmt::Debug>(fn_name: &str, store: &mut [Option<T>; 2], val: T) -> Result<(), Exn> {
    for entry in store.iter_mut() {
        if entry.is_none() {
            *entry = Some(val);
            return Ok(());
        }
    }
    panic!("called {fn_name}() more than twice with '{val:?}'");
}

impl delegate::Revision for Recorder {
    fn find_ref(&mut self, input: &BStr) -> Result<(), Exn> {
        self.called(Call::FindRef);
        set_val("find_ref", &mut self.find_ref, input.into())
    }

    fn disambiguate_prefix(
        &mut self,
        input: gix_hash::Prefix,
        hint: Option<delegate::PrefixHint<'_>>,
    ) -> Result<(), Exn> {
        self.called(Call::DisambiguatePrefix);
        if self.opts.reject_prefix {
            bail!(message!("disambiguate_prefix rejected").erased());
        }
        set_val("disambiguate_prefix", &mut self.prefix, input)?;
        if let Some(hint) = hint {
            set_val(
                "disambiguate_prefix",
                &mut self.prefix_hint,
                match hint {
                    delegate::PrefixHint::DescribeAnchor { ref_name, generation } => PrefixHintOwned::DescribeAnchor {
                        ref_name: ref_name.into(),
                        generation,
                    },
                    delegate::PrefixHint::MustBeCommit => PrefixHintOwned::MustBeCommit,
                },
            )?;
        }
        Ok(())
    }

    fn reflog(&mut self, entry: delegate::ReflogLookup) -> Result<(), Exn> {
        self.called(Call::Reflog);
        set_val(
            "current_branch_reflog",
            &mut self.current_branch_reflog_entry,
            match entry {
                delegate::ReflogLookup::Entry(no) => no.to_string(),
                delegate::ReflogLookup::Date(time) => {
                    let mut buf = Vec::new();
                    time.write_to(&mut buf).unwrap();
                    BString::from(buf).to_string()
                }
            },
        )
    }

    fn nth_checked_out_branch(&mut self, branch: usize) -> Result<(), Exn> {
        assert_ne!(branch, 0);
        self.called(Call::NthCheckedOutBranch);
        set_val("nth_checked_out_branch", &mut self.nth_checked_out_branch, branch)
    }

    fn sibling_branch(&mut self, kind: delegate::SiblingBranch) -> Result<(), Exn> {
        self.called(Call::SiblingBranch);
        set_val("sibling_branch", &mut self.sibling_branch, format!("{kind:?}"))
    }
}

impl delegate::Navigate for Recorder {
    fn traverse(&mut self, kind: delegate::Traversal) -> Result<(), Exn> {
        self.called(Call::Traverse);
        self.traversal.push(kind);
        Ok(())
    }

    fn peel_until(&mut self, kind: delegate::PeelTo) -> Result<(), Exn> {
        self.called(Call::PeelUntil);
        self.peel_to.push(match kind {
            delegate::PeelTo::ObjectKind(kind) => PeelToOwned::ObjectKind(kind),
            delegate::PeelTo::ValidObject => PeelToOwned::ExistingObject,
            delegate::PeelTo::Path(path) => PeelToOwned::Path(path.into()),
            delegate::PeelTo::RecursiveTagObject => PeelToOwned::RecursiveTagObject,
        });
        Ok(())
    }

    fn find(&mut self, regex: &BStr, negated: bool) -> Result<(), Exn> {
        self.called(Call::Find);
        self.patterns.push((regex.into(), negated));
        Ok(())
    }

    fn index_lookup(&mut self, path: &BStr, stage: u8) -> Result<(), Exn> {
        self.called(Call::IndexLookup);
        self.index_lookups.push((path.into(), stage));
        Ok(())
    }
}

impl delegate::Kind for Recorder {
    fn kind(&mut self, kind: spec::Kind) -> Result<(), Exn> {
        self.called(Call::Kind);
        if self.opts.reject_kind {
            bail!(message!("kind() was rejected").erased());
        }
        if self.kind.is_none() {
            self.kind = Some(kind);
        } else if !self.opts.no_internal_assertions {
            panic!("called kind more than once with '{kind:?}'");
        }
        Ok(())
    }
}

impl Delegate for Recorder {
    fn done(&mut self) -> Result<(), Exn> {
        self.done = true;
        Ok(())
    }
}

fn parse(spec: &str) -> Recorder {
    try_parse_opts(spec, Options::default()).unwrap()
}

fn try_parse(spec: &str) -> Result<Recorder, Exn<spec::parse::Error>> {
    try_parse_opts(spec, Default::default())
}

fn try_parse_opts(spec: &str, options: Options) -> Result<Recorder, Exn<spec::parse::Error>> {
    let mut rec = Recorder::with(options);
    spec::parse(spec.into(), &mut rec)?;
    Ok(rec)
}

#[test]
fn empty_specs_are_valid() {
    // they should of course be invalid for the delegate. CLIs may pre-process the input as well if they wish
    // but git itself doesn't do that.
    for spec in [" ", "\n\t"] {
        let rec = parse(spec);
        assert_eq!(rec.calls, 1);
    }
    let rec = parse("");
    assert_eq!(rec.calls, 0, "but we do not bother to call the delegate with nothing");
    assert!(rec.done);
}

#[test]
fn all_characters_are_taken_verbatim_which_includes_whitespace() {
    let spec = "  HEAD \n";
    let rec = parse(spec);
    assert!(rec.kind.is_none());
    assert_eq!(rec.get_ref(0), spec);
}

mod fuzz {
    use crate::spec::parse::{try_parse_opts, Options};

    #[test]
    fn failures() {
        for spec in [
            "@{6255520 day ago}: ",
            "|^--",
            "^^-^",
            "^^-",
            ":/!-",
            "A6a^-09223372036854775808",
            "^^^^^^-(",
        ] {
            drop(
                try_parse_opts(
                    spec,
                    Options {
                        no_internal_assertions: true,
                        ..Default::default()
                    },
                )
                .unwrap_err(),
            );
        }
    }
}
mod anchor;
mod kind;
mod navigate;
