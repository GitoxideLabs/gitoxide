use bstr::BString;

use crate::{Driver, Pipeline, driver, eol};

/// Define how to perform CRLF round-trip checking when converting to git.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub enum CrlfRoundTripCheck {
    /// Fail with an error if CRLF conversion isn't round-trip safe.
    Fail,
    /// Emit a warning using `gix_trace::warn!`, but don't fail.
    ///
    /// Note that the parent application has to setup tracing to make these events visible, along with a parent `span!`.
    #[default]
    Warn,
    /// Do nothing, do not perform round-trip check at all.
    Skip,
}

/// Additional configuration for the filter pipeline.
#[derive(Default, Clone)]
pub struct Options {
    /// Available (external) driver programs to invoke if attributes for path configure them.
    pub drivers: Vec<Driver>,
    /// Global options to configure end-of-line conversions, to worktree or to git.
    pub eol_config: eol::Configuration,
    /// How to perform round-trip checks during end-of-line conversions to git.
    pub crlf_roundtrip_check: CrlfRoundTripCheck,
    /// All worktree encodings for round-trip checks should be performed.
    pub encodings_with_roundtrip_check: Vec<&'static encoding_rs::Encoding>,
    /// The object hash to use when applying the `ident` filter.
    pub object_hash: gix_hash::Kind,
}

/// Context that typically doesn't change throughout the lifetime of a pipeline, for use with `process` filters.
///
/// Note that this is quite specific to third-party filters that actually make use of this additional context.
#[derive(Default, Debug, Clone)]
pub struct Context {
    /// The name of the reference that `HEAD` is pointing to. It's passed to `process` filters if present.
    pub ref_name: Option<BString>,
    /// The root-level tree that contains the current entry directly or indirectly, or the commit owning the tree (if available).
    ///
    /// This is passed to `process` filters if present.
    pub treeish: Option<gix_hash::ObjectId>,
    /// The actual blob-hash of the data we are processing. It's passed to `process` filters if present.
    ///
    /// Note that this hash might be different from the `$Id$` of the respective `ident` filter, as the latter generates the hash itself.
    pub blob: Option<gix_hash::ObjectId>,
}

const ATTRS: [&str; 6] = ["crlf", "ident", "filter", "eol", "text", "working-tree-encoding"];

/// Lifecycle
impl Pipeline {
    /// Create a new pipeline with configured `drivers` (which should be considered safe to invoke), which are passed `context`.
    /// `eol_config` serves as fallback to understand how to convert line endings if no line-ending attributes are present.
    /// `crlf_roundtrip_check` corresponds to the git-configuration of `core.safecrlf`.
    /// `object_hash` is relevant for the `ident` filter.
    pub fn new(context: gix_command::Context, options: Options) -> Self {
        let mut attrs = gix_attributes::search::Outcome::default();
        attrs.initialize_with_selection(&Default::default(), ATTRS);
        Pipeline {
            attrs,
            context: Context::default(),
            processes: driver::State::new(context),
            options,
            bufs: Default::default(),
        }
    }

    /// Turn ourselves into state managing possibly running driver processes.
    ///
    /// This can be used to control how these are terminated via [driver::State::shutdown()].
    pub fn into_driver_state(self) -> driver::State {
        self.processes
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Pipeline::new(Default::default(), Default::default())
    }
}

/// Access
impl Pipeline {
    /// Return a mutable reference to the state that handles long running processes.
    /// Interacting with it directly allows to handle delayed results.
    pub fn driver_state_mut(&mut self) -> &mut driver::State {
        &mut self.processes
    }

    /// Provide mutable context that is made available to the process filters.
    ///
    /// The context set here is relevant for the [`convert_to_git()`][Self::convert_to_git()] and
    /// [`convert_to_worktree()`][Self::convert_to_worktree()] methods.
    pub fn driver_context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    /// Return a set of options for configuration after instantiation.
    pub fn options_mut(&mut self) -> &mut Options {
        &mut self.options
    }

    /// Return our double-buffers for reuse by the caller.
    pub fn buffers_mut(&mut self) -> &mut gix_utils::Buffers {
        &mut self.bufs
    }
}

///
pub mod convert;

pub(crate) mod util;
