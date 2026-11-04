use std::time::Duration;

use gix_fsmonitor::client::daemon::Options;

use crate::{
    Result,
    bstr::BStr,
    config::{cache::util::ApplyLeniency, tree::gitoxide::FsMonitor},
};

pub(super) fn options(repo: &crate::Repository) -> Result<Options> {
    let integer = |key: &'static crate::config::tree::keys::UnsignedInteger| {
        repo.config.resolved.integer_filter(key, repo.filter_config_section())
    };
    let socket_timeout = FsMonitor::SOCKET_TIMEOUT
        .try_into_u64(integer(&FsMonitor::SOCKET_TIMEOUT))
        .with_leniency(repo.config.lenient_config)?
        .unwrap_or(FsMonitor::SOCKET_TIMEOUT_DEFAULT);
    Ok(Options {
        connect_retries: FsMonitor::CONNECT_RETRIES
            .try_into_usize(integer(&FsMonitor::CONNECT_RETRIES))
            .with_leniency(repo.config.lenient_config)?
            .unwrap_or(FsMonitor::CONNECT_RETRIES_DEFAULT),
        connect_retry_delay: Duration::from_millis(
            FsMonitor::CONNECT_RETRY_DELAY
                .try_into_u64(integer(&FsMonitor::CONNECT_RETRY_DELAY))
                .with_leniency(repo.config.lenient_config)?
                .unwrap_or(FsMonitor::CONNECT_RETRY_DELAY_DEFAULT),
        ),
        socket_timeout: (socket_timeout != 0).then(|| Duration::from_millis(socket_timeout)),
        max_response_size: FsMonitor::MAX_RESPONSE_SIZE
            .try_into_usize(integer(&FsMonitor::MAX_RESPONSE_SIZE))
            .with_leniency(repo.config.lenient_config)?
            .unwrap_or(FsMonitor::MAX_RESPONSE_SIZE_DEFAULT),
    })
}

pub(super) fn query(repo: &crate::Repository, token: &BStr, options: Options) -> Result<Vec<u8>> {
    let git_dir = gix_path::realpath_opts(repo.git_dir(), repo.current_dir(), gix_path::realpath::MAX_SYMLINKS)?;
    // shortcut: remote Git directories can use a different socket location; fall back to scanning until supported.
    gix_fsmonitor::client::daemon::query(
        &git_dir.join("fsmonitor--daemon.ipc"),
        token,
        || repo.command_context(),
        &options,
    )
}

#[cfg(test)]
mod tests {
    use gix_testtools::TestResult;

    use super::*;

    #[test]
    fn limits_use_trusted_configuration_and_validate_before_connecting() -> TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let root = gix_testtools::scripted_fixture_read_only("make_status_fsmonitor.sh")?;
        let overrides = [
            "gitoxide.fsmonitor.connectRetries=0",
            "gitoxide.fsmonitor.connectRetryDelay=0",
            "gitoxide.fsmonitor.socketTimeout=0",
            "gitoxide.fsmonitor.maxResponseSize=9",
        ];
        let repo = crate::open_opts(&root, crate::open::Options::isolated().config_overrides(overrides))?;
        let limits = options(&repo)?;
        assert_eq!(limits.connect_retries, 0, "zero disables connection retries");
        assert_eq!(limits.connect_retry_delay, Duration::ZERO, "zero retries immediately");
        assert_eq!(limits.socket_timeout, None, "zero disables socket timeouts");
        assert_eq!(limits.max_response_size, 9, "the response limit counts payload bytes");

        for open_options in [
            crate::open::Options::isolated()
                .config_overrides(overrides)
                .filter_config_section(|_| false),
            crate::open::Options::isolated().config_overrides(["gitoxide.fsmonitor.maxResponseSize=-1"]),
        ] {
            let repo = crate::open_opts(&root, open_options)?;
            let limits = options(&repo)?;
            assert_eq!(
                limits,
                Options::default(),
                "configuration and client defaults must agree"
            );
            assert_eq!(limits.connect_retries, FsMonitor::CONNECT_RETRIES_DEFAULT);
            assert_eq!(
                limits.connect_retry_delay,
                Duration::from_millis(FsMonitor::CONNECT_RETRY_DELAY_DEFAULT)
            );
            assert_eq!(
                limits.socket_timeout,
                Some(Duration::from_millis(FsMonitor::SOCKET_TIMEOUT_DEFAULT))
            );
            assert_eq!(limits.max_response_size, FsMonitor::MAX_RESPONSE_SIZE_DEFAULT);
        }

        for key in [
            "connectRetries",
            "connectRetryDelay",
            "socketTimeout",
            "maxResponseSize",
        ] {
            let repo = crate::open_opts(
                &root,
                crate::open::Options::isolated()
                    .strict_config(true)
                    .config_overrides([format!("gitoxide.fsmonitor.{key}=-1")]),
            )?;
            assert!(options(&repo).is_err(), "negative {key} is rejected before connecting");
        }
        Ok(())
    }
}
