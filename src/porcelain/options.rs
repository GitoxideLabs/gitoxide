use std::path::PathBuf;

use clap_complete::Shell;

#[derive(Debug, clap::Parser)]
#[clap(name = "ein", about = "The rusty git", version = option_env!("GIX_VERSION"))]
#[clap(subcommand_required = true)]
pub struct Args {
    /// Do not display verbose messages and progress information.
    #[clap(long, short = 'q')]
    pub quiet: bool,

    /// Bring up a terminal user interface displaying progress visually.
    #[clap(long, conflicts_with("quiet"))]
    pub progress: bool,
    /// The number of threads to use. If unset, use all cores, if 0 use all physical cores.
    #[clap(short = 't', long)]
    pub threads: Option<usize>,

    /// The progress TUI will stay up even though the work is already completed.
    ///
    /// Use this to be able to read progress messages or additional information visible in the TUI log pane.
    #[clap(long, conflicts_with("quiet"), requires("progress"))]
    pub progress_keep_open: bool,

    #[clap(subcommand)]
    pub cmd: Subcommands,
}

#[derive(Debug, clap::Subcommand)]
pub enum Subcommands {
    /// Initialize the repository in the current directory.
    #[clap(visible_alias = "initialize")]
    Init {
        /// The directory in which to initialize a new git repository.
        ///
        /// Defaults to the current working directory.
        directory: Option<PathBuf>,
    },
    /// A selection of useful tools.
    #[cfg(feature = "gitoxide-core-tools")]
    #[clap(subcommand)]
    Tool(ToolCommands),
    /// Generate shell completions to stdout or a directory.
    #[clap(visible_alias = "generate-completions", visible_alias = "shell-completions")]
    Completions {
        /// The shell to generate completions for. Otherwise it's derived from the environment.
        #[clap(long, short)]
        shell: Option<Shell>,
        /// The output directory in case multiple files are generated. If not provided, will write to stdout.
        out_dir: Option<String>,
    },
    /// Panic immediately, to test panic behavior.
    #[cfg(debug_assertions)]
    Panic,
}

#[cfg(feature = "gitoxide-core-tools")]
#[derive(Debug, clap::Subcommand)]
#[clap(subcommand_required = true)]
#[clap(visible_alias = "t")]
pub enum ToolCommands {
    /// Find all repositories in a given directory.
    Find {
        /// If set, print additional information to help understand why the traversal is slow.
        ///
        /// Typically it will encounter too many paths without a git repository, forcing a lot
        /// of additional paths to be searched unnecessarily.
        #[clap(long, short = 'd')]
        debug: bool,
        /// The directory in which to find all git repositories.
        ///
        /// Defaults to the current working directory.
        root: Option<PathBuf>,
    },
    /// Move all repositories found in a directory into a structure matching their clone URLs.
    Organize {
        #[clap(long)]
        /// The operation will be in dry-run mode unless this flag is set.
        execute: bool,

        #[clap(long, short = 'f')]
        /// The directory to use when finding input repositories to move into position.
        ///
        /// Defaults to the current working directory.
        repository_source: Option<PathBuf>,

        #[clap(long, short = 't')]
        /// The directory to which to move repositories found in the repository-source.
        ///
        /// Defaults to the current working directory.
        destination_directory: Option<PathBuf>,
    },
    #[cfg(feature = "gitoxide-core-tools-query")]
    Query(tools::Query),
    EstimateHours(tools::EstimateHours),
}

#[cfg(feature = "gitoxide-core-tools")]
pub mod tools {
    use std::path::PathBuf;

    use gix::bstr::BString;

