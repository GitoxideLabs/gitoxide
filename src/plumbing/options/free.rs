#[derive(Debug, clap::Subcommand)]
#[clap(visible_alias = "no-repo")]
pub enum Subcommands {
    /// Subcommands for interacting with commit-graphs
    #[clap(subcommand)]
    CommitGraph(commitgraph::Subcommands),
    /// Subcommands for interacting with mailmaps
    Mailmap {
        #[clap(flatten)]
        cmd: mailmap::Platform,
    },
    /// Subcommands for interacting with pack files and indices
    #[clap(subcommand)]
    Pack(pack::Subcommands),
    /// Subcommands for interacting with a worktree index, typically at .git/index
    Index(index::Platform),
    /// Show information about repository discovery and when opening a repository at the current path.
    Discover,
    /// Show the trust level implied by the ownership of one or more paths.
    Trust {
        /// The paths whose ownership-derived trust should be shown.
        #[clap(required = true)]
        paths: Vec<std::path::PathBuf>,
    },
}

///
pub mod commitgraph {
    use std::path::PathBuf;

    #[derive(Debug, clap::Subcommand)]
    pub enum Subcommands {
        /// Verify the integrity of a commit graph
        Verify {
            /// The path to '.git/objects/info/', '.git/objects/info/commit-graphs/', or '.git/objects/info/commit-graph' to validate.
            path: PathBuf,
            /// output statistical information about the pack
            #[clap(long, short = 's')]
            statistics: bool,
        },
    }
}

pub mod index {
    use std::path::PathBuf;

    #[derive(Debug, clap::Parser)]
    pub struct Platform {
        /// The object format to assume when reading files that don't inherently know about it, or when writing files.
        #[clap(long, default_value_t = gix::hash::Kind::default(), value_parser = crate::shared::AsHashKind)]
        pub object_hash: gix::hash::Kind,

        /// The path to the index file.
        #[clap(short = 'i', long, default_value = ".git/index")]
        pub index_path: PathBuf,

        /// Subcommands
        #[clap(subcommand)]
        pub cmd: Subcommands,
    }

    #[derive(Debug, clap::Subcommand)]
    pub enum Subcommands {
        /// Create an index from a list of empty files, one per line of the input.
        FromList {
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
            /// The file to read the index entries from, one path per line.
            file: PathBuf,
        },
        /// Validate constraints and assumptions of an index along with its integrity.
        Verify,
        /// Print information about the index structure
        Info {
            /// Do not extract specific extension information to gain only a superficial idea of the index's composition.
            #[clap(long)]
            no_details: bool,
        },
        /// Checkout the index into a directory with exclusive write access, similar to what would happen during clone.
        CheckoutExclusive {
            /// The path to `.git` repository from which objects can be obtained to write the actual files referenced
            /// in the index. Use this measure the impact on extracting objects on overall performance.
            #[clap(long, short = 'r')]
            repository: Option<PathBuf>,
            /// Ignore errors and keep checking out as many files as possible, and report all errors at the end of the operation.
            #[clap(long, short = 'k')]
            keep_going: bool,
            /// Enable to query the object database yet write only empty files. This is useful to measure the overhead of ODB query
            /// compared to writing the bytes to disk.
            #[clap(long, short = 'e', requires = "repository")]
            empty_files: bool,
            /// The directory into which to write all index entries.
            directory: PathBuf,
        },
    }
}

///
pub mod pack {
    use std::{ffi::OsString, path::PathBuf};

    use gitoxide_core as core;

