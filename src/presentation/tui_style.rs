//! Semantic layouts for SST's interactive alternate-screen monitors.
//!
//! ANSI colors encode information and remain readable in an external console.
//! Rounded section frames are *real vector primitives* drawn by Slint from
//! TerminalPanel geometry. No box drawing characters or fake ASCII borders.

use crate::core::ports::TerminalPanel;

// A single palette shared by all native fullscreen tools.
pub use super::theme::{BAD, GOOD, LABEL, RESET, WARN};
pub const ACCENT: &str = super::theme::ACCENT;
pub const CONTENT: &str = super::theme::TEXT;

/// One alternate-screen view; body text is kept independent of native panel
/// strokes. Even a non-Slint stdout produces semantic headings and statuses.
pub struct LiveScreen {
    text: String,
    line: usize,
    panels: Vec<TerminalPanel>,
}

impl LiveScreen {
    pub fn new() -> Self {
        // Write an actual terminal header, not a graphical Slint overlay.
        // Users can select and copy its characters like any other output.
        let native_view = crate::adapters::terminal::io::output_sender().is_some();
        let header = if native_view {
            crate::presentation::tool_screen::active_tool_title()
                .unwrap_or_else(|| "SST / MONITOR".to_owned())
        } else {
            String::new()
        };
        let heading = if native_view {
            format!("  {ACCENT}SST  /  {header}{RESET}\r\n\r\n")
        } else {
            String::new()
        };
        Self {
            text: heading,
            line: if native_view { 2 } else { 0 },
            panels: Vec::new(),
        }
    }

    pub fn section(&mut self, title: &str, body: &str) {
        let start = self.line;
        self.text.push_str("  ");
        self.text.push_str(ACCENT);
        self.text.push_str("▌ ");
        self.text.push_str(title);
        self.text.push_str(RESET);
        self.text.push_str("\r\n");
        self.line += 1;

        for line in body.trim_end_matches('\n').split('\n') {
            if line.is_empty() && body.trim().is_empty() { break; }
            self.text.push_str("  ");
            self.text.push_str(line);
            self.text.push_str("\r\n");
            self.line += 1;
        }

        if self.panels.len() < 4 {
            self.panels.push(TerminalPanel {
                start_row: start.min(u16::MAX as usize) as u16,
                rows: (self.line - start).min(u16::MAX as usize) as u16,
            });
        }
        self.text.push_str("\r\n");
        self.line += 1;
    }

    pub fn footer(&mut self, text: &str) {
        self.text.push_str("  ");
        self.text.push_str(LABEL);
        self.text.push_str(text);
        self.text.push_str(RESET);
        self.text.push_str("\r\n");
        self.line += 1;
    }

    pub fn finish(self) -> (String, Vec<TerminalPanel>) {
        (self.text, self.panels)
    }
}

/// Monitors must fit within terminal cells, not pixel width. ANSI escape
/// sequences do not contribute to the visible width.
pub fn shorten(value: &str, max: usize) -> String {
    let max = max.max(1);
    if value.chars().count() <= max { return value.to_owned(); }
    if max <= 2 { return value.chars().take(max).collect(); }
    format!("{}…", value.chars().take(max - 1).collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_report_real_cell_coordinates_without_character_frames() {
        let mut screen = LiveScreen::new();
        screen.section("RED", "escaneando");
        screen.section("DISPOSITIVOS", "IP  NOMBRE\n10.0.0.3  PC");
        let (text, panels) = screen.finish();
        assert_eq!(panels.len(), 2);
        assert_eq!(panels[0].start_row, 0);
        assert_eq!(panels[0].rows, 2);
        assert_eq!(panels[1].start_row, 3);
        assert_eq!(panels[1].rows, 3);
        assert!(text.contains(ACCENT));
        assert!(!text.contains("┌"));
        assert!(!text.contains("└"));
        assert!(!text.contains("──"));
    }

    #[test]
    fn clipped_display_columns_are_measured_by_characters() {
        assert_eq!(shorten("aaaaaaaaaa", 6), "aaaaa…");
        assert_eq!(shorten("SST", 8), "SST");
    }
}
