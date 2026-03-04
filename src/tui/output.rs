/// Global TUI output channel.
///
/// When the TUI is active every `tui_println!` call sends the formatted string
/// into this channel instead of writing to stdout. `EnhancedApp::update()` drains
/// the channel each frame and pushes lines into the chat pane as system messages.
///
/// When the TUI is NOT active (normal CLI usage) `tui_println!` falls back to the
/// standard `println!` macro so behaviour is unchanged.
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

pub use std::sync::mpsc::{Receiver, Sender};

static TUI_ACTIVE: AtomicBool = AtomicBool::new(false);
static TUI_SENDER: OnceLock<Sender<String>> = OnceLock::new();

/// Call this before entering the TUI event loop.
/// Returns the `Receiver` that `EnhancedApp` should hold and drain every frame.
pub fn activate() -> Receiver<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    // Store sender; ignore error on double-init (tests)
    let _ = TUI_SENDER.set(tx);
    TUI_ACTIVE.store(true, Ordering::SeqCst);
    rx
}

/// Call this after the TUI event loop exits so subsequent output goes to stdout.
pub fn deactivate() {
    TUI_ACTIVE.store(false, Ordering::SeqCst);
}

/// Returns `true` while the TUI owns the terminal.
#[inline]
pub fn is_active() -> bool {
    TUI_ACTIVE.load(Ordering::SeqCst)
}

/// Send a line to the TUI chat pane. Called by the `tui_println!` macro.
pub fn send_line(line: String) {
    if let Some(tx) = TUI_SENDER.get() {
        let _ = tx.send(line);
    }
}

/// A tracing `MakeWriter` that routes all log output through the TUI channel.
/// When the TUI is not active it falls back to stderr (same as the default subscriber).
pub struct TuiWriter;

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for TuiWriter {
    type Writer = TuiWriterInner;

    fn make_writer(&'a self) -> Self::Writer {
        TuiWriterInner { buf: Vec::new() }
    }
}

pub struct TuiWriterInner {
    buf: Vec<u8>,
}

impl std::io::Write for TuiWriterInner {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.buf.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Drop for TuiWriterInner {
    fn drop(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        let text = String::from_utf8_lossy(&self.buf);
        let trimmed = text.trim_end_matches('\n');
        if trimmed.is_empty() {
            return;
        }
        if is_active() {
            send_line(trimmed.to_string());
        }
        // When TUI is not active we intentionally drop the output — the
        // sink subscriber is only installed for TUI commands.
    }
}

/// Macro: send a formatted line to the TUI chat pane if active, otherwise `println!`.
///
/// Usage identical to `println!`:
/// ```ignore
/// tui_println!("hello {}", name);
/// ```
#[macro_export]
macro_rules! tui_println {
    ($($arg:tt)*) => {
        {
            let line = format!($($arg)*);
            if $crate::tui::output::is_active() {
                $crate::tui::output::send_line(line);
            } else {
                println!("{}", line);
            }
        }
    };
}
