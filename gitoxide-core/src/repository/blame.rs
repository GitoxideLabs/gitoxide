use std::{ffi::OsStr, path::PathBuf, str::Lines};

use gix::bstr::BStr;

pub fn blame_file(mut repo: gix::Repository, file: &OsStr, out: impl std::io::Write) -> anyhow::Result<()> {
    repo.object_cache_size_if_unset(repo.compute_object_cache_size_for_tree_diffs(&**repo.index_or_empty()?));

    let suspect = repo.head()?.peel_to_commit_in_place()?;
    let traverse: Vec<_> = gix::traverse::commit::Simple::new(Some(suspect.id), &repo.objects).collect();
    let mut resource_cache = repo.diff_resource_cache_for_tree_diff()?;

    let work_dir: PathBuf = repo.work_dir().expect("TODO").into();
    let file_path: &BStr = gix::path::os_str_into_bstr(file)?;

    let blame_entries = gix::blame::blame_file(
        &repo.objects,
        traverse,
        &mut resource_cache,
        suspect.id,
        work_dir.clone(),
        file_path,
    )
    .expect("TODO");

    let absolute_path = work_dir.join(file);
    let file_content = std::fs::read_to_string(absolute_path).expect("TODO");
    let lines = file_content.lines();

    write_blame_entries(out, lines, blame_entries)?;

    Ok(())
}

fn write_blame_entries(
    mut out: impl std::io::Write,
    mut lines: Lines<'_>,
    blame_entries: Vec<gix::blame::BlameEntry>,
) -> Result<(), std::io::Error> {
    for blame_entry in blame_entries {
        for line_number in blame_entry.range_in_blamed_file {
            let line = lines.next().unwrap();

            writeln!(
                out,
                "{} {} {}",
                blame_entry.commit_id.to_hex_with_len(8),
                // `line_number` is 0-based, but we want to show 1-based line numbers (as `git`
                // does).
                line_number + 1,
                line
            )?;
        }
    }

    Ok(())
}
