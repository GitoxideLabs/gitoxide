#[cfg(feature = "prodash-render-line")]
pub const DEFAULT_FRAME_RATE: f32 = 6.0;

pub type ProgressRange = std::ops::RangeInclusive<prodash::progress::key::Level>;
pub const STANDARD_RANGE: ProgressRange = 2..=2;

/// If verbose is true, the env logger will be forcibly set to 'info' logging level. Otherwise env logging facilities
/// will just be initialized.
pub fn init_env_logger() {
    if cfg!(feature = "small") {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
            .format_module_path(false)
            .init();
    } else {
        env_logger::init();
    }
}

#[cfg(feature = "prodash-render-line")]
pub fn progress_tree() -> std::sync::Arc<prodash::tree::Root> {
    prodash::tree::root::Options {
        message_buffer_capacity: 200,
        ..Default::default()
    }
    .into()
}

#[cfg(not(feature = "prodash-render-line"))]
pub struct LogCreator;

#[cfg(not(feature = "prodash-render-line"))]
impl LogCreator {
    pub fn add_child(&self, name: &str) -> prodash::progress::Log {
        prodash::progress::Log::new(name, Some(1))
    }
}

#[cfg(not(feature = "prodash-render-line"))]
fn progress_tree() -> LogCreator {
    LogCreator
}

#[cfg(feature = "pretty-cli")]
pub mod pretty {
    use std::io::{self, stderr, stdout};

    use gix::{Result, error::ResultExt};
    use gix_features::progress;

    use crate::shared::ProgressRange;

    pub fn prepare_and_run<T>(
        name: &str,
        verbose: bool,
        range: impl Into<Option<ProgressRange>>,
        run: impl FnOnce(
            progress::DoOrDiscard<prodash::tree::Item>,
            &mut dyn std::io::Write,
            &mut dyn std::io::Write,
        ) -> Result<T>,
    ) -> Result<T> {
        crate::shared::init_env_logger();

        if !verbose {
            let stdout = stdout();
            let mut stdout_lock = stdout.lock();
            return gix::trace::coarse!("run")
                .into_scope(|| run(progress::DoOrDiscard::from(None), &mut stdout_lock, &mut stderr()));
        }

        let progress = crate::shared::progress_tree();
        let sub_progress = progress.add_child(name);

        use crate::shared::{self, STANDARD_RANGE};
        let handle = shared::setup_line_renderer_range(&progress, range.into().unwrap_or(STANDARD_RANGE));

        let mut out = Vec::<u8>::new();
        let mut err = Vec::<u8>::new();
        let res = gix::trace::coarse!("run")
            .into_scope(|| run(progress::DoOrDiscard::from(Some(sub_progress)), &mut out, &mut err));
        handle.shutdown_and_wait();
        write_output(&out, &err, &mut stdout(), &mut stderr()).or_error()?;
        res
    }

    fn write_output(out: &[u8], err: &[u8], stdout: &mut dyn io::Write, stderr: &mut dyn io::Write) -> io::Result<()> {
        let stdout_result = stdout.write_all(out);
        let stderr_result = stderr.write_all(err);
        stdout_result.and(stderr_result)
    }

    #[cfg(test)]
    mod output_tests {
        use super::{io, write_output};

        #[test]
        fn stderr_is_written_even_when_stdout_fails() {
            let mut stdout = &mut [][..];
            let mut stderr = Vec::new();
            let error = write_output(b"output", b"diagnostic", &mut stdout, &mut stderr)
                .expect_err("the empty stdout slice cannot accept output");
            assert_eq!(error.kind(), io::ErrorKind::WriteZero, "stdout errors are retained");
            assert_eq!(stderr, b"diagnostic", "stderr is written despite the stdout error");
        }
    }

    pub(crate) type TraceOutput = std::sync::Arc<std::sync::Mutex<Vec<u8>>>;

