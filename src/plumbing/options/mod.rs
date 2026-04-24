use std::path::PathBuf;

use clap_complete::Shell;
use gitoxide_core as core;
use gix::bstr::BString;

use crate::shared::{AsRange, AsTime};

#[derive(Debug, clap::Parser)]
#[clap(name = "gix", about = "The git underworld", version = option_env!("GIX_VERSION"))]
#[clap(subcommand_required = true)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The repository to access.
    #[clap(short = 'r', long, default_value = ".")]
    pub repository: PathBuf,

    /// Add these values to the configuration in the form of `key=value` or `key`.
    ///
    /// For example, if `key` is `core.abbrev`, set configuration like `[core] abbrev = key`,
    /// or `remote.origin.url = foo` to set `[remote "origin"] url = foo`.
    #[clap(long, short = 'c', value_parser = crate::shared::AsBString)]
    pub config: Vec<BString>,

    /// The amount of threads to use for some operations.
    ///
    /// If unset, or the value is 0, there is no limit and all logical cores can be used.
    #[clap(long, short = 't')]
    pub threads: Option<usize>,

    /// Display verbose messages and progress information
    #[clap(long, short = 'v')]
    pub verbose: bool,

    /// Display structured `tracing` output in a tree-like structure.
    #[clap(long)]
    #[cfg(feature = "tracing")]
    pub trace: bool,

    /// Turn off verbose message display for commands where these are shown by default.
    #[clap(long, conflicts_with("verbose"))]
    pub no_verbose: bool,

    /// Bring up a terminal user interface displaying progress visually.
    #[cfg(feature = "prodash-render-tui")]
    #[clap(long, conflicts_with("verbose"))]
    pub progress: bool,

    /// Don't default malformed configuration flags, but show an error instead. Ignore IO errors as well.
    ///
    /// Note that some subcommands use strict mode by default.
    // TODO: needs a 'lenient' mutually exclusive counterpart. Opens the gate to auto-verbose some commands, and add --no-verbose
    //       for these.
    #[clap(long, short = 's')]
    pub strict: bool,

    /// The progress TUI will stay up even though the work is already completed.
    ///
    /// Use this to be able to read progress messages or additional information visible in the TUI log pane.
    #[cfg(feature = "prodash-render-tui")]
    #[clap(long, conflicts_with("verbose"), requires("progress"))]
    pub progress_keep_open: bool,

    /// Determine the format to use when outputting statistics.
    #[clap(
        long,
        short = 'f',
        default_value = "human",
        value_parser = crate::shared::AsOutputFormat
    )]
    pub format: core::OutputFormat,

    /// The object format to assume when reading files that don't inherently know about it, or when writing files.
    #[clap(long, default_value_t = gix::hash::Kind::default(), value_parser = crate::shared::AsHashKind)]
    pub object_hash: gix::hash::Kind,

    #[clap(subcommand)]
    pub cmd: Subcommands,
}

#[derive(Debug, clap::Subcommand)]
pub enum Subcommands {
    /// Subcommands for creating worktree archives.
    #[cfg(feature = "gitoxide-core-tools-archive")]
    Archive(archive::Platform),
    /// Interact with branches.
    #[clap(visible_alias = "branches")]
    Branch(branch::Platform),
    /// Remove untracked files from the working tree.
    #[cfg(feature = "gitoxide-core-tools-clean")]
    Clean(clean::Command),
    /// Subcommands for interacting with commit-graph files.
    #[clap(subcommand)]
    CommitGraph(commitgraph::Subcommands),
    /// Interact with the object database.
    #[clap(subcommand)]
    Odb(odb::Subcommands),
    /// Check for missing objects.
    Fsck(fsck::Platform),
    /// Interact with tree objects.
    #[clap(subcommand)]
    Tree(tree::Subcommands),
    /// Interact with commit objects.
    #[clap(subcommand)]
    Commit(commit::Subcommands),
    /// Interact with tag objects.
    #[clap(visible_alias = "tags")]
    Tag(tag::Platform),
    /// Verify the integrity of the entire repository
    Verify {
        #[clap(flatten)]
        args: free::pack::VerifyOptions,
    },
    /// Query and obtain information about revisions.
    #[clap(subcommand)]
    Revision(revision::Subcommands),
    /// A program just like `git credential`.
    #[clap(subcommand)]
    Credential(credential::Subcommands),
    /// Fetch data from remotes and store it in the repository.
    #[cfg(feature = "gitoxide-core-blocking-client")]
    Fetch(fetch::Platform),
    /// Clone a repository into a new directory.
    #[cfg(feature = "gitoxide-core-blocking-client")]
    Clone(clone::Platform),
    /// Update remote refs along with associated objects.
    #[cfg(feature = "gitoxide-core-blocking-client")]
    Push(push::Platform),
    /// Interact with the mailmap.
    #[clap(subcommand)]
    Mailmap(mailmap::Subcommands),
    /// Interact with the remote hosts.
    #[cfg(any(feature = "gitoxide-core-async-client", feature = "gitoxide-core-blocking-client"))]
    Remote(remote::Platform),
    /// Interact with the attribute files like .gitattributes.
    #[clap(subcommand, visible_alias = "attrs")]
    Attributes(attributes::Subcommands),
    /// Interact with the exclude files like .gitignore.
    #[clap(subcommand)]
    Exclude(exclude::Subcommands),
    /// Interact with a worktree index like .git/index.
    #[clap(subcommand)]
    Index(index::Subcommands),
    /// Interact with submodules.
    #[clap(alias = "submodules")]
    Submodule(submodule::Platform),
    /// Show whatever object is at the given spec.
    ///
    /// Visible-aliased to `cat-file` for parity with `git cat-file`; the
    /// canonical gix spelling remains `cat`.
    #[clap(visible_alias = "cat-file")]
    Cat {
        /// Pretty-print `<object>` content (the default for `gix cat`).
        ///
        /// Accepted for `git cat-file -p` parity. `gix cat` already emits
        /// a type-appropriate pretty representation for trees, blobs,
        /// commits, and tags, so the flag is a compat no-op.
        #[clap(short = 'p', long = "pretty")]
        pretty: bool,
        /// Exit with zero status if `<object>` exists and is a valid object,
        /// non-zero status (1) otherwise. Produces no stdout output on either
        /// path. Mirrors `git cat-file -e`.
        #[clap(short = 'e')]
        exists: bool,
        /// Print the object's type name (`blob`, `tree`, `commit`, or `tag`)
        /// followed by a newline. Mirrors `git cat-file -t`.
        #[clap(short = 't')]
        print_type: bool,
        /// Print the object's size in bytes followed by a newline.
        /// Mirrors `git cat-file -s`.
        #[clap(short = 's')]
        print_size: bool,
        /// Use the mailmap file to rewrite author/committer/tagger idents
        /// in `-p` / `-s` output for commits and tags. Canonical spelling
        /// `--use-mailmap`; `--mailmap` is a git-compat alias.
        ///
        /// Accepted for parity. On fixtures without a `.mailmap` file
        /// the flag is semantically a no-op; actual ident rewriting is
        /// wired up when the first real-mailmap row (`--use-mailmap -s`
        /// against a seeded `.mailmap`) lands.
        #[clap(long = "use-mailmap", visible_alias = "mailmap")]
        use_mailmap: bool,
        /// Disable mailmap ident rewriting. Canonical spelling
        /// `--no-use-mailmap`; `--no-mailmap` is a git-compat alias.
        #[clap(long = "no-use-mailmap", visible_alias = "no-mailmap")]
        no_use_mailmap: bool,
        /// Historical option retained for compat with scripts that pass
        /// it. git's OPT_HIDDEN_BOOL marks this as a no-op ("historical
        /// option -- no-op"). Accepted and ignored.
        #[clap(long = "allow-unknown-type")]
        allow_unknown_type: bool,
        /// Apply textconv filters to the object's content before emitting.
        /// Mirrors `git cat-file --textconv`.
        ///
        /// With no textconv filter configured the output equals the raw
        /// blob bytes. Requires `<object>` to be of form `<tree-ish>:<path>`
        /// unless `--path=<path>` is given.
        #[clap(long)]
        textconv: bool,
        /// Apply worktree filters (smudge / EOL conversion) to the blob
        /// before emitting. Mirrors `git cat-file --filters`.
        ///
        /// With no filters configured the output equals the raw blob bytes.
        /// Same path-resolution requirement as `--textconv`.
        #[clap(long)]
        filters: bool,
        /// Associate `<object>` with this path for filter-attribute lookup.
        /// Mirrors `git cat-file --path=<path>`. Requires `--textconv` or
        /// `--filters`; erroring 129 otherwise (git: "'--path=...' needs
        /// '--filters' or '--textconv'").
        #[clap(long, value_name = "PATH")]
        path: Option<String>,
        /// Read object names from stdin and emit `<oid> <type> <size> LF`
        /// per input line (no contents). Mirrors `git cat-file
        /// --batch-check`. An optional `=<format>` argument overrides the
        /// default format — supported atoms today: `%(objectname)`,
        /// `%(objecttype)`, `%(objectsize)`.
        #[clap(
            long,
            value_name = "FORMAT",
            num_args = 0..=1,
            default_missing_value = "",
            require_equals = true,
        )]
        batch_check: Option<String>,
        /// Read object names from stdin, emit `<oid> <type> <size> LF
        /// <contents> LF` per input. Mirrors `git cat-file --batch`.
        /// Same format-string grammar as --batch-check.
        #[clap(
            long,
            value_name = "FORMAT",
            num_args = 0..=1,
            default_missing_value = "",
            require_equals = true,
        )]
        batch: Option<String>,
        /// Read `info <obj>` / `contents <obj>` / `flush` commands from
        /// stdin. Mirrors `git cat-file --batch-command`.
        #[clap(
            long,
            value_name = "FORMAT",
            num_args = 0..=1,
            default_missing_value = "",
            require_equals = true,
        )]
        batch_command: Option<String>,
        /// Under `--batch*`, NUL-terminate both input and output.
        /// Mirrors `git cat-file -Z` (recommended for scripting).
        #[clap(short = 'Z')]
        nul_terminated: bool,
        /// Under `--batch*`, NUL-terminate input only (output stays LF).
        /// Deprecated in favor of `-Z`. Mirrors `git cat-file -z`.
        #[clap(short = 'z')]
        nul_input: bool,
        /// Under `--batch*`, use stdio buffering for output. Accepted for
        /// compat; observable effect is batch timing, not content.
        #[clap(long)]
        buffer: bool,
        /// Under `--batch*`, visit --batch-all-objects in pack-storage
        /// order rather than hash-sorted.
        #[clap(long)]
        unordered: bool,
        /// Under `--batch[-check]`, ignore stdin and iterate every object
        /// in the odb (including alternates). Mirrors git's
        /// `--batch-all-objects`.
        #[clap(long)]
        batch_all_objects: bool,
        /// Under `--batch*`, follow in-tree symlinks when requesting
        /// `<tree-ish>:<path>` inputs. Mirrors `git cat-file
        /// --follow-symlinks`.
        #[clap(long)]
        follow_symlinks: bool,
        /// The object to print, optionally preceded by a type hint.
        ///
        /// Two positional shapes:
        ///
        /// * `<object>` — with a mode flag (-e / -p / -t / -s) OR alone
        ///   (legacy `gix cat` default to pretty-print).
        /// * `<type> <object>` — no mode flag; assert `<object>` peels to
        ///   `<type>` (`blob`/`tree`/`commit`/`tag`) and emit raw bytes
        ///   (mirrors `git cat-file <type> <object>`).
        #[clap(num_args = 1..=2, value_name = "[TYPE] OBJECT")]
        args: Vec<String>,
    },
    /// Check for changes in the repository, treating this as an error.
    IsClean,
    /// Check for changes in the repository, treating their absence as an error.
    IsChanged,
    /// Show which git configuration values are used or planned.
    ConfigTree,
    Status(status::Platform),
    Config(config::Platform),
    #[cfg(feature = "gitoxide-core-tools-corpus")]
    Corpus(corpus::Platform),
    MergeBase(merge_base::Command),
    Merge(merge::Platform),
    /// Print paths relevant to the Git installation.
    Env,
    Diff(diff::Platform),
    Log(log::Platform),
    Worktree(worktree::Platform),
    /// Subcommands that need no Git repository to run.
    #[clap(subcommand)]
    Free(free::Subcommands),
    /// Blame lines in a file.
    Blame {
        /// Print additional statistics to help understanding performance.
        #[clap(long, short = 's')]
        statistics: bool,
        /// The file to create the blame information for.
        file: std::ffi::OsString,
        /// Only blame lines in the given 1-based inclusive range '<start>,<end>', e.g. '20,40'.
        #[clap(short='L', value_parser=AsRange, action=clap::ArgAction::Append)]
        ranges: Vec<std::ops::RangeInclusive<u32>>,
        /// Don't consider commits before the given date.
        #[clap(long,  value_parser=AsTime, value_name = "DATE")]
        since: Option<gix::date::Time>,
    },
    /// Generate shell completions to stdout or a directory.
    #[clap(visible_alias = "generate-completions", visible_alias = "shell-completions")]
    Completions {
        /// The shell to generate completions for. Otherwise it's derived from the environment.
        #[clap(long, short)]
        shell: Option<Shell>,
        /// The output directory in case multiple files are generated. If not provided, will write to stdout.
        out_dir: Option<String>,
    },
}

