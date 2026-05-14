use filetime::FileTime;
use gix_index::entry::{
    Stat,
    stat::{Options, Time},
};

mod matches {
    use gix_index::entry::{
        Stat,
        stat::{Options, Time},
    };

    #[test]
    fn use_nsec() {
        let stat1 = Stat {
            mtime: Time { secs: 0, nsecs: 0 },
            ctime: Time { secs: 0, nsecs: 0 },
            dev: 0,
            ino: 0,
            uid: 0,
            gid: 0,
            size: 0,
        };
        let stat2 = Stat {
            mtime: Time { secs: 0, nsecs: 10 },
            ctime: Time { secs: 0, nsecs: 0 },
            dev: 0,
            ino: 0,
            uid: 0,
            gid: 0,
            size: 0,
        };

        assert!(
            stat1.matches(&stat2, Options::default()),
            "nsec differences don't matter without use_nsec"
        );
        assert!(
            !stat1.matches(
                &stat2,
                Options {
                    use_nsec: true,
                    ..Default::default()
                },
            ),
            "use_nsec works"
        );
        assert!(
            stat1.matches(
                &stat2,
                Options {
                    use_nsec: true,
                    check_stat: false,
                    ..Default::default()
                },
            ),
            "nsec differences don't matter without check_stat"
        );
    }

    #[test]
    fn use_ctime() {
        let stat1 = Stat {
            mtime: Time { secs: 0, nsecs: 0 },
            ctime: Time { secs: 1, nsecs: 2 },
            dev: 0,
            ino: 0,
            uid: 0,
            gid: 0,
            size: 0,
        };
        let mut stat2 = Stat {
            mtime: Time { secs: 0, nsecs: 0 },
            ctime: Time { secs: 3, nsecs: 4 },
            dev: 0,
            ino: 0,
            uid: 0,
            gid: 0,
            size: 0,
        };

        assert!(
            !stat1.matches(&stat2, Options::default()),
            "ctime is different so stat doesn't match (trust_ctime=true)"
        );
        assert!(
            stat1.matches(
                &stat2,
                Options {
                    trust_ctime: false,
                    ..Default::default()
                },
            ),
            "stat matches even tough ctime is different (trust_ctime=false)"
        );
        stat2.ctime.secs = 1;
        assert!(
            stat1.matches(&stat2, Options::default()),
            "ctime seconds are the same so stat matches (trust_ctime=true,use_nsec=false)"
        );
        assert!(
            !stat1.matches(
                &stat2,
                Options {
                    use_nsec: true,
                    ..Default::default()
                },
            ),
            "ctime nsecs are different so stat doesn't match (trust_ctime=true,use_nsec=false)"
        );
    }

    #[test]
    fn use_stdev() {
        let stat1 = Stat {
            mtime: Time { secs: 0, nsecs: 0 },
            ctime: Time { secs: 0, nsecs: 0 },
            dev: 0,
            ino: 0,
            uid: 0,
            gid: 0,
            size: 0,
        };
        let stat2 = Stat {
            mtime: Time { secs: 0, nsecs: 0 },
            ctime: Time { secs: 0, nsecs: 0 },
            dev: 1,
            ino: 0,
            uid: 0,
            gid: 0,
            size: 0,
        };

        assert!(
            stat1.matches(&stat2, Options::default()),
            "differences in dev number are ignored"
        );
        assert!(
            !stat1.matches(
                &stat2,
                Options {
                    use_stdev: true,
                    ..Default::default()
                },
            ),
            "differences in dev number change comparison result if use_stdev=true"
        );
    }