    pub(crate) struct TraceGuard(
        #[cfg_attr(
            not(any(feature = "tracing", feature = "gitoxide-core-tools-corpus")),
            allow(dead_code)
        )]
        Option<TraceOutput>,
    );

    #[cfg(feature = "gitoxide-core-tools-corpus")]
    impl TraceGuard {
        pub(crate) fn output(&self) -> Option<TraceOutput> {
            self.0.clone()
        }
    }

    #[cfg(feature = "tracing")]
    impl Drop for TraceGuard {
        fn drop(&mut self) {
            use std::io::Write;

            let Some(output) = self.0.as_ref() else {
                return;
            };
            let output = output.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            let _ = anstream::stderr().write_all(&output);
        }
    }

    #[cfg(feature = "tracing")]
    pub(crate) fn init_tracing(trace: u8) -> Result<TraceGuard> {
        if trace == 0 {
            return Ok(TraceGuard(None));
        }
        let output = TraceOutput::default();
        tracing::dispatcher::set_global_default(gitoxide_core::trace::subscriber(trace, output.clone(), None)?)
            .or_error()?;
        Ok(TraceGuard(Some(output)))
    }

    #[cfg(not(feature = "tracing"))]
    pub(crate) fn init_tracing(trace: u8) -> Result<TraceGuard> {
        use gix::error::bail;

        if trace != 0 {
            bail!(gix::error::unsupported("tracing support is not compiled in"));
        }
        Ok(TraceGuard(None))
    }

    #[cfg(all(test, feature = "tracing"))]
    mod tests {
        use std::io::Write;

        use anstream::{AutoStream, ColorChoice};
        use gix::error::TestResult;

        use super::TraceOutput;

        #[test]
        fn terminal_adaptation_preserves_or_strips_forest_colors() -> TestResult {
            let output = TraceOutput::default();
            let dispatch = gitoxide_core::trace::subscriber(1, output.clone(), None)?;
            tracing::dispatcher::with_default(&dispatch, || tracing::info!("visible event"));
            let output = output.lock().expect("trace output lock is not poisoned");
            assert!(
                output.windows(2).any(|bytes| bytes == b"\x1b["),
                "forest terminal traces contain ANSI styling"
            );

            for (choice, colored) in [(ColorChoice::AlwaysAnsi, true), (ColorChoice::Never, false)] {
                let mut stream = AutoStream::new(Vec::new(), choice);
                stream.write_all(&output)?;
                assert_eq!(
                    stream.into_inner().windows(2).any(|bytes| bytes == b"\x1b["),
                    colored,
                    "terminal adaptation follows its color choice"
                );
            }
            Ok(())
        }
    }
}

#[cfg(feature = "prodash-render-line")]
pub fn setup_line_renderer_range(
    progress: &std::sync::Arc<prodash::tree::Root>,
    levels: std::ops::RangeInclusive<prodash::progress::key::Level>,
) -> prodash::render::line::JoinHandle {
    prodash::render::line(
        std::io::stderr(),
        std::sync::Arc::downgrade(progress),
        prodash::render::line::Options {
            level_filter: Some(levels),
            frames_per_second: DEFAULT_FRAME_RATE,
            initial_delay: Some(std::time::Duration::from_secs(1)),
            timestamp: true,
            throughput: true,
            hide_cursor: true,
            ..prodash::render::line::Options::default()
        }
        .auto_configure(prodash::render::line::StreamKind::Stderr),
    )
}

mod clap {
    use std::{ffi::OsStr, str::FromStr};

    use clap::{Arg, Command, Error, builder, builder::PossibleValue, error::ErrorKind};
    use gitoxide_core as core;
    use gix::bstr::BString;

    #[derive(Clone)]
    pub struct AsBString;

    impl builder::TypedValueParser for AsBString {
        type Value = BString;

        fn parse_ref(&self, _cmd: &Command, _arg: Option<&Arg>, value: &OsStr) -> Result<Self::Value, Error> {
            gix::env::os_str_to_bstring(value).ok_or_else(|| Error::new(ErrorKind::InvalidUtf8))
        }
    }

    #[derive(Clone)]
    pub struct AsOutputFormat;