#[cfg(feature = "gitoxide-core-tools-archive")]
pub mod archive {
    use std::path::PathBuf;

    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
    pub enum Format {
        /// An internal format that is for debugging, it should not be persisted and cannot be read back.
        ///
        /// However, it represents that bare data stream without with minimal overhead, and is a good
        /// metric for throughput.
        Internal,
        /// Use the `.tar` file format, uncompressed.
        Tar,
        /// Use the `.tar.gz` file format, compressed with `gzip`.
        TarGz,
        /// Use the `.zip` container format.
        Zip,
    }

    #[derive(Debug, clap::Parser)]
    pub struct Platform {
        /// Explicitly set the format. Otherwise derived from the suffix of the output file.
        #[clap(long, short = 'f', value_enum)]
        pub format: Option<Format>,
        /// Apply the prefix verbatim to any path we add to the archive. Use a trailing `/` if prefix is a directory.
        #[clap(long)]
        pub prefix: Option<String>,
        /// The compression strength to use for `.zip` and `.tar.gz` archives, valid from 0-9.
        #[clap(long, short = 'l', requires = "format")]
        pub compression_level: Option<u8>,
        /// Add the given path to the archive. Directories will always be empty.
        #[clap(long, short = 'p')]
        pub add_path: Vec<PathBuf>,
        /// Add the new file from a slash-separated path, which must happen in pairs of two, first the path, then the content.
        #[clap(long, short = 'v')]
        pub add_virtual_file: Vec<String>,
        /// The file to write the archive to.
        ///
        /// It's extension determines the archive format, unless `--format` is set.
        pub output_file: PathBuf,

        /// The revspec of the commit or tree to traverse, or the tree at `HEAD` if unspecified.
        ///
        /// If commit, the commit timestamp will be used as timestamp for each file in the archive.
        pub treeish: Option<String>,
    }
}

pub mod branch {
    /// Flag-bearing top-level surface for `gix branch`, mirroring git's
    /// cmdmode (`-l`/`-d`/`-D`/`-m`/`-M`/`-c`/`-C`/`--show-current`/
    /// `--edit-description`) + modifier flags. Resolution matches
    /// `vendor/git/builtin/branch.c` cmd_branch: when no cmdmode is
    /// given, list mode is the default; non-option args without a
    /// cmdmode trigger create mode.
    #[derive(Debug, clap::Parser)]
    pub struct Platform {
        /// List branches. Also the default when no cmdmode is given.
        /// Mirrors `OPT_BOOL('l', "list", ...)` in builtin/branch.c.
        #[clap(short = 'l', long = "list")]
        pub list: bool,

        /// List (or with `-d`, delete) remote-tracking branches.
        /// Mirrors `OPT_SET_INT_F('r', "remotes", &filter.kind,
        /// REF_REMOTE_BRANCH, ...)`.
        #[clap(short = 'r', long = "remotes")]
        pub remotes: bool,

        /// List both local and remote-tracking branches.
        /// Mirrors `OPT_SET_INT_F('a', "all", &filter.kind,
        /// REF_REMOTE_BRANCH | REF_LOCAL_BRANCH, ...)`.
        #[clap(short = 'a', long = "all")]
        pub all: bool,

        /// Delete fully-merged branches. Mirrors
        /// `OPT_BIT('d', "delete", &delete, ..., 1)`.
        #[clap(short = 'd', long = "delete")]
        pub delete: bool,

        /// Delete branches even if not merged. Shortcut for
        /// `--delete --force`. Mirrors
        /// `OPT_BIT('D', NULL, &delete, ..., 2)`.
        #[clap(short = 'D')]
        pub delete_force: bool,

        /// Move/rename a branch together with its config and reflog.
        /// Mirrors `OPT_BIT('m', "move", &rename, ..., 1)`.
        #[clap(short = 'm', long = "move")]
        pub move_: bool,

        /// Move/rename a branch even if target exists. Shortcut for
        /// `--move --force`. Mirrors
        /// `OPT_BIT('M', NULL, &rename, ..., 2)`.
        #[clap(short = 'M')]
        pub move_force: bool,

        /// Copy a branch together with its config and reflog. Mirrors
        /// `OPT_BIT('c', "copy", &copy, ..., 1)`.
        #[clap(short = 'c', long = "copy")]
        pub copy: bool,

        /// Copy a branch even if target exists. Shortcut for
        /// `--copy --force`. Mirrors `OPT_BIT('C', NULL, &copy, ..., 2)`.
        #[clap(short = 'C')]
        pub copy_force: bool,

        /// Print the name of the current branch (empty on detached HEAD).
        /// Mirrors `OPT_BOOL(0, "show-current", ...)`.
        #[clap(long = "show-current")]
        pub show_current: bool,

        /// Open EDITOR to edit the branch description.
        /// Mirrors `OPT_BOOL(0, "edit-description", ...)`.
        #[clap(long = "edit-description")]
        pub edit_description: bool,

        /// Force creation/move/rename/deletion, reset of an existing
        /// branch, etc. Mirrors `OPT__FORCE`.
        #[clap(short = 'f', long = "force")]
        pub force: bool,

        /// Verbose listing (sha + subject + upstream on `-vv`).
        /// Mirrors `OPT__VERBOSE` (count-style).
        #[clap(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
        pub verbose: u8,

        /// Suppress informational messages. Mirrors `OPT__QUIET`.
        #[clap(short = 'q', long = "quiet")]
        pub quiet: bool,

        /// Set upstream tracking (`branch.<name>.remote` +
        /// `branch.<name>.merge`). With no argument (bare `-u`), this
        /// is parsed positionally so gix can accept both
        /// `-u <upstream>` and `--set-upstream-to=<upstream>`. Mirrors
        /// `OPT_STRING('u', "set-upstream-to", ...)`.
        #[clap(short = 'u', long = "set-upstream-to", value_name = "upstream")]
        pub set_upstream_to: Option<String>,

        /// Remove upstream information. Mirrors
        /// `OPT_BOOL(0, "unset-upstream", ...)`.
        #[clap(long = "unset-upstream")]
        pub unset_upstream: bool,

        /// Set up tracking info on a newly created branch. With no
        /// value, defaults to `direct` (use the start-point's branch
        /// as upstream); `inherit` copies the start-point's upstream.
        /// `require_equals` mirrors git's PARSE_OPT_OPTARG: the value
        /// form is `--track=direct`, never `--track direct` (which
        /// would consume the next positional).
        /// Mirrors `OPT_CALLBACK_F('t', "track", ..., PARSE_OPT_OPTARG)`.
        #[clap(short = 't', long = "track", value_name = "mode", num_args = 0..=1, default_missing_value = "direct", require_equals = true)]
        pub track: Option<String>,

        /// Do not set up tracking, even if `branch.autoSetupMerge` is
        /// enabled. Mirrors git's `--no-track`.
        #[clap(long = "no-track")]
        pub no_track: bool,

        /// Experimental: recurse into submodules on branch creation if
        /// `submodule.propagateBranches` is set. Mirrors
        /// `OPT_BOOL(0, "recurse-submodules", ...)`.
        #[clap(long = "recurse-submodules")]
        pub recurse_submodules: bool,

        /// Force reflog creation for the branch. Mirrors
        /// `OPT_BOOL(0, "create-reflog", ...)`.
        #[clap(long = "create-reflog")]
        pub create_reflog: bool,

        /// Abbrev length for SHAs in `-v` listings. Defaults to 7.
        /// Mirrors `OPT__ABBREV`.
        #[clap(long = "abbrev", value_name = "n")]
        pub abbrev: Option<usize>,

        /// Print full SHAs (overrides `--abbrev`). Mirrors git's
        /// `--no-abbrev` which sets abbrev=0 in parse_opt_abbrev_cb.
        #[clap(long = "no-abbrev")]
        pub no_abbrev: bool,

        /// List only branches containing `<commit>` (HEAD if omitted).
        /// Implies list mode. Mirrors `OPT_CONTAINS`.
        #[clap(long = "contains", value_name = "commit", num_args = 0..=1, default_missing_value = "HEAD")]
        pub contains: Option<std::ffi::OsString>,

        /// List only branches NOT containing `<commit>` (HEAD if
        /// omitted). Implies list mode. Mirrors `OPT_NO_CONTAINS`.
        #[clap(long = "no-contains", value_name = "commit", num_args = 0..=1, default_missing_value = "HEAD")]
        pub no_contains: Option<std::ffi::OsString>,

        /// List only branches whose tips are reachable from `<commit>`
        /// (HEAD if omitted). Implies list mode. Mirrors `OPT_MERGED`.
        #[clap(long = "merged", value_name = "commit", num_args = 0..=1, default_missing_value = "HEAD")]
        pub merged: Option<std::ffi::OsString>,

        /// List only branches whose tips are NOT reachable from
        /// `<commit>` (HEAD if omitted). Implies list mode. Mirrors
        /// `OPT_NO_MERGED`.
        #[clap(long = "no-merged", value_name = "commit", num_args = 0..=1, default_missing_value = "HEAD")]
        pub no_merged: Option<std::ffi::OsString>,

        /// List only branches whose ref directly points at `<object>`.
        /// Implies list mode. Mirrors `OPT_CALLBACK(0, "points-at", ...)`.
        #[clap(long = "points-at", value_name = "object")]
        pub points_at: Option<std::ffi::OsString>,

        /// Format string interpolating `%(<atom>)` fields, same atom
        /// set as `for-each-ref`. Mirrors
        /// `OPT_STRING(0, "format", ...)`.
        #[clap(long = "format", value_name = "format")]
        pub format_string: Option<String>,

        /// Skip rows whose `--format` expansion is empty. Mirrors
        /// `OPT_BOOL(0, "omit-empty", ...)`.
        #[clap(long = "omit-empty")]
        pub omit_empty: bool,

        /// Sort key (for-each-ref syntax). Prefix `-` for descending.
        /// Multiple uses accumulate; last key wins as primary. Mirrors
        /// `OPT_REF_SORT`.
        #[clap(long = "sort", value_name = "key", action = clap::ArgAction::Append)]
        pub sort: Vec<String>,

        /// Pack rows into columns. With no arg equivalent to
        /// `--column=always`. Mirrors `OPT_COLUMN`.
        #[clap(long = "column", value_name = "options", num_args = 0..=1, default_missing_value = "always", conflicts_with = "no_column")]
        pub column: Option<String>,

        /// Disable column packing (`--column=never`). Mirrors git's
        /// `--no-column`.
        #[clap(long = "no-column", conflicts_with = "column")]
        pub no_column: bool,

        /// Highlight current/local/remote-tracking branches.
        /// Mirrors `OPT__COLOR`.
        #[clap(long = "color", value_name = "when", num_args = 0..=1, default_missing_value = "always", conflicts_with = "no_color")]
        pub color: Option<String>,

        /// Turn off branch coloring (`--color=never`).
        #[clap(long = "no-color", conflicts_with = "color")]
        pub no_color: bool,

        /// Case-insensitive sort/filter. Mirrors
        /// `OPT_BOOL('i', "ignore-case", ...)`.
        #[clap(short = 'i', long = "ignore-case")]
        pub ignore_case: bool,

        /// Positional args: branch names, patterns, start-points, or
        /// old/new name pairs for move/copy. git's cmd_branch reads
        /// these via argc after parse_options.
        pub args: Vec<std::ffi::OsString>,
    }
}

pub mod status {
    use gix::bstr::BString;

    use crate::shared::{CheckPathSpec, ParseRenameFraction};

    #[derive(Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
    pub enum Submodules {
        /// display all information about submodules, including ref changes, modifications and untracked files.
        #[default]
        All,
        /// Compare only the configuration of the superprojects commit with the actually checked out `HEAD` commit.
        RefChange,
        /// See if there are worktree modifications compared to the index, but do not check for untracked files.
        Modifications,
        /// Ignore all submodule changes.
        None,
    }

