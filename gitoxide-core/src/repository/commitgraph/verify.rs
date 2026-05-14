use crate::OutputFormat;

/// A general purpose context for many operations provided here
pub struct Context<W1: std::io::Write, W2: std::io::Write> {
    /// A stream to which to output errors
    pub err: W2,
    /// A stream to which to output operation results
    pub out: W1,
    pub output_statistics: Option<OutputFormat>,
}

pub(crate) mod function {
    use std::io;

    use crate::{OutputFormat, repository::commitgraph::verify::Context};
    use anyhow::Result;

    pub fn verify<W1, W2>(
        repo: gix::Repository,
        Context {
            err: _err,
            mut out,
            output_statistics,
        }: Context<W1, W2>,
    ) -> Result<gix::commitgraph::verify::Outcome>
    where
        W1: io::Write,
        W2: io::Write,
    {
        let g = repo.commit_graph()?;

        #[allow(clippy::unnecessary_wraps, unknown_lints)]
        fn noop_processor(_commit: &gix::commitgraph::file::Commit<'_>) -> std::result::Result<(), std::fmt::Error> {
            Ok(())
        }
        let stats = g.verify_integrity(noop_processor)?;

        #[cfg_attr(not(feature = "serde"), allow(clippy::single_match))]
        match output_statistics {
            Some(OutputFormat::Human) => drop(print_human_output(&mut out, &stats)),
            #[cfg(feature = "serde")]
            Some(OutputFormat::Json) => serde_json::to_writer_pretty(out, &stats)?,
            _ => {}
        }

        Ok(stats)
    }

    fn print_human_output(out: &mut impl io::Write, stats: &gix::commitgraph::verify::Outcome) -> io::Result<()> {
        writeln!(out, "number of commits with the given number of parents")?;
        let mut parent_counts: Vec<_> = stats.parent_counts.iter().map(|(a, b)| (*a, *b)).collect();
        parent_counts.sort_by_key(|e| e.0);
        for (parent_count, commit_count) in parent_counts.into_iter() {
            writeln!(out, "\t{parent_count:>2}: {commit_count}")?;
        }
        writeln!(out, "\t->: {}", stats.num_commits)?;

        write!(out, "\nlongest path length between two commits: ")?;
        if let Some(n) = stats.longest_path_length {
            writeln!(out, "{n}")?;
        } else {
            writeln!(out, "unknown")?;
        }

        Ok(())
    }
}
