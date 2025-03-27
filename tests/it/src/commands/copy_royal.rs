pub(super) mod function {
    use anyhow::Context;
    use gix::fs::Stack;
    use gix::pathspec::Pattern;
    use std::path::{Path, PathBuf};

    pub fn copy_royal(
        dry_run: bool,
        worktree_dir: &Path,
        destination_dir: PathBuf,
        patterns: Vec<Pattern>,
    ) -> anyhow::Result<()> {
        let prefix = if dry_run { "WOULD" } else { "Will" };
        let repo = gix::open(worktree_dir)?;
        let index = repo.index()?;
        let mut specs = repo.pathspec(
            true,
            // TODO: ideally this could accept patterns already.
            patterns.into_iter().map(|p| p.to_bstring()),
            true,
            &index,
            gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
        )?;
        let mut create_dir = CreateDir { dry_run };
        let mut stack = gix::fs::Stack::new(destination_dir);
        for (rela_path, _entry) in specs
            .index_entries_with_paths(&index)
            .context("Didn't find a single entry to copy")?
        {
            let rela_path = gix::path::from_bstr(rela_path);
            let src = worktree_dir.join(&rela_path);
            stack.make_relative_path_current(&*rela_path, &mut create_dir)?;
            let dst = stack.current();

            eprintln!(
                "{prefix} copy '{src}' to '{dst}'",
                src = src.display(),
                dst = dst.display()
            );
            if !dry_run {
                let content = std::fs::read_to_string(&src).with_context(|| {
                    format!(
                        "Need UTF-8 decodable content in '{src}' - skip binaries with pathspec",
                        src = src.display()
                    )
                })?;
                std::fs::write(dst, remapped(&content))?;
            }
        }
        Ok(())
    }

    pub fn remapped(i: &str) -> String {
        i.chars()
            .filter_map(|c| {
                Some(if c.is_alphabetic() {
                    if c.is_uppercase() {
                        match (c as u32) % 10 {
                            0 => 'A',
                            1 => 'E',
                            2 => 'I',
                            3 => 'O',
                            4 => 'U',
                            5 => 'X',
                            6 => 'R',
                            7 => 'S',
                            8 => 'T',
                            9 => 'Y',
                            _ => unreachable!(),
                        }
                    } else {
                        match (c as u32) % 10 {
                            0 => 'a',
                            1 => 'e',
                            2 => 'i',
                            3 => 'o',
                            4 => 'u',
                            5 => 'x',
                            6 => 'r',
                            7 => 's',
                            8 => 't',
                            9 => 'y',
                            _ => unreachable!(),
                        }
                    }
                } else if c.is_whitespace() || c.is_ascii_punctuation() || c.is_ascii_digit() {
                    c
                } else {
                    return None;
                })
            })
            .collect()
    }

    struct CreateDir {
        dry_run: bool,
    }

    impl gix::fs::stack::Delegate for CreateDir {
        fn push_directory(&mut self, stack: &Stack) -> std::io::Result<()> {
            if !self.dry_run && !stack.current().is_dir() {
                std::fs::create_dir(stack.current())?;
            }
            Ok(())
        }

        fn push(&mut self, _is_last_component: bool, _stack: &Stack) -> std::io::Result<()> {
            Ok(())
        }

        fn pop_directory(&mut self) {}
    }
}
pub use function::remapped;
