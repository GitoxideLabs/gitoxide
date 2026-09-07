//! Migration probes against the published upstream release candidate.
//!
//! These exercise gitoxide's callback, reporting, and recovery boundaries before replacing its exception implementation.
//! Assertions about differences describe migration blockers, rather than a desired long-term upstream contract.

use std::error::Error as StdError;
use std::fmt;
use std::io;
use std::sync::{Arc, atomic};

use exn::{Exn, IteratorExt, ResultExt};
use gix_error::{Message, ValidationError, message};

#[test]
fn callback_erasure_preserves_typed_errors_subtrees_and_allocations() {
    fn typed() -> exn::Result<(), Message> {
        Err(io::Error::from(io::ErrorKind::TimedOut)).or_raise(|| message("read failed"))
    }

    fn callback() -> Result<(), Exn> {
        typed()?;
        Ok(())
    }

    fn plain_callback() -> Result<(), Exn> {
        Err(io::Error::from(io::ErrorKind::Interrupted))?;
        Ok(())
    }

    let typed = typed().expect_err("the read times out");
    let root_address = std::ptr::from_ref(typed.frame());
    let cause_address = std::ptr::from_ref(&typed.frame().children()[0]);
    let root_location = typed.frame().location();
    let erased: Exn = typed.into();
    assert_eq!(
        std::ptr::from_ref(erased.frame()),
        root_address,
        "erasure reuses the root allocation"
    );
    assert_eq!(
        std::ptr::from_ref(&erased.frame().children()[0]),
        cause_address,
        "erasure reuses the child frames"
    );
    assert_eq!(
        erased.frame().location(),
        root_location,
        "erasure retains the creation location"
    );
    assert!(
        erased.frame().error().is::<Message>(),
        "the erased root keeps its concrete error type"
    );

    let error = callback()
        .or_raise(|| ValidationError::new("callback failed"))
        .expect_err("the callback propagates its typed error");
    let callback_frame = &error.frame().children()[0];
    assert!(
        callback_frame.error().is::<Message>(),
        "adding context retains the erased callback root"
    );
    assert_eq!(
        callback_frame.children()[0]
            .error()
            .downcast_ref::<io::Error>()
            .map(io::Error::kind),
        Some(io::ErrorKind::TimedOut),
        "the actionable cause survives both erasure and contextualization"
    );
    assert_eq!(
        plain_callback()
            .expect_err("the operation is interrupted")
            .downcast_ref::<io::Error>()
            .map(io::Error::kind),
        Some(io::ErrorKind::Interrupted),
        "plain errors can also cross an erased callback boundary with question-mark"
    );
}

#[test]
fn aggregation_accepts_heterogeneous_callback_errors() {
    let io = Exn::new(io::Error::from(io::ErrorKind::TimedOut));
    let validation = Exn::new(ValidationError::new_with_input("invalid revision", "HEAD~x"));
    let children: Vec<Exn> = vec![io.into(), validation.into()];
    let error = children.into_iter().raise(message("both callbacks failed"));
    let children = error.frame().children();
    assert_eq!(children.len(), 2, "aggregation retains both independent failures");
    assert!(
        children[0].error().is::<io::Error>(),
        "the first callback retains its I/O error"
    );
    assert!(
        children[1].error().is::<ValidationError>(),
        "the second callback retains its validation error"
    );
}

#[derive(Debug)]
struct WithSource {
    source_calls: Arc<atomic::AtomicUsize>,
    source: io::Error,
}

impl fmt::Display for WithSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("transport failed")
    }
}

impl StdError for WithSource {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source_calls.fetch_add(1, atomic::Ordering::Relaxed);
        Some(&self.source)
    }
}

#[test]
fn native_sources_require_a_different_storage_and_traversal_policy() {
    let source_calls = Arc::new(atomic::AtomicUsize::new(0));
    let make_error = || WithSource {
        source_calls: Arc::clone(&source_calls),
        source: io::Error::from(io::ErrorKind::TimedOut),
    };
    let local = gix_error::Exn::new(make_error());
    assert_eq!(
        source_calls.load(atomic::Ordering::Relaxed),
        0,
        "gix captures the error without inspecting its sources"
    );
    assert!(
        local.frame().children().is_empty(),
        "native sources are not mutable owned frames in gix"
    );

    let upstream = Exn::new(make_error());
    assert!(
        source_calls.load(atomic::Ordering::Relaxed) > 0,
        "RC1 eagerly inspects native sources at construction"
    );
    let copied_source = &upstream.frame().children()[0];
    assert!(
        !copied_source.error().is::<io::Error>(),
        "the copied source frame contains a message instead of the I/O type"
    );
    assert_eq!(
        copied_source.location(),
        upstream.frame().location(),
        "copied sources inherit their owner's location"
    );
    assert!(
        upstream
            .frame()
            .error()
            .source()
            .expect("the original root still owns its source")
            .is::<io::Error>(),
        "the original native type remains reachable through the stored root's source chain"
    );

    let local = local.into_error();
    assert!(local.can_retry(), "gix classifies the concrete native I/O source");
    assert!(
        local
            .iter_errors_with_locations()
            .find(|source| source.error().is::<io::Error>())
            .expect("the I/O source is retained")
            .location()
            .is_none(),
        "native sources have no independently captured location in gix"
    );
}

