#[test]
fn probe() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::File::create(dir.path().join("config")).unwrap();
    let caps = gix_fs::Capabilities::probe(dir.path());

    let entries: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_str() != Some("config"))
        .map(|e| e.path())
        .collect();
    assert_eq!(
        entries.len(),
        0,
        "there should be no left-over files after probing, found {entries:?}"
    );
    if cfg!(unix) {
        assert!(caps.symlink, "Unix should always be able to create symlinks");
        assert!(caps.executable_bit, "Unix should always honor executable bits");
    }
}

#[test]
fn parallel_probe() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::File::create(dir.path().join("config")).unwrap();
    let baseline = gix_fs::Capabilities::probe(dir.path());

    let (tx, rx) = crossbeam_channel::unbounded::<()>();
    let threads: Vec<_> = (0..10)
        .map(|_id| {
            std::thread::spawn({
                let dir = dir.path().to_owned();
                let rx = rx.clone();
                move || {
                    for _ in rx {}
                    let actual = gix_fs::Capabilities::probe(&dir);
                    assert_eq!(actual, baseline);
                }
            })
        })
        .collect();
    drop((rx, tx));
    for thread in threads {
        thread.join().expect("no panic");
    }
}
