pub struct Options {
    pub verbatim: bool,
}

pub(super) mod function {
    use anyhow::{bail, Context};
    use gix::{
        blame::BlamePathEntry,
        bstr::{BStr, BString, ByteSlice},
        objs::FindExt,
        ObjectId,
    };
    use std::{
        collections::{BTreeSet, VecDeque},
        ffi::OsStr,
        path::{Path, PathBuf},
    };

    use super::Options;

    pub fn blame_copy_royal(
        dry_run: bool,
        worktree_dir: &Path,
        destination_dir: PathBuf,
        file: &OsStr,
        Options { verbatim }: Options,
    ) -> anyhow::Result<()> {
        let prefix = if dry_run { "WOULD" } else { "Will" };
        let repo = gix::open(worktree_dir)?;

        let suspect: gix::ObjectId = repo.head()?.into_peeled_id()?.into();
        let cache: Option<gix::commitgraph::Graph> = repo.commit_graph_if_enabled()?;
        let mut resource_cache = repo.diff_resource_cache_for_tree_diff()?;
        let diff_algorithm = repo.diff_algorithm()?;

        let options = gix::blame::Options {
            diff_algorithm,
            range: gix::blame::BlameRanges::default(),
            since: None,
            rewrites: Some(gix::diff::Rewrites::default()),
            debug_track_path: true,
        };

        let index = repo.index_or_empty()?;

        // The following block, including the `TODO` comment, comes from
        // `gitoxide_core::repository::blame`.
        let file = gix::path::os_str_into_bstr(file)?;
        let specs = repo.pathspec(
            false,
            [file],
            true,
            &index,
            gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping.adjust_for_bare(repo.is_bare()),
        )?;
        // TODO: there should be a way to normalize paths without going through patterns, at least in this case maybe?
        //       `Search` actually sorts patterns by excluding or not, all that can lead to strange results.
        let file = specs
            .search()
            .patterns()
            .map(|p| p.path().to_owned())
            .next()
            .expect("exactly one pattern");

        let outcome = gix::blame::file(
            &repo.objects,
            suspect,
            cache,
            &mut resource_cache,
            file.as_bstr(),
            options,
        )?;

        let blame_path = outcome
            .blame_path
            .expect("blame path to be present as `debug_track_path == true`");

        // TODO
        // Potentially make `"assets"` configurable (it is in `git_to_sh`).
        let assets = destination_dir.join("assets");

        eprintln!("{prefix} create directory '{assets}'", assets = assets.display());

        if !dry_run {
            std::fs::create_dir_all(&assets)?;
        }

        let mut buf = Vec::new();

        for blame_path_entry in &blame_path {
            let src: &BStr = blame_path_entry.source_file_path.as_bstr();
            let dst = assets.join(format!("{}.commit", blame_path_entry.commit_id));

            eprintln!(
                "{prefix} copy file '{}' at commit {} to '{dst}'",
                src,
                blame_path_entry.commit_id,
                dst = dst.display()
            );

            if !dry_run {
                let blob = repo.objects.find_blob(&blame_path_entry.blob_id, &mut buf)?.data;

                if verbatim {
                    std::fs::write(dst, blob)?;
                } else {
                    let blob = std::str::from_utf8(blob).with_context(|| {
                        format!(
                            "Entry in blob '{blob_id}' was not valid UTF8 and can't be remapped",
                            blob_id = blame_path_entry.blob_id
                        )
                    })?;

                    let blob = crate::commands::copy_royal::remapped(blob);

                    std::fs::write(dst, blob)?;
                };
            }
        }

        let mut blame_script = BlameScript::new(blame_path, Options { verbatim });

        blame_script.generate()?;

        let script_file = destination_dir.join("create-history.sh");

        eprintln!(
            "{prefix} write script file at '{script_file}'",
            script_file = script_file.display()
        );

        if !dry_run {
            std::fs::write(script_file, blame_script.script)?;
        }

        Ok(())
    }

    struct BlameScript {
        blame_path: Vec<BlamePathEntry>,
        queue: VecDeque<(ObjectId, BString)>,
        seen: BTreeSet<(ObjectId, BString)>,
        script: String,
        options: Options,
    }

    impl BlameScript {
        fn new(blame_path: Vec<BlamePathEntry>, options: Options) -> Self {
            let mut script = String::new();

            script.push_str(
                r"#!/bin/sh

set -e

git init
echo .gitignore >> .gitignore
echo assets/ >> .gitignore
echo create-history.sh >> .gitignore

",
            );

            Self {
                blame_path,
                queue: VecDeque::default(),
                seen: BTreeSet::default(),
                script,
                options,
            }
        }

        fn generate(&mut self) -> anyhow::Result<()> {
            let roots = self
                .blame_path
                .iter()
                .filter(|blame_path_entry| blame_path_entry.previous_blob_id.is_null())
                .collect::<Vec<_>>();

            let [root] = roots[..] else {
                bail!(
                    "Expected to find one single root in blame path, but found {}",
                    roots.len()
                );
            };

            self.queue.push_back((root.blob_id, root.source_file_path.clone()));

            while let Some((blob_id, ref source_file_path)) = self.queue.pop_front() {
                if !self.seen.contains(&(blob_id, source_file_path.clone())) {
                    self.process_entry(blob_id, source_file_path.clone())?;

                    self.seen.insert((blob_id, source_file_path.clone()));
                }
            }

            Ok(())
        }