    #[test]
    fn check_stat() {
        let stat1 = Stat {
            mtime: Time { secs: 0, nsecs: 0 },
            ctime: Time { secs: 0, nsecs: 0 },
            dev: 0,
            ino: 0,
            uid: 0,
            gid: 0,
            size: 0,
        };
        let mut stat2 = stat1;
        assert!(
            stat1.matches(&stat2, Options::default()),
            "identical stats always match"
        );
        assert!(
            stat1.matches(
                &stat2,
                Options {
                    check_stat: false,
                    ..Default::default()
                },
            ),
            "identical stats always match"
        );
        stat2.ino = 1;
        assert!(
            !stat1.matches(&stat2, Options::default()),
            "inode is different => mismatch (check_stat=true)"
        );
        assert!(
            stat1.matches(
                &stat2,
                Options {
                    check_stat: false,
                    ..Default::default()
                },
            ),
            "inode difference doesnt' matter (check_stat=false)"
        );
        stat2 = stat1;
        stat2.uid = 1;
        assert!(
            !stat1.matches(&stat2, Options::default()),
            "uid is different => mismatch (check_stat=true)"
        );
        assert!(
            stat1.matches(
                &stat2,
                Options {
                    check_stat: false,
                    ..Default::default()
                },
            ),
            "uid difference doesnt' matter (check_stat=false)"
        );
        stat2 = stat1;
        stat2.gid = 1;
        assert!(
            !stat1.matches(&stat2, Options::default()),
            "gid is different => mismatch (check_stat=true)"
        );
        assert!(
            stat1.matches(
                &stat2,
                Options {
                    check_stat: false,
                    ..Default::default()
                },
            ),
            "gid difference doesnt' matter (check_stat=false)"
        );
        stat2 = stat1;
        stat2.size = 1;
        assert!(
            !stat1.matches(&stat2, Options::default()),
            "size is different => mismatch (check_stat=true)"
        );
        assert!(
            !stat1.matches(
                &stat2,
                Options {
                    check_stat: false,
                    ..Default::default()
                },
            ),
            "size is different => mismatch (check_stat=false)"
        );
    }
}

#[test]
fn is_racy() {
    let stat1 = Stat {
        mtime: Time { secs: 1, nsecs: 10 },
        ctime: Time { secs: 0, nsecs: 0 },
        dev: 0,
        ino: 0,
        uid: 0,
        gid: 0,
        size: 0,
    };
    assert!(
        stat1.is_racy(FileTime::from_unix_time(1, 0), Options::default()),
        "entry with mtime identical (seconds) to timestamp is racy (use_nsec=false)"
    );
    assert!(
        stat1.is_racy(
            FileTime::from_unix_time(1, 0),
            Options {
                use_nsec: true,
                ..Default::default()
            },
        ),
        "entry with mtime after timestamp (nanoseconds) is racy (use_nsec=true)"
    );
    assert!(
        stat1.is_racy(FileTime::from_unix_time(1, 10), Options::default()),
        "entry with mtime identical (seconds) to timestamp is racy (use_nsec=false)"
    );
    assert!(
        stat1.is_racy(
            FileTime::from_unix_time(1, 10),
            Options {
                use_nsec: true,
                ..Default::default()
            },
        ),
        "entry with mtime identical (seconds and nanseconds) to timestamp is racy (use_nsec=true)"
    );
    assert!(
        stat1.is_racy(FileTime::from_unix_time(1, 20), Options::default()),
        "entry with mtime identical (seconds) to timestamp is racy (use_nsec=false)"
    );
    assert!(
        !stat1.is_racy(
            FileTime::from_unix_time(1, 20),
            Options {
                use_nsec: true,
                ..Default::default()
            },
        ),
        "entry with mtime before (nanoseconds) timestamp is not racy (use_nsec=true)"
    );
    assert!(
        !stat1.is_racy(FileTime::from_unix_time(2, 0), Options::default()),
        "entry with mtime before (seconds) timestamp is not racy (use_nsec=false)"
    );
    assert!(
        !stat1.is_racy(
            FileTime::from_unix_time(2, 0),
            Options {
                use_nsec: true,
                ..Default::default()
            },
        ),
        "entry with mtime before (seconds) timestamp is not racy (use_nsec=true)"
    );
}
