pub(super) mod function {
    use anyhow::Context;
    use std::{
        collections::HashSet,
        path::{Path, PathBuf},
    };

    use gix::{
        bstr::{BString, ByteSlice},
        objs::FindExt,
    };

    pub fn create_diff_cases(
        dry_run: bool,
        sliders_file: PathBuf,
        worktree_dir: &Path,
        destination_dir: PathBuf,
        count: usize,
        asset_dir: Option<BString>,
    ) -> anyhow::Result<()> {
        let prefix = if dry_run { "WOULD" } else { "Will" };
        let sliders = std::fs::read_to_string(&sliders_file)?;

        eprintln!(
            "Read '{}' which has {} lines",
            sliders_file.display(),
            sliders.lines().count()
        );

        let sliders: HashSet<_> = sliders
            .lines()
            .take(count)
            .map(|line| {
                let parts: Vec<_> = line.split_ascii_whitespace().collect();

                match parts[..] {
                    [before, after, ..] => (before, after),
                    _ => todo!(),
                }
            })
            .collect();

        let repo = gix::open(worktree_dir)?;

        let asset_dir = asset_dir.unwrap_or("assets".into());
        let assets = destination_dir.join(asset_dir.to_os_str()?);

        eprintln!("{prefix} create directory '{assets}'", assets = assets.display());
        if !dry_run {
            std::fs::create_dir_all(&assets)?;
        }

        let mut buf = Vec::new();

        let script_name = "make_diff_for_sliders_repo.sh";

        let mut blocks: Vec<String> = vec![format!(
            r#"#!/usr/bin/env bash
set -eu -o pipefail

ROOT="$(cd "$(dirname "${{BASH_SOURCE[0]}}")" && pwd)"

mkdir -p {asset_dir}
"#
        )];

        for (before, after) in sliders.iter() {
            let revspec = repo.rev_parse(*before)?;
            let old_blob_id = revspec
                .single()
                .context(format!("rev-spec '{before}' must resolve to a single object"))?;

            let revspec = repo.rev_parse(*after)?;
            let new_blob_id = revspec
                .single()
                .context(format!("rev-spec '{after}' must resolve to a single object"))?;

            let dst_old_blob = assets.join(format!("{old_blob_id}.blob"));
            let dst_new_blob = assets.join(format!("{new_blob_id}.blob"));
            if !dry_run {
                let old_blob = repo.objects.find_blob(&old_blob_id, &mut buf)?.data;
                std::fs::write(dst_old_blob, old_blob)?;

                let new_blob = repo.objects.find_blob(&new_blob_id, &mut buf)?.data;
                std::fs::write(dst_new_blob, new_blob)?;
            }

            blocks.push(format!(
                r#"git -c diff.algorithm=myers diff --no-index "$ROOT/{asset_dir}/{old_blob_id}.blob" "$ROOT/{asset_dir}/{new_blob_id}.blob" > {old_blob_id}-{new_blob_id}.myers.baseline || true
git -c diff.algorithm=histogram diff --no-index "$ROOT/{asset_dir}/{old_blob_id}.blob" "$ROOT/{asset_dir}/{new_blob_id}.blob" > {old_blob_id}-{new_blob_id}.histogram.baseline || true
cp "$ROOT/{asset_dir}/{old_blob_id}.blob" assets/
cp "$ROOT/{asset_dir}/{new_blob_id}.blob" assets/
"#
            ));
        }

        let script_file = destination_dir.join(script_name);
        eprintln!(
            "{prefix} write script file at '{script_file}'",
            script_file = script_file.display()
        );

        if !dry_run {
            let script = blocks.join("\n");
            std::fs::write(script_file, script)?;
        }

        Ok(())
    }
}
