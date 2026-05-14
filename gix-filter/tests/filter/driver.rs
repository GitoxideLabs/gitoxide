static DRIVER: &str = concat!(env!("CARGO"), " run --example arrow");

mod baseline {
    use serial_test::serial;

    use crate::driver::DRIVER;

    #[serial]
    #[test]
    fn our_implementation_used_by_git() -> crate::Result {
        let mut exe = DRIVER.to_owned();
        if cfg!(windows) {
            exe = exe.replace('\\', "/");
        }
        gix_testtools::scripted_fixture_read_only_with_args_single_archive("baseline.sh", [exe])?;
        Ok(())
    }
}

mod shutdown {
    use std::time::Duration;

    use gix_filter::driver::{Operation, Process, shutdown::Mode};

    use crate::driver::apply::driver_with_process;

    pub(crate) fn extract_client(
        res: Option<gix_filter::driver::Process<'_>>,
    ) -> &mut gix_filter::driver::process::Client {
        match res {
            Some(Process::SingleFile { .. }) | None => {
                unreachable!("process is configured")
            }
            Some(Process::MultiFile { client, .. }) => client,
        }
    }

    #[test]
    fn ignore_when_waiting() -> crate::Result {
        let mut state = gix_filter::driver::State::default();
        let driver = driver_with_process();
        let client = extract_client(state.maybe_launch_process(&driver, Operation::Clean, "does not matter".into())?);

        assert!(
            client
                .invoke("wait-1-s", &mut None.into_iter(), &mut &b""[..])?
                .is_success(),
            "this lets the process wait for a second using our hidden command"
        );

        let start = std::time::Instant::now();
        assert_eq!(state.shutdown(Mode::Ignore)?.len(), 1, "we only launch one process");
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "when ignoring processes, there should basically be no wait time"
        );
        Ok(())
    }
}

pub(crate) mod apply {
    use std::{io::Read, sync::LazyLock};

    use bstr::{BStr, BString, ByteSlice};
    use gix_filter::{
        Driver, driver,
        driver::{Operation, apply, apply::Delay},
    };
    use serial_test::serial;

    use crate::driver::{DRIVER, shutdown::extract_client};

    fn driver_no_process() -> Driver {
        let mut driver = driver_with_process();
        driver.process = None;
        driver
    }

    pub(crate) fn driver_with_process() -> Driver {
        let mut exe = DRIVER.to_owned();
        if cfg!(windows) {
            exe = exe.replace('\\', "/");
        }
        Driver {
            name: "arrow".into(),
            clean: Some((exe.clone() + " clean %f").into()),
            smudge: Some((exe.clone() + " smudge %f").into()),
            process: Some((exe + " process").into()),
            required: true,
        }
    }

    #[serial]
    #[test]
    fn missing_driver_means_no_filter_is_applied() -> crate::Result {
        let mut state = gix_filter::driver::State::default();
        let mut driver = driver_no_process();
        driver.smudge = None;
        assert!(
            state
                .apply(
                    &driver,
                    &mut std::io::empty(),
                    Operation::Smudge,
                    context_from_path("ignored")
                )?
                .is_none()
        );

        driver.clean = None;
        assert!(
            state
                .apply(
                    &driver,
                    &mut std::io::empty(),
                    Operation::Clean,
                    context_from_path("ignored")
                )?
                .is_none()
        );
        Ok(())
    }

    #[serial]
    #[test]
    fn a_crashing_process_can_restart_it() -> crate::Result {
        let mut state = gix_filter::driver::State::default();
        let driver = driver_with_process();
        let err = match state.apply(
            &driver,
            &mut std::io::empty(),
            Operation::Smudge,
            context_from_path("fail"),
        ) {
            Ok(_) => panic!("expecting an error as invalid context was passed"),
            Err(err) => err,
        };
        assert!(
            matches!(err, gix_filter::driver::apply::Error::ProcessInvoke { .. }),
            "{err:?}: cannot invoke if failure is requested"
        );

        let mut filtered = state
            .apply(
                &driver,
                &mut std::io::empty(),
                Operation::Smudge,
                context_from_path("fine"),
            )
            .expect("process restarts fine")
            .expect("filter applied");
        let mut buf = Vec::new();
        filtered.read_to_end(&mut buf)?;
        assert_eq!(buf.len(), 0, "nothing was done if input is empty, but it was applied");
        Ok(())
    }

    #[serial]
    #[test]
    fn process_status_abort_disables_capability() -> crate::Result {
        let mut state = gix_filter::driver::State::default();
        let driver = driver_with_process();
        let client = extract_client(state.maybe_launch_process(&driver, Operation::Clean, "does not matter".into())?);

        assert!(
            client
                .invoke("next-smudge-aborts", &mut None.into_iter(), &mut &b""[..])?
                .is_success()
        );
        assert!(
            matches!(state.apply(&driver, &mut std::io::empty(), Operation::Smudge, context_from_path("any")), Err(driver::apply::Error::ProcessStatus {status: driver::process::Status::Named(name), ..}) if name == "abort")
        );
        assert!(
            state
                .apply(
                    &driver,
                    &mut std::io::empty(),
                    Operation::Smudge,
                    context_from_path("any")
                )?
                .is_none(),
            "smudge is now disabled permanently"
        );
        Ok(())
    }

