use std::{ffi::OsStr, path::PathBuf};

use bstr::ByteSlice;
use gix_features::fs::walkdir::Parallelism;

#[test]
fn common_values_and_names_by_path() -> crate::Result {
    let modules = module_files()
        .map(|(path, stripped)| {
            gix_submodule::File::from_bytes(&std::fs::read(path).unwrap(), stripped, &Default::default())
        })
        .collect::<Result<Vec<_>, _>>()?;

    assert_eq!(
        modules
            .iter()
            .map(|m| { m.config_path().expect("present").to_owned() })
            .collect::<Vec<_>>(),
        [
            "empty-clone/.gitmodules",
            "multiple/.gitmodules",
            "not-a-submodule/.gitmodules",
            "recursive-clone/.gitmodules",
            "recursive-clone/submodule/.gitmodules",
            "relative-clone/.gitmodules",
            "relative-clone/submodule/.gitmodules",
            "super-clone/.gitmodules",
            "super-clone/submodule/.gitmodules",
            "super/.gitmodules",
            "super/submodule/.gitmodules",
            "top-only-clone/.gitmodules"
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect::<Vec<_>>(),
        "config_path() yields the path provided when instantiating (for .gitmodules), and not the path of a submodule."
    );

    assert_eq!(
        {
            let mut v = modules.iter().flat_map(gix_submodule::File::names).collect::<Vec<_>>();
            v.sort();
            v.dedup();
            v
        },
        [".a/..c", "a/b", r"a/d\", r"a\e", "submodule"]
            .into_iter()
            .map(|n| n.as_bytes().as_bstr())
            .collect::<Vec<_>>(),
        "names can be iterated"
    );

    for module in &modules {
        for name in module.names() {
            let path = module.path(name)?;
            assert_eq!(module.name_by_path(path.as_ref()).expect("found"), name);
        }
    }
    Ok(())
}

fn module_files() -> impl Iterator<Item = (PathBuf, PathBuf)> {
    let dir = gix_testtools::scripted_fixture_read_only("basic.sh").expect("valid fixture");
    gix_features::fs::walkdir_sorted_new(&dir, Parallelism::Serial, false)
        .follow_links(false)
        .into_iter()
        .filter_map(move |entry| {
            let entry = entry.unwrap();
            (entry.file_name() == OsStr::new(".gitmodules")).then(|| {
                (
                    entry.path().into_owned(),
                    entry
                        .path()
                        .strip_prefix(&dir)
                        .expect("can only provide sub-dirs")
                        .to_owned(),
                )
            })
        })
}