        fn process_entry(&mut self, blob_id: ObjectId, source_file_path: BString) -> anyhow::Result<()> {
            let blame_path_entry = self.blame_path_entry(blob_id, source_file_path.clone());
            let parents = self.parents_of(blob_id, source_file_path.clone());
            let children = self.children_of(blob_id, source_file_path.clone());

            let src = if self.options.verbatim {
                source_file_path.clone()
            } else {
                let source_file_path = std::str::from_utf8(source_file_path.as_slice()).with_context(|| {
                    format!("Source file path '{source_file_path}' was not valid UTF8 and can't be remapped",)
                })?;

                crate::commands::copy_royal::remapped(source_file_path).into()
            };
            let commit_id = blame_path_entry.commit_id;

            let delete_previous_file_script = match blame_path_entry.previous_source_file_path {
                Some(previous_source_file_path) if source_file_path != previous_source_file_path => {
                    let src = if self.options.verbatim {
                        previous_source_file_path
                    } else {
                        let source_file_path =
                        std::str::from_utf8(previous_source_file_path.as_slice()).with_context(|| {
                            format!("Source file path '{previous_source_file_path}' was not valid UTF8 and can't be remapped",)
                        })?;

                        crate::commands::copy_royal::remapped(source_file_path).into()
                    };

                    format!(
                        r"# delete previous version of file
git rm {src}
"
                    )
                }
                _ => String::new(),
            };

            let script = format!(
                r"# make file {src} contain content at commit {commit_id}
mkdir -p $(dirname {src})
cp ./assets/{commit_id}.commit ./{src}
# create commit
git add {src}
git commit -m {commit_id}
"
            );

            if parents.is_empty() {
                self.script.push_str(delete_previous_file_script.as_str());
                self.script.push_str(script.as_str());
            } else {
                let ([first], rest) = parents.split_at(1) else {
                    unreachable!();
                };

                self.script
                    .push_str(format!("git checkout tag-{}\n", first.commit_id).as_str());

                if rest.is_empty() {
                    self.script.push_str(delete_previous_file_script.as_str());
                    self.script.push_str(script.as_str());
                } else {
                    self.script.push_str(
                        format!(
                            "git merge --no-commit {} || true\n",
                            rest.iter()
                                .map(|blame_path_entry| format!("tag-{}", blame_path_entry.commit_id))
                                .collect::<Vec<_>>()
                                .join(" ")
                        )
                        .as_str(),
                    );

                    self.script.push_str(delete_previous_file_script.as_str());

                    // TODO
                    // If `git merge {}` is the only difference to `script` above, we can
                    // potentially simplify.
                    let script = format!(
                        r"# make file {src} contain content at commit {commit_id}
mkdir -p $(dirname {src})
cp ./assets/{commit_id}.commit ./{src}
# create merge commit
git add {src}
git commit -m {commit_id}
",
                    );

                    self.script.push_str(script.as_str());
                }
            }

            self.script.push_str(format!("git tag tag-{commit_id}\n\n").as_str());

            if children.is_empty() {
                return Ok(());
            }

            for child in children {
                let parents_of_child = self.parents_of(child.blob_id, child.source_file_path.clone());

                assert!(!parents_of_child.is_empty());

                if parents_of_child.len() == 1 {
                    self.queue.push_front((child.blob_id, child.source_file_path.clone()));
                } else {
                    self.queue.push_back((child.blob_id, child.source_file_path.clone()));
                }
            }

            Ok(())
        }

        fn parents_of(&self, blob_id: ObjectId, source_file_path: BString) -> Vec<BlamePathEntry> {
            let blame_path_entries = self.blame_path_entries(blob_id, source_file_path.clone());

            blame_path_entries
                .iter()
                .flat_map(|blame_path_entry| {
                    if blame_path_entry.previous_blob_id.is_null() {
                        Vec::new()
                    } else {
                        let parent_blob_id = blame_path_entry.previous_blob_id;
                        let parent_source_file_path = &blame_path_entry.previous_source_file_path;

                        self.blame_path
                            .iter()
                            .filter(|&blame_path_entry| {
                                blame_path_entry.blob_id == parent_blob_id
                                    && Some(&blame_path_entry.source_file_path) == parent_source_file_path.as_ref()
                            })
                            .cloned()
                            .collect()
                    }
                })
                .collect()
        }

        fn children_of(&self, blob_id: ObjectId, source_file_path: BString) -> Vec<BlamePathEntry> {
            self.blame_path
                .iter()
                .filter(|&blame_path_entry| {
                    blame_path_entry.previous_blob_id == blob_id
                        && blame_path_entry.previous_source_file_path.as_ref() == Some(&source_file_path)
                })
                .cloned()
                .collect()
        }

        fn blame_path_entry(&self, blob_id: ObjectId, source_file_path: BString) -> BlamePathEntry {
            self.blame_path
                .iter()
                .find(|blame_path_entry| {
                    blame_path_entry.blob_id == blob_id && blame_path_entry.source_file_path == source_file_path
                })
                .expect("TODO")
                .clone()
        }

        fn blame_path_entries(&self, blob_id: ObjectId, source_file_path: BString) -> Vec<BlamePathEntry> {
            self.blame_path
                .iter()
                .filter(|&blame_path_entry| {
                    blame_path_entry.blob_id == blob_id && blame_path_entry.source_file_path == source_file_path
                })
                .cloned()
                .collect()
        }
    }
}
