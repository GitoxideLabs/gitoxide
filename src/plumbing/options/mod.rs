use std::path::PathBuf;

use clap_complete::Shell;
use gitoxide_core as core;
use gix::bstr::BString;

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

    #[clap(long, short = 't')]
    /// The amount of threads to use for some operations.
    ///
    /// If unset, or the value is 0, there is no limit and all logical cores can be used.
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

    /// Bring up a terminal user interface displaying progress visually
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
    /// Subcommands for creating worktree archives
    #[cfg(feature = "gitoxide-core-tools-archive")]
    Archive(archive::Platform),
    #[cfg(feature = "gitoxide-core-tools-clean")]
    Clean(clean::Command),
    /// Subcommands for interacting with commit-graph files
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
    /// Fetch data from remotes and store it in the repository
    #[cfg(feature = "gitoxide-core-blocking-client")]
    Fetch(fetch::Platform),
    #[cfg(feature = "gitoxide-core-blocking-client")]
    Clone(clone::Platform),
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
    #[clap(subcommand)]
    Index(index::Subcommands),
    /// Interact with submodules.
    #[clap(alias = "submodules")]
    Submodule(submodule::Platform),
    /// Show whatever object is at the given spec.
    Cat {
        /// The object to print to stdout.
        revspec: String,
    },
    IsClean,
    IsChanged,
    /// Show which git configuration values are used or planned.
    ConfigTree,
    Status(status::Platform),
    Config(config::Platform),
    #[cfg(feature = "gitoxide-core-tools-corpus")]
    Corpus(corpus::Platform),
    MergeBase(merge_base::Command),
    Merge(merge::Platform),
    Diff(diff::Platform),
    Worktree(worktree::Platform),
    /// Subcommands that need no git repository to run.
    #[clap(subcommand)]
    Free(free::Subcommands),
    /// Blame lines in a file
    Blame {
        file: std::ffi::OsString,
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

pub mod status {
    use crate::shared::{CheckPathSpec, ParseRenameFraction};
    use gix::bstr::BString;

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
        /// display all ignored files and directories, but collapse them if possible to simplify.
        #[default]
        Collapsed,
        /// Show exact matches. Note that this may show directories if these are a match as well.
        ///
        /// Simplification will not happen in this mode.
        Matching,
        // TODO: figure out how to implement traditional, which right now can't be done as it requires ignored folders
        //       to be fully expanded. This should probably be implemented in `gix_dir` which then simply works by not
        //       allowing to ignore directories, naturally traversing the entire content.
    }

    #[derive(Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
    pub enum Format {
        /// A basic format that is easy to read, and useful for a first glimpse as flat list.
        #[default]
        Simplified,
        /// Output very similar to `git status --porcelain=2`.
        PorcelainV2,
    }

    #[derive(Debug, clap::Parser)]
    #[command(about = "compute repository status similar to `git status`")]
    pub struct Platform {
        /// The way status data is displayed.
        #[clap(long, short = 'f')]
        pub format: Option<Format>,
        /// If enabled, show ignored files and directories.
        #[clap(long)]
        pub ignored: Option<Option<Ignored>>,
        /// Define how to display the submodule status. Defaults to git configuration if unset.
        #[clap(long)]
        pub submodules: Option<Submodules>,
        /// Print additional statistics to help understanding performance.
        #[clap(long, short = 's')]
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
        /// List all worktrees, along with some accompanying information
        List,
    }
}

#[cfg(feature = "gitoxide-core-tools-corpus")]
pub mod corpus {
    use std::path::PathBuf;

    #[derive(Debug, clap::Parser)]
    #[command(about = "run algorithms on a corpus of git repositories and store their results for later analysis")]
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

    #[derive(Debug, clap::Parser)]
    #[command(about = "perform merges of various kinds")]
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
            #[clap(value_name = "OURS", value_parser = crate::shared::AsBString)]
            theirs: BString,
        },
    }
}

pub mod diff {
    use gix::bstr::BString;

    /// Print all changes between two objects
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
    }
}

pub mod config {
    use gix::bstr::BString;

    /// Print all entries in a configuration file or access other sub-commands
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

#[cfg(feature = "gitoxide-core-blocking-client")]
pub mod fetch {
    use std::num::NonZeroU32;

    use gix::remote::fetch::Shallow;

    #[derive(Debug, clap::Parser)]
    pub struct Platform {
        /// Don't change the local repository, but otherwise try to be as accurate as possible.
        #[clap(long, short = 'n')]
        pub dry_run: bool,

        /// Output additional typically information provided by the server as part of the connection handshake.
        #[clap(long, short = 'H')]
        pub handshake_info: bool,

        /// Print statistics about negotiation phase.
        #[clap(long, short = 's')]
        pub negotiation_info: bool,

        /// Open the commit graph used for negotiation and write an SVG file to PATH.
        #[clap(long, value_name = "PATH", short = 'g')]
        pub open_negotiation_graph: Option<std::path::PathBuf>,

        #[clap(flatten)]
        pub shallow: ShallowOptions,

        /// The name of the remote to connect to, or the url of the remote to connect to directly.
        ///
        /// If unset, the current branch will determine the remote.
        #[clap(long, short = 'r')]
        pub remote: Option<String>,

        /// Override the built-in and configured ref-specs with one or more of the given ones.
        #[clap(value_parser = crate::shared::AsBString)]
        pub ref_spec: Vec<gix::bstr::BString>,
    }

