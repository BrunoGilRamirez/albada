//! Local-first logging for the desktop application.
//!
//! A `log` crate implementation that writes to a timestamped file in the app
//! data directory and mirrors to stderr (visible in the `tauri dev` terminal).
//! Panics are captured with a panic hook so command failures are diagnosable
//! without telemetry or external transmission.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

use log::{LevelFilter, Log, Metadata, Record};

pub const LOG_FILE_NAME: &str = "albada.log";

/// Writes formatted records to a log file and stderr.
pub struct LocalLogger {
    file: Mutex<File>,
}

impl LocalLogger {
    pub fn new(log_dir: &std::path::Path) -> Result<Self, std::io::Error> {
        std::fs::create_dir_all(log_dir)?;
        let path = log_dir.join(LOG_FILE_NAME);
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            file: Mutex::new(file),
        })
    }

    fn format(record: &Record<'_>) -> String {
        let ts = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
        format!(
            "[{ts}] [{}] [{}] {}\n",
            record.level(),
            record.target(),
            record.args()
        )
    }
}

impl Log for LocalLogger {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = Self::format(record);
        if let Ok(mut file) = self.file.lock() {
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();
        }
        eprint!("{line}");
    }

    fn flush(&self) {
        if let Ok(mut file) = self.file.lock() {
            let _ = file.flush();
        }
    }
}

/// Install the logger as the global `log` sink.
pub fn init(log_dir: &std::path::Path) -> Result<(), log::SetLoggerError> {
    let logger = match LocalLogger::new(log_dir) {
        Ok(logger) => logger,
        Err(err) => {
            eprintln!("WARNING: could not open log file: {err}");
            return log::set_boxed_logger(Box::new(NoopLogger));
        }
    };
    let level = if cfg!(debug_assertions) {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    log::set_boxed_logger(Box::new(logger))?;
    log::set_max_level(level);
    Ok(())
}

/// Logger used when the log file cannot be opened; keeps stderr output only.
struct NoopLogger;

impl Log for NoopLogger {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }
    fn log(&self, record: &Record<'_>) {
        eprintln!(
            "[{}] [{}] {}",
            record.level(),
            record.target(),
            record.args()
        );
    }
    fn flush(&self) {}
}

/// Install a panic hook that records the panic (with a backtrace) into the log
/// file. Command panics otherwise surface as opaque "unexpected" errors on the
/// frontend.
pub fn install_panic_hook(log_dir: &std::path::Path) {
    let log_path = log_dir.join(LOG_FILE_NAME);
    if let Err(err) = std::fs::create_dir_all(log_dir) {
        eprintln!("WARNING: could not create log dir: {err}");
    }
    std::panic::set_hook(Box::new(move |info| {
        let bt = std::backtrace::Backtrace::force_capture();
        let message = format!("panic: {info}\n{bt}\n");
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
            let _ = writeln!(file, "{message}");
        }
        eprintln!("{message}");
    }));
}

/// Convenience wrapper that logs command entry, success, and failure.
///
/// Usage:
/// ```ignore
/// log_command!("create_task", db.with_conn(|conn| SqliteTaskRepo.create(...)))
/// ```
#[macro_export]
macro_rules! log_command {
    ($name:expr, $body:expr) => {{
        log::info!("{}: begin", $name);
        #[allow(clippy::redundant_closure_call)]
        let __result = (|| $body)();
        match &__result {
            Ok(_) => log::info!("{}: ok", $name),
            Err(e) => log::error!("{}: error: {e:?}", $name),
        }
        __result
    }};
}