    #[derive(Debug, clap::Subcommand)]
    pub enum Subcommands {
        /// Subcommands for interacting with pack indices (.idx)
        #[clap(subcommand)]
        Index(index::Subcommands),
        /// Subcommands for interacting with multi-pack indices (named "multi-pack-index")
        MultiIndex(multi_index::Platform),
        /// Create a new pack with a set of objects.
        Create {
            #[clap(long, short = 'r')]
            /// the directory containing the '.git' repository from which objects should be read.
            repository: Option<PathBuf>,

            #[clap(long, short = 'e', value_parser = AsObjectExpansion)]
            /// the way objects are expanded. They differ in costs.
            ///
            /// Possible values are "none" and "tree-traversal". Default is "none".
            expansion: Option<core::pack::create::ObjectExpansion>,

            #[clap(long, default_value_t = 3, requires = "nondeterministic_count")]
            /// The amount of threads to use when counting and the `--nondeterminisitc-count` flag is set, defaulting
            /// to the globally configured threads.
            ///
            /// Use it to have different trade-offs between counting performance and cost in terms of CPU, as the scaling
            /// here is everything but linear. The effectiveness of each core seems to be no more than 30%.
            counting_threads: usize,

            #[clap(long)]
            /// if set, the counting phase may be accelerated using multithreading.
            ///
            /// On the flip side, however, one will loose deterministic counting results which affects the
            /// way the resulting pack is structured.
            nondeterministic_count: bool,

            #[clap(long, short = 's')]
            /// If set statistical information will be presented to inform about pack creation details.
            /// It's a form of instrumentation for developers to help improve pack generation.
            statistics: bool,

            #[clap(long)]
            /// The size in megabytes for a cache to speed up pack access for packs with long delta chains.
            /// It is shared among all threads, so 4 threads would use their own cache 1/4th of the size.
            ///
            /// If unset, no cache will be used.
            pack_cache_size_mb: Option<usize>,

            #[clap(long)]
            /// The size in megabytes for a cache to speed up accessing entire objects, bypassing object database access when hit.
            /// It is shared among all threads, so 4 threads would use their own cache 1/4th of the size.
            ///
            /// This cache type is currently only effective when using the 'diff-tree' object expansion.
            ///
            /// If unset, no cache will be used.
            object_cache_size_mb: Option<usize>,

            #[clap(long)]
            /// if set, delta-objects whose base object wouldn't be in the pack will not be recompressed as base object, but instead
            /// refer to its base object using its object id.
            ///
            /// This allows for smaller packs but requires the receiver of the pack to resolve these ids before storing the pack.
            /// Packs produced with this option enabled are only valid in transit, but not at rest.
            thin: bool,

            /// The directory into which to write the pack file.
            #[clap(long, short = 'o')]
            output_directory: Option<PathBuf>,

            /// The tips from which to start the commit graph iteration, either as fully qualified commit hashes
            /// or as branch names.
            ///
            /// If empty, we expect to read objects on stdin and default to 'none' as expansion mode.
            /// Otherwise the expansion mode is 'tree-traversal' by default.
            tips: Vec<OsString>,
        },
        /// Use the gix-protocol to receive a pack, emulating a clone.
        #[cfg(any(feature = "gitoxide-core-async-client", feature = "gitoxide-core-blocking-client"))]
        Receive {
            /// The protocol version to use. Valid values are 1 and 2
            #[clap(long, short = 'p')]
            protocol: Option<core::net::Protocol>,

            /// the directory into which to write references. Existing files will be overwritten.
            ///
            /// Note that the directory will be created if needed.
            #[clap(long, short = 'd')]
            refs_directory: Option<PathBuf>,

            /// The URLs or path from which to receive the pack.
            ///
            /// See here for a list of supported URLs: <https://www.git-scm.com/docs/git-clone#_git_urls>
            url: String,

            /// If set once or more times, these references will be fetched instead of all advertised ones.
            ///
            /// Note that this requires a reasonably modern git server.
            #[clap(long = "reference", short = 'r')]
            refs: Vec<String>,

            /// The directory into which to write the received pack and index.
            ///
            /// If unset, they will be discarded.
            directory: Option<PathBuf>,
        },
        /// Dissolve a pack into its loose objects.
        ///
        /// Note that this effectively removes delta compression for an average compression of 2x, creating one file per object in the process.
        /// Thus this should only be done to dissolve small packs after a fetch.
        Explode {
            #[clap(long)]
            /// Read written objects back and assert they match their source. Fail the operation otherwise.
            ///
            /// Only relevant if an object directory is set.
            verify: bool,

            /// delete the pack and index file after the operation is successful
            #[clap(long)]
            delete_pack: bool,

            /// The amount of checks to run
            #[clap(
                long,
                short = 'c',
                default_value = "all",
                value_parser = AsSafetyCheck
            )]
            check: core::pack::explode::SafetyCheck,

