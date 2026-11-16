//! Progress is written directly to SST's terminal output stream.
//! Metrics come exclusively from bytes written by curl to its destination.
//! No GUI overlay, card, popup or reserved terminal rows are involved.
use std::{
    fs,
    io::IsTerminal,
    path::Path,
    process::{Command, Output, Stdio},
    sync::{Mutex, OnceLock},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};

use crate::adapters::terminal::io;

#[derive(Clone, Debug)]
pub struct TransferSnapshot {
    pub received: u64,
    pub expected: Option<u64>,
    pub bytes_per_second: f64,
    pub finished: bool,
    updated_at: Instant,
}

impl TransferSnapshot {
    fn new(_label: &str, received: u64, expected: Option<u64>, rate: f64) -> Self {
        Self {
            received,
            expected: expected.filter(|value| *value > 0),
            bytes_per_second: rate,
            finished: false,
            updated_at: Instant::now(),
        }
    }

    pub fn fraction(&self) -> f32 {
        self.expected
            .filter(|size| *size > 0)
            .map(|size| (self.received.min(size) as f64 / size as f64) as f32)
            .unwrap_or_default()
    }

    pub fn percentage(&self) -> String {
        match self.expected {
            Some(_) => format!("{}%", (f64::from(self.fraction()) * 100.0).round() as u32),
            None => "—%".to_owned(),
        }
    }

    pub fn amount(&self) -> String {
        format!(
            "{} / {}",
            human_bytes(self.received),
            self.expected.map(human_bytes).unwrap_or_else(|| "desconocido".into())
        )
    }

    pub fn speed(&self) -> String {
        if self.bytes_per_second > 0.0 {
            format!("{}/s", human_bytes(self.bytes_per_second as u64))
        } else {
            "—/s".to_owned()
        }
    }

    pub fn remaining(&self) -> String {
        match (self.expected, self.bytes_per_second) {
            (Some(total), rate) if rate > 0.0 && total >= self.received =>
                eta((total - self.received) as f64 / rate),
            _ => "—".to_owned(),
        }
    }
}

// One transfer is shown at a time: SST runs one foreground command per session.
// The UI reads a snapshot from its own Slint event loop, never touching the
// worker's terminal buffer or custom image/background configuration.
static GUI_TRANSFER: OnceLock<Mutex<Option<TransferSnapshot>>> = OnceLock::new();

fn transfer_state() -> &'static Mutex<Option<TransferSnapshot>> {
    GUI_TRANSFER.get_or_init(|| Mutex::new(None))
}

/// A completed transfer remains visible briefly so even fast downloads can be
/// observed, then the native overlay disappears without altering the terminal.
pub fn snapshot() -> Option<TransferSnapshot> {
    let mut state = transfer_state().lock().unwrap_or_else(|e| e.into_inner());
    if state.as_ref().is_some_and(|s| s.finished && s.updated_at.elapsed() >= Duration::from_secs(2)) {
        *state = None;
    }
    state.clone()
}

fn human_bytes(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MiB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.0} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn eta(seconds: f64) -> String {
    let secs = seconds.ceil().max(0.0) as u64;
    if secs >= 3600 {
        format!("{}h {:02}m", secs / 3600, secs % 3600 / 60)
    } else {
        format!("{:02}:{:02}", secs / 60, secs % 60)
    }
}

/// libcaca handles the terminal-mode progress composition. The SST GUI does
/// not print this ASCII/Unicode bar: it presents genuine rounded Slint shapes.
fn draw(label: &str, current: u64, expected: Option<u64>, rate: f64, columns: usize) -> String {
    // Build a single PTY row, never a floating UI element. Width is measured
    // in terminal columns, not pixels, and no output may wrap into the next row.
    let snapshot = TransferSnapshot::new(label, current, expected, rate);
    let label = label.chars().filter(|c| !c.is_control()).take(18).collect::<String>();
    let prefix = format!("[{label}] ");
    let detail = format!(
        "{}  {}  {}  ETA {}",
        snapshot.percentage(), snapshot.amount(), snapshot.speed(), snapshot.remaining()
    );
    let width = columns.saturating_sub(1).max(1);
    let progress_width = width.saturating_sub(prefix.chars().count() + detail.chars().count() + 3);
    // Unicode block glyphs occupy one terminal cell each.
    if progress_width >= 6 {
        let filled = snapshot.expected.map(|_| {
            (f64::from(snapshot.fraction()) * progress_width as f64).round() as usize
        }).unwrap_or(0).min(progress_width);
        let bar = format!("{}{}", "█".repeat(filled), "░".repeat(progress_width - filled));
        return format!("{prefix}\x1b[96m{bar}\x1b[0m  {detail}");
    }
    // A narrow PTY must still show live progress without wrapping.
    let compact = format!("{} {}", snapshot.percentage(), snapshot.speed());
    let available = width.saturating_sub(compact.chars().count() + 1);
    let short_label: String = label.chars().take(available).collect();
    format!("{short_label} {compact}")
}

