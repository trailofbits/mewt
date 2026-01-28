//! Logging utilities that integrate with `indicatif` progress bars.
//! - Logs go to stdout via a custom writer (`BarAwareWriter`).
//! - When a bar is active, completed lines use `ProgressBar::println`, preserving the bar.
//! - Progress bars render on stderr (indicatif default); logs stay on stdout.
//! - Log level comes from CLI flags or config file (`mewt.toml`); default is Info.
//! - Colors are applied with `console::style`.
//! - A `Mutex<Vec<u8>>` buffers bytes to assemble whole lines without interleaving.

use std::io::{self, Write};

use console::style;
use fern::Dispatch;
use indicatif::WeakProgressBar;
use indicatif::{ProgressBar, ProgressStyle};
use log::LevelFilter;
use once_cell::sync::OnceCell;
use std::sync::Mutex;

use crate::types::config::config;

/// Global registry of the active progress bar as a `WeakProgressBar`.
/// Wrapped in a `Mutex` inside a `OnceCell` so logging can access it
/// without keeping the bar alive.
static ACTIVE_BAR: OnceCell<Mutex<Option<WeakProgressBar>>> = OnceCell::new();

/// Register or clear the active progress bar. Stores a weak reference so the
/// bar can drop naturally.
fn set_active_progress_bar(bar: Option<&ProgressBar>) {
    let cell = ACTIVE_BAR.get_or_init(|| Mutex::new(None));
    let weak = bar.map(|b| b.downgrade());
    if let Ok(mut guard) = cell.lock() {
        *guard = weak;
    }
}

/// Create a sized progress bar with the default style, set its message, and
/// register it as the active bar.
pub fn new_progress_bar(len: u64, message: impl Into<String>) -> ProgressBar {
    let bar = ProgressBar::new(len);
    bar.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] {wide_bar:.cyan/blue} {pos}/{len} (ETA: {eta}) {msg}",
        )
        .expect("valid progress template")
        .progress_chars("#>-"),
    );
    bar.set_message(message.into());
    set_active_progress_bar(Some(&bar));
    bar
}

/// Finish and clear the provided progress bar, then clear the active bar registry.
pub fn end_progress_bar(bar: &ProgressBar) {
    bar.finish_and_clear();
    set_active_progress_bar(None);
}

/// Writer that buffers bytes until a newline and then emits whole lines.
/// If a progress bar is active, lines are printed via `bar.println(...)`;
/// otherwise they are written to stdout. The internal `Mutex<Vec<u8>>`
/// ensures that line assembly is synchronized and not interleaved.
struct BarAwareWriter {
    buffer: Mutex<Vec<u8>>, // accumulates bytes until a '\n' is seen
}

impl Write for BarAwareWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        {
            let mut buf_guard = self.buffer.lock().unwrap();
            buf_guard.extend_from_slice(buf);
        }

        loop {
            let drained: Option<Vec<u8>> = {
                let mut buf_guard = self.buffer.lock().unwrap();
                buf_guard
                    .iter()
                    .position(|&b| b == b'\n')
                    .map(|pos| buf_guard.drain(..=pos).collect::<Vec<u8>>())
            };

            match drained {
                Some(mut line_bytes) => {
                    if line_bytes.last() == Some(&b'\n') {
                        line_bytes.pop();
                    }
                    let line = String::from_utf8_lossy(&line_bytes);
                    let mut printed_via_bar = false;
                    if let Some(cell) = ACTIVE_BAR.get()
                        && let Ok(guard) = cell.lock()
                        && let Some(ref weak) = *guard
                        && let Some(bar) = weak.upgrade()
                    {
                        bar.println(&line);
                        printed_via_bar = true;
                    }
                    if !printed_via_bar {
                        let _ = writeln!(io::stdout(), "{}", line);
                    }
                }
                None => break,
            }
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Ok(mut buf_guard) = self.buffer.lock()
            && !buf_guard.is_empty()
        {
            let line = String::from_utf8_lossy(&buf_guard);
            let mut printed_via_bar = false;
            if let Some(cell) = ACTIVE_BAR.get()
                && let Ok(guard) = cell.lock()
                && let Some(ref weak) = *guard
                && let Some(bar) = weak.upgrade()
            {
                bar.println(&line);
                printed_via_bar = true;
            }
            if !printed_via_bar {
                let _ = writeln!(io::stdout(), "{}", line);
            }
            buf_guard.clear();
        }
        io::stdout().flush()
    }
}

/// Initialize logging with `fern`, using `BarAwareWriter` to coexist with active
/// progress bars. Level comes from `RUST_LOG`/`MEWT_LOG`, default Info.
pub fn init_logging() {
    let level = match config().log.level.to_lowercase().as_str() {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        "warn" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        _ => LevelFilter::Info,
    };

    let _ = Dispatch::new()
        .level(level)
        .format(|out, message, record| {
            use log::Level;
            let prefix = match record.level() {
                Level::Trace => format!("{}", style("[T]").cyan()),
                Level::Debug => format!("{}", style("[D]").blue()),
                Level::Info => "".to_string(),
                Level::Warn => format!("{}", style("[W]").yellow()),
                Level::Error => format!("{}", style("[E]").red()),
            };
            out.finish(format_args!("{} {}", prefix, message))
        })
        .chain(Box::new(BarAwareWriter {
            buffer: Mutex::new(Vec::new()),
        }) as Box<dyn Write + Send>)
        .apply();
}
