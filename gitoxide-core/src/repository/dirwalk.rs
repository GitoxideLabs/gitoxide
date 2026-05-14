use anyhow::bail;
use gix::{
    bstr::BString,
    dir::{
        EntryRef,
        walk::{self, EmissionMode},
    },
};

use crate::OutputFormat;

#[derive(Copy, Clone)]
pub enum Untracked {
    Collapsed,
    Matching,
}

pub struct Options {
    pub output_format: OutputFormat,
    pub statistics: bool,
    pub untracked: Untracked,
}

pub fn walk(
    repo: gix::Repository,
    patterns: Vec<BString>,
    mut out: impl std::io::Write,
    mut err: impl std::io::Write,
    Options {
        output_format,
        statistics,
        untracked,
    }: Options,
) -> anyhow::Result<()> {
    if output_format != OutputFormat::Human {
        bail!("Only human format is supported right now");
    }
    let index = repo.index_or_empty()?;
    let options = repo.dirwalk_options()?.emit_untracked(match untracked {
        Untracked::Collapsed => EmissionMode::CollapseDirectory,
        Untracked::Matching => EmissionMode::Matching,
    });

    let start = std::time::Instant::now();
    let mut delegate = Count::default();
    let outcome = repo.dirwalk(
        &index,
        patterns,
        &gix::interrupt::IS_INTERRUPTED,
        options,
        &mut delegate,
    )?;

    if statistics {
        writeln!(
            err,
            "dirwalk done {} entries in {:.2?}",
            delegate.entries,
            start.elapsed()
        )?;
        writeln!(err, "{:?}", outcome.dirwalk)?;
    } else {
        writeln!(out, "{}", delegate.entries)?;
    }
    Ok(())
}

#[derive(Default)]
struct Count {
    entries: u64,
}

impl walk::Delegate for Count {
    fn emit(
        &mut self,
        _entry: EntryRef<'_>,
        _collapsed_directory_status: Option<gix::dir::entry::Status>,
    ) -> walk::Action {
        self.entries += 1;
        std::ops::ControlFlow::Continue(())
    }
}
