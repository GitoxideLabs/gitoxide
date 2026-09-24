use crate::Result;
use gix_glob::pattern::Case;

mod create_directory;

mod attributes;
mod ignore;

fn probe_case() -> Result<Case> {
    Ok(
        if gix_fs::Capabilities::probe(
            &gix_discover::upwards(".".as_ref())?
                .0
                .into_repository_and_work_tree_directories()
                .0,
        )
        .ignore_case
        {
            Case::Fold
        } else {
            Case::Sensitive
        },
    )
}
