use std::{ffi::OsStr, path::PathBuf};

use clap::{
    builder::{OsStringValueParser, TypedValueParser},
    Arg, Command, Error,
};
use gix::bstr::BString;

#[derive(Debug, clap::Parser)]
#[clap(name = "it", about = "internal tools to help create test cases")]
pub struct Args {
    #[clap(subcommand)]
    pub cmd: Subcommands,
}

#[derive(Debug, clap::Subcommand)]
pub enum Subcommands {
    /// Generate a shell script that creates a git repository containing all commits that are
    /// traversed when following a given file through the Git history just as `git blame` would.
    ///
    /// This command extracts the file’s history so that blame, when run on the repository created
    /// by the script, shows the same characteristics, in particular bugs, as the original, but in
    /// a way that does not resemble the original source file's content to any greater extent than
    /// is useful and necessary.
    ///
    /// Note that this should not be used to redact sensitive information. The obfuscation leaves
    /// numerous properties of the source intact, such that it may be feasible to reconstruct the
    /// input.
    ///
    /// This command can also be helpful in debugging the blame algorithm itself.
    ///
    /// ### Terminology
    ///
    /// A **blame history** is the set of commits that the blame algorithm, at some point, treated
    /// as potential suspects for any line in a file. It is a subset of all commits that ever
    /// changed a file in its history.
    ///
    /// With respect to branches and merge commits, the **blame history** will not necessarily be
    /// identical to the file's history in the source repository. This is because the blame
    /// algorithm will stop following a file's history for branches that only touch lines for which
    /// the source has already been found. The **blame history**, thus, looks likely "cleaner" and
    /// "simpler" than the source history.
    #[clap(visible_alias = "bcr")]
    BlameCopyRoyal {
        /// Don't really copy anything.
        #[clap(long, short = 'n')]
        dry_run: bool,
        /// The git root whose history to extract the blame-relevant parts from.
        worktree_dir: PathBuf,
        /// The directory into which to copy the files.
        destination_dir: PathBuf,
        /// The directory to place assets in.
        #[clap(long)]
        asset_dir: Option<BString>,
        /// The file to extract the history for.
        file: std::ffi::OsString,
        /// Do not use `copy-royal` to obfuscate the content of blobs, but copy it verbatim.
        ///
        /// Note that, for producing cases for the gitoxide test suite, we usually prefer only to
        /// take blobs verbatim if the source repository was purely for testing.
        #[clap(long)]
        verbatim: bool,
    },
    /// Copy a tree so that it diffs the same but does not resemble the original files' content to
    /// any greater extent than is useful and necessary.
    ///
    /// The idea is that this preserves the patterns that are usually sufficient to reproduce cases
    /// for tests of diffs, both for making the tests work and for keeping the diffs understandable
    /// to developers working on the tests, while avoiding keeping large verbatim fragments of code
    /// based on which the test cases were created. The benefits of "reducing" the code to these
    /// patterns include that the original meaning and function of code will not be confused with
    /// the code of gitoxide itself, will not distract from the effects observed in their diffs,
    /// and will not inadvertently be caught up in code cleanup efforts (e.g. attempting to adjust
    /// style or fix bugs) that would make sense in code of gitoxide itself but that would subtly
    /// break data test fixtures if done on their data.
    ///
    /// Note that this should not be used to redact sensitive information. The obfuscation leaves
    /// numerous properties of the source intact, such that it may be feasible to reconstruct the
    /// input.
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
        /// Note that, for producing cases for the gitoxide test suite, we usually prefer only to
        /// take blobs verbatim if the source repository was purely for testing.
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
    /// Take a slider file generated with the help of [diff-slider-tools] and turn it into a series
    /// of baseline diffs to be used in [slider-rs].
    ///
    /// See [make-diff-for-sliders-repo] for details.
    ///
    /// [diff-slider-tools]: https://github.com/mhagger/diff-slider-tools
    /// [slider-rs]: gix-diff/tests/diff/blob/slider.rs
    /// [make-diff-for-sliders-repo]: gix-diff/tests/fixtures/make_diff_for_sliders_repo.sh
    CreateDiffCases {
        /// Don't really copy anything.
        #[clap(long, short = 'n')]
        dry_run: bool,
        /// The `.sliders` file that contains a list of sliders.
        #[clap(long)]
        sliders_file: PathBuf,
        /// The git root to extract the diff-related parts from.
        #[clap(long)]
        worktree_dir: PathBuf,
        /// The directory into which to copy the files.
        #[clap(long)]
        destination_dir: PathBuf,
        /// The number of sliders to generate test cases for.
        #[clap(long, default_value_t = 10)]
        count: usize,
        /// The directory to place assets in.
        #[clap(long)]
        asset_dir: Option<BString>,
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
    /// due to effects of `build.env.passthrough`, container customization, and preexisting special
    /// cases in wrapper scripts shipped in default `cross` containers (such as to `LD_PRELOAD`).
    ///
    /// Since one use for checking environment variables is to investigate the effects of
    /// environments that contain variable names or values that are not valid Unicode, this avoids
    /// requiring that environment variables all be Unicode. Any name or value that is not Unicode
    /// is shown in its Rust debug representation. This is always quoted. To decrease ambiguity,
    /// any name or value containing a literal double quote or newline is also shown in its debug
    /// representation. Names and values without such content are shown literally and not quoted.
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
