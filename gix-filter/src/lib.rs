//! A library for implementing everything needed to deal with git filter pipelines.
//!
//! Generally, multiple filters are applied in a row forming a pipeline, with each filter being a stage in that pipeline.
//! This pipeline is pre-determined with each stage being configurable.
//!
//! The transformation on an input buffer goes in two ways: either a filter is applied, or its effects are undone. Differentiating
//! between these states is important to avoid comparing unfiltered buffers with filtered ones, for example.
//!
//! This crate implements the building blocks in terms of applying and undoing filters, along with logic to decide whether
//! or not to apply such a filter.
//!
//! # Encodings
//!
//! UTF-8/16/32 (including Git's BOM variants), ASCII and ISO-8859-1 are always available.
//! Optional encoding groups must be selected by the application with `encoding-western`,
//! `encoding-legacy` or `encoding-asian`. Combine `encoding-legacy` and `encoding-asian` for the complete repertoire.
//! This crate disables `gix-encoding`'s defaults so transitive dependencies do not force optional
//! mapping tables into a build. Standalone plumbing users that need Windows-1252 or ISO-8859-15
//! must enable `encoding-western`; users of `gix` select groups through its corresponding features.
#![deny(missing_docs)]
#![forbid(unsafe_code)]

use bstr::BString;
/// The `gix-attributes` crate whose types are mentioned in the public API of [Pipeline::convert_to_worktree()].
pub use gix_attributes as attributes;
/// A forwarding of the `gix_encoding` crate for its types and convenience.
pub use gix_encoding as encoding;

/// a filter to replace `$Id$` with a git-hash of the buffer.
pub mod ident;

/// convert line endings in buffers
pub mod eol;

/// change encodings based on the `working-tree-encoding` attribute.
pub mod worktree;

/// use filter programs to perform any kind of conversion.
pub mod driver;

///
pub mod pipeline;

/// The standard git filter pipeline comprised of multiple standard filters and support for external filters.
///
/// It's configuring itself for each provided path based on the path's attributes, implementing the complex logic that governs it.
#[derive(Clone)]
pub struct Pipeline {
    /// The object hash to use when applying the `ident` filter.
    object_hash: gix_hash::Kind,
    /// Various options that are all defaultable.
    options: pipeline::Options,
    /// Storage for the attributes of each item we should process, configured for use with all attributes that concern us.
    attrs: gix_attributes::search::Outcome,
    /// Additional context to pass to process filters.
    context: pipeline::Context,
    /// State needed to keep running filter processes.
    processes: driver::State,
    /// A utility to handle multiple buffers to keep results of various filters.
    bufs: gix_utils::Buffers,
}

/// A declaration of a driver program.
///
/// It consists of up to three program declarations.
#[derive(Debug, Clone)]
pub struct Driver {
    /// The name of the driver as stored in the configuration.
    pub name: BString,

    /// The program invocation that cleans a worktree file for storage in `git`.
    ///
    /// Note that the command invocation may need its `%f` argument substituted with the name of the file to process. It will be quoted.
    pub clean: Option<BString>,
    /// The program invocation that readies a file stored in `git` for the worktree.
    ///
    /// Note that the command invocation may need its `%f` argument substituted with the name of the file to process. It will be quoted.
    pub smudge: Option<BString>,
    /// the long-running program that can typically handle both smudge and clean, and possibly delay processing as well.
    pub process: Option<BString>,
    /// If `true`, the `clean` or `smudge` programs need to succeed in order to make their content usable. Otherwise a failed
    /// single-file filter falls back to its original input. This requires buffering its output until the exit status is known.
    ///
    /// Note that this is more of a suggestion for long-running process filters as their API is streaming in nature.
    pub required: bool,
}

fn clear_and_set_capacity(buf: &mut Vec<u8>, cap: usize) -> Result<(), std::collections::TryReserveError> {
    buf.clear();
    if buf.capacity() < cap {
        buf.try_reserve(cap)?;
        debug_assert!(buf.capacity() >= cap, "{} >= {}", buf.capacity(), cap);
    }
    Ok(())
}
