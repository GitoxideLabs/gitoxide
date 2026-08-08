use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use anyhow::{Context, Result};
use tracing_subscriber::{filter::Targets, prelude::*};

const FILE_PREFIX: &str = "tix.log";
const RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

pub(crate) fn init() -> Result<tracing::subscriber::DefaultGuard> {
    let directory = log_directory().context("could not determine the platform log directory")?;
    fs::create_dir_all(&directory)
        .with_context(|| format!("could not create log directory at {}", directory.display()))?;
    let cleanup_errors = prune(&directory, SystemTime::now());
    let appender = tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix(FILE_PREFIX)
        .build(&directory)
        .context("could not open the daily diagnostic log")?;
    let subscriber = tracing_subscriber::registry().with(
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_target(false)
            .with_writer(appender)
            .with_filter(
                Targets::new()
                    .with_default(tracing::Level::WARN)
                    .with_target("gix_tix", tracing::Level::DEBUG),
            ),
    );
    let guard = tracing::subscriber::set_default(subscriber);
    tracing::info!(path = %directory.display(), "initialized diagnostics");
    for error in cleanup_errors {
        tracing::warn!(%error, "could not prune an old diagnostic log");
    }
    Ok(guard)
}

#[cfg(target_os = "macos")]
fn log_directory() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|dirs| dirs.home_dir().join("Library/Logs/org.GitoxideLabs.tix"))
}

#[cfg(target_os = "linux")]
fn log_directory() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "GitoxideLabs", "tix").and_then(|dirs| dirs.state_dir().map(Path::to_owned))
}

#[cfg(target_os = "windows")]
fn log_directory() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|dirs| dirs.data_local_dir().join("GitoxideLabs/tix/logs"))
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn log_directory() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "GitoxideLabs", "tix").map(|dirs| dirs.data_local_dir().join("logs"))
}

fn prune(directory: &Path, now: SystemTime) -> Vec<String> {
    let mut errors = Vec::new();
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(err) => {
            errors.push(err.to_string());
            return errors;
        }
    };
    for entry in entries {
        let result = (|| -> std::io::Result<()> {
            let entry = entry?;
            let name = entry.file_name();
            if !name.to_string_lossy().starts_with(&format!("{FILE_PREFIX}.")) {
                return Ok(());
            }
            let age = now.duration_since(entry.metadata()?.modified()?).unwrap_or_default();
            if age > RETENTION {
                fs::remove_file(entry.path())?;
            }
            Ok(())
        })();
        if let Err(err) = result {
            errors.push(err.to_string());
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use std::{fs::File, time::UNIX_EPOCH};

    use super::*;

    #[test]
    fn prunes_only_expired_daily_logs() -> gix_testtools::Result {
        let directory = std::env::temp_dir().join(format!(
            "gix-tix-log-prune-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        fs::create_dir(&directory)?;
        let old = directory.join("tix.log.older");
        let recent = directory.join("tix.log.recent");
        let unrelated = directory.join("other.log.old");
        File::create(&old)?.set_modified(UNIX_EPOCH)?;
        File::create(&recent)?;
        File::create(&unrelated)?.set_modified(UNIX_EPOCH)?;

        assert!(prune(&directory, SystemTime::now()).is_empty());
        assert!(!old.exists(), "expired tix logs are removed");
        assert!(recent.exists(), "recent tix logs are retained");
        assert!(unrelated.exists(), "unrelated files are retained");
        fs::remove_dir_all(directory)?;
        Ok(())
    }
}
