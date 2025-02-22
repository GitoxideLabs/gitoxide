#[cfg(any(feature = "prodash-render-line", feature = "prodash-render-tui"))]
pub const DEFAULT_FRAME_RATE: f32 = 6.0;

#[allow(unused)]
pub type ProgressRange = std::ops::RangeInclusive<prodash::progress::key::Level>;
#[allow(unused)]
pub const STANDARD_RANGE: ProgressRange = 2..=2;

/// If verbose is true, the env logger will be forcibly set to 'info' logging level. Otherwise env logging facilities
/// will just be initialized.
#[allow(unused)] // Squelch warning because it's used in porcelain as well and we can't know that at compile time
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
pub fn progress_tree(trace: bool) -> std::sync::Arc<prodash::tree::Root> {
    prodash::tree::root::Options {
        message_buffer_capacity: if trace { 10_000 } else { 200 },
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

#[cfg(not(any(feature = "prodash-render-tui", feature = "prodash-render-line")))]
fn progress_tree() -> LogCreator {
    LogCreator
}

#[cfg(feature = "pretty-cli")]
pub mod pretty {
    use std::io::{stderr, stdout};

    use anyhow::Result;
    use gix_features::progress;

    use crate::shared::ProgressRange;

    #[cfg(feature = "small")]
    pub fn prepare_and_run<T>(
        name: &str,
        trace: bool,
        verbose: bool,
        progress: bool,
        #[cfg_attr(not(feature = "prodash-render-tui"), allow(unused_variables))] progress_keep_open: bool,
        range: impl Into<Option<ProgressRange>>,
        run: impl FnOnce(
            progress::DoOrDiscard<prodash::tree::Item>,
            &mut dyn std::io::Write,
            &mut dyn std::io::Write,
        ) -> Result<T>,
    ) -> Result<T> {
        crate::shared::init_env_logger();

        match (verbose, progress) {
            (false, false) => {
                let stdout = stdout();
                let mut stdout_lock = stdout.lock();
                let stderr = stderr();
                let mut stderr_lock = stderr.lock();
                run(progress::DoOrDiscard::from(None), &mut stdout_lock, &mut stderr_lock)
            }
            (true, false) => {
                let progress = crate::shared::progress_tree(trace);
                let sub_progress = progress.add_child(name);

                use crate::shared::{self, STANDARD_RANGE};
                let handle = shared::setup_line_renderer_range(&progress, range.into().unwrap_or(STANDARD_RANGE));

                let mut out = Vec::<u8>::new();
                let res = run(progress::DoOrDiscard::from(Some(sub_progress)), &mut out, &mut stderr());
                handle.shutdown_and_wait();
                std::io::Write::write_all(&mut stdout(), &out)?;
                res
            }
            #[cfg(not(feature = "prodash-render-tui"))]
            (_, true) => {
                unreachable!("BUG: This branch can't be run without a TUI built-in")
            }
        }
    }

    #[cfg(feature = "tracing")]
    fn init_tracing(
        enable: bool,
        reverse_lines: bool,
        progress: &gix::progress::prodash::tree::Root,
    ) -> anyhow::Result<()> {
        if enable {
            let processor = tracing_forest::Printer::new().formatter({
                let progress = std::sync::Mutex::new(progress.add_child("tracing"));
                move |tree: &tracing_forest::tree::Tree| -> Result<String, std::fmt::Error> {
                    use gix::Progress;
                    use tracing_forest::Formatter;
                    let progress = &mut progress.lock().unwrap();
                    let tree = tracing_forest::printer::Pretty.fmt(tree)?;
                    if reverse_lines {
                        for line in tree.lines().rev() {
                            progress.info(line.into());
                        }
                    } else {
                        for line in tree.lines() {
                            progress.info(line.into());
                        }
                    }
                    Ok(String::new())
                }
            });
            use tracing_subscriber::layer::SubscriberExt;
            let subscriber = tracing_subscriber::Registry::default().with(tracing_forest::ForestLayer::from(processor));
            tracing::subscriber::set_global_default(subscriber)?;
        } else {
            tracing::subscriber::set_global_default(tracing_subscriber::Registry::default())?;
        }
        Ok(())
    }

    #[cfg(not(feature = "small"))]
    pub fn prepare_and_run<T: Send + 'static>(
        name: &str,
        trace: bool,
        verbose: bool,
        progress: bool,
        #[cfg_attr(not(feature = "prodash-render-tui"), allow(unused_variables))] progress_keep_open: bool,
        range: impl Into<Option<ProgressRange>>,
        run: impl FnOnce(
                progress::DoOrDiscard<prodash::tree::Item>,
                &mut dyn std::io::Write,
                &mut dyn std::io::Write,
            ) -> Result<T>
            + Send
            + 'static,
    ) -> Result<T> {
        crate::shared::init_env_logger();

        match (verbose, progress) {
            (false, false) => {
                let stdout = stdout();
                let mut stdout_lock = stdout.lock();
                run(progress::DoOrDiscard::from(None), &mut stdout_lock, &mut stderr())
            }
            (true, false) => {
                use crate::shared::{self, STANDARD_RANGE};
                let progress = shared::progress_tree(trace);
                let sub_progress = progress.add_child(name);
                init_tracing(trace, false, &progress)?;

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
            #[cfg(not(feature = "prodash-render-tui"))]
            (_, true) => {
                unreachable!("BUG: This branch can't be run without a TUI built-in")
            }
            #[cfg(feature = "prodash-render-tui")]
            (_, true) => {
                use std::io::Write;

                use crate::shared;

                enum Event<T> {
                    UiDone,
                    ComputationDone(Result<T>, Vec<u8>),
                }
                let progress = prodash::tree::Root::new();
                let sub_progress = progress.add_child(name);

                let render_tui = prodash::render::tui(
                    stdout(),
                    std::sync::Arc::downgrade(&progress),
                    prodash::render::tui::Options {
                        title: "gitoxide".into(),
                        frames_per_second: shared::DEFAULT_FRAME_RATE,
                        stop_if_progress_missing: !progress_keep_open,
                        throughput: true,
                        ..Default::default()
                    },
                )
                .expect("tui to come up without io error");
                let (tx, rx) = std::sync::mpsc::sync_channel::<Event<T>>(1);
                let ui_handle = std::thread::spawn({
                    let tx = tx.clone();
                    move || {
                        futures_lite::future::block_on(render_tui);
                        tx.send(Event::UiDone).ok();
                    }
                });
                let thread = std::thread::spawn({
                    let name = name.to_owned();
                    move || {
                        let _trace = init_tracing(trace, true, &progress).ok();
                        // We might have something interesting to show, which would be hidden by the alternate screen if there is a progress TUI
                        // We know that the printing happens at the end, so this is fine.
                        let mut out = Vec::new();
                        let res = gix::trace::coarse!("run", name = name).into_scope(|| {
                            run(progress::DoOrDiscard::from(Some(sub_progress)), &mut out, &mut stderr())
                        });
                        tx.send(Event::ComputationDone(res, out)).ok();
                    }
                });
                loop {
                    match rx.recv() {
                        Ok(Event::UiDone) => {
                            // We don't know why the UI is done, usually it's the user aborting.
                            // We need the computation to stop as well so let's wait for that to happen
                            gix::interrupt::trigger();
                            continue;
                        }
                        Ok(Event::ComputationDone(res, out)) => {
                            ui_handle.join().ok();
                            stdout().write_all(&out)?;
                            break res;
                        }
                        Err(_err) => match thread.join() {
                            Ok(()) => unreachable!("BUG: We shouldn't fail to receive unless the thread has panicked"),
                            Err(panic) => std::panic::resume_unwind(panic),
                        },
                    }
                }
            }
        }
    }
}

#[allow(unused)]
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
            initial_delay: Some(std::time::Duration::from_millis(1000)),
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

    use clap::{builder, builder::PossibleValue, error::ErrorKind, Arg, Command, Error};
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
        type Value = gix::pathspec::Pattern;

        fn parse_ref(&self, cmd: &Command, arg: Option<&Arg>, value: &OsStr) -> Result<Self::Value, Error> {
            OsStringValueParser::new()
                .try_map(|arg| {
                    let arg: &std::path::Path = arg.as_os_str().as_ref();
                    gix::pathspec::parse(gix::path::into_bstr(arg).as_ref(), *PATHSPEC_DEFAULTS)
                })
                .parse_ref(cmd, arg, value)
        }
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
                .try_map(|arg| gix::date::parse(&arg, Some(std::time::SystemTime::now())))
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
        type Value = std::ops::Range<u32>;

        fn parse_ref(&self, cmd: &Command, arg: Option<&Arg>, value: &OsStr) -> Result<Self::Value, Error> {
            StringValueParser::new()
                .try_map(|arg| -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
                    let parts = arg.split_once(',');
                    if let Some((start, end)) = parts {
                        let start = u32::from_str(start)?;
                        let end = u32::from_str(end)?;

                        if start <= end {
                            return Ok(start..end);
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
    ParseRenameFraction,
};

#[cfg(test)]
mod value_parser_tests {
    use super::{AsRange, ParseRenameFraction};
    use clap::Parser;

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
            pub arg: Option<std::ops::Range<u32>>,
        }

        let c = Cmd::parse_from(["cmd", "-l=1,10"]);
        assert_eq!(c.arg, Some(1..10));
    }
}