    #[derive(Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
    pub enum Ignored {
        /// Display all ignored files and directories, but collapse them if possible to simplify.
        /// `traditional` is accepted as a git-compat alias — gix's collapsing
        /// currently behaves close enough to git's traditional mode for
        /// effect-mode parity; true traditional (fully-expanded ignored
        /// folders) requires gix_dir work documented on the original TODO.
        #[default]
        #[clap(alias = "traditional")]
        Collapsed,
        /// Show exact matches. Note that this may show directories if these are a match as well.
        ///
        /// Simplification will not happen in this mode.
        Matching,
        /// Suppress ignored-file listing (git's `--ignored=no`). Dispatch
        /// maps this to an absent `ignored` option in the core Options —
        /// i.e., behavior identical to omitting `--ignored` entirely.
        No,
    }

    #[derive(Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
    pub enum Format {
        /// A basic format that is easy to read, and useful for a first glimpse as flat list.
        #[default]
        Simplified,
        /// Output very similar to `git status --porcelain=2`.
        PorcelainV2,
        /// Byte-exact compatibility with `git status -s` / `git status --short`.
        Short,
    }

    /// Version argument for `--porcelain[=<version>]`. Defaults to V1 to
    /// match git's behavior when `--porcelain` is given without a version.
    #[derive(Default, Debug, Copy, Clone, PartialEq, Eq, clap::ValueEnum)]
    pub enum PorcelainVersion {
        #[default]
        V1,
        V2,
    }

    /// Mode argument for `-u<mode>` / `--untracked-files[=<mode>]`.
    /// Defaults to All (the git default when `-u` is bare).
    #[derive(Default, Debug, Copy, Clone, PartialEq, Eq, clap::ValueEnum)]
    pub enum UntrackedMode {
        No,
        Normal,
        #[default]
        All,
    }

    /// Mode argument for `--ignore-submodules[=<when>]`. Defaults to All
    /// (the git default when flag is bare). Modes map to gix's submodule-
    /// ignore semantics (gix::submodule::config::Ignore).
    #[derive(Default, Debug, Copy, Clone, PartialEq, Eq, clap::ValueEnum)]
    pub enum IgnoreSubmodulesMode {
        None,
        Untracked,
        Dirty,
        #[default]
        All,
    }

    #[derive(Debug, clap::Parser)]
    #[command(about = "Compute repository status similar to `git status`")]
    pub struct Platform {
        /// The way status data is displayed.
        #[clap(long, short = 'f')]
        pub format: Option<Format>,
        /// Give the output in the short format, matching `git status -s` byte-for-byte.
        /// Equivalent to `--format=short`; conflicts with `--format`.
        #[clap(short = 's', long = "short", conflicts_with = "format")]
        pub short: bool,
        /// Give the output in the long-format (the default). Accepted for
        /// compatibility with `git status --long`; conflicts with `--short`
        /// and `--format` just like in git.
        #[clap(long = "long", conflicts_with_all = ["short", "format"])]
        pub long: bool,
        /// Show the branch and tracking info even in short-format — equivalent
        /// to git's `-b` / `--branch`. Has no effect on the long format
        /// (which always shows the branch).
        #[clap(short = 'b', long = "branch")]
        pub branch: bool,
        /// Show the number of entries currently stashed away. Accepted for
        /// compatibility with `git status --show-stash`; stash-count emission
        /// is not yet implemented (would require reflog traversal of
        /// refs/stash), so this flag is currently a no-op under effect mode.
        #[clap(long = "show-stash")]
        pub show_stash: bool,
        /// Give the output in an easy-to-parse format for scripts. Defaults
        /// to v1 when given without a value. Maps to `--format=short` for
        /// v1 (byte-identical for our supported fixtures) and `--format=
        /// porcelain-v2` for v2. Conflicts with `--short` and `--format`,
        /// matching git.
        #[clap(long = "porcelain", value_name = "VERSION", num_args = 0..=1,
               default_missing_value = "v1",
               conflicts_with_all = ["short", "format"])]
        pub porcelain: Option<PorcelainVersion>,
        /// Show the textual changes staged to be committed (-v) or also the
        /// worktree-vs-index diff (-vv). Accepted for compat with
        /// `git status -v`/`-vv`; diff emission is not yet implemented,
        /// so under effect mode this is currently a no-op that yields
        /// exit-code parity.
        #[clap(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
        pub verbose: u8,
        /// Show untracked files. The mode must be stuck to the option
        /// (`-uno`, never `-u no`). Bare `-u` / `--untracked-files`
        /// default to `all` — matching git.
        #[clap(short = 'u', long = "untracked-files",
               value_name = "MODE", num_args = 0..=1,
               default_missing_value = "all")]
        pub untracked_files: Option<UntrackedMode>,
        /// Ignore changes to submodules when looking for changes. Accepted
        /// for compat with `git status --ignore-submodules[=<when>]`; the
        /// existing `--submodules` flag already covers most of the range
        /// in gix-native spelling. Under effect mode this is a no-op that
        /// yields exit-code parity — submodule-visibility wiring is
        /// deferred to a submodule-fixture iteration.
        #[clap(long = "ignore-submodules",
               value_name = "WHEN", num_args = 0..=1,
               default_missing_value = "all")]
        pub ignore_submodules: Option<IgnoreSubmodulesMode>,
        /// Terminate entries with NUL, instead of LF. Implies
        /// `--porcelain=v1` (i.e. Format::Short) if no other format is given.
        #[clap(short = 'z', action = clap::ArgAction::SetTrue)]
        pub null_terminator: bool,
        /// Display untracked files in columns (git's `--column[=<opts>]`).
        /// Accepted for compat; column formatting is not yet implemented —
        /// under effect mode this is a no-op that yields exit-code parity.
        #[clap(long = "column", value_name = "OPTIONS", num_args = 0..=1,
               default_missing_value = "always")]
        pub column: Option<String>,
        /// Disable column display (git's `--no-column`). Accepted for compat.
        #[clap(long = "no-column", action = clap::ArgAction::SetTrue,
               conflicts_with = "column")]
        pub no_column: bool,
        /// Display detailed ahead/behind counts for the branch relative to
        /// its upstream. Accepted for compat; the header rendering already
        /// happens in gix's long format when an upstream is configured, so
        /// this is a no-op under effect mode.
        #[clap(long = "ahead-behind", action = clap::ArgAction::SetTrue)]
        pub ahead_behind: bool,
        /// Do not display ahead/behind counts. Accepted for compat.
        #[clap(long = "no-ahead-behind", action = clap::ArgAction::SetTrue,
               conflicts_with = "ahead_behind")]
        pub no_ahead_behind: bool,
        /// Turn on rename detection (git's `--renames`). Accepted for compat;
        /// the existing `--index-worktree-renames` flag covers gix-native
        /// wiring. Under effect mode this is a no-op.
        #[clap(long = "renames", action = clap::ArgAction::SetTrue)]
        pub renames: bool,
        /// Turn off rename detection (git's `--no-renames`). Accepted for
        /// compat; conflicts with `--renames` and `--find-renames`.
        #[clap(long = "no-renames", action = clap::ArgAction::SetTrue,
               conflicts_with_all = ["renames", "find_renames"])]
        pub no_renames: bool,
        /// Turn on rename detection with an optional similarity threshold
        /// (git's `--find-renames[=<n>]`). Accepted for compat; conflicts
        /// with `--no-renames`.
        #[clap(long = "find-renames", value_name = "N",
               num_args = 0..=1, default_missing_value = "50",
               conflicts_with = "no_renames")]
        pub find_renames: Option<String>,
        /// If enabled, show ignored files and directories.
        #[clap(long)]
        pub ignored: Option<Option<Ignored>>,
        /// Define how to display the submodule status. Defaults to git configuration if unset.
        #[clap(long)]
        pub submodules: Option<Submodules>,
        /// Print additional statistics to help understanding performance.
        #[clap(long)]
        pub statistics: bool,
        /// Don't write back a changed index, which forces this operation to always be idempotent.
        #[clap(long)]
        pub no_write: bool,
        /// Enable rename tracking between the index and the working tree, preventing the collapse of folders as well.
        #[clap(long, value_parser = ParseRenameFraction)]
        pub index_worktree_renames: Option<Option<f32>>,
        /// The git path specifications to list attributes for, or unset to read from stdin one per line.
        #[clap(value_parser = CheckPathSpec)]
        pub pathspec: Vec<BString>,
    }
}

pub mod merge_base {
    #[derive(Debug, clap::Parser)]
    #[command(about = "A command for calculating all merge-bases")]
    pub struct Command {
        /// A revspec for the first commit.
        pub first: String,
        /// Revspecs for the other commits to compute the merge-base with.
        pub others: Vec<String>,
    }
}

pub mod worktree {
    #[derive(Debug, clap::Parser)]
    #[command(about = "Commands for handling worktrees")]
    pub struct Platform {
        #[clap(subcommand)]
        pub cmd: SubCommands,
    }

    #[derive(Debug, clap::Subcommand)]
    pub enum SubCommands {
        /// List all worktrees, along with some accompanying information.
        List,
    }
}

#[cfg(feature = "gitoxide-core-tools-corpus")]
pub mod corpus {
    use std::path::PathBuf;

    #[derive(Debug, clap::Parser)]
    #[command(about = "Run algorithms on a corpus of git repositories and store their results for later analysis")]
    pub struct Platform {
        /// The path to the database to read and write depending on the sub-command.
        #[arg(long, default_value = "corpus.db")]
        pub db: PathBuf,
        /// The path to the root of the corpus to search repositories in.
        #[arg(long, short = 'p', default_value = ".")]
        pub path: PathBuf,
        #[clap(subcommand)]
        pub cmd: SubCommands,
    }

    #[derive(Debug, clap::Subcommand)]
    pub enum SubCommands {
        /// Perform a corpus run on all registered repositories.
        Run {
            /// Don't run any task, but print all repos that would be traversed once.
            ///
            /// Note that this will refresh repositories if necessary and store them in the database, it just won't run tasks.
            #[clap(long, short = 'n')]
            dry_run: bool,

            /// The SQL that will be appended to the actual select statement for repositories to apply additional filtering, like `LIMIT 10`.
            ///
            /// The string must be trusted even though the engine will only execute a single statement.
            #[clap(long, short = 'r')]
            repo_sql_suffix: Option<String>,

            /// The short_names of the tasks to include when running.
            #[clap(long, short = 't')]
            include_task: Vec<String>,
        },
        /// Re-read all repositories under the corpus directory, and add or update them.
        Refresh,
    }
}

pub mod merge {
    use gix::bstr::BString;

    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
    pub enum ResolveWith {
        /// Use ours then theirs in case of conflict.
        Union,
        /// Use only ours in case of conflict.
        Ours,
        /// Use only theirs in case of conflict.
        Theirs,
    }

    impl From<ResolveWith> for gix::merge::blob::builtin_driver::text::Conflict {
        fn from(value: ResolveWith) -> Self {
            match value {
                ResolveWith::Union => gix::merge::blob::builtin_driver::text::Conflict::ResolveWithUnion,
                ResolveWith::Ours => gix::merge::blob::builtin_driver::text::Conflict::ResolveWithOurs,
                ResolveWith::Theirs => gix::merge::blob::builtin_driver::text::Conflict::ResolveWithTheirs,
            }
        }
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
    pub enum FileFavor {
        /// Use only ours in case of conflict.
        Ours,
        /// Use only theirs in case of conflict.
        Theirs,
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
    pub enum TreeFavor {
        /// Use only the previous tree entry in case of conflict.
        Ancestor,
        /// Use only ours tree entry in case of conflict.
        Ours,
    }

    impl From<FileFavor> for gix::merge::tree::FileFavor {
        fn from(value: FileFavor) -> Self {
            match value {
                FileFavor::Ours => gix::merge::tree::FileFavor::Ours,
                FileFavor::Theirs => gix::merge::tree::FileFavor::Theirs,
            }
        }
    }

    impl From<TreeFavor> for gix::merge::tree::TreeFavor {
        fn from(value: TreeFavor) -> Self {
            match value {
                TreeFavor::Ancestor => gix::merge::tree::TreeFavor::Ancestor,
                TreeFavor::Ours => gix::merge::tree::TreeFavor::Ours,
            }
        }
    }

    #[derive(Debug, clap::Parser)]
    pub struct SharedOptions {
        /// Keep all objects to be written in memory to avoid any disk IO.
        ///
        /// Note that the resulting tree won't be available or inspectable.
        #[clap(long, short = 'm')]
        pub in_memory: bool,
        /// Decide how to resolve content conflicts in files. If unset, write conflict markers and fail.
        #[clap(long, short = 'f')]
        pub file_favor: Option<FileFavor>,
        /// Decide how to resolve conflicts in trees, i.e. modification/deletion. If unset, try to preserve both states and fail.
        #[clap(long, short = 't')]
        pub tree_favor: Option<TreeFavor>,
        /// Print additional information about conflicts for debugging.
        #[clap(long, short = 'd')]
        pub debug: bool,
    }