    impl builder::TypedValueParser for AsOutputFormat {
        type Value = core::OutputFormat;

        fn parse_ref(&self, cmd: &Command, arg: Option<&Arg>, value: &OsStr) -> Result<Self::Value, Error> {
            builder::StringValueParser::new()
                .try_map(|arg| core::OutputFormat::from_str(&arg))
                .parse_ref(cmd, arg, value)
        }

        fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
            Some(Box::new(core::OutputFormat::variants().iter().map(PossibleValue::new)))
        }
    }

    #[derive(Clone)]
    pub struct AsHashKind;

    impl builder::TypedValueParser for AsHashKind {
        type Value = gix::hash::Kind;

        fn parse_ref(&self, cmd: &Command, arg: Option<&Arg>, value: &OsStr) -> Result<Self::Value, Error> {
            builder::StringValueParser::new()
                .try_map(|arg| gix::hash::Kind::from_str(&arg))
                .parse_ref(cmd, arg, value)
        }

        fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
            #[cfg(all(feature = "sha1", not(feature = "sha256")))]
            {
                Some(Box::new([PossibleValue::new("SHA1")].into_iter()))
            }
            #[cfg(all(feature = "sha256", not(feature = "sha1")))]
            {
                Some(Box::new([PossibleValue::new("SHA256")].into_iter()))
            }
            #[cfg(all(feature = "sha256", feature = "sha1"))]
            {
                Some(Box::new(
                    [PossibleValue::new("SHA1"), PossibleValue::new("SHA256")].into_iter(),
                ))
            }
        }
    }

    use clap::builder::{OsStringValueParser, StringValueParser, TypedValueParser};

    #[derive(Clone)]
    pub struct AsPathSpec;

    static PATHSPEC_DEFAULTS: std::sync::LazyLock<gix::pathspec::Defaults> = std::sync::LazyLock::new(|| {
        gix::pathspec::Defaults::from_environment(&mut |n| std::env::var_os(n)).unwrap_or_default()
    });

    impl TypedValueParser for AsPathSpec {
        type Value = BString;

        fn parse_ref(&self, cmd: &Command, arg: Option<&Arg>, value: &OsStr) -> Result<Self::Value, Error> {
            OsStringValueParser::new()
                .try_map(|arg| -> gix::Result<_> {
                    let arg = gix::path::into_bstr(std::path::PathBuf::from(arg));
                    gix::pathspec::parse(arg.as_ref(), *PATHSPEC_DEFAULTS)?;
                    Ok(arg.into_owned())
                })
                .parse_ref(cmd, arg, value)
        }
    }

    pub fn parse_pathspec_argument(value: BString) -> gix::pathspec::Pattern {
        gix::pathspec::parse(value.as_ref(), *PATHSPEC_DEFAULTS)
            .expect("AsPathSpec validated the pathspec before storing its argument")
    }

    #[derive(Clone)]
    pub struct CheckPathSpec;

    impl TypedValueParser for CheckPathSpec {
        type Value = BString;

        fn parse_ref(&self, cmd: &Command, arg: Option<&Arg>, value: &OsStr) -> Result<Self::Value, Error> {
            OsStringValueParser::new()
                .try_map(|arg| -> gix::Result<_> {
                    let arg = gix::path::into_bstr(std::path::PathBuf::from(arg));
                    gix::pathspec::parse(arg.as_ref(), Default::default())?;
                    Ok(arg.into_owned())
                })
                .parse_ref(cmd, arg, value)
        }
    }

    #[derive(Clone)]
    pub struct ParseRenameFraction;

    impl TypedValueParser for ParseRenameFraction {
        type Value = f32;

        fn parse_ref(&self, cmd: &Command, arg: Option<&Arg>, value: &OsStr) -> Result<Self::Value, Error> {
            StringValueParser::new()
                .try_map(|arg: String| -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
                    if arg.ends_with('%') {
                        let val = u32::from_str(&arg[..arg.len() - 1])?;
                        Ok(val as f32 / 100.0)
                    } else {
                        let val = u32::from_str(&arg)?;
                        let num = format!("0.{val}");
                        Ok(f32::from_str(&num)?)
                    }
                })
                .parse_ref(cmd, arg, value)
        }
    }

    #[derive(Clone)]
    pub struct AsTime;

    impl TypedValueParser for AsTime {
        type Value = gix::date::Time;

        fn parse_ref(&self, cmd: &Command, arg: Option<&Arg>, value: &OsStr) -> Result<Self::Value, Error> {
            StringValueParser::new()
                .try_map(|arg| gix::date::parse(&arg, Some(gix::date::Zoned::now())))
                .parse_ref(cmd, arg, value)
        }
    }

    #[derive(Clone)]
    pub struct AsPartialRefName;

    impl TypedValueParser for AsPartialRefName {
        type Value = gix::refs::PartialName;

        fn parse_ref(&self, cmd: &Command, arg: Option<&Arg>, value: &OsStr) -> Result<Self::Value, Error> {
            AsBString
                .try_map(gix::refs::PartialName::try_from)
                .parse_ref(cmd, arg, value)
        }
    }

    #[derive(Clone)]
    pub struct AsRange;

    impl TypedValueParser for AsRange {
        type Value = std::ops::RangeInclusive<u32>;

        fn parse_ref(&self, cmd: &Command, arg: Option<&Arg>, value: &OsStr) -> Result<Self::Value, Error> {
            StringValueParser::new()
                .try_map(|arg| -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
                    let parts = arg.split_once(',');
                    if let Some((start, end)) = parts {
                        let start = u32::from_str(start)?;
                        let end = u32::from_str(end)?;

                        if start <= end {
                            return Ok(start..=end);
                        }
                    }

                    Err(Box::new(Error::new(ErrorKind::ValueValidation)))
                })
                .parse_ref(cmd, arg, value)
        }
    }
}
pub use self::clap::{
    AsBString, AsHashKind, AsOutputFormat, AsPartialRefName, AsPathSpec, AsRange, AsTime, CheckPathSpec,
    ParseRenameFraction, parse_pathspec_argument,
};

