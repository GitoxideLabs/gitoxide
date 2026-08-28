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
    use std::io::{stderr, stdout};

    use anyhow::Result;
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
        std::io::Write::write_all(&mut stdout(), &out)?;
        std::io::Write::write_all(&mut stderr(), &err)?;
        res
    }

    #[cfg(feature = "tracing")]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum TraceFormat {
        Forest,
        Flat,
    }

    #[cfg(feature = "tracing")]
    fn trace_settings(trace: u8) -> Option<(TraceFormat, tracing_subscriber::filter::LevelFilter)> {
        use tracing_subscriber::filter::LevelFilter;

        match trace {
            1 => Some((TraceFormat::Forest, LevelFilter::INFO)),
            2 => Some((TraceFormat::Forest, LevelFilter::DEBUG)),
            3 => Some((TraceFormat::Flat, LevelFilter::DEBUG)),
            4 => Some((TraceFormat::Flat, LevelFilter::TRACE)),
            _ => None,
        }
    }

    #[cfg(feature = "tracing")]
    #[derive(Clone)]
    struct TraceWriter(TraceOutput);

    #[cfg(feature = "tracing")]
    struct TraceWriteGuard<'a>(std::sync::MutexGuard<'a, Vec<u8>>);

    #[cfg(feature = "tracing")]
    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for TraceWriter {
        type Writer = TraceWriteGuard<'a>;

        fn make_writer(&'a self) -> Self::Writer {
            TraceWriteGuard(self.0.lock().unwrap_or_else(std::sync::PoisonError::into_inner))
        }
    }

    #[cfg(feature = "tracing")]
    impl std::io::Write for TraceWriteGuard<'_> {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
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
    fn trace_subscriber(trace: u8, output: TraceOutput) -> anyhow::Result<Box<dyn tracing::Subscriber + Send + Sync>> {
        use tracing_subscriber::{Layer, layer::SubscriberExt};

        let (format, level) =
            trace_settings(trace).ok_or_else(|| anyhow::anyhow!("trace level must be between one and four"))?;
        Ok(match format {
            TraceFormat::Forest => {
                let printer = tracing_forest::Printer::new().writer(TraceWriter(output));
                Box::new(
                    tracing_subscriber::Registry::default()
                        .with(tracing_forest::ForestLayer::from(printer).with_filter(level)),
                )
            }
            TraceFormat::Flat => Box::new(
                tracing_subscriber::Registry::default().with(
                    tracing_subscriber::fmt::layer()
                        .with_ansi(true)
                        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
                        .with_writer(TraceWriter(output))
                        .with_filter(level),
                ),
            ),
        })
    }

    #[cfg(feature = "tracing")]
    pub(crate) fn init_tracing(trace: u8) -> anyhow::Result<TraceGuard> {
        if trace == 0 {
            return Ok(TraceGuard(None));
        }
        let output = TraceOutput::default();
        tracing::subscriber::set_global_default(trace_subscriber(trace, output.clone())?)?;
        Ok(TraceGuard(Some(output)))
    }

    #[cfg(not(feature = "tracing"))]
    pub(crate) fn init_tracing(trace: u8) -> anyhow::Result<TraceGuard> {
        anyhow::ensure!(trace == 0, "tracing support is not compiled in");
        Ok(TraceGuard(None))
    }

    #[cfg(all(test, feature = "tracing"))]
    mod tests {
        use std::io::Write;

        use anstream::{AutoStream, ColorChoice};
        use tracing_subscriber::filter::LevelFilter;

        use super::{TraceFormat, TraceOutput, trace_settings, trace_subscriber};

        #[test]
        fn trace_repetitions_choose_format_and_level() {
            assert_eq!(trace_settings(0), None);
            assert_eq!(trace_settings(1), Some((TraceFormat::Forest, LevelFilter::INFO)));
            assert_eq!(trace_settings(2), Some((TraceFormat::Forest, LevelFilter::DEBUG)));
            assert_eq!(trace_settings(3), Some((TraceFormat::Flat, LevelFilter::DEBUG)));
            assert_eq!(trace_settings(4), Some((TraceFormat::Flat, LevelFilter::TRACE)));
            assert_eq!(trace_settings(5), None);
        }

        #[test]
        fn flat_traces_include_closed_spans_in_the_deferred_output() -> anyhow::Result<()> {
            let output = TraceOutput::default();
            let subscriber = trace_subscriber(3, output.clone())?;
            tracing::subscriber::with_default(subscriber, || {
                let span = tracing::debug_span!("operation");
                let _entered = span.enter();
                tracing::debug!("visible event");
                tracing::trace!("filtered event");
            });
            let output = output.lock().expect("trace output lock is not poisoned");
            let output = String::from_utf8_lossy(&output);
            assert_eq!(
                output.matches("visible event").count(),
                1,
                "debug events are retained: {output}"
            );
            assert!(output.contains("close"), "span completion is retained: {output}");
            assert!(output.contains("\x1b["), "flat terminal traces contain ANSI styling");
            assert!(
                !output.contains("filtered event"),
                "the selected level still filters: {output}"
            );
            Ok(())
        }

        #[test]
        fn terminal_adaptation_preserves_or_strips_forest_colors() -> anyhow::Result<()> {
            let output = TraceOutput::default();
            let subscriber = trace_subscriber(1, output.clone())?;
            tracing::subscriber::with_default(subscriber, || tracing::info!("visible event"));
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
            Some(Box::new([PossibleValue::new("SHA1")].into_iter()))
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
                .try_map(|arg| -> Result<_, gix::pathspec::parse::Error> {
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
                .try_map(|arg| -> Result<_, gix::pathspec::parse::Error> {
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
                .try_map(|arg| gix::date::parse(&arg, Some(gix::date::Zoned::now())).map_err(gix::Exn::into_inner))
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