    #[derive(Debug, clap::Parser)]
    #[command(about = "Perform merges of various kinds")]
    pub struct Platform {
        #[clap(subcommand)]
        pub cmd: SubCommands,
    }

    #[derive(Debug, clap::Subcommand)]
    pub enum SubCommands {
        /// Merge a file by specifying ours, base and theirs.
        File {
            /// Decide how to resolve conflicts. If unset, write conflict markers and fail.
            #[clap(long, short = 'c')]
            resolve_with: Option<ResolveWith>,

            /// A path or revspec to our file.
            #[clap(value_name = "OURS", value_parser = crate::shared::AsBString)]
            ours: BString,
            /// A path or revspec to the base for both ours and theirs.
            #[clap(value_name = "BASE", value_parser = crate::shared::AsBString)]
            base: BString,
            /// A path or revspec to their file.
            #[clap(value_name = "THEIRS", value_parser = crate::shared::AsBString)]
            theirs: BString,
        },

        /// Merge a tree by specifying ours, base and theirs, writing it to the object database.
        Tree {
            #[clap(flatten)]
            opts: SharedOptions,

            /// A revspec to our treeish.
            #[clap(value_name = "OURS", value_parser = crate::shared::AsBString)]
            ours: BString,
            /// A revspec to the base as treeish for both ours and theirs.
            #[clap(value_name = "BASE", value_parser = crate::shared::AsBString)]
            base: BString,
            /// A revspec to their treeish.
            #[clap(value_name = "THEIRS", value_parser = crate::shared::AsBString)]
            theirs: BString,
        },
        /// Merge a commits by specifying ours, and theirs, writing the tree to the object database.
        Commit {
            #[clap(flatten)]
            opts: SharedOptions,

            /// A revspec to our committish.
            #[clap(value_name = "OURS", value_parser = crate::shared::AsBString)]
            ours: BString,
            /// A revspec to their committish.
            #[clap(value_name = "THEIRS", value_parser = crate::shared::AsBString)]
            theirs: BString,
        },
    }
}

pub mod diff {
    use gix::bstr::BString;

    /// Print all changes between two objects.
    #[derive(Debug, clap::Parser)]
    pub struct Platform {
        #[clap(subcommand)]
        pub cmd: SubCommands,
    }

    #[derive(Debug, clap::Subcommand)]
    pub enum SubCommands {
        /// Diff two trees.
        Tree {
            /// A rev-spec representing the 'before' or old tree.
            #[clap(value_parser = crate::shared::AsBString)]
            old_treeish: BString,
            /// A rev-spec representing the 'after' or new tree.
            #[clap(value_parser = crate::shared::AsBString)]
            new_treeish: BString,
        },
        /// Diff two versions of a file.
        File {
            /// A rev-spec representing the 'before' or old state of the file, like '@~100:file'
            #[clap(value_parser = crate::shared::AsBString)]
            old_revspec: BString,
            /// A rev-spec representing the 'after' or new state of the file, like ':file'
            #[clap(value_parser = crate::shared::AsBString)]
            new_revspec: BString,
        },
    }
}

pub mod log {
    use gix::bstr::BString;

    /// List all commits in a repository, optionally limited to those that change a given path.
    #[derive(Debug, clap::Parser)]
    pub struct Platform {
        /// Walk all refs under `refs/` (plus HEAD) as if they were listed as revisions.
        #[clap(long)]
        pub all: bool,

        /// Walk every ref under `refs/heads/` as if listed as revisions.
        #[clap(long)]
        pub branches: bool,

        /// Walk every ref under `refs/tags/` as if listed as revisions.
        #[clap(long)]
        pub tags: bool,

        /// Walk every ref under `refs/remotes/` as if listed as revisions.
        #[clap(long)]
        pub remotes: bool,

        /// Show only the first <n> commits matching the traversal.
        #[clap(short = 'n', long, value_name = "n")]
        pub max_count: Option<usize>,

        /// Skip <n> commits before showing the rest of the traversal.
        #[clap(long, value_name = "n")]
        pub skip: Option<usize>,

        /// Skip merge commits (commits with more than one parent).
        #[clap(long, conflicts_with = "merges")]
        pub no_merges: bool,

        /// Show only merge commits.
        #[clap(long)]
        pub merges: bool,

        /// Require commits to have at least <n> parents.
        #[clap(long, value_name = "n")]
        pub min_parents: Option<usize>,

        /// Require commits to have at most <n> parents.
        #[clap(long, value_name = "n")]
        pub max_parents: Option<usize>,

        // --- traversal order (accepted but semantics deferred) ---
        /// Reverse the order of commits emitted by the traversal.
        #[clap(long)]
        pub reverse: bool,
        /// Show commits in topological order (gix's default walker order).
        #[clap(long)]
        pub topo_order: bool,
        /// Show commits in reverse chronological order.
        #[clap(long)]
        pub date_order: bool,
        /// Show commits in reverse chronological order of author date.
        #[clap(long)]
        pub author_date_order: bool,
        /// Follow only the first parent for merge commits.
        #[clap(long)]
        pub first_parent: bool,

        // --- grep filters (accepted but semantics deferred) ---
        /// Filter commits by message pattern (may be repeated).
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "pattern")]
        pub grep: Vec<BString>,
        /// Case-insensitive matching for --grep.
        #[clap(short = 'i', long = "regexp-ignore-case")]
        pub regexp_ignore_case: bool,
        /// Invert the --grep match.
        #[clap(long)]
        pub invert_grep: bool,
        /// Require all repeated --grep patterns to match.
        #[clap(long)]
        pub all_match: bool,
        /// Treat --grep as a POSIX extended regular expression.
        #[clap(short = 'E', long = "extended-regexp")]
        pub extended_regexp: bool,
        /// Treat --grep as a literal string.
        #[clap(short = 'F', long = "fixed-strings")]
        pub fixed_strings: bool,
        /// Filter by author pattern.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "pattern")]
        pub author: Option<BString>,
        /// Filter by committer pattern.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "pattern")]
        pub committer: Option<BString>,
        /// Limit to commits more recent than <time>.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "time", alias = "after")]
        pub since: Option<BString>,
        /// Limit to commits older than <time>.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "time", alias = "before")]
        pub until: Option<BString>,

