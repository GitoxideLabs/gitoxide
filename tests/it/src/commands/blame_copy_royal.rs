pub struct Options {
    pub verbatim: bool,
}

pub(super) mod function {
    use anyhow::Context;
    use gix::{
        blame::BlamePathEntry,
        bstr::{BString, ByteSlice},
        objs::FindExt,
        ObjectId,
    };
    use std::{
        collections::BTreeSet,
        ffi::OsStr,
        fmt::Display,
        path::{Path, PathBuf},
    };

    use super::Options;

    pub fn blame_copy_royal(
        dry_run: bool,
        worktree_dir: &Path,
        destination_dir: PathBuf,
        asset_dir: Option<BString>,
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
            ranges: gix::blame::BlameRanges::default(),
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

        let blame_infos = outcome
            .blame_path
            .expect("blame path to be present as `debug_track_path == true`");

        let asset_dir = asset_dir.unwrap_or("assets".into());
        let assets = destination_dir.join(asset_dir.to_os_str()?);
        eprintln!("{prefix} create directory '{assets}'", assets = assets.display());

        if !dry_run {
            std::fs::create_dir_all(&assets)?;
        }

        let mut buf = Vec::new();

        eprintln!("{prefix} perform {} asset copy operations", blame_infos.len(),);
        for blame_path_entry in &blame_infos {
            let dst = assets.join(format!("{}.commit", blame_path_entry.commit_id));
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
                }
            }
        }

        let mut blame_script = BlameScript::new(blame_infos, asset_dir, Options { verbatim });
        blame_script.generate()?;

        let script_file = destination_dir.join("create-history.sh");
        eprintln!(
            "{prefix} write script file at '{script_file}'",
            script_file = script_file.display()
        );

        if !dry_run {
            let blocks: Vec<_> = blame_script
                .script
                .iter()
                .map(std::string::ToString::to_string)
                .collect();

            std::fs::write(script_file, blocks.join(""))?;
        }

        Ok(())
    }

    enum BlameScriptOperation {
        InitRepository(BString),
        RemoveFile(String),
        CommitFile(BString, BString, ObjectId),
        CheckoutTag(ObjectId),
        PrepareMerge(Vec<ObjectId>),
        CreateTag(ObjectId),
    }

    impl Display for BlameScriptOperation {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                BlameScriptOperation::InitRepository(asset_dir) => write!(
                    f,
                    r"#!/bin/sh

set -e

git init
echo .gitignore >> .gitignore
echo {asset_dir}/ >> .gitignore
echo create-history.sh >> .gitignore

"
                ),
                BlameScriptOperation::RemoveFile(src) => write!(
                    f,
                    r"# delete previous version of file
git rm {src}
"
                ),
                BlameScriptOperation::CommitFile(asset_dir, src, commit_id) => {
                    writeln!(f, r"# make file {src} contain content at commit {commit_id}")?;
                    if let Some(pos) = src.rfind_byte(b'/') {
                        let dirname = src[..pos].as_bstr();
                        writeln!(f, "mkdir -p \"{dirname}\"")?;
                    }
                    write!(
                        f,
                        r"cp ./{asset_dir}/{commit_id}.commit ./{src}
git add {src}
git commit -m {commit_id}
"
                    )
                }
                BlameScriptOperation::CheckoutTag(commit_id) => writeln!(f, "git checkout tag-{commit_id}"),
                BlameScriptOperation::PrepareMerge(commit_ids) => writeln!(
                    f,
                    "git merge --no-commit {} || true",
                    commit_ids
                        .iter()
                        .map(|commit_id| format!("tag-{commit_id}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                ),
                BlameScriptOperation::CreateTag(commit_id) => write!(f, "git tag tag-{commit_id}\n\n"),
            }
        }
    }

    struct BlameScript {
        blame_infos: Vec<BlamePathEntry>,
        seen: BTreeSet<ObjectId>,
        script: Vec<BlameScriptOperation>,
        asset_dir: BString,
        options: Options,
    }

    impl BlameScript {
        fn new(blame_infos: Vec<BlamePathEntry>, asset_dir: BString, options: Options) -> Self {
            let script = vec![BlameScriptOperation::InitRepository(asset_dir.clone())];

            Self {
                blame_infos,
                seen: BTreeSet::default(),
                script,
                asset_dir,
                options,
            }
        }

        fn generate(&mut self) -> anyhow::Result<()> {
            // `self.blame_path`, before calling `reverse`, has parents before children, with the
            // history’s root being the last element. We reverse the order in place so that all
            // methods can rely on the assumption that the root comes first, followed by its
            // descendants. That way, we can use a simple `for` loop to iterate through
            // `self.blame_infos` below.
            self.blame_infos.reverse();

            for blame_path_entry in self.blame_infos.clone() {
                if !self.seen.contains(&blame_path_entry.commit_id) {
                    self.process_entry(&blame_path_entry)?;
                }

                self.seen.insert(blame_path_entry.commit_id);
            }

            Ok(())
        }

        fn process_entry(&mut self, blame_path_entry: &BlamePathEntry) -> anyhow::Result<()> {
            let source_file_path = blame_path_entry.source_file_path.clone();
            let parents = self.parents_of(blame_path_entry);

            let src = if self.options.verbatim {
                source_file_path.clone()
            } else {
                let source_file_path = std::str::from_utf8(source_file_path.as_slice()).with_context(|| {
                    format!("Source file path '{source_file_path}' was not valid UTF8 and can't be remapped",)
                })?;

                crate::commands::copy_royal::remapped(source_file_path).into()
            };
            let commit_id = blame_path_entry.commit_id;

            let delete_previous_file_operation = match &blame_path_entry.previous_source_file_path {
                Some(previous_source_file_path) if source_file_path != *previous_source_file_path => {
                    let src = if self.options.verbatim {
                        previous_source_file_path.to_string()
                    } else {
                        let source_file_path =
                        std::str::from_utf8(previous_source_file_path.as_slice()).with_context(|| {
                            format!("Source file path '{previous_source_file_path}' was not valid UTF8 and can't be remapped",)
                        })?;

                        crate::commands::copy_royal::remapped(source_file_path)
                    };

                    Some(BlameScriptOperation::RemoveFile(src))
                }
                _ => None,
            };

            if parents.is_empty() {
                if let Some(delete_previous_file_operation) = delete_previous_file_operation {
                    self.script.push(delete_previous_file_operation);
                }
                self.script
                    .push(BlameScriptOperation::CommitFile(self.asset_dir.clone(), src, commit_id));
            } else {
                let ([first], rest) = parents.split_at(1) else {
                    unreachable!();
                };

                self.script.push(BlameScriptOperation::CheckoutTag(first.commit_id));

                if rest.is_empty() {
                    if let Some(delete_previous_file_operation) = delete_previous_file_operation {
                        self.script.push(delete_previous_file_operation);
                    }
                    self.script
                        .push(BlameScriptOperation::CommitFile(self.asset_dir.clone(), src, commit_id));
                } else {
                    self.script.push(BlameScriptOperation::PrepareMerge(
                        rest.iter().map(|blame_path_entry| blame_path_entry.commit_id).collect(),
                    ));

                    if let Some(delete_previous_file_operation) = delete_previous_file_operation {
                        self.script.push(delete_previous_file_operation);
                    }
                    self.script
                        .push(BlameScriptOperation::CommitFile(self.asset_dir.clone(), src, commit_id));
                }
            }

            self.script.push(BlameScriptOperation::CreateTag(commit_id));

            Ok(())
        }

        fn parents_of(&self, child: &BlamePathEntry) -> Vec<BlamePathEntry> {
            // In almost all cases, `children` will only have one element. The exception are merge
            // commits where there’s changes against each parent. Each of these changes would
            // produce a diff that’s represented in `self.blame_path`.
            let mut children: Vec<_> = self
                .blame_infos
                .iter()
                .enumerate()
                .filter(|(_, x)| x.commit_id == child.commit_id)
                .collect();

            children.sort_by_key(|(_, x)| x.parent_index);

            let parents = children
                .iter()
                .filter_map(|(index, child)| {
                    let parent_blob_id = child.previous_blob_id;
                    let parent_source_file_path = &child.previous_source_file_path;

                    // When we search for a parent we only have to consider entries up to and
                    // excluding `index` as anything after `index` can only be a child.
                    self.blame_infos[..(*index)]
                        .iter()
                        .rfind(|&x| {
                            x.blob_id == parent_blob_id && Some(&x.source_file_path) == parent_source_file_path.as_ref()
                        })
                        .cloned()
                })
                .collect();

            parents
        }
    }
}
