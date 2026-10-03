use std::collections::HashMap;

use gix::{
    Count, Progress, Result,
    bstr::ByteSlice,
    error::{OptionExt, ResultExt, bail, corruption, message},
    prelude::ObjectIdExt,
};
use rusqlite::{OptionalExtension, params};

use crate::{
    query,
    query::{Command, engine::update::FileMode},
};

impl query::Engine {
    pub fn run(
        &self,
        cmd: Command,
        mut out: impl std::io::Write,
        mut progress: impl gix::NestedProgress,
    ) -> Result<()> {
        match cmd {
            Command::TracePath { spec } => {
                let is_excluded = spec.is_excluded();
                let relpath = if spec.signature.contains(gix::pathspec::MagicSignature::TOP) {
                    let root = self.repo.workdir().unwrap_or_else(|| self.repo.git_dir());
                    let path = root.join(gix::path::from_bstr(spec.path()).as_ref());
                    self.repo
                        .normalize_path(gix::path::into_bstr(path).as_ref())?
                        .into_owned()
                } else {
                    self.repo.normalize_path(spec.path())?.into_owned()
                };
                if relpath.is_empty() || is_excluded {
                    bail!(gix::error::validation(format!(
                        "Invalid pathspec {spec} - path must not be empty, not be excluded, and wildcards are taken literally"
                    )))
                }
                let file_id: usize = self
                    .con
                    .query_row(
                        "SELECT file_id FROM files WHERE file_path = ?",
                        params![relpath.to_str_lossy()],
                        |r| r.get(0),
                    )
                    .optional()
                    .or_error()?
                    .ok_or_raise(|| {
                        message!("Path \"{relpath}\" not found anywhere in recorded history").not_found()
                    })?;

                let mut by_file_id = self
                    .con
                    .prepare("SELECT hash, mode, source_file_id, has_diff, lines_added, lines_removed from commit_file where file_id = ? order by mode").or_error()?;
                let mut path_by_id = self
                    .con
                    .prepare("SELECT file_path from files where file_id = ?")
                    .or_error()?;
                let mut seen = HashMap::<usize, String>::new();
                seen.insert(file_id, relpath.to_string());

                let mut stack = vec![file_id];
                let mut info = Vec::new();
                let start = std::time::Instant::now();
                let mut progress = progress.add_child("run sql query");
                progress.init(None, gix::progress::count("round"));
                while let Some(file_id) = stack.pop() {
                    let rows = by_file_id
                        .query_map([file_id], |r| {
                            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
                        })
                        .or_error()?;
                    progress.inc();
                    for row in rows {
                        let (hash, mode, source_file_id, has_diff, lines_added, lines_removed): (
                            Vec<u8>,
                            usize,
                            Option<usize>,
                            bool,
                            usize,
                            usize,
                        ) = row.or_error()?;
                        let commit_id = gix::ObjectId::try_from(hash.as_slice())?;
                        let commit_time = commit_id
                            .attach(&self.repo)
                            .object()?
                            .into_commit()
                            .committer()?
                            .time()?;
                        let mode = FileMode::from_usize(mode).ok_or_raise(|| corruption("invalid file mode"))?;
                        info.push(trace_path::Info {
                            commit_id,
                            commit_time,
                            file_id,
                            mode,
                            diff: has_diff.then_some(trace_path::Diff {
                                lines_added,
                                lines_removed,
                            }),
                            source_file_id,
                        });
                        let Some(source_id) = source_file_id else {
                            continue;
                        };
                        if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(source_id) {
                            stack.push(source_id);
                            e.insert(path_by_id.query_row([source_id], |r| r.get(0)).or_error()?);
                        }
                    }
                }

                info.sort_by_key(|a| a.commit_id);
                let max_diff_lines = info
                    .iter()
                    .map(|i| i.diff.map_or(0, |d| d.lines_removed + d.lines_added))
                    .max()
                    .unwrap_or_default();
                let mut found = 0;
                progress.show_throughput(start);
                for info in self
                    .commits
                    .iter()
                    .filter_map(|c| info.binary_search_by(|i| i.commit_id.cmp(c)).ok().map(|idx| &info[idx]))
                {
                    found += 1;
                    info.write_to(&mut out, &self.repo, &seen, max_diff_lines).or_error()?;
                }
                let missing = info.len() - found;
                if missing > 0 {
                    writeln!(
                        out,
                        "{missing} file(s) were found in history that are not reachable from HEAD"
                    )
                    .or_error()?;
                }
                Ok(())
            }
        }
    }
}

mod trace_path {
    use std::collections::HashMap;

    use gix::prelude::ObjectIdExt;

    use crate::query::engine::update::FileMode;

    #[derive(Debug, Default, Copy, Clone)]
    pub struct Diff {
        pub lines_added: usize,
        pub lines_removed: usize,
    }

    impl Diff {
        fn format(&self, max_diff_lines: usize) -> String {
            const NUM_CHARS: f32 = 10.0;
            let mut buf = String::with_capacity(NUM_CHARS as usize);
            if max_diff_lines != 0 {
                let num_plus = ((self.lines_added as f32 / max_diff_lines as f32) * NUM_CHARS).ceil() as usize;
                let num_minus = ((self.lines_removed as f32 / max_diff_lines as f32) * NUM_CHARS) as usize;
                buf.extend((0..num_plus).map(|_| '+'));
                buf.extend((0..num_minus).map(|_| '-'));
            }
            buf.extend((buf.len()..NUM_CHARS as usize).map(|_| ' '));
            buf
        }
    }

    #[derive(Debug)]
    pub struct Info {
        pub commit_id: gix::ObjectId,
        pub commit_time: gix::date::Time,
        pub file_id: usize,
        pub mode: FileMode,
        pub diff: Option<Diff>,
        pub source_file_id: Option<usize>,
    }

    impl Info {
        pub fn write_to(
            &self,
            mut out: impl std::io::Write,
            repo: &gix::Repository,
            path_by_id: &HashMap<usize, String>,
            max_diff_lines: usize,
        ) -> std::io::Result<()> {
            let id = self.commit_id.attach(repo);
            match self.source_file_id {
                Some(source_id) => {
                    writeln!(
                        out,
                        "{}| {} | {} {} {} ➡ {}",
                        self.diff.unwrap_or_default().format(max_diff_lines),
                        self.commit_time.format_or_unix(gix::date::time::format::SHORT),
                        id.shorten_or_id(),
                        self.mode.as_str(),
                        path_by_id[&source_id],
                        path_by_id[&self.file_id],
                    )
                }
                None => {
                    writeln!(
                        out,
                        "{}| {} | {} {} {}",
                        self.diff.unwrap_or_default().format(max_diff_lines),
                        self.commit_time.format_or_unix(gix::date::time::format::SHORT),
                        id.shorten_or_id(),
                        self.mode.as_str(),
                        path_by_id[&self.file_id]
                    )
                }
            }
        }
    }
}
