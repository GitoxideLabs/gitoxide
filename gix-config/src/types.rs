use std::collections::{HashMap, VecDeque};

use gix_features::threading::OwnShared;

use crate::{
    file,
    file::{Metadata, SectionBodyIdsLut, SectionId},
    parse::section,
};

/// A list of known sources for git configuration in order of ascending precedence.
///
/// This means values from the first one will be overridden by values in the second one, and so forth.
/// Note that included files via `include.path` and `includeIf.<condition>.path` inherit
/// their source.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Source {
    /// A special configuration file that ships with the git installation, and is thus tied to the used git binary.
    GitInstallation,
    /// System-wide configuration path. This is defined as
    /// `$(prefix)/etc/gitconfig` (where prefix is the git-installation directory).
    System,
    /// A platform defined location for where a user's git application configuration should be located.
    /// If `$XDG_CONFIG_HOME` is not set or empty, `$HOME/.config/git/config` will be used
    /// on unix.
    Git,
    /// This is usually `~/.gitconfig` on unix.
    User,
    /// The configuration of the repository itself, located in `.git/config`.
    Local,
    /// Configuration specific to a worktree as created with `git worktree` and
    /// typically located in `$GIT_DIR/config.worktree` if `extensions.worktreeConfig`
    /// is enabled.
    Worktree,
    /// Values parsed from the environment using `GIT_CONFIG_COUNT`,
    /// `GIT_CONFIG_KEY_N` and `GIT_CONFIG_VALUE_N` where `N` is incremented from 0 up to the
    /// value of `GIT_CONFIG_COUNT`.
    Env,
    /// Values set from the command-line, typically controlled by the user running a program.
    Cli,
    /// Entirely internal from a programmatic source, and can be used to have (near final) say in configuration values.
    Api,
    /// Values obtained from specific environment variables that override values in the git configuration.
    ///
    /// For example, `HTTP_PROXY` overrides `http.proxy`, no matter where it is specified, and thus
    /// controls the value similar to how it's done in `git`.
    EnvOverride,
}

/// High level `git-config` reader and writer.
///
/// This is the full-featured implementation that can deserialize, serialize,
/// and edit `git-config` files without loss of whitespace or comments.
///
/// # 'multivar' behavior
///
/// `git` is flexible enough to allow users to set a key multiple times in
/// any number of identically named sections. When this is the case, the key
/// is known as a _"multivar"_. In this case, [`raw_value()`] follows the
/// "last one wins".
///
/// Concretely, the following config has a multivar, `a`, with the values
/// of `b`, `c`, and `d`, while `e` is a single variable with the value
/// `f g h`.
///
/// ```text
/// [core]
///     a = b
///     a = c
/// [core]
///     a = d
///     e = f g h
/// ```
///
/// Calling methods that fetch or set only one value (such as [`raw_value()`])
/// key `a` with the above config will fetch `d` or replace `d`, since the last
/// valid config key/value pair is `a = d`:
///
/// # Filtering
///
/// All methods exist in a `*_filter(…, filter)` version to allow skipping sections by
/// their metadata. That way it's possible to select values based on their `gix_sec::Trust`
/// for example, or by their location.
///
/// Note that the filter may be executed even on sections that don't contain the key in question,
/// even though the section will have matched the `name` and `subsection_name` respectively.
///
/// ```
/// # use std::convert::TryFrom;
/// # let git_config = gix_config::File::try_from("[core]a=b\n[core]\na=c\na=d").unwrap();
/// assert_eq!(git_config.raw_value("core.a").unwrap(), "d");
/// ```
///
/// Consider the `multi` variants of the methods instead, if you want to work
/// with all values.
///
/// # Equality
///
/// In order to make it useful, equality will ignore all non-value bearing information, hence compare
/// only sections and their names, as well as all of their values. The ordering matters, of course.
///
/// [`raw_value()`]: Self::raw_value
#[derive(Eq, Clone, Debug, Default)]
pub struct File {
    /// Append-only storage for parsed and generated event payload bytes.
    pub(crate) event_backing: Vec<u8>,
    /// The list of events that occur before any section.
    ///
    /// Git accepts global properties before the first section, so this may also
    /// contain key/value events.
    pub(crate) frontmatter_events: crate::parse::FrontMatterEvents,
    /// Frontmatter events to be placed after the given section.
    pub(crate) frontmatter_post_section: HashMap<SectionId, crate::parse::FrontMatterEvents>,
    /// Section name to section id lookup tree, with section bodies for subsections being in a non-terminal
    /// variant of `SectionBodyIds`.
    pub(crate) section_lookup_tree: HashMap<section::Name, Vec<SectionBodyIdsLut>>,
    /// This indirection with the SectionId as the key is critical to flexibly
    /// supporting `git-config` sections, as duplicated keys are permitted.
    pub(crate) sections: HashMap<SectionId, file::SectionData>,
    /// Internal monotonically increasing counter for section ids.
    pub(crate) section_id_counter: usize,
    /// Section order for output ordering.
    pub(crate) section_order: VecDeque<SectionId>,
    /// The source of the File itself, which is attached to new sections automatically.
    pub(crate) meta: OwnShared<Metadata>,
}