            /// Compress bytes even when using the sink, i.e. no object directory is specified
            ///
            /// This helps to determine overhead related to compression. If unset, the sink will
            /// only create hashes from bytes, which is usually limited by the speed at which input
            /// can be obtained.
            #[clap(long)]
            sink_compress: bool,

            /// The '.pack' or '.idx' file to explode into loose objects
            pack_path: PathBuf,

            /// The path into which all objects should be written. Commonly '.git/objects'
            object_path: Option<PathBuf>,
        },
        /// Verify the integrity of a pack, index or multi-index file
        Verify {
            #[clap(flatten)]
            args: VerifyOptions,

            /// The '.pack', '.idx' or 'multi-pack-index' file to validate.
            path: PathBuf,
        },
    }

    #[derive(Debug, clap::Parser)]
    pub struct VerifyOptions {
        /// output statistical information
        #[clap(long, short = 's')]
        pub statistics: bool,
        /// The algorithm used to verify packs. They differ in costs.
        #[clap(
            long,
            short = 'a',
            default_value = "less-time",
            value_parser = AsAlgorithm
        )]
        pub algorithm: core::pack::verify::Algorithm,

        #[clap(long, conflicts_with("re_encode"))]
        /// Decode and parse tags, commits and trees to validate their correctness beyond hashing correctly.
        ///
        /// Malformed objects should not usually occur, but could be injected on purpose or accident.
        /// This will reduce overall performance.
        pub decode: bool,

        #[clap(long)]
        /// Decode and parse tags, commits and trees to validate their correctness, and re-encode them.
        ///
        /// This flag is primarily to test the implementation of encoding, and requires to decode the object first.
        /// Encoding an object after decoding it should yield exactly the same bytes.
        /// This will reduce overall performance even more, as re-encoding requires to transform zero-copy objects into
        /// owned objects, causing plenty of allocation to occur.
        pub re_encode: bool,
    }

    ///
    pub mod multi_index {
        use std::path::PathBuf;

        #[derive(Debug, clap::Parser)]
        pub struct Platform {
            /// The path to the index file.
            #[clap(short = 'i', long, default_value = ".git/objects/pack/multi-pack-index")]
            pub multi_index_path: PathBuf,

            /// Subcommands
            #[clap(subcommand)]
            pub cmd: Subcommands,
        }

        #[derive(Debug, clap::Subcommand)]
        pub enum Subcommands {
            /// Display all entries of a multi-index as: *oid* *pack-id* *pack-offset*
            Entries,
            /// Print general information about a multi-index file
            Info,
            /// Verify a multi-index quickly without inspecting objects themselves
            Verify,
            /// Create a multi-pack index from one or more pack index files, overwriting possibly existing files.
            Create {
                /// Paths to the pack index files to read (with .idx extension).
                ///
                /// Note for the multi-index to be useful, it should be side-by-side with the supplied `.idx` files.
                #[clap(required = true)]
                index_paths: Vec<PathBuf>,
            },
        }
    }

    ///
    pub mod index {
        use std::path::PathBuf;

        use gitoxide_core as core;

        use super::AsIterationMode;

        #[derive(Debug, clap::Subcommand)]
        pub enum Subcommands {
            /// create a pack index from a pack data file.
            Create {
                /// Specify how to iterate the pack, defaults to 'verify'
                ///
                /// Valid values are
                ///
                ///  **as-is** do not do anything and expect the pack file to be valid as per the trailing hash,
                ///  **verify** the input ourselves and validate that it matches with the hash provided in the pack,
                ///  **restore** hash the input ourselves and ignore failing entries, instead finish the pack with the hash we computed
                ///  to keep as many objects as possible.
                #[clap(
                    long,
                    short = 'i',
                    default_value = "verify",
                    value_parser = AsIterationMode
                )]
                iteration_mode: core::pack::index::IterationMode,

                /// Path to the pack file to read (with .pack extension).
                ///
                /// If unset, the pack file is expected on stdin.
                #[clap(long, short = 'p')]
                pack_path: Option<PathBuf>,

                /// The folder into which to place the pack and the generated index file
                ///
                /// If unset, only informational output will be provided to standard output.
                directory: Option<PathBuf>,
            },
        }
    }

    mod clap_util {
        use std::{ffi::OsStr, str::FromStr};

        use clap::{
            builder::{NonEmptyStringValueParser, PossibleValue, TypedValueParser},
            Arg, Command, Error,
        };

        #[derive(Clone)]
        pub struct AsObjectExpansion;

        impl TypedValueParser for AsObjectExpansion {
            type Value = gitoxide_core::pack::create::ObjectExpansion;

            fn parse_ref(&self, cmd: &Command, arg: Option<&Arg>, value: &OsStr) -> Result<Self::Value, Error> {
                NonEmptyStringValueParser::new()
                    .try_map(|arg| gitoxide_core::pack::create::ObjectExpansion::from_str(&arg))
                    .parse_ref(cmd, arg, value)
            }

            fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
                Some(Box::new(
                    gitoxide_core::pack::create::ObjectExpansion::variants()
                        .iter()
                        .map(PossibleValue::new),
                ))
            }
        }

        #[derive(Clone)]
        pub struct AsSafetyCheck;

        impl TypedValueParser for AsSafetyCheck {
            type Value = gitoxide_core::pack::explode::SafetyCheck;

            fn parse_ref(&self, cmd: &Command, arg: Option<&Arg>, value: &OsStr) -> Result<Self::Value, Error> {
                NonEmptyStringValueParser::new()
                    .try_map(|arg| gitoxide_core::pack::explode::SafetyCheck::from_str(&arg))
                    .parse_ref(cmd, arg, value)
            }

            fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
                Some(Box::new(
                    gitoxide_core::pack::explode::SafetyCheck::variants()
                        .iter()
                        .map(PossibleValue::new),
                ))
            }
        }

        #[derive(Clone)]
        pub struct AsAlgorithm;

        impl TypedValueParser for AsAlgorithm {
            type Value = gitoxide_core::pack::verify::Algorithm;

            fn parse_ref(&self, cmd: &Command, arg: Option<&Arg>, value: &OsStr) -> Result<Self::Value, Error> {
                NonEmptyStringValueParser::new()
                    .try_map(|arg| gitoxide_core::pack::verify::Algorithm::from_str(&arg))
                    .parse_ref(cmd, arg, value)
            }

            fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
                Some(Box::new(
                    gitoxide_core::pack::verify::Algorithm::variants()
                        .iter()
                        .map(PossibleValue::new),
                ))
            }
        }

        #[derive(Clone)]
        pub struct AsIterationMode;

        impl TypedValueParser for AsIterationMode {
            type Value = gitoxide_core::pack::index::IterationMode;

            fn parse_ref(&self, cmd: &Command, arg: Option<&Arg>, value: &OsStr) -> Result<Self::Value, Error> {
                NonEmptyStringValueParser::new()
                    .try_map(|arg| gitoxide_core::pack::index::IterationMode::from_str(&arg))
                    .parse_ref(cmd, arg, value)
            }

            fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
                Some(Box::new(
                    gitoxide_core::pack::index::IterationMode::variants()
                        .iter()
                        .map(PossibleValue::new),
                ))
            }
        }
    }
    use clap_util::{AsAlgorithm, AsIterationMode, AsObjectExpansion, AsSafetyCheck};
}

///
pub mod mailmap {
    use std::path::PathBuf;

    #[derive(Debug, clap::Parser)]
    pub struct Platform {
        /// The path to the mailmap file.
        #[clap(short = 'p', long, default_value = ".mailmap")]
        pub path: PathBuf,

        /// Subcommands
        #[clap(subcommand)]
        pub cmd: Subcommands,
    }

    #[derive(Debug, clap::Subcommand)]
    pub enum Subcommands {
        /// Parse all entries in the mailmap and report malformed lines or collisions.
        Verify,
    }
}