        // --- pretty/format (accepted but semantics deferred) ---
        /// Shorthand for `--pretty=oneline --abbrev-commit`.
        #[clap(long)]
        pub oneline: bool,
        /// Pretty-format preset (oneline, short, medium, full, fuller, raw, reference)
        /// or `format:<fmt>`.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "format")]
        pub pretty: Option<BString>,
        /// Custom format string (shorthand for `--pretty=format:<fmt>`).
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "fmt")]
        pub format: Option<BString>,
        /// Abbreviate shown commit hashes.
        #[clap(long)]
        pub abbrev_commit: bool,
        /// Show full commit hashes (cancels --abbrev-commit).
        #[clap(long)]
        pub no_abbrev_commit: bool,
        /// Length of abbreviated hashes.
        #[clap(long, value_name = "n")]
        pub abbrev: Option<usize>,

        // --- decoration (accepted but semantics deferred) ---
        /// Show ref names at each commit.
        #[clap(long, num_args = 0..=1, default_missing_value = "short", value_name = "mode")]
        pub decorate: Option<String>,
        /// Disable decoration.
        #[clap(long)]
        pub no_decorate: bool,
        /// Include refs matching <pattern> for decoration (may be repeated).
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "pattern")]
        pub decorate_refs: Vec<BString>,
        /// Exclude refs matching <pattern> from decoration (may be repeated).
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "pattern")]
        pub decorate_refs_exclude: Vec<BString>,
        /// Clear any prior --decorate-refs / --decorate-refs-exclude.
        #[clap(long)]
        pub clear_decorations: bool,
        /// Prepend the source ref name to each commit line.
        #[clap(long)]
        pub source: bool,
        /// Display a text-based commit graph.
        #[clap(long)]
        pub graph: bool,

        // --- diff output (accepted but semantics deferred) ---
        /// Show the diff each commit introduces.
        #[clap(short = 'p', long)]
        pub patch: bool,
        /// Suppress any diff output.
        #[clap(short = 's', long = "no-patch")]
        pub no_patch: bool,
        /// Print diffstat per commit.
        #[clap(long)]
        pub stat: bool,
        /// Print only the summary line of --stat.
        #[clap(long)]
        pub shortstat: bool,
        /// Print machine-friendly diffstat.
        #[clap(long)]
        pub numstat: bool,
        /// List only affected file names.
        #[clap(long)]
        pub name_only: bool,
        /// List affected file names with status letters.
        #[clap(long)]
        pub name_status: bool,
        /// Emit git-diff --raw output.
        #[clap(long)]
        pub raw: bool,
        /// Detect renames in diff output.
        #[clap(short = 'M', long = "find-renames")]
        pub find_renames: bool,

        // --- date (accepted but semantics deferred) ---
        /// Date format for committer / author dates.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "mode")]
        pub date: Option<BString>,

        // --- diff merges (accepted but semantics deferred) ---
        /// Short form of `--diff-merges=separate`.
        #[clap(short = 'm')]
        pub diff_all_merge_parents: bool,
        /// Short form of `--diff-merges=combined`.
        #[clap(short = 'c')]
        pub diff_combined: bool,
        /// Short form of `--diff-merges=dense-combined`.
        #[clap(long = "cc")]
        pub diff_cc: bool,
        /// Merge-diff mode selector.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "mode")]
        pub diff_merges: Option<BString>,

        // --- misc log (accepted but semantics deferred) ---
        /// Rewrite author/committer names via .mailmap.
        #[clap(long, alias = "use-mailmap")]
        pub mailmap: bool,
        /// Skip .mailmap even if configured.
        #[clap(long, alias = "no-use-mailmap")]
        pub no_mailmap: bool,
        /// Emit `log size <bytes>` per commit.
        #[clap(long)]
        pub log_size: bool,
        /// Include notes from refs/notes/commits.
        #[clap(long)]
        pub notes: bool,
        /// Suppress notes even if a default is configured.
        #[clap(long)]
        pub no_notes: bool,
        /// Verify and print commit signatures.
        #[clap(long)]
        pub show_signature: bool,

        // --- color (accepted but semantics deferred) ---
        /// Color control (always | never | auto).
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "when", num_args = 0..=1, default_missing_value = "always")]
        pub color: Option<BString>,
        /// Disable color output (alias for --color=never).
        #[clap(long)]
        pub no_color: bool,

        // --- boundary / ancestry / negation ---
        /// Mark excluded-range endpoints with "-".
        #[clap(long)]
        pub boundary: bool,
        /// Restrict commits to those on a direct A..B ancestry path.
        #[clap(long)]
        pub ancestry_path: bool,
        /// Reverse the include/exclude sense of subsequent revisions. Accepted
        /// as a bool here; positional state-flip semantics are deferred.
        #[clap(long)]
        pub not: bool,

        // --- file-specific (accepted but semantics deferred) ---
        /// Follow renames for a single file's history.
        #[clap(long)]
        pub follow: bool,
        /// Show full diff of commits that touch the pathspec.
        #[clap(long)]
        pub full_diff: bool,
        /// Line-range trace `start,end:file` (may be repeated).
        #[clap(short = 'L', value_parser = crate::shared::AsBString, value_name = "range:file")]
        pub line_range: Vec<BString>,

        // --- pickaxe (accepted but semantics deferred) ---
        /// Show only commits that add or remove a line matching <regex>.
        #[clap(short = 'G', value_parser = crate::shared::AsBString, value_name = "regex")]
        pub pickaxe_regex_g: Option<BString>,
        /// Show only commits whose diff changes the occurrence count of <string>.
        #[clap(short = 'S', value_parser = crate::shared::AsBString, value_name = "string")]
        pub pickaxe_string_s: Option<BString>,
        /// Treat -S <string> as a regex (implied by -G).
        #[clap(long)]
        pub pickaxe_regex: bool,
        /// Include merge commits when pickaxe-matching.
        #[clap(long)]
        pub pickaxe_all: bool,

        // --- cherry / left-right family (accepted but semantics deferred) ---
        /// Show only commits that are cherry-pickable (equivalence class).
        #[clap(long)]
        pub cherry: bool,
        /// Mark commits on left and right sides of a symmetric range.
        #[clap(long)]
        pub cherry_mark: bool,
        /// Like --cherry-mark, but omit commits equivalent to their mirror.
        #[clap(long)]
        pub cherry_pick: bool,
        /// Show only commits reachable from the left side of A...B.
        #[clap(long)]
        pub left_only: bool,
        /// Show only commits reachable from the right side of A...B.
        #[clap(long)]
        pub right_only: bool,
        /// Mark which side of a symmetric range each commit is reachable from.
        #[clap(long)]
        pub left_right: bool,

        // --- reflog walk ---
        /// Walk reflog entries rather than commit ancestry.
        #[clap(short = 'g', long = "walk-reflogs")]
        pub walk_reflogs: bool,
        /// Limit commits by reflog message regex (requires --walk-reflogs).
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "pattern")]
        pub grep_reflog: Option<BString>,

        // --- history simplification ---
        /// Simplify history, showing only decoration-carrying commits.
        #[clap(long)]
        pub simplify_by_decoration: bool,
        /// Simplify merges to only the "interesting" ones.
        #[clap(long)]
        pub simplify_merges: bool,
        /// Show full history (no parent rewriting).
        #[clap(long)]
        pub full_history: bool,
        /// Alias for --full-history.
        #[clap(long)]
        pub dense: bool,
        /// Alias for --simplify-by-decoration — sparse history.
        #[clap(long)]
        pub sparse: bool,
        /// Don't walk ancestors — emit only the given revisions.
        #[clap(long)]
        pub no_walk: bool,
        /// Re-enable walking after a prior --no-walk.
        #[clap(long)]
        pub do_walk: bool,
        /// Emit commits in the order encountered (not topo-sorted).
        #[clap(long)]
        pub in_commit_order: bool,

        // --- extra ref-selection companions ---
        /// Exclude refs matching <pattern> from --all/--branches/--tags/--remotes.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "pattern")]
        pub exclude: Vec<BString>,
        /// Include refs matching <pattern> (glob).
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "pattern")]
        pub glob: Vec<BString>,
        /// Include refs from alternate object stores.
        #[clap(long)]
        pub alternate_refs: bool,

        // --- parents / children / misc display ---
        /// Print the parent(s) on each commit line.
        #[clap(long)]
        pub parents: bool,
        /// Print the children of each commit.
        #[clap(long)]
        pub children: bool,
        /// Show merge commits only if they rejoin the main branch.
        #[clap(long)]
        pub show_pulls: bool,
        /// Emit separator marks between linear history segments.
        #[clap(long)]
        pub show_linear_break: bool,
        /// NUL-terminate output records instead of newlines.
        #[clap(short = 'z')]
        pub z: bool,
        /// Print only the commit count instead of the walk.
        #[clap(long)]
        pub count: bool,

        // --- submodule diff control ---
        /// Submodule diff rendering mode.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "mode")]
        pub submodule: Option<BString>,

        // --- diff-options surface (rev-list + log share this whole block) ---
        /// Unified-context width (long form of -U<n>).
        #[clap(long, value_name = "n")]
        pub unified: Option<usize>,
        /// Print a condensed summary of extended header information.
        #[clap(long)]
        pub summary: bool,
        /// Compact summary (--summary + per-commit compacting).
        #[clap(long)]
        pub compact_summary: bool,
        /// Alias for the Myers diff algorithm's minimal-diff variant.
        #[clap(long)]
        pub minimal: bool,
        /// Use the "patience diff" algorithm.
        #[clap(long)]
        pub patience: bool,
        /// Use the "histogram diff" algorithm.
        #[clap(long)]
        pub histogram: bool,
        /// Filter diff output by status letters.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "filter")]
        pub diff_filter: Option<BString>,
        /// Filter commits that change an object matching <oid>.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "oid")]
        pub find_object: Option<BString>,
        /// Try harder to detect copies.
        #[clap(long)]
        pub find_copies_harder: bool,
        /// Exit with 0 if no changes, 1 if changes.
        #[clap(long)]
        pub exit_code: bool,
        /// Complain about whitespace / conflict markers introduced.
        #[clap(long)]
        pub check: bool,
        /// Emit binary patches.
        #[clap(long)]
        pub binary: bool,
        /// Emit full index hashes in diff output.
        #[clap(long)]
        pub full_index: bool,
        /// For merges, re-merge and show the diff against the recorded merge.
        #[clap(long)]
        pub remerge_diff: bool,
        /// Dirstat rendering mode.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "mode", num_args = 0..=1, default_missing_value = "")]
        pub dirstat: Option<BString>,
        /// Use a configured external diff program.
        #[clap(long)]
        pub ext_diff: bool,
        /// Do not use a configured external diff program.
        #[clap(long)]
        pub no_ext_diff: bool,
        /// Apply textconv filters before diffing.
        #[clap(long)]
        pub textconv: bool,
        /// Do not apply textconv filters.
        #[clap(long)]
        pub no_textconv: bool,
        /// Treat all files as text.
        #[clap(long)]
        pub text: bool,
        /// Emit patch alongside --raw.
        #[clap(long)]
        pub patch_with_raw: bool,
        /// Emit patch alongside --stat.
        #[clap(long)]
        pub patch_with_stat: bool,
        /// Highlight moved lines within diffs.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "mode", num_args = 0..=1, default_missing_value = "default")]
        pub color_moved: Option<BString>,
        /// Emit word-level diff.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "mode", num_args = 0..=1, default_missing_value = "plain")]
        pub word_diff: Option<BString>,
        /// Regex defining word boundaries for --word-diff.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "regex")]
        pub word_diff_regex: Option<BString>,
        /// Emit whitespace-error highlights.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "kind")]
        pub ws_error_highlight: Option<BString>,
        /// Include enclosing function in each hunk.
        #[clap(long = "function-context", short = 'W')]
        pub function_context: bool,
        /// Minimum lines between merged hunks.
        #[clap(long, value_name = "lines")]
        pub inter_hunk_context: Option<usize>,
        /// Enable indent-heuristic for hunk boundaries.
        #[clap(long)]
        pub indent_heuristic: bool,
        /// Disable indent-heuristic.
        #[clap(long)]
        pub no_indent_heuristic: bool,
        /// Drop deletion-only hunks in repeatable order.
        #[clap(long)]
        pub irreversible_delete: bool,
        /// Disable rename detection.
        #[clap(long)]
        pub no_renames: bool,
        /// Treat empty files as the empty-content sentinel for rename detection.
        #[clap(long)]
        pub rename_empty: bool,
        /// Disable --rename-empty.
        #[clap(long)]
        pub no_rename_empty: bool,
        /// Ignore any line containing only whitespace.
        #[clap(long)]
        pub ignore_all_space: bool,
        /// Ignore changes whose lines are blank.
        #[clap(long)]
        pub ignore_blank_lines: bool,
        /// Ignore CR at end of line.
        #[clap(long)]
        pub ignore_cr_at_eol: bool,
        /// Ignore lines matching the regex.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "regex")]
        pub ignore_matching_lines: Option<BString>,
        /// Ignore whitespace at end of line.
        #[clap(long)]
        pub ignore_space_at_eol: bool,
        /// Ignore amount of whitespace (but not presence).
        #[clap(long)]
        pub ignore_space_change: bool,
        /// Prefix for source side of diff.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "prefix")]
        pub src_prefix: Option<BString>,
        /// Prefix for destination side of diff.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "prefix")]
        pub dst_prefix: Option<BString>,
        /// Suppress a/ and b/ prefixes in diff output.
        #[clap(long)]
        pub no_prefix: bool,
        /// Make pathnames relative to <dir>.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "path", num_args = 0..=1, default_missing_value = "")]
        pub relative: Option<BString>,
        /// Disable --relative.
        #[clap(long)]
        pub no_relative: bool,
        /// Direct diff output to <file>.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "file")]
        pub output: Option<BString>,

        // --- rev-list companions applicable to log ---
        /// Include HEAD's reflog entries as extra tips.
        #[clap(long)]
        pub reflog: bool,
        /// Read additional arguments from stdin.
        #[clap(long)]
        pub stdin: bool,
        /// Tolerate missing object references during the walk.
        #[clap(long)]
        pub ignore_missing: bool,
        /// Show commits touching files with unmerged state.
        #[clap(long)]
        pub merge: bool,
        /// Filter --since as a predicate (not a traversal boundary).
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "time")]
        pub since_as_filter: Option<BString>,
        /// Like --first-parent but only for the exclude list.
        #[clap(long)]
        pub exclude_first_parent_only: bool,
        /// Drop empty commits from the walk.
        #[clap(long)]
        pub remove_empty: bool,
        /// Only include refs from the current worktree.
        #[clap(long)]
        pub single_worktree: bool,

        // --- pretty companions ---
        /// Decoding for commit encoding.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "encoding")]
        pub encoding: Option<BString>,
        /// Expand tabs in commit messages before rendering.
        #[clap(long, value_name = "n", num_args = 0..=1, default_missing_value = "8")]
        pub expand_tabs: Option<usize>,
        /// Disable --expand-tabs.
        #[clap(long)]
        pub no_expand_tabs: bool,

        // --- third steward-pass: remainder of the user-visible flag surface ---
        /// Use POSIX basic regex for --grep and friends.
        #[clap(long)]
        pub basic_regexp: bool,
        /// Use Perl-compatible regex for --grep and friends.
        #[clap(short = 'P', long)]
        pub perl_regexp: bool,
        /// Exclude hidden refs when selecting heads.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "section")]
        pub exclude_hidden: Option<BString>,
        /// Output commits suitable for feeding into `git bisect`.
        #[clap(long)]
        pub bisect: bool,
        /// Shorthand for `--date=relative`.
        #[clap(long)]
        pub relative_date: bool,
        /// Alias for --diff-merges=dd.
        #[clap(long = "dd")]
        pub dd: bool,
        /// Suppress merge-commit diffs.
        #[clap(long)]
        pub no_diff_merges: bool,
        /// In combined diffs, emit paths from each parent.
        #[clap(long)]
        pub combined_all_paths: bool,
        /// Override the "+" marker for added lines.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "char")]
        pub output_indicator_new: Option<BString>,
        /// Override the "-" marker for removed lines.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "char")]
        pub output_indicator_old: Option<BString>,
        /// Override the " " marker for context lines.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "char")]
        pub output_indicator_context: Option<BString>,
        /// Show tree objects in diff output.
        #[clap(short = 't')]
        pub show_tree_objects: bool,
        /// Anchored-diff algorithm anchor text (may be repeated).
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "text")]
        pub anchored: Vec<BString>,
        /// Cumulative dirstat.
        #[clap(long)]
        pub cumulative: bool,
        /// Dirstat counting by file instead of lines.
        #[clap(long)]
        pub dirstat_by_file: bool,
        /// Disable --color-moved.
        #[clap(long)]
        pub no_color_moved: bool,
        /// Whitespace-handling for moved-line coloring.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "mode")]
        pub color_moved_ws: Option<BString>,
        /// Disable --color-moved-ws.
        #[clap(long)]
        pub no_color_moved_ws: bool,
        /// Word-based diff coloring with optional regex.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "regex", num_args = 0..=1, default_missing_value = "")]
        pub color_words: Option<BString>,
        /// Break rewrites into delete-then-create pairs.
        #[clap(short = 'B', long = "break-rewrites", value_parser = crate::shared::AsBString, value_name = "n/m", num_args = 0..=1, default_missing_value = "")]
        pub break_rewrites: Option<BString>,
        /// Detect file copies (short `-C[<n>]`, long `--find-copies`).
        #[clap(short = 'C', long = "find-copies", value_parser = crate::shared::AsBString, value_name = "n", num_args = 0..=1, default_missing_value = "")]
        pub find_copies: Option<BString>,
        /// Rename-detection exhaustive scan cap (`-l<num>`).
        #[clap(short = 'l', value_name = "num")]
        pub rename_detection_limit: Option<usize>,
        /// Path-ordering file (short `-O<file>`, long `--orderfile`).
        #[clap(short = 'O', long = "orderfile", value_parser = crate::shared::AsBString, value_name = "file")]
        pub orderfile: Option<BString>,
        /// Skip to first-matching path when emitting diffs.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "path")]
        pub skip_to: Option<BString>,
        /// Rotate the diff output so <path> appears first.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "path")]
        pub rotate_to: Option<BString>,
        /// Reverse the sense of old/new in diff output.
        #[clap(short = 'R')]
        pub reverse_diff: bool,
        /// Submodule-diff rendering control (distinct from --submodule).
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "when", num_args = 0..=1, default_missing_value = "all")]
        pub ignore_submodules: Option<BString>,
        /// Emit diffs with the default `a/` and `b/` prefixes.
        #[clap(long)]
        pub default_prefix: bool,
        /// Prepend <prefix> to every diff output line.
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "prefix")]
        pub line_prefix: Option<BString>,
        /// Treat intent-to-add paths as absent in the index.
        #[clap(long)]
        pub ita_invisible_in_index: bool,
        /// Show notes by default.
        #[clap(long)]
        pub show_notes_by_default: bool,
        /// Show notes from a specific notes-ref (deprecated alias of --notes).
        #[clap(long, value_parser = crate::shared::AsBString, value_name = "ref", num_args = 0..=1, default_missing_value = "")]
        pub show_notes: Option<BString>,
        /// Show standard notes (deprecated alias).
        #[clap(long)]
        pub standard_notes: bool,
        /// Suppress standard notes (deprecated alias).
        #[clap(long)]
        pub no_standard_notes: bool,
        /// Revision(s) to walk from. Multiple revspecs are accepted (git's
        /// rev-list grammar). Only the first is honored today; --not / multi-
        /// revspec composition is a later parity row.
        #[clap(value_parser = crate::shared::AsBString)]
        pub revspecs: Vec<BString>,

        /// The path specification(s) to limit commits to. Must follow a `--` separator.
        #[clap(value_parser = crate::shared::AsBString, last = true, num_args = 0..)]
        pub pathspec: Vec<BString>,
    }
}