fn source_messages(error: &(dyn StdError + 'static)) -> Vec<String> {
    std::iter::successors(Some(error), |&error| error.source())
        .map(|error| format!("{error:#}"))
        .collect()
}

#[test]
fn boxed_conversion_needs_a_full_tree_adapter_for_anyhow() {
    let local = gix_error::Exn::raise_all([message("first"), message("second")], message("aggregate")).into_chain();
    assert_eq!(
        source_messages(&local),
        ["aggregate", "first", "second"],
        "gix flattens every branch into the standard source chain"
    );

    let upstream = [message("first"), message("second")]
        .into_iter()
        .raise(message("aggregate"));
    let boxed: Box<dyn StdError + Send + Sync> = upstream.into();
    assert_eq!(
        source_messages(boxed.as_ref()),
        ["aggregate", "first"],
        "RC1's standard source chain only follows the first child"
    );
    assert_eq!(
        boxed
            .downcast_ref::<exn::Frame>()
            .expect("RC1 boxes its root frame")
            .children()
            .len(),
        2,
        "the second branch is retained in the frame tree even though standard source traversal omits it"
    );
    let anyhow = anyhow::Error::from_boxed(boxed);
    assert_eq!(
        anyhow.chain().count(),
        2,
        "the default anyhow bridge does not flatten sibling frames"
    );
}

#[test]
fn standard_error_conversion_needs_type_aware_classification() {
    let local = gix_error::Exn::new(io::Error::from(io::ErrorKind::TimedOut)).into_error();
    assert!(
        gix_error::can_retry(&local),
        "the gix boundary exposes a retryable I/O error"
    );

    let upstream = Exn::new(io::Error::from(io::ErrorKind::TimedOut));
    let boxed: Box<dyn StdError + Send + Sync> = upstream.into();
    assert!(
        !gix_error::can_retry(boxed.as_ref()),
        "a boxed upstream Frame is not recognized by the existing classifier"
    );
    assert!(
        gix_error::can_retry(
            boxed
                .downcast_ref::<exn::Frame>()
                .expect("the box owns a Frame")
                .error()
        ),
        "an adapter can recover the concrete error through the public frame API"
    );
}

#[test]
fn revision_error_reparenting_needs_owned_frame_access() {
    // This is the ownership pattern used by gix/src/revision/spec/parse/mod.rs.
    let mut parse = gix_error::Exn::raise_all([message("invalid suffix")], ValidationError::new("could not parse"));
    let delegate = gix_error::Exn::new(message("ambiguous object"));
    let sources: Vec<_> = parse.drain_children().collect();
    let combined = parse.chain(delegate.chain_all(sources));
    let children = combined.frame().children();
    assert_eq!(
        children.len(),
        1,
        "the parse error now has only the delegate error as its child"
    );
    assert_eq!(
        children[0].error().to_string(),
        "ambiguous object",
        "the delegate failure is inserted below the parse error"
    );
    assert_eq!(
        children[0].children()[0].error().to_string(),
        "invalid suffix",
        "the original source moves below the delegate error"
    );

    // RC1 can construct this topology when all causes are known up front. Its public API cannot detach an existing
    // frame's children or move its root error out, which is needed after the parser has already returned an exception.
    let delegate = [message("invalid suffix")]
        .into_iter()
        .raise(message("ambiguous object"));
    let upstream = delegate.raise(ValidationError::new("could not parse"));
    assert_eq!(
        upstream.frame().children()[0].children()[0].error().to_string(),
        "invalid suffix",
        "up-front aggregation can express the final topology"
    );
}

#[test]
fn owned_root_recovery_changes_boundary_downcasting() {
    // gix's configuration validator and commit-description adapter recover the concrete root with into_inner().
    fn local_boundary() -> Result<(), Box<dyn StdError + Send + Sync>> {
        let parsed: Result<(), _> = Err(gix_error::Exn::new(ValidationError::new_with_input(
            "invalid date",
            "tomorrowish",
        )));
        parsed.map_err(gix_error::Exn::into_inner)?;
        Ok(())
    }

    fn upstream_boundary() -> Result<(), Box<dyn StdError + Send + Sync>> {
        Err(Exn::new(ValidationError::new_with_input("invalid date", "tomorrowish")))?;
        Ok(())
    }

    let local = local_boundary().expect_err("the date is invalid");
    assert!(
        local.is::<ValidationError>(),
        "recovering the owned root exposes the concrete error at the boundary"
    );
    let upstream = upstream_boundary().expect_err("the date is invalid");
    assert!(
        !upstream.is::<ValidationError>(),
        "boxing the upstream exception exposes a frame instead of the concrete root"
    );
    assert!(
        upstream
            .downcast_ref::<exn::Frame>()
            .expect("the boxed exception owns its frame")
            .error()
            .is::<ValidationError>(),
        "the concrete root can still be borrowed through the upstream frame"
    );
}

#[test]
fn diagnostic_formatting_needs_a_gix_specific_adapter() {
    let local = gix_error::Exn::new(message("source")).raise(message("context"));
    let upstream = Exn::new(message("source")).raise(message("context"));
    assert!(
        !format!("{local:#?}").contains(", at "),
        "gix's alternate Debug omits caller locations"
    );
    assert!(
        format!("{upstream:#?}").contains(", at "),
        "RC1's alternate Debug retains caller locations"
    );
    assert!(
        format!("{local:#}").contains("source"),
        "gix's alternate Display includes the error tree"
    );
    assert_eq!(
        format!("{upstream:#}"),
        "context",
        "RC1's alternate Display shows only the root message"
    );
}