    #[cfg(feature = "gitoxide-core-tools-query")]
    #[derive(Debug, clap::Parser)]
    #[command(
        about = "a database accelerated engine to extract information and query it",
        visible_alias = "q"
    )]
    pub struct Query {
        /// The total amount of object cache memory in MB. Bigger repos may benefit from more memory.
        ///
        /// 0 disables it.
        #[arg(long, short = 'o', default_value_t = 200)]
        pub object_cache_size_mb: usize,
        /// Find identical copies in the entire tree, not only in the set of modified files.
        ///
        /// This is an expensive option, and typically cuts speed in half.
        #[arg(long, short = 'C')]
        pub find_copies_harder: bool,
        /// Path to the Git repository to generate the database for.
        #[arg(default_value = ".")]
        pub repo_dir: std::path::PathBuf,
        #[clap(subcommand)]
        pub cmd: Option<query::Command>,
    }

    #[cfg(feature = "gitoxide-core-tools-query")]
    pub mod query {
        use crate::shared::AsPathSpec;

        #[derive(Debug, clap::Subcommand)]
        pub enum Command {
            /// Follow a file through the entire history reachable from HEAD.
            #[command(visible_alias = "trace-file")]
            TracePath {
                /// The path to trace through history.
                // This can't be here anymore as we also need to compile without `Send` support.
                #[clap(value_parser = AsPathSpec)]
                path: gix::bstr::BString,
            },
        }
    }

    #[derive(Debug, clap::Parser)]
    #[clap(
        about = "Estimate hours worked based on a commit history",
        long_about = "See https://github.com/kimmobrunfeldt/git-hours#how-it-works for details",
        visible_alias = "h",
        visible_alias = "hours"
    )]
    pub struct EstimateHours {
        /// The directory containing a '.git/' folder.
        #[clap(value_parser = validator::IsRepo)]
        #[clap(default_value = ".")]
        pub working_dir: PathBuf,
        /// The name of the revision as spec, like 'HEAD' or 'main' at which to start iterating the commit graph.
        #[clap(default_value("HEAD"), value_parser = crate::shared::AsBString)]
        pub rev_spec: BString,
        /// Ignore github bots which match the `[bot]` search string.
        #[clap(short = 'b', long)]
        pub no_bots: bool,
        /// Collect additional information about file modifications, additions and deletions (without rename tracking).
        #[clap(short = 'f', long)]
        pub file_stats: bool,
        /// Collect additional information about lines added and deleted (without rename tracking).
        ///
        /// Note that this implies the work to be done for file-stats, so it should be set as well.
        #[clap(short = 'l', long)]
        pub line_stats: bool,
        /// Show personally identifiable information before the summary. Includes names and email addresses.
        #[clap(short = 'p', long)]
        pub show_pii: bool,
        /// Omit unifying identities by name and email which can lead to the same author appear multiple times
        /// due to using different names or email addresses.
        #[clap(short = 'i', long)]
        pub omit_unify_identities: bool,
    }

    mod validator {
        use std::{ffi::OsStr, path::PathBuf};

        use anyhow::Context;

        #[derive(Clone)]
        pub struct IsRepo;

        impl clap::builder::TypedValueParser for IsRepo {
            type Value = PathBuf;

            fn parse_ref(
                &self,
                cmd: &clap::Command,
                _arg: Option<&clap::Arg>,
                value: &OsStr,
            ) -> Result<Self::Value, clap::Error> {
                assure_is_repo(value).map_err(|e| {
                    let mut err = clap::Error::new(clap::error::ErrorKind::InvalidValue).with_cmd(cmd);
                    err.insert(
                        clap::error::ContextKind::InvalidValue,
                        clap::error::ContextValue::String(e.to_string()),
                    );
                    err
                })?;
                Ok(value.into())
            }
        }

        fn assure_is_repo(dir: &OsStr) -> anyhow::Result<()> {
            let git_dir = PathBuf::from(dir).join(".git");
            let p = gix::path::realpath(&git_dir)
                .with_context(|| format!("Could not canonicalize git repository at '{}'", git_dir.display()))?;
            if p.extension().unwrap_or_default() == "git"
                || p.file_name().unwrap_or_default() == ".git"
                || p.join("HEAD").is_file()
            {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Path '{}' needs to be a directory containing '.git/'",
                    p.display()
                ))
            }
        }
    }
}