    #[serial]
    #[test]
    fn process_status_strange_shuts_down_process() -> crate::Result {
        let mut state = gix_filter::driver::State::default();
        let driver = driver_with_process();
        let client = extract_client(state.maybe_launch_process(&driver, Operation::Clean, "does not matter".into())?);

        assert!(
            client
                .invoke(
                    "next-invocation-returns-strange-status-and-smudge-fails-permanently",
                    &mut None.into_iter(),
                    &mut &b""[..]
                )?
                .is_success()
        );
        assert!(
            matches!(state.apply(&driver, &mut std::io::empty(), Operation::Smudge, context_from_path("any")), Err(driver::apply::Error::ProcessStatus {status: driver::process::Status::Named(name), ..}) if name == "send-term-signal")
        );
        let mut filtered = state
            .apply(&driver, &mut &b"hi\n"[..], Operation::Smudge, context_from_path("any"))?
            .expect("the process won't fail as it got restarted");
        let mut buf = Vec::new();
        filtered.read_to_end(&mut buf)?;
        assert_eq!(buf.as_bstr(), "➡hi\n", "the process works again as expected");
        Ok(())
    }

    #[serial]
    #[test]
    fn smudge_and_clean_failure_is_translated_to_observable_error_for_required_drivers() -> crate::Result {
        let mut state = gix_filter::driver::State::default();
        let driver = driver_no_process();
        assert!(driver.required);

        let mut filtered = state
            .apply(
                &driver,
                &mut &b"hello\nthere\n"[..],
                driver::Operation::Smudge,
                context_from_path("do/fail"),
            )?
            .expect("filter present");
        let mut buf = Vec::new();
        let err = filtered.read_to_end(&mut buf).unwrap_err();
        assert!(err.to_string().ends_with(" failed"));

        Ok(())
    }

    #[serial]
    #[test]
    fn smudge_and_clean_failure_means_nothing_if_required_is_false() -> crate::Result {
        let mut state = gix_filter::driver::State::default();
        let mut driver = driver_no_process();
        driver.required = false;

        let mut filtered = state
            .apply(
                &driver,
                &mut &b"hello\nthere\n"[..],
                driver::Operation::Clean,
                context_from_path("do/fail"),
            )?
            .expect("filter present");
        let num_read = std::io::copy(&mut filtered, &mut std::io::sink())?;
        assert_eq!(
            num_read, 0,
            "the example fails right away so no output is produced to stdout"
        );

        Ok(())
    }

    #[serial]
    #[test]
    fn smudge_and_clean_series() -> crate::Result {
        let mut state = gix_filter::driver::State::default();
        for mut driver in [driver_no_process(), driver_with_process()] {
            assert!(
                driver.required,
                "we want errors to definitely show, and don't expect them"
            );
            if driver.process.is_none() {
                // on CI on MacOS, the process seems to actually exit with non-zero status, let's see if this fixes it.
                driver.required = false;
            }

            let input = "hello\nthere\n";
            let mut filtered = state
                .apply(
                    &driver,
                    &mut input.as_bytes(),
                    driver::Operation::Smudge,
                    context_from_path("some/path.txt"),
                )?
                .expect("filter present");
            let mut buf = Vec::new();
            filtered.read_to_end(&mut buf)?;
            drop(filtered);
            assert_eq!(
                buf.as_bstr(),
                "➡hello\n➡there\n",
                "arrow applies indentation in smudge mode"
            );

            let smudge_result = buf.clone();
            let mut filtered = state
                .apply(
                    &driver,
                    &mut smudge_result.as_bytes(),
                    driver::Operation::Clean,
                    context_from_path("some/path.txt"),
                )?
                .expect("filter present");
            buf.clear();
            filtered.read_to_end(&mut buf)?;
            assert_eq!(
                buf.as_bstr(),
                input,
                "the clean filter reverses the smudge filter (and we call the right one)"
            );
        }
        state.shutdown(gix_filter::driver::shutdown::Mode::WaitForProcesses)?;
        Ok(())
    }