pub mod config {
    use gix::bstr::BString;

    /// Print all entries in a configuration file or access other sub-commands.
    #[derive(Debug, clap::Parser)]
    #[clap(subcommand_required(false))]
    pub struct Platform {
        /// The filter terms to limit the output to matching sections and subsections only.
        ///
        /// Typical filters are `branch` or `remote.origin` or `remote.or*` - git-style globs are supported
        /// and comparisons are case-insensitive.
        #[clap(value_parser = crate::shared::AsBString)]
        pub filter: Vec<BString>,
    }
}

// `fetch` lives in its own file to match the `push` convention; see
// src/plumbing/options/fetch.rs.

#[cfg(feature = "gitoxide-core-blocking-client")]
pub mod clone {
    use std::{ffi::OsString, num::NonZeroU32, path::PathBuf};

    use gix::remote::fetch::Shallow;

    #[derive(Debug, clap::Parser)]
    pub struct Platform {
        /// Output additional typically information provided by the server as part of the connection handshake.
        #[clap(long, short = 'H')]
        pub handshake_info: bool,

        /// Run verbosely — does not affect progress status, only the banner/info lines.
        ///
        /// Parse-only: clone currently prints no banner regardless; the flag is
        /// accepted for CLI compatibility with git's `OPT__VERBOSITY(-v)`.
        #[clap(long, short = 'v')]
        pub verbose: bool,

        /// Operate quietly — no progress on stderr.
        ///
        /// Parse-only: gix clone does not auto-start a progress TUI when
        /// stderr is piped anyway, so the flag is currently a no-op. Accepted
        /// for CLI compatibility with git's `OPT__VERBOSITY(-q)`.
        #[clap(long, short = 'q')]
        pub quiet: bool,

        /// Force progress reporting even when stderr is not a TTY.
        ///
        /// Parse-only: clashes with gix's top-level `--progress` TUI; the
        /// subcommand flag is accepted to mirror git's per-subcommand
        /// `OPT_BOOL(0, "progress", ...)` and currently has no runtime effect.
        #[clap(long = "progress", conflicts_with = "quiet")]
        pub force_progress: bool,

        /// Do not checkout HEAD after the clone is complete.
        ///
        /// Parse-only: empty-upstream clones have no files to check out
        /// regardless. When a row exercises a non-empty checkout, wire
        /// this through to skip main_worktree in gitoxide-core's clone.
        #[clap(long, short = 'n')]
        pub no_checkout: bool,

        /// The clone will be bare and a working tree checkout won't be available.
        #[clap(long)]
        pub bare: bool,

        /// Set up a mirror of the source repository. Implies --bare; actual
        /// refspec translation (+refs/*:refs/*) and `remote.<name>.mirror=true`
        /// is a TODO for bytes-parity — currently just implies bare.
        #[clap(long)]
        pub mirror: bool,

        /// Do not clone any tags. Useful to reduce the size of the clone if only branches are needed.
        #[clap(long)]
        pub no_tags: bool,

        /// Clone tags (the default). Accepted for CLI symmetry with
        /// `--no-tags`; the flag cancels out a prior `--no-tags` on
        /// the same line. Parse-only.
        #[clap(long, overrides_with = "no_tags")]
        pub tags: bool,

        /// Initialize and clone submodules after the main clone.
        /// Multi-valued. Parse-only — gix clone doesn't yet recurse
        /// into submodules.
        ///
        /// `require_equals = true` mirrors git's parse_options PARSE_OPT_OPTARG
        /// behavior for long flags: `--recurse-submodules` alone means "all"
        /// (default ".") and `--recurse-submodules=<pathspec>` supplies an
        /// explicit pathspec, but `--recurse-submodules <next>` treats `<next>`
        /// as a positional, not as this flag's value.
        #[clap(long = "recurse-submodules", value_name = "PATHSPEC", num_args = 0..=1, default_missing_value = ".", require_equals = true)]
        pub recurse_submodules: Vec<String>,

        /// Alias for `--recurse-submodules`. Parse-only.
        #[clap(long = "recursive", value_name = "PATHSPEC", num_args = 0..=1, default_missing_value = ".", require_equals = true)]
        pub recursive: Vec<String>,

        /// Clone submodules shallowly (depth 1). Parse-only.
        #[clap(long, overrides_with = "_no_shallow_submodules")]
        pub shallow_submodules: bool,

        /// Opposite of `--shallow-submodules`. Parse-only.
        #[clap(long = "no-shallow-submodules", overrides_with = "shallow_submodules")]
        pub _no_shallow_submodules: bool,

        /// Clone submodules using their remote-tracking branch. Parse-only.
        #[clap(long, overrides_with = "_no_remote_submodules")]
        pub remote_submodules: bool,

        /// Opposite of `--remote-submodules`. Parse-only.
        #[clap(long = "no-remote-submodules", overrides_with = "remote_submodules")]
        pub _no_remote_submodules: bool,

        /// Apply the partial-clone filter to submodules too. Requires
        /// `--filter` and `--recurse-submodules`. Parse-only.
        #[clap(long)]
        pub also_filter_submodules: bool,

        /// Partial-clone filter spec (e.g., `blob:none`). Parse-only —
        /// gix clone's promisor-remote pipeline is not yet wired.
        #[clap(long, value_name = "SPEC")]
        pub filter: Option<String>,

        /// Non-default upload-pack path for ssh transport. Parse-only —
        /// gix's ssh transport doesn't negotiate the remote command yet.
        #[clap(long = "upload-pack", short = 'u', value_name = "PATH")]
        pub upload_pack: Option<OsString>,

        /// Protocol v2 server option; multi-valued, order preserved.
        /// Parse-only.
        #[clap(long = "server-option", value_name = "OPTION")]
        pub server_option: Vec<String>,

        /// Restrict IP address family to IPv4. Parse-only (no-op on
        /// file:// transport).
        #[clap(long, short = '4', overrides_with = "ipv6")]
        pub ipv4: bool,

        /// Restrict IP address family to IPv6. Parse-only.
        #[clap(long, short = '6', overrides_with = "ipv4")]
        pub ipv6: bool,

        /// Submodule-fetch parallelism. Parse-only (no submodule fetching yet).
        #[clap(long, short = 'j', value_name = "N")]
        pub jobs: Option<u32>,

        /// Template directory for init. Parse-only — gix init doesn't
        /// consume templates today.
        #[clap(long, value_name = "DIR")]
        pub template: Option<PathBuf>,

        /// Place .git at <dir> with a gitfile link back from the worktree.
        /// Parse-only — gix doesn't redirect the git dir today.
        #[clap(long = "separate-git-dir", value_name = "DIR")]
        pub separate_git_dir: Option<PathBuf>,

        /// Ref storage format. Parse-only — gix uses `files` regardless.
        /// Unknown values die 128.
        #[clap(long = "ref-format", value_name = "FMT")]
        pub ref_format: Option<String>,

        /// Clone-scoped `-c/--config=<key=value>` overrides, merged with
        /// the top-level `-c` list before the initial fetch.
        #[clap(long = "config", short = 'c', value_parser = crate::shared::AsBString, value_name = "KEY=VAL")]
        pub config_overrides: Vec<gix::bstr::BString>,

        /// Bundle URI to fetch before the real fetch. Incompatible with
        /// --depth / --shallow-since / --shallow-exclude. Parse-only —
        /// gix doesn't consume bundle URIs today; missing-URI bundles
        /// fall through to a regular clone (matching git's behavior).
        #[clap(long = "bundle-uri", value_name = "URI")]
        pub bundle_uri: Option<String>,

        /// Employ a sparse-checkout initialized to just the toplevel directory.
        ///
        /// Parse-only: empty-upstream clones have no toplevel anything to
        /// check out regardless.
        #[clap(long)]
        pub sparse: bool,

        /// Use `<name>` in place of `origin` for the remote-tracking remote.
        ///
        /// Parse-only: gix clone uses `origin` unconditionally today.
        /// Wiring deferred to a row that exercises the resulting
        /// refs/remotes/<name>/ layout.
        #[clap(long, short = 'o', value_name = "NAME")]
        pub origin: Option<OsString>,

        /// Use hard links / direct copy for the .git/objects dir when the
        /// source is local. `--no-local` forces the regular Git transport
        /// even for local paths. Parse-only (gix clone fetches via gix's
        /// file:// transport regardless; the flag is currently a no-op).
        #[clap(long, short = 'l', overrides_with = "_no_local")]
        pub local: bool,

        /// Opposite of `--local`. Parse-only (see `--local`).
        #[clap(long = "no-local", overrides_with = "local")]
        pub _no_local: bool,

        /// Force copy of the .git/objects directory instead of hardlinking
        /// when the source is local. Parse-only — gix currently never
        /// hardlinks objects regardless.
        #[clap(long)]
        pub no_hardlinks: bool,

        /// Setup the clone to share objects with the source repository via
        /// objects/info/alternates. Parse-only — gix doesn't wire the
        /// alternates file today, so the clone is a full copy regardless.
        #[clap(long, short = 's')]
        pub shared: bool,

        /// Fail if the source repository is shallow. Parse-only — gix
        /// accepts shallow upstreams without complaint today.
        #[clap(long, overrides_with = "_no_reject_shallow")]
        pub reject_shallow: bool,

        /// Opposite of `--reject-shallow`. Parse-only.
        #[clap(long = "no-reject-shallow", overrides_with = "reject_shallow")]
        pub _no_reject_shallow: bool,

        /// Use <repo>/objects as an alternate. Multi-valued. Parse-only —
        /// gix doesn't wire alternates today.
        #[clap(long = "reference", value_name = "REPO")]
        pub reference: Vec<PathBuf>,

        /// Like --reference but warns rather than aborts on missing alternate.
        /// Multi-valued. Parse-only.
        #[clap(long = "reference-if-able", value_name = "REPO")]
        pub reference_if_able: Vec<PathBuf>,

        /// Borrow objects from --reference repos only for the clone, then
        /// stop borrowing. Parse-only.
        #[clap(long)]
        pub dissociate: bool,

        /// Clone only the history leading to the tip of a single branch.
        /// Parse-only.
        #[clap(long, overrides_with = "_no_single_branch")]
        pub single_branch: bool,

        /// Opposite of `--single-branch`. Parse-only.
        #[clap(long = "no-single-branch", overrides_with = "single_branch")]
        pub _no_single_branch: bool,

        #[clap(flatten)]
        pub shallow: ShallowOptions,

        /// The url of the remote to connect to, like `https://github.com/byron/gitoxide`.
        ///
        /// Optional at the Clap level so the dispatch arm can emit git's
        /// `usage_msg_opt("You must specify a repository to clone.")` exit-129
        /// contract (see cmd_clone in vendor/git/builtin/clone.c) instead of
        /// Clap's generic missing-required-arg exit-2.
        pub remote: Option<OsString>,

        /// The name of the reference to check out.
        #[clap(long = "ref", value_parser = crate::shared::AsPartialRefName, value_name = "REF_NAME")]
        pub ref_name: Option<gix::refs::PartialName>,

        /// Point the newly created HEAD at <name> (a branch or tag) on the
        /// remote, instead of the remote's own HEAD.
        ///
        /// Mapped onto `--ref` at dispatch time when `--ref` is not set,
        /// so gix's existing PartialName-driven HEAD resolution fires
        /// against `<name>`. Mirrors git's `-b/--branch=<name>` in
        /// cmd_clone.
        #[clap(long, short = 'b', value_name = "NAME")]
        pub branch: Option<String>,

        /// The directory to initialize with the new repository and to which all data should be written.
        pub directory: Option<PathBuf>,

        /// Overflow positionals beyond `<repo> [<dir>]`.
        ///
        /// Captured here so the dispatch arm can mirror cmd_clone's
        /// `usage_msg_opt("Too many arguments.")` exit-129 contract rather
        /// than Clap's generic unexpected-positional exit-2.
        #[clap(trailing_var_arg = true, hide = true)]
        pub extra_positionals: Vec<OsString>,
    }