#[cfg(test)]
mod value_parser_tests {
    use clap::Parser;

    use super::{AsRange, AsTime, ParseRenameFraction};

    #[test]
    fn rename_fraction() {
        #[derive(Debug, clap::Parser)]
        pub struct Cmd {
            #[clap(long, short='a', value_parser = ParseRenameFraction)]
            pub arg: Option<Option<f32>>,
        }

        let c = Cmd::parse_from(["cmd", "-a"]);
        assert_eq!(c.arg, Some(None), "this means we need to fill in the default");

        let c = Cmd::parse_from(["cmd", "-a=50%"]);
        assert_eq!(c.arg, Some(Some(0.5)), "percentages become a fraction");

        let c = Cmd::parse_from(["cmd", "-a=100%"]);
        assert_eq!(c.arg, Some(Some(1.0)));

        let c = Cmd::parse_from(["cmd", "-a=5"]);
        assert_eq!(c.arg, Some(Some(0.5)), "another way to specify fractions");

        let c = Cmd::parse_from(["cmd", "-a=75"]);
        assert_eq!(c.arg, Some(Some(0.75)));
    }

    #[test]
    fn range() {
        #[derive(Debug, clap::Parser)]
        pub struct Cmd {
            #[clap(long, short='l', value_parser = AsRange)]
            pub arg: Option<std::ops::RangeInclusive<u32>>,
        }

        let c = Cmd::parse_from(["cmd", "-l=1,10"]);
        assert_eq!(c.arg, Some(1..=10));
    }

    #[test]
    fn since() {
        #[derive(Debug, clap::Parser)]
        pub struct Cmd {
            #[clap(long, long="since", value_parser = AsTime)]
            pub arg: Option<gix::date::Time>,
        }

        let c = Cmd::parse_from(["cmd", "--since", "2 weeks ago"]);
        assert!(matches!(c.arg, Some(gix::date::Time { .. })));
    }
}