    #[derive(Debug, clap::Parser)]
    pub struct ShallowOptions {
        /// Fetch with the history truncated to the given number of commits as seen from the remote.
        #[clap(long, help_heading = Some("SHALLOW"), conflicts_with_all = ["shallow_since", "shallow_exclude", "deepen", "unshallow"])]
        pub depth: Option<NonZeroU32>,

        /// Extend the current shallow boundary by the given amount of commits, with 0 meaning no change.
        #[clap(long, help_heading = Some("SHALLOW"), value_name = "DEPTH", conflicts_with_all = ["depth", "shallow_since", "shallow_exclude", "unshallow"])]
        pub deepen: Option<u32>,

        /// Cutoff all history past the given date. Can be combined with shallow-exclude.
        #[clap(long, help_heading = Some("SHALLOW"), value_parser = crate::shared::AsTime, value_name = "DATE", conflicts_with_all = ["depth", "deepen", "unshallow"])]
        pub shallow_since: Option<gix::date::Time>,

        /// Cutoff all history past the tag-name or ref-name. Can be combined with shallow-since.
        #[clap(long, help_heading = Some("SHALLOW"), value_parser = crate::shared::AsPartialRefName, value_name = "REF_NAME", conflicts_with_all = ["depth", "deepen", "unshallow"])]
        pub shallow_exclude: Vec<gix::refs::PartialName>,

        /// Remove the shallow boundary and fetch the entire history available on the remote.
        #[clap(long, help_heading = Some("SHALLOW"), conflicts_with_all = ["shallow_since", "shallow_exclude", "depth", "deepen"])]
        pub unshallow: bool,
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
            } else if let Some(depth) = opts.deepen {
                Shallow::Deepen(depth)
            } else if opts.unshallow {
                Shallow::undo()
            } else {
                Shallow::default()
            }
        }
    }
}

#[cfg(feature = "gitoxide-core-blocking-client")]
pub mod clone {
    use std::{ffi::OsString, num::NonZeroU32, path::PathBuf};

    use gix::remote::fetch::Shallow;

    #[derive(Debug, clap::Parser)]
    pub struct Platform {
        /// Output additional typically information provided by the server as part of the connection handshake.
        #[clap(long, short = 'H')]
        pub handshake_info: bool,

        /// The clone will be bare and a working tree checkout won't be available.
        #[clap(long)]
        pub bare: bool,

        /// Do not clone any tags. Useful to reduce the size of the clone if only branches are needed.
        #[clap(long)]
        pub no_tags: bool,

        #[clap(flatten)]
        pub shallow: ShallowOptions,

        /// The url of the remote to connect to, like `https://github.com/byron/gitoxide`.
        pub remote: OsString,

        /// The name of the reference to check out.
        #[clap(long = "ref", value_parser = crate::shared::AsPartialRefName, value_name = "REF_NAME")]
        pub ref_name: Option<gix::refs::PartialName>,

        /// The directory to initialize with the new repository and to which all data should be written.
        pub directory: Option<PathBuf>,
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
    use crate::shared::CheckPathSpec;
    use gix::bstr::BString;

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
        /// Verify the integrity of a commit graph file
        Verify {
            /// output statistical information about the graph.
            #[clap(long, short = 's')]
            statistics: bool,
        },
        /// List all entries in the commit-graph file as reachable by starting from `HEAD`.
        List {
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
            /// The version that would be diffed (Worktree + Text-Conversion)
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
            /// How many commits to list at most.
            #[clap(long, short = 'l')]
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
    use crate::shared::CheckPathSpec;
    use gix::bstr::BString;

    #[derive(Debug, clap::Subcommand)]
    pub enum Subcommands {
        /// Run `git check-attr`  and `git check-ignore` on all files of the index or all files passed via stdin and validate that
        /// we get the same outcome when computing attributes.
        ValidateBaseline {
            /// Print various statistics to stderr
            #[clap(long, short = 's')]
            statistics: bool,
            /// Don't validated excludes as obtaining them with `check-ignore` can be very slow.
            #[clap(long)]
            no_ignore: bool,
        },
        /// List all attributes of the given path-specs and display the result similar to `git check-attr`.
        Query {
            /// Print various statistics to stderr
            #[clap(long, short = 's')]
            statistics: bool,
            /// The git path specifications to list attributes for, or unset to read from stdin one per line.
            #[clap(value_parser = CheckPathSpec)]
            pathspec: Vec<BString>,
        },
    }
}

pub mod exclude {
    use std::ffi::OsString;

    use crate::shared::CheckPathSpec;
    use gix::bstr::BString;

    #[derive(Debug, clap::Subcommand)]
    pub enum Subcommands {
        /// Check if path-specs are excluded and print the result similar to `git check-ignore`.
        Query {
            /// Print various statistics to stderr
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

    use crate::shared::CheckPathSpec;
    use gix::bstr::BString;

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
        /// Print all entries to standard output
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
            /// Print various statistics to stderr
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
            /// If none is given it will be kept in memory only as a way to measure performance. One day we will probably write the index
            /// back by default, but that requires us to write more of the index to work.
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
        /// Print all direct submodules to standard output
        List {
            /// Set the suffix to append if the repository is dirty (not counting untracked files).
            #[clap(short = 'd', long)]
            dirty_suffix: Option<Option<String>>,
        },
    }
}

///
pub mod free;
