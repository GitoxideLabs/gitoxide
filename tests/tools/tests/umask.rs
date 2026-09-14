#[test]
#[cfg(unix)]
fn subprocess_umask_leaves_parent_unchanged() -> std::io::Result<()> {
    let original = gix_testtools::umask();
    for mask in [0o077, 0o022] {
        if gix_testtools::run_with_umask(mask)? {
            assert_eq!(gix_testtools::umask(), mask, "the subprocess uses the requested mask");
        }
    }
    assert_eq!(gix_testtools::umask(), original, "the calling process keeps its mask");
    Ok(())
}

#[test]
#[cfg(unix)]
#[cfg_attr(
    not(any(target_os = "linux", target_os = "android")),
    ignore = "The test itself uses /proc"
)]
fn umask() {
    use std::{
        fs::File,
        io::{BufRead, BufReader},
    };

    use bstr::ByteSlice;
    // Check against the umask obtained via a less portable but also completely safe method.
    let less_portable = BufReader::new(File::open("/proc/self/status").expect("can open"))
        .split(b'\n')
        .find_map(|line| line.expect("can read").strip_prefix(b"Umask:\t").map(ToOwned::to_owned))
        .expect("has umask line")
        .to_str()
        .expect("umask line is valid UTF-8")
        .to_owned();
    let more_portable = format!("{:04o}", gix_testtools::umask());
    assert_eq!(more_portable, less_portable);
}