    #[derive(Debug, clap::Parser)]
    pub struct ShallowOptions {
        /// Create a shallow clone with the history truncated to the given number of commits.
        #[clap(long, help_heading = Some("SHALLOW"), conflicts_with_all = ["shallow_since", "shallow_exclude"])]
        pub depth: Option<NonZeroU32>,

        /// Cutoff all history past the given date. Can be combined with shallow-exclude.
        #[clap(long, help_heading = Some("SHALLOW"), value_parser = crate::shared::AsTime, value_name = "DATE")]
        pub shallow_since: Option<gix::date::Time>,

        /// Cutoff all history past the tag-name or ref-name. Can be combined with shallow-since.
        #[clap(long, help_heading = Some("SHALLOW"), value_parser = crate::shared::AsPartialRefName, value_name = "REF_NAME")]
        pub shallow_exclude: Vec<gix::refs::PartialName>,
    }

    impl From<ShallowOptions> for Shallow {
        fn from(opts: ShallowOptions) -> Self {
            if let Some(depth) = opts.depth {
                Shallow::DepthAtRemote(depth)
            } else if !opts.shallow_exclude.is_empty() {
                Shallow::Exclude {
                    remote_refs: opts.shallow_exclude,
                    since_cutoff: opts.shallow_since,
                }
            } else if let Some(cutoff) = opts.shallow_since {
                Shallow::Since { cutoff }
            } else {
                Shallow::default()
            }
        }
    }
}

#[cfg(any(feature = "gitoxide-core-async-client", feature = "gitoxide-core-blocking-client"))]
pub mod remote {
    #[derive(Debug, clap::Parser)]
    pub struct Platform {
        /// The name of the remote to connect to, or the URL of the remote to connect to directly.
        ///
        /// If unset, the current branch will determine the remote.
        #[clap(long, short = 'n')]
        pub name: Option<String>,

        /// Output additional typically information provided by the server as part of the connection handshake.
        #[clap(long, short = 'H')]
        pub handshake_info: bool,

        /// Subcommands
        #[clap(subcommand)]
        pub cmd: Subcommands,
    }

    #[derive(Debug, clap::Subcommand)]
    #[clap(visible_alias = "remotes")]
    pub enum Subcommands {
        /// Print all references available on the remote.
        Refs,
        /// Print all references available on the remote as filtered through ref-specs.
        RefMap {
            /// Also display remote references that were sent by the server, but filtered by the refspec locally.
            #[clap(long, short = 'u')]
            show_unmapped_remote_refs: bool,
            /// Override the built-in and configured ref-specs with one or more of the given ones.
            #[clap(value_parser = crate::shared::AsBString)]
            ref_spec: Vec<gix::bstr::BString>,
        },
    }
}

pub mod mailmap {
    use gix::bstr::BString;

    #[derive(Debug, clap::Subcommand)]
    pub enum Subcommands {
        /// Print all entries in configured mailmaps, inform about errors as well.
        Entries,
        /// Print the canonical form of contacts according to the configured mailmaps.
        Check {
            /// One or more `Name <email>` or `<email>` to pass through the mailmap.
            contacts: Vec<BString>,
        },
    }
}

#[cfg(feature = "gitoxide-core-tools-clean")]
pub mod clean {
    use gix::bstr::BString;

    use crate::shared::CheckPathSpec;

    #[derive(Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
    pub enum FindRepository {
        All,
        #[default]
        NonBare,
    }

    impl From<FindRepository> for gitoxide_core::repository::clean::FindRepository {
        fn from(value: FindRepository) -> Self {
            match value {
                FindRepository::All => gitoxide_core::repository::clean::FindRepository::All,
                FindRepository::NonBare => gitoxide_core::repository::clean::FindRepository::NonBare,
            }
        }
    }

    #[derive(Debug, clap::Parser)]
    pub struct Command {
        /// Print additional debug information to help understand decisions it made.
        #[arg(long)]
        pub debug: bool,
        /// A dummy to easy with muscle-memory. This flag is assumed if provided or not, and has no effect.
        #[arg(short = 'n', long)]
        pub dry_run: bool,
        /// Actually perform the operation, which deletes files on disk without chance of recovery.
        #[arg(long, short = 'e')]
        pub execute: bool,
        /// Remove ignored (and expendable) files.
        #[arg(long, short = 'x')]
        pub ignored: bool,
        /// Remove precious files.
        #[arg(long, short = 'p')]
        pub precious: bool,
        /// Remove whole directories.
        #[arg(long, short = 'd')]
        pub directories: bool,
        /// Remove nested repositories, even outside ignored directories.
        #[arg(long, short = 'r')]
        pub repositories: bool,
        /// Pathspec patterns are used to match the result of the dirwalk, not the dirwalk itself.
        ///
        /// Use this if there is trouble using wildcard pathspecs, which affect the directory walk
        /// in reasonable, but often unexpected ways.
        #[arg(long, short = 'm')]
        pub pathspec_matches_result: bool,
        /// Enter ignored directories to skip repositories contained within.
        ///
        /// This identifies and avoids deleting separate repositories that are nested inside
        /// ignored directories eligible for removal.
        #[arg(long)]
        pub skip_hidden_repositories: Option<FindRepository>,
        /// What kind of repositories to find inside of untracked directories.
        #[arg(long, default_value = "non-bare")]
        pub find_untracked_repositories: FindRepository,
        /// The git path specifications to list attributes for, or unset to read from stdin one per line.
        #[clap(value_parser = CheckPathSpec)]
        pub pathspec: Vec<BString>,
    }
}

pub mod odb {
    #[derive(Debug, clap::Subcommand)]
    pub enum Subcommands {
        /// Print all object names.
        Entries,
        /// Provide general information about the object database.
        Info,
        /// Count and obtain information on all, possibly duplicate, objects in the database.
        #[clap(visible_alias = "statistics")]
        Stats {
            /// Lookup headers again, but without preloading indices.
            #[clap(long)]
            extra_header_lookup: bool,
        },
    }
}

pub mod fsck {
    #[derive(Debug, clap::Parser)]
    pub struct Platform {
        /// A revspec to start the connectivity check from.
        pub spec: Option<String>,
    }
}

pub mod tree {
    #[derive(Debug, clap::Subcommand)]
    pub enum Subcommands {
        /// Print entries in a given tree
        Entries {
            /// Traverse the entire tree and its subtrees respectively, not only this tree.
            #[clap(long, short = 'r')]
            recursive: bool,

            /// Provide files size as well. This is expensive as the object is decoded entirely.
            #[clap(long, short = 'e')]
            extended: bool,

            /// The revspec of the tree to traverse, or the tree at `HEAD` if unspecified.
            treeish: Option<String>,
        },
        /// Provide information about a tree.
        Info {
            /// Provide files size as well. This is expensive as the object is decoded entirely.
            #[clap(long, short = 'e')]
            extended: bool,
            /// The revspec of the tree to traverse, or the tree at `HEAD` if unspecified.
            treeish: Option<String>,
        },
    }
}

pub mod commit {
    #[derive(Debug, clap::Subcommand)]
    pub enum Subcommands {
        /// Verify the signature of a commit.
        Verify {
            /// A specification of the revision to verify, or the current `HEAD` if unset.
            rev_spec: Option<String>,
        },
        /// Sign a commit and print the signed commit's id to stdout.
        ///
        /// This command does not change symbolic refs.
        Sign {
            /// A specification of the revision to sign, or the current `HEAD` if unset.
            rev_spec: Option<String>,
        },
        /// Describe the current commit or the given one using the name of the closest annotated tag in its ancestry.
        Describe {
            /// Use annotated tag references only, not all tags.
            #[clap(long, short = 't', conflicts_with("all_refs"))]
            annotated_tags: bool,

            /// Use all references under the `ref/` namespaces, which includes tag references, local and remote branches.
            #[clap(long, short = 'a', conflicts_with("annotated_tags"))]
            all_refs: bool,

            /// Only follow the first parent when traversing the commit graph.
            #[clap(long, short = 'f')]
            first_parent: bool,

            /// Always display the long format, even if that would not be necessary as the id is located directly on a reference.
            #[clap(long, short = 'l')]
            long: bool,

            /// Consider only the given `n` candidates. This can take longer, but potentially produces more accurate results.
            #[clap(long, short = 'c', default_value = "10")]
            max_candidates: usize,

            /// Print information on stderr to inform about performance statistics
            #[clap(long, short = 's')]
            statistics: bool,

            #[clap(long)]
            /// If there was no way to describe the commit, fallback to using the abbreviated input revision.
            always: bool,

            /// Set the suffix to append if the repository is dirty (not counting untracked files).
            #[clap(short = 'd', long)]
            dirty_suffix: Option<Option<String>>,

            /// A specification of the revision to use, or the current `HEAD` if unset.
            rev_spec: Option<String>,
        },
    }
}

pub mod tag {
    #[derive(Debug, clap::Parser)]
    pub struct Platform {
        /// List all tags (git-compat alias). Listing is also the default
        /// behavior when no create / delete / verify mode is given, so
        /// this flag is observable only as a mode indicator. Maps to
        /// git's `OPT_CMDMODE('l', "list", ..., 'l')` in
        /// vendor/git/builtin/tag.c.
        #[clap(short = 'l', long = "list")]
        pub list: bool,

        /// Delete existing tags with the given names. Each name
        /// becomes a `refs/tags/<name>` deletion; missing names print
        /// `error: tag '<name>' not found.` and contribute to a final
        /// non-zero exit. Mirrors `OPT_CMDMODE('d', "delete", ...)`.
        #[clap(short = 'd', long = "delete", conflicts_with_all = ["list", "verify"])]
        pub delete: bool,

        /// Verify the signature of the given annotated tags. Mirrors
        /// `OPT_CMDMODE('v', "verify", ...)`.
        #[clap(short = 'v', long = "verify", conflicts_with_all = ["list", "delete"])]
        pub verify: bool,

        /// Replace an existing tag with the given name instead of
        /// failing. Mirrors `OPT__FORCE`.
        #[clap(short = 'f', long = "force")]
        pub force: bool,

        /// Make an unsigned, annotated tag object.
        /// Mirrors `OPT_BOOL('a', "annotate", ...)`.
        #[clap(short = 'a', long = "annotate")]
        pub annotate: bool,

        /// Tag message. When multiple `-m` options are given, values
        /// are concatenated as separate paragraphs.
        /// Mirrors `OPT_CALLBACK_F('m', "message", ..., parse_msg_arg)`.
        #[clap(short = 'm', long = "message", value_name = "msg", action = clap::ArgAction::Append)]
        pub message: Vec<String>,

        /// Read message from file. Use `-` for stdin.
        /// Mirrors `OPT_FILENAME('F', "file", ...)`.
        #[clap(short = 'F', long = "file", value_name = "file")]
        pub file: Option<std::path::PathBuf>,

        /// Force edit of tag message. Mirrors `OPT_BOOL('e', "edit", ...)`.
        #[clap(short = 'e', long = "edit")]
        pub edit: bool,

        /// Append a `<token>[=<value>]` trailer to the tag message.
        /// Multiple uses accumulate. Mirrors `OPT_STRVEC(0, "trailer", ...)`.
        #[clap(long = "trailer", value_name = "tok[=val]", action = clap::ArgAction::Append)]
        pub trailer: Vec<String>,

        /// Message cleanup mode: `verbatim` / `whitespace` / `strip`
        /// (default `strip`). Mirrors `OPT_CLEANUP`.
        #[clap(long = "cleanup", value_name = "mode")]
        pub cleanup: Option<String>,

        /// Create a reflog for the tag. Mirrors
        /// `OPT_BOOL(0, "create-reflog", ...)`.
        #[clap(long = "create-reflog", overrides_with = "no_create_reflog")]
        pub create_reflog: bool,

        /// Override an earlier `--create-reflog`. Mirrors git's
        /// negation pair.
        #[clap(long = "no-create-reflog", overrides_with = "create_reflog")]
        pub no_create_reflog: bool,

        /// Create a GPG-signed annotated tag. Mirrors
        /// `OPT_BOOL('s', "sign", ...)`.
        #[clap(short = 's', long = "sign", conflicts_with = "no_sign")]
        pub sign: bool,

        /// Override `tag.gpgSign` = true. Mirrors git's `--no-sign`.
        #[clap(long = "no-sign", conflicts_with = "sign")]
        pub no_sign: bool,

        /// Sign using the given key id. Mirrors
        /// `OPT_STRING('u', "local-user", ..., "key-id")`.
        #[clap(short = 'u', long = "local-user", value_name = "key-id")]
        pub local_user: Option<String>,