/// Execute curl without a terminal child. SST's native GUI receives progress
/// snapshots and draws its own graphics; only external interactive consoles
/// receive libcaca-composed terminal fallback output.
pub fn run(
    command: &mut Command,
    destination: &Path,
    expected: Option<u64>,
    label: &str,
) -> Result<Output> {
    let embedded = io::output_sender().is_some();
    let cli_terminal = !embedded && std::io::stdout().is_terminal();
    command.stdout(Stdio::null()).stderr(Stdio::piped());
    let mut child = command.spawn().context("no se pudo iniciar la transferencia")?;
    let cols = if cli_terminal {
        io::size().map(|(cols, _)| cols as usize).unwrap_or(100)
    } else {
        100
    };
    let started = Instant::now();
    let mut last = Instant::now() - Duration::from_secs(1);

    if embedded || cli_terminal {
        // The transfer occupies one ordinary terminal line. No Slint graphic
        // is published: updates repaint this line in-place through VT100.
        io::write_raw(b"\x1b[?25l\r\x1b[2K")?;
    }
    loop {
        if child.try_wait()?.is_some() { break; }
        if (embedded || cli_terminal) && last.elapsed() >= Duration::from_millis(120) {
            let current = fs::metadata(destination).map(|m| m.len()).unwrap_or(0);
            let rate = current as f64 / started.elapsed().as_secs_f64().max(0.001);
            let line = draw(label, current, expected, rate, if embedded { io::size().map(|(c, _)| c as usize).unwrap_or(cols) } else { cols });
            let _ = io::write_raw(format!("\r\x1b[2K{line}").as_bytes());
            last = Instant::now();
        }
        thread::sleep(Duration::from_millis(75));
    }
    let output = child.wait_with_output()?;
    if embedded || cli_terminal {
        let current = fs::metadata(destination).map(|m| m.len()).unwrap_or(0);
        let rate = current as f64 / started.elapsed().as_secs_f64().max(0.001);
        let line = draw(label, current, expected, rate, cols);
        let suffix = if output.status.success() { "\x1b[92m✓\x1b[0m" } else { "\x1b[91m✕\x1b[0m" };
        let _ = io::write_raw(format!("\r\x1b[2K{line} {suffix}\r\n\x1b[?25h").as_bytes());
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::{TransferSnapshot, draw, eta, human_bytes};

    #[test]
    fn uses_real_file_bytes_without_fake_percentage_or_eta() {
        let title = draw("SST", 5_242_880, Some(10_485_760), 1_048_576.0, 120);
        assert!(title.contains("50%"));
        assert!(title.contains("5.0 MiB / 10.0 MiB"));
        assert!(title.contains("ETA 00:05"));
        let unknown = draw("SST", 1024, None, 0.0, 100);
        assert!(unknown.contains("—%"));
        assert!(unknown.contains("ETA —"));
        assert_eq!(human_bytes(1_048_576), "1.0 MiB");
        assert_eq!(eta(61.0), "01:01");
    }

    #[test]
    fn native_snapshot_protects_unknown_sizes_and_clamps_overflow() {
        let s = TransferSnapshot::new("INSTP", 250, None, 0.0);
        assert_eq!(s.fraction(), 0.0);
        assert_eq!(s.percentage(), "—%");
        assert_eq!(s.remaining(), "—");
        let s = TransferSnapshot::new("SST", 200, Some(100), 10.0);
        assert_eq!(s.fraction(), 1.0);
        assert_eq!(s.percentage(), "100%");
        assert_eq!(s.remaining(), "—");
    }
}
