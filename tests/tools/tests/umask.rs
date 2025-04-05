#[test]
#[cfg(unix)]
#[cfg_attr(
    not(any(target_os = "linux", target_os = "android")),
    ignore = "The test itself uses /proc"
)]
fn umask() {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

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