        /// Sorting and filtering tags are case insensitive. Maps to
        /// git's `OPT_BOOL('i', "ignore-case", ...)`.
        #[clap(short = 'i', long)]
        pub ignore_case: bool,

        /// Display tag listing in columns. Clap wires the flag; multi-
        /// column packing itself is a follow-up (effect-mode parity
        /// confirms only exit-code match). Mirrors `OPT_COLUMN`.
        #[clap(long, value_name = "options", num_args = 0..=1, default_missing_value = "always", conflicts_with = "no_column")]
        pub column: Option<String>,

        /// Equivalent to `--column=never` — one tag per line.
        #[clap(long, conflicts_with = "column")]
        pub no_column: bool,

        /// Respect colors in `--format`. Clap wires the flag; without
        /// a `%(color:*)` atom in the format string this is a no-op,
        /// so effect-mode parity suffices. Mirrors `OPT__COLOR`.
        #[clap(long, value_name = "when", num_args = 0..=1, default_missing_value = "always")]
        pub color: Option<String>,

        /// Sort by for-each-ref key; prefix `-` for descending. Multiple
        /// uses make later keys take precedence. Clap-wired here; the
        /// sort key interpreter is a follow-up — current output uses
        /// the default alphabetical refname order, which matches git's
        /// behavior when `--sort=refname` is given explicitly.
        /// Mirrors `OPT_REF_SORT`.
        #[clap(long, value_name = "key")]
        pub sort: Vec<String>,

        /// Format string interpolating `%(<atom>)` fields. Defaults to
        /// `%(refname:strip=2)` (plain tag name). Mirrors
        /// `OPT_STRING("format", ...)`; supports a narrow subset of
        /// for-each-ref atoms — see `tag::list` implementation.
        #[clap(long = "format", value_name = "format")]
        pub format_string: Option<String>,

        /// Do not output a newline after formatted refs where the
        /// format expands to the empty string. Mirrors
        /// `OPT_BOOL(0, "omit-empty", ...)`.
        #[clap(long)]
        pub omit_empty: bool,

        /// Print the first `<num>` lines of each tag's annotation (or
        /// commit message for lightweight tags). Implies list mode.
        /// Mirrors git's integer flag `-n[<num>]`.
        #[clap(short = 'n', num_args = 0..=1, default_missing_value = "1", value_name = "num")]
        pub annotation_lines: Option<usize>,

        /// Only list tags of `<object>` (HEAD if omitted). Implies list
        /// mode. Mirrors git's `--points-at` with `PARSE_OPT_LASTARG_DEFAULT`
        /// + `defval = "HEAD"`.
        #[clap(long, value_name = "object", num_args = 0..=1, default_missing_value = "HEAD")]
        pub points_at: Option<std::ffi::OsString>,

        /// Only list tags that contain `<commit>` in their ancestry
        /// (HEAD if omitted). Mirrors `OPT_CONTAINS`.
        #[clap(long, value_name = "commit", num_args = 0..=1, default_missing_value = "HEAD")]
        pub contains: Option<std::ffi::OsString>,

        /// Only list tags that do NOT contain `<commit>` in their
        /// ancestry (HEAD if omitted). Mirrors `OPT_NO_CONTAINS`.
        #[clap(long, value_name = "commit", num_args = 0..=1, default_missing_value = "HEAD")]
        pub no_contains: Option<std::ffi::OsString>,

        /// Only list tags whose tagged commit is reachable from `<commit>`
        /// (HEAD if omitted). Mirrors `OPT_MERGED`.
        #[clap(long, value_name = "commit", num_args = 0..=1, default_missing_value = "HEAD")]
        pub merged: Option<std::ffi::OsString>,

        /// Only list tags whose tagged commit is NOT reachable from
        /// `<commit>` (HEAD if omitted). Mirrors `OPT_NO_MERGED`.
        #[clap(long, value_name = "commit", num_args = 0..=1, default_missing_value = "HEAD")]
        pub no_merged: Option<std::ffi::OsString>,

        /// Shell glob patterns to filter listed tags (fnmatch(3), OR'd).
        /// Only meaningful in list mode. Matches git-tag(1)'s
        /// `[<pattern>...]` positional after `-l`.
        pub patterns: Vec<std::ffi::OsString>,
    }
}

pub mod credential {
    #[derive(Debug, clap::Subcommand)]
    pub enum Subcommands {
        /// Get the credentials fed for `url=<url>` via STDIN.
        #[clap(visible_alias = "get")]
        Fill,
        /// Approve the information piped via STDIN as obtained with last call to `fill`
        #[clap(visible_alias = "store")]
        Approve,
        /// Try to resolve the given revspec and print the object names.
        #[clap(visible_alias = "erase")]
        Reject,
    }
}

///
pub mod commitgraph {
    #[derive(Debug, clap::Subcommand)]
    pub enum Subcommands {
        /// Verify the integrity of a commit graph file.
        Verify {
            /// output statistical information about the graph.
            #[clap(long, short = 's')]
            statistics: bool,
        },
        /// List all entries in the commit-graph file as reachable by starting from `HEAD`.
        List {
            /// Display long hashes, instead of expensively shortened versions for best performance.
            #[clap(long, short = 'l')]
            long_hashes: bool,
            /// The rev-spec to list reachable commits from.
            #[clap(default_value = "@")]
            spec: std::ffi::OsString,
        },
    }
}

pub mod revision {
    pub mod resolve {
        #[derive(Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
        pub enum TreeMode {
            /// Show the raw bytes - only useful for piping into files for use with tooling.
            Raw,
            /// Display a tree in human-readable form.
            #[default]
            Pretty,
        }

        #[derive(Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
        pub enum BlobFormat {
            /// The version stored in the Git Object Database.
            #[default]
            Git,
            /// The version that would be checked out into the worktree, including filters.
            Worktree,
            /// The version that would be diffed (Worktree + Text-Conversion).
            Diff,
            /// The version that would be diffed if there is a text-conversion, or the one stored in Git otherwise.
            DiffOrGit,
        }
    }

    #[derive(Debug, clap::Subcommand)]
    #[clap(visible_alias = "rev", visible_alias = "r")]
    pub enum Subcommands {
        /// List all commits reachable from the given rev-spec.
        #[clap(visible_alias = "l")]
        List {
            /// Display long hashes, instead of expensively shortened versions for best performance.
            #[clap(long, short = 'l')]
            long_hashes: bool,
            /// How many commits to list at most.
            #[clap(long)]
            limit: Option<usize>,
            /// Write the graph as SVG file to the given path.
            #[clap(long, short = 's')]
            svg: Option<std::path::PathBuf>,
            /// The rev-spec to list reachable commits from.
            #[clap(default_value = "@")]
            spec: std::ffi::OsString,
        },
        /// Provide the revision specification like `@~1` to explain.
        #[clap(visible_alias = "e")]
        Explain { spec: std::ffi::OsString },
        /// Try to resolve the given revspec and print the object names.
        #[clap(visible_alias = "query", visible_alias = "parse", visible_alias = "p")]
        Resolve {
            /// Instead of resolving a rev-spec, explain what would be done for the first spec.
            ///
            /// Equivalent to the `explain` subcommand.
            #[clap(short = 'e', long)]
            explain: bool,
            /// Also show the name of the reference which led to the object.
            #[clap(short = 'r', long, conflicts_with = "explain")]
            reference: bool,
            /// Show the first resulting object similar to how `git cat-file` would, but don't show the resolved spec.
            #[clap(short = 'c', long, conflicts_with = "explain")]
            cat_file: bool,
            /// How to display blobs.
            #[clap(short = 'b', long, default_value = "git")]
            blob_format: resolve::BlobFormat,
            /// How to display trees as obtained with `@:dirname` or `@^{tree}`.
            #[clap(short = 't', long, default_value = "pretty")]
            tree_mode: resolve::TreeMode,
            /// rev-specs like `@`, `@~1` or `HEAD^2`.
            #[clap(required = true)]
            specs: Vec<std::ffi::OsString>,
        },
        /// Return the names and hashes of all previously checked-out branches.
        #[clap(visible_alias = "prev")]
        PreviousBranches,
    }
}

pub mod attributes {
    use gix::bstr::BString;

    use crate::shared::CheckPathSpec;

    #[derive(Debug, clap::Subcommand)]
    pub enum Subcommands {
        /// Run `git check-attr` and `git check-ignore` on all files of the index or all files
        /// passed via stdin and validate that we get the same outcome when computing attributes.
        ValidateBaseline {
            /// Print various statistics to stderr.
            #[clap(long, short = 's')]
            statistics: bool,
            /// Don't validated excludes as obtaining them with `check-ignore` can be very slow.
            #[clap(long)]
            no_ignore: bool,
        },
        /// List all attributes of the given path-specs and display the result similar to `git check-attr`.
        Query {
            /// Print various statistics to stderr.
            #[clap(long, short = 's')]
            statistics: bool,
            /// The Git path specifications to list attributes for, or unset to read from stdin one per line.
            #[clap(value_parser = CheckPathSpec)]
            pathspec: Vec<BString>,
        },
    }
}

pub mod exclude {
    use std::ffi::OsString;

    use gix::bstr::BString;

    use crate::shared::CheckPathSpec;

    #[derive(Debug, clap::Subcommand)]
    pub enum Subcommands {
        /// Check if path-specs are excluded and print the result similar to `git check-ignore`.
        Query {
            /// Print various statistics to stderr.
            #[clap(long, short = 's')]
            statistics: bool,
            /// Show actual ignore patterns instead of un-excluding an entry.
            ///
            /// That way one can understand why an entry might not be excluded.
            #[clap(long, short = 'i')]
            show_ignore_patterns: bool,
            /// Additional patterns to use for exclusions. They have the highest priority.
            ///
            /// Useful for undoing previous patterns using the '!' prefix.
            #[clap(long, short = 'p')]
            patterns: Vec<OsString>,
            /// The git path specifications to check for exclusion, or unset to read from stdin one per line.
            #[clap(value_parser = CheckPathSpec)]
            pathspec: Vec<BString>,
        },
    }
}

pub mod index {
    use std::path::PathBuf;

    use gix::bstr::BString;

    use crate::shared::CheckPathSpec;

    pub mod entries {
        #[derive(Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
        pub enum Format {
            /// Show only minimal information, useful for first glances.
            #[default]
            Simple,
            /// Show much more information that is still human-readable.
            Rich,
        }
    }

    #[derive(Debug, clap::Subcommand)]
    pub enum Subcommands {
        /// Print all entries to standard output.
        Entries {
            /// How to output index entries.
            #[clap(long, short = 'f', default_value = "simple", value_enum)]
            format: entries::Format,
            /// Do not visualize excluded entries or attributes per path.
            #[clap(long)]
            no_attributes: bool,
            /// Load attribute and ignore files from the index, don't look at the worktree.
            ///
            /// This is to see what IO for probing attribute/ignore files does to performance.
            #[clap(long, short = 'i', conflicts_with = "no_attributes")]
            attributes_from_index: bool,
            /// Display submodule entries as well if their repository exists.
            #[clap(long, short = 'r')]
            recurse_submodules: bool,
            /// Print various statistics to stderr.
            #[clap(long, short = 's')]
            statistics: bool,
            /// The git path specifications to match entries to print.
            #[clap(value_parser = CheckPathSpec)]
            pathspec: Vec<BString>,
        },
        /// Create an index from a tree-ish.
        #[clap(visible_alias = "read-tree")]
        FromTree {
            /// Overwrite the specified index file if it already exists.
            #[clap(long, short = 'f')]
            force: bool,
            /// Path to the index file to be written.
            ///
            /// If none is given it will be kept in memory only as a way to measure performance.
            /// One day we will probably write the index back by default, but that requires us to
            /// write more of the index to work.
            #[clap(long, short = 'i')]
            index_output_path: Option<PathBuf>,
            /// Don't write the trailing hash for a performance gain.
            #[clap(long, short = 's')]
            skip_hash: bool,
            /// A revspec that points to the to generate the index from.
            spec: std::ffi::OsString,
        },
    }
}

pub mod submodule {
    #[derive(Debug, clap::Parser)]
    pub struct Platform {
        #[clap(subcommand)]
        pub cmds: Option<Subcommands>,
    }

    #[derive(Debug, clap::Subcommand)]
    pub enum Subcommands {
        /// Print all direct submodules to standard output.
        List {
            /// Set the suffix to append if the repository is dirty (not counting untracked files).
            #[clap(short = 'd', long)]
            dirty_suffix: Option<Option<String>>,
        },
    }
}

///
pub mod free;

#[cfg(feature = "gitoxide-core-blocking-client")]
pub mod push;

#[cfg(feature = "gitoxide-core-blocking-client")]
pub mod fetch;
