use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use anyhow::{Result, anyhow};
use clap::{CommandFactory, Parser};
use gitoxide_core as core;

use crate::{
    porcelain::options::{Args, Subcommands},
    shared::pretty::prepare_and_run,
};

pub fn main() -> Result<()> {
    let args: Args = Args::parse_from(gix::env::args_os());
    let should_interrupt = Arc::new(AtomicBool::new(false));
    #[allow(unsafe_code)]
    unsafe {
        // SAFETY: The closure doesn't use mutexes or memory allocation, so it should be safe to call from a signal handler.
        gix::interrupt::init_handler(1, {
            let should_interrupt = Arc::clone(&should_interrupt);
            move || should_interrupt.store(true, Ordering::SeqCst)
        })?;
    }
    let trace = false;
    let verbose = !args.quiet;
    let progress = args.progress;
    #[cfg(feature = "gitoxide-core-tools")]
    let threads = args.threads;
    let progress_keep_open = args.progress_keep_open;

    match args.cmd {
        #[cfg(debug_assertions)]
        Subcommands::Panic => prepare_and_run(
            "panic-behaviour",
            trace,
            verbose,
            progress,
            progress_keep_open,
            crate::shared::STANDARD_RANGE,
            move |_progress, _out, _err| panic!("something went very wrong"),
        ),
        Subcommands::Init { directory } => core::repository::init(directory).map(|_| ()),
        #[cfg(feature = "gitoxide-core-tools")]
        Subcommands::Tool(tool) => match tool {
            #[cfg(feature = "gitoxide-core-tools-query")]
            crate::porcelain::options::ToolCommands::Query(crate::porcelain::options::tools::Query {
                object_cache_size_mb,
                find_copies_harder,
                repo_dir,
                cmd,
            }) => {
                use gitoxide_core::query;
                prepare_and_run(
                    "query",
                    trace,
                    verbose,
                    progress,
                    progress_keep_open,
                    crate::shared::STANDARD_RANGE,
                    move |mut progress, out, err| {
                        let engine = query::prepare(
                            &repo_dir,
                            &mut progress,
                            &mut *err,
                            query::Options {
                                object_cache_size_mb,
                                find_copies_harder,
                                threads,
                            },
                        )?;
                        match cmd {
                            None => writeln!(err, "Choose a command for the query engine")?,
                            Some(crate::porcelain::options::tools::query::Command::TracePath { path }) => {
                                engine.run(query::Command::TracePath { spec: path }, out, progress)?;
                            }
                        }
                        Ok(())
                    },
                )
            }
            crate::porcelain::options::ToolCommands::EstimateHours(
                crate::porcelain::options::tools::EstimateHours {
                    working_dir,
                    rev_spec,
                    no_bots,
                    file_stats,
                    line_stats,
                    show_pii,
                    omit_unify_identities,
                },
            ) => {
                use gitoxide_core::hours;
                prepare_and_run(
                    "estimate-hours",
                    trace,
                    verbose,
                    progress,
                    progress_keep_open,
                    crate::shared::STANDARD_RANGE,
                    move |progress, out, _err| {
                        hours::estimate(
                            &working_dir,
                            rev_spec.as_ref(),
                            progress,
                            hours::Context {
                                show_pii,
                                ignore_bots: no_bots,
                                threads,
                                file_stats,
                                line_stats,
                                omit_unify_identities,
                                out,
                            },
                        )
                    },
                )
            }
            crate::porcelain::options::ToolCommands::Find { root, debug } => {
                use gitoxide_core::organize;
                prepare_and_run(
                    "find",
                    trace,
                    verbose,
                    progress,
                    progress_keep_open,
                    crate::shared::STANDARD_RANGE,
                    move |progress, out, _err| {
                        organize::discover(
                            root.unwrap_or_else(|| [std::path::Component::CurDir].iter().collect()),
                            out,
                            progress,
                            debug,
                            threads,
                        )
                    },
                )
            }
            crate::porcelain::options::ToolCommands::Organize {
                destination_directory,
                execute,
                repository_source,
            } => {
                use gitoxide_core::organize;
                prepare_and_run(
                    "organize",
                    trace,
                    verbose,
                    progress,
                    progress_keep_open,
                    crate::shared::STANDARD_RANGE,
                    move |progress, _out, _err| {
                        organize::run(
                            if execute {
                                organize::Mode::Execute
                            } else {
                                organize::Mode::Simulate
                            },
                            repository_source.unwrap_or_else(|| [std::path::Component::CurDir].iter().collect()),
                            destination_directory.unwrap_or_else(|| [std::path::Component::CurDir].iter().collect()),
                            progress,
                            threads,
                        )
                    },
                )
            }
        },
        Subcommands::Completions { shell, out_dir } => {
            let mut app = Args::command();

            let shell = shell
                .or_else(clap_complete::Shell::from_env)
                .ok_or_else(|| anyhow!("The shell could not be derived from the environment"))?;

            let bin_name = app.get_name().to_owned();
            if let Some(out_dir) = out_dir {
                clap_complete::generate_to(shell, &mut app, bin_name, &out_dir)?;
            } else {
                clap_complete::generate(shell, &mut app, bin_name, &mut std::io::stdout());
            }
            Ok(())
        }
    }?;
    Ok(())
}
