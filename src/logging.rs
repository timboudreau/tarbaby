use std::path::PathBuf;
use tracing_appender::{
    non_blocking::{NonBlocking, WorkerGuard},
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{
    EnvFilter, Registry, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

#[must_use]
pub fn init_logging(
    log_dir: Option<PathBuf>,
    async_logging: bool,
    json_logging: bool,
) -> LoggerGuard {
    let filter = EnvFilter::from_default_env();
    let (result, app) = appender(async_logging, log_dir);
    if json_logging {
        // This is ugly, but difficult to abstract since the compiler wants to know the type before
        // we do

        let base = tracing_subscriber::fmt::layer()
            .json()
            .flatten_event(true)
            .with_current_span(true)
            .with_span_list(true);

        match app {
            AnyAppender::NonBlocking(non_blocking) => {
                let layer = base.with_writer(non_blocking);
                if Registry::default()
                    .with(filter)
                    .with(layer)
                    .try_init()
                    .is_ok()
                {
                    result
                } else {
                    LoggerGuard::Empty
                }
            }
            AnyAppender::Blocking(rolling_file_appender) => {
                let layer = base.with_writer(rolling_file_appender);
                if Registry::default()
                    .with(filter)
                    .with(layer)
                    .try_init()
                    .is_ok()
                {
                    result
                } else {
                    LoggerGuard::Empty
                }
            }
            AnyAppender::StdOut => {
                let layer = base.with_writer(std::io::stdout);
                if Registry::default()
                    .with(filter)
                    .with(layer)
                    .try_init()
                    .is_ok()
                {
                    result
                } else {
                    LoggerGuard::Empty
                }
            }
        }
    } else {
        let base = tracing_subscriber::fmt().with_env_filter(filter);
        match app {
            AnyAppender::NonBlocking(non_blocking) => {
                if let Ok(_) = base.with_writer(non_blocking).try_init() {
                    result
                } else {
                    LoggerGuard::Empty
                }
            }
            AnyAppender::Blocking(rolling_file_appender) => {
                if let Ok(_) = base.with_writer(rolling_file_appender).try_init() {
                    result
                } else {
                    LoggerGuard::Empty
                }
            }
            AnyAppender::StdOut => {
                if let Ok(_) = base.with_writer(std::io::stdout).try_init() {
                    result
                } else {
                    LoggerGuard::Empty
                }
            }
        }
    }
}

fn appender(async_logging: bool, dir: Option<PathBuf>) -> (LoggerGuard, AnyAppender) {
    if let Some(dir) = dir {
        file_appender(async_logging, dir)
    } else {
        console_appender(async_logging)
    }
}

fn console_appender(async_logging: bool) -> (LoggerGuard, AnyAppender) {
    if async_logging {
        let (result, guard) = tracing_appender::non_blocking(std::io::stdout());
        (
            LoggerGuard::NonBlocking(guard),
            AnyAppender::NonBlocking(result),
        )
    } else {
        (LoggerGuard::Empty, AnyAppender::StdOut)
    }
}

fn file_appender(async_logging: bool, dir: PathBuf) -> (LoggerGuard, AnyAppender) {
    let appender = tracing_appender::rolling::Builder::default()
        .filename_prefix("tarbaby")
        .filename_suffix("log")
        .latest_symlink("latest")
        .rotation(Rotation::DAILY)
        .build(&dir)
        .expect("Could not create log file appender");
    if async_logging {
        let (result, guard) = tracing_appender::non_blocking::NonBlockingBuilder::default()
            .buffered_lines_limit(64)
            .lossy(false)
            .thread_name("logging")
            .finish(appender);
        (
            LoggerGuard::NonBlocking(guard),
            AnyAppender::NonBlocking(result),
        )
    } else {
        (LoggerGuard::Empty, AnyAppender::Blocking(appender))
    }
}

/// If we are using async logging, we need to hold a reference to the worker guard, or
/// logging stops, so we use this type to hang onto it across the life of the app.
pub enum LoggerGuard {
    Empty,
    #[allow(unused)]
    NonBlocking(WorkerGuard),
}

// Just gives us a common return type for the heterogenous appender types we have
// to return, which do not share a common type.
enum AnyAppender {
    NonBlocking(NonBlocking),
    Blocking(RollingFileAppender),
    StdOut,
}
