pub(super) mod function {

    use std::{
        collections::HashSet,
        path::{Path, PathBuf},
    };

    use gix::{
        Result,
        bstr::{BString, ByteSlice},
        error::{OptionExt, ResultExt, message},
        objs::FindExt,
    };

    pub fn create_diff_cases(
        dry_run: bool,
        sliders_file: PathBuf,
        worktree_dir: &Path,
        destination_dir: PathBuf,
        count: usize,
        asset_dir: Option<BString>,
    ) -> Result<()> {
        let prefix = if dry_run { "WOULD" } else { "Will" };
        let sliders = std::fs::read_to_string(&sliders_file).or_error()?;

        eprintln!(
            "Read \"{}\" which has {} lines",
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
                    _ => unreachable!(),
                }
            })
            .collect();

        let repo = gix::open_opts(worktree_dir, gix::open::Options::isolated())?;

        let asset_dir = asset_dir.unwrap_or("assets".into());
        let assets = destination_dir.join(asset_dir.to_os_str().or_error()?);

        eprintln!("{prefix} create directory \"{}\"", assets.display());
        if !dry_run {
            std::fs::create_dir_all(&assets).or_error()?;
        }

        let mut buf = Vec::new();
        let script_name = "make_diff_for_sliders_repo.sh";

        let mut blocks: Vec<String> = vec![
            r#"#!/usr/bin/env bash
set -eu -o pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Keep runtime assets fixed: slider::baseline reads assets/, while --asset-dir only locates source blobs.
mkdir -p assets
"#
            .to_owned(),
        ];

        for (before, after) in sliders.iter().copied() {
            let revspec = repo.rev_parse(before)?;
            let old_blob_id = revspec
                .single()
                .ok_or_raise(|| message!("rev-spec '{before}' must resolve to a single object"))?;

            let revspec = repo.rev_parse(after)?;
            let new_blob_id = revspec
                .single()
                .ok_or_raise(|| message!("rev-spec '{after}' must resolve to a single object"))?;

            let dst_old_blob = assets.join(format!("{old_blob_id}.blob"));
            let dst_new_blob = assets.join(format!("{new_blob_id}.blob"));
            if !dry_run {
                let old_blob = repo.objects.find_blob(&old_blob_id, &mut buf)?.data;
                std::fs::write(dst_old_blob, old_blob).or_error()?;

                let new_blob = repo.objects.find_blob(&new_blob_id, &mut buf)?.data;
                std::fs::write(dst_new_blob, new_blob).or_error()?;
            }

            blocks.push(format!(
                r#"git -c diff.algorithm=myers diff --no-index --no-ext-diff --no-color --indent-heuristic "$ROOT/{asset_dir}/{old_blob_id}.blob" "$ROOT/{asset_dir}/{new_blob_id}.blob" > {old_blob_id}-{new_blob_id}.myers.baseline || true
git -c diff.algorithm=myers diff --no-index --no-ext-diff --no-color --no-indent-heuristic "$ROOT/{asset_dir}/{old_blob_id}.blob" "$ROOT/{asset_dir}/{new_blob_id}.blob" > {old_blob_id}-{new_blob_id}.myers.no-indent.baseline || true
git -c diff.algorithm=histogram diff --no-index --no-ext-diff --no-color --indent-heuristic "$ROOT/{asset_dir}/{old_blob_id}.blob" "$ROOT/{asset_dir}/{new_blob_id}.blob" > {old_blob_id}-{new_blob_id}.histogram.baseline || true
git -c diff.algorithm=histogram diff --no-index --no-ext-diff --no-color --no-indent-heuristic "$ROOT/{asset_dir}/{old_blob_id}.blob" "$ROOT/{asset_dir}/{new_blob_id}.blob" > {old_blob_id}-{new_blob_id}.histogram.no-indent.baseline || true
cp "$ROOT/{asset_dir}/{old_blob_id}.blob" assets/
cp "$ROOT/{asset_dir}/{new_blob_id}.blob" assets/
"#
            ));
        }

        let script_file = destination_dir.join(script_name);
        eprintln!("{prefix} write script file at \"{}\"", script_file.display());

        if !dry_run {
            let script = blocks.join("\n");
            std::fs::write(script_file, script).or_error()?;
        }

        Ok(())
    }
}
