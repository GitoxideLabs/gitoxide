use clap::builder::{OsStringValueParser, TypedValueParser};
use clap::{Arg, Command, Error};
use std::ffi::OsStr;
use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
#[clap(name = "it", about = "internal tools to help create test cases")]
pub struct Args {
    #[clap(subcommand)]
    pub cmd: Subcommands,
}

#[derive(Debug, clap::Subcommand)]
pub enum Subcommands {
    /// Copy a tree so that it diffs the same but can't be traced back uniquely to its source.
    ///
    /// The idea is that we don't want to deal with licensing, it's more about patterns in order to
    /// reproduce cases for tests.
    #[clap(visible_alias = "cr")]
    CopyRoyal {
        /// Don't really copy anything.
        #[clap(long, short = 'n')]
        dry_run: bool,
        /// The git root whose tracked files to copy.
        worktree_dir: PathBuf,
        /// The directory into which to copy the files.
        destination_dir: PathBuf,
        /// The pathspecs to determine which paths to copy from `worktree_dir`.
        ///
        /// None will copy everything.
        #[clap(value_parser = AsPathSpec)]
        patterns: Vec<gix::pathspec::Pattern>,
    },
    /// Serialize a git repository as linear history while degenerating content into a shell script that reproduces it.
    #[clap(visible_alias = "gts")]
    GitToSh {
        /// The amount of commits to copy from `committish`.
        ///
        /// If 0, all traversable commits will be copied.
        #[clap(long, short = 'c', default_value_t = 0)]
        count: usize,
        /// Do not use `copy-royal` to degenerate information of blobs, but take blobs verbatim.
        ///
        /// Note that this should only be done if the source repository is purely for testing
        /// or was created by yourself.
        #[clap(long)]
        verbatim: bool,
        /// The directory into which the blobs and tree declarations will be written.
        #[clap(long, short = 'o', default_value = ".")]
        output_dir: PathBuf,
        /// The path to the git repository to serialize.
        repo_dir: PathBuf,
        /// The name of the directory within `output_dir` for storing blobs and trees.
        name: String,
        /// A revspec of the commit to start the iteration from, like `@`.
        ///
        /// Note that the history will be serialized, and multiple parents aren't allowed.
        committish: String,
        /// The pathspecs to determine which paths to copy from each commit's tree.
        ///
        /// None will copy everything.
        #[clap(value_parser = AsPathSpec)]
        patterns: Vec<gix::pathspec::Pattern>,
    },
    /// Check for executable bits that disagree with shebangs.
    ///
    /// This checks committed and staged files, but not anything unstaged, to find shell scripts
    /// that either begin with a `#!` but not `+x` permissions, or do not begin with `#!` but do
    /// have `+x` permissions. Such mismatches are reported but not automatically corrected. Some
    /// platforms (at least Windows) do not have such permissions, but Git still represents them.
    ///
    /// This currently only checks files name with an `.sh` suffix, and only operates on the
    /// current repository. Its main use is checking that fixture scripts are have correct modes.
    #[clap(visible_alias = "cm")]
    CheckMode {},
    /// Print environment variables as `NAME=value` lines.
    ///
    /// It is useful to be able to observe environment variables that are set when running code
    /// with tools such as `cargo` or `cross`. Commands like `cargo run -p internal-tools -- env`
    /// include environment changes from `cargo` itself. With `cross`, changes are more extensive,
    /// due to the effect of `build.env.passthrough`, container customization, and existing special
    /// cases in wrapper scripts shipped in default `cross` containers (such as to `LD_PRELOAD`).
    ///
    /// Since one use for checking environment variables is to investigate the effects of
    /// environments that contain variable names or values that are not valid Unicode, this avoids
    /// requiring that environment variables all be Unicode. Any name or value that is not Unicode
    /// is shown in its Rust debug representation. This is always quoted, and to decrease ambiguity
    /// any name or (more likely) value that contains literal double quotes is likewise shown in
    /// its debug representation so that it is always clear if a quotation mark is just for
    /// display. Each name and value is otherwise shown literally.
    #[clap(visible_alias = "e")]
    Env {},
}

#[derive(Clone)]
pub struct AsPathSpec;

impl TypedValueParser for AsPathSpec {
    type Value = gix::pathspec::Pattern;

    fn parse_ref(&self, cmd: &Command, arg: Option<&Arg>, value: &OsStr) -> Result<Self::Value, Error> {
        let pathspec_defaults =
            gix::pathspec::Defaults::from_environment(&mut |n| std::env::var_os(n)).unwrap_or_default();
        OsStringValueParser::new()
            .try_map(move |arg| {
                let arg: &std::path::Path = arg.as_os_str().as_ref();
                gix::pathspec::parse(gix::path::into_bstr(arg).as_ref(), pathspec_defaults)
            })
            .parse_ref(cmd, arg, value)
    }
}