    #[test]
    fn smudge_and_clean_delayed() -> crate::Result {
        let mut state = gix_filter::driver::State::default();
        let driver = driver_with_process();
        let input = "hello\nthere\n";
        let process_key = extract_delayed_key(state.apply_delayed(
            &driver,
            &mut input.as_bytes(),
            driver::Operation::Smudge,
            Delay::Allow,
            context_from_path("sub/a.txt"),
        )?);

        let paths = state.list_delayed_paths(&process_key)?;
        assert_eq!(
            paths.len(),
            1,
            "delayed paths have to be queried again and are available until that happens"
        );
        assert_eq!(paths[0], "sub/a.txt");

        let mut filtered = state.fetch_delayed(&process_key, paths[0].as_ref(), driver::Operation::Smudge)?;
        let mut buf = Vec::new();
        filtered.read_to_end(&mut buf)?;
        drop(filtered);
        assert_eq!(
            buf.as_bstr(),
            "➡hello\n➡there\n",
            "arrow applies indentation also in delayed mode"
        );

        let paths = state.list_delayed_paths(&process_key)?;
        assert_eq!(paths.len(), 0, "delayed paths are consumed once fetched");

        let process_key = extract_delayed_key(state.apply_delayed(
            &driver,
            &mut buf.as_bytes(),
            driver::Operation::Clean,
            Delay::Allow,
            context_from_path("sub/b.txt"),
        )?);

        let paths = state.list_delayed_paths(&process_key)?;
        assert_eq!(
            paths.len(),
            1,
            "we can do another round of commands with the same process (at least if the implementation supports it), it's probably not done in practice"
        );
        assert_eq!(paths[0], "sub/b.txt");

        let mut filtered = state.fetch_delayed(&process_key, paths[0].as_ref(), driver::Operation::Clean)?;
        let mut buf = Vec::new();
        filtered.read_to_end(&mut buf)?;
        drop(filtered);
        assert_eq!(
            buf.as_bstr(),
            input,
            "it's possible to apply clean in delayed mode as well"
        );

        let paths = state.list_delayed_paths(&process_key)?;
        assert_eq!(paths.len(), 0, "delayed paths are consumed once fetched");

        state.shutdown(gix_filter::driver::shutdown::Mode::WaitForProcesses)?;
        Ok(())
    }

    #[serial]
    #[test]
    fn large_file_with_cat_filter_does_not_hang() -> crate::Result {
        // This test reproduces issue #2080 where using `cat` as a filter with a large file
        // causes a deadlock. The pipe buffer is typically 64KB on Linux, so we use files
        // larger than that to ensure the buffer fills up.

        // Typical pipe buffer sizes on Unix systems
        const PIPE_BUFFER_SIZE: usize = 64 * 1024; // 64KB

        let mut state = gix_filter::driver::State::default();

        // Create a driver that uses `cat` command (which echoes input to output immediately)
        let driver = Driver {
            name: "cat".into(),
            clean: Some(cat_invocation().to_owned()),
            smudge: Some(cat_invocation().to_owned()),
            process: None,
            required: false,
        };

        // Test with multiple sizes to ensure robustness
        for size in [
            PIPE_BUFFER_SIZE,
            2 * PIPE_BUFFER_SIZE,
            8 * PIPE_BUFFER_SIZE,
            16 * PIPE_BUFFER_SIZE,
        ] {
            let input = vec![b'a'; size];

            // Apply the filter - this should not hang
            let mut filtered = state
                .apply(
                    &driver,
                    &mut input.as_slice(),
                    driver::Operation::Smudge,
                    context_from_path("large-file.txt"),
                )?
                .expect("filter present");

            let mut output = Vec::new();
            filtered.read_to_end(&mut output)?;

            assert_eq!(
                input.len(),
                output.len(),
                "cat should pass through all data unchanged for {size} bytes"
            );
            assert_eq!(input, output, "cat should not modify the data");
        }
        Ok(())
    }

    #[serial]
    #[test]
    fn large_file_with_cat_filter_early_drop() -> crate::Result {
        // Test that dropping the reader early doesn't cause issues (thread cleanup)
        let mut state = gix_filter::driver::State::default();

        let driver = Driver {
            name: "cat".into(),
            clean: Some(cat_invocation().to_owned()),
            smudge: Some(cat_invocation().to_owned()),
            process: None,
            required: false,
        };

        let input = vec![b'x'; 256 * 1024];

        // Apply the filter but only read a small amount
        let mut filtered = state
            .apply(
                &driver,
                &mut input.as_slice(),
                driver::Operation::Clean,
                context_from_path("early-drop.txt"),
            )?
            .expect("filter present");

        let mut output = vec![0u8; 100];
        filtered.read_exact(&mut output)?;
        assert_eq!(output, vec![b'x'; 100], "should read first 100 bytes");

        // Drop the reader early - the thread should still clean up properly
        drop(filtered);

        Ok(())
    }

    fn extract_delayed_key(res: Option<apply::MaybeDelayed<'_>>) -> driver::Key {
        match res {
            Some(apply::MaybeDelayed::Immediate(_)) | None => {
                unreachable!("must use process that supports delaying")
            }
            Some(apply::MaybeDelayed::Delayed(key)) => key,
        }
    }

    fn context_from_path(path: &str) -> apply::Context<'_, '_> {
        apply::Context {
            rela_path: path.into(),
            ref_name: None,
            treeish: None,
            blob: None,
        }
    }

    fn cat_invocation() -> &'static BStr {
        if cfg!(windows) {
            static CAT: LazyLock<Option<BString>> = LazyLock::new(|| {
                gix_command::prepare("command -v cat | cygpath --mixed --file -")
                    .with_shell()
                    .spawn()
                    .ok()?
                    .wait_with_output()
                    .ok()
                    .filter(|output| output.status.success())?
                    .stdout
                    .strip_suffix(b"\n")
                    .map(BStr::new)
                    .map(gix_quote::single)
            });
            CAT.as_deref().map_or_else(|| b"cat.exe".into(), BStr::new)
        } else {
            b"cat".into()
        }
    }
}
