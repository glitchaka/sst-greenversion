//! Full-client-area tool report viewer. Uses the same native terminal and PTY;
//! leaving the alternate screen restores the original console and scrollback.
//! A tool never creates a new window or fabricates telemetry.
use std::time::Duration;

use anyhow::Result;

use crate::core::ports::{TerminalFactory, TerminalKey, TerminalPanel};

use super::theme;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReportLine {
    text: String,
    heading: bool,
}

fn is_rule(text: &str) -> bool {
    let text = text.trim();
    text.len() >= 5 && text.chars().all(|ch| matches!(ch, '-' | '=' | '─'))
}

fn is_heading(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty()
        && text.chars().count() <= 68
        && text.chars().any(char::is_alphabetic)
        && text.chars().filter(char::is_alphabetic).all(|ch| !ch.is_lowercase())
        && !text.contains(": ")
}

fn wrapped_lines(input: &str, max_width: usize) -> Vec<ReportLine> {
    let max_width = max_width.max(16);
    let mut out = Vec::new();
    for original in input.lines() {
        if is_rule(original) {
            continue;
        }
        let text = original.trim_end();
        if text.trim().is_empty() {
            if !out.last().is_some_and(|line: &ReportLine| line.text.is_empty()) {
                out.push(ReportLine { text: String::new(), heading: false });
            }
            continue;
        }
        let heading = is_heading(text);
        let indent = text.chars().take_while(|ch| ch.is_whitespace()).count().min(6);
        let prefix = " ".repeat(indent);
        let available = max_width.saturating_sub(indent).max(10);
        let mut current = prefix.clone();
        for word in text.trim().split_whitespace() {
            let len = current.chars().count();
            let word_len = word.chars().count();
            let sep = usize::from(!current.trim().is_empty());
            if len + sep + word_len > max_width && !current.trim().is_empty() {
                out.push(ReportLine { text: current, heading });
                current = format!("{prefix}  ");
            }
            if !current.trim().is_empty() { current.push(' '); }
            // Very long paths/hashes: keep all characters, never ellipsize evidence.
            if word_len > available {
                for ch in word.chars() {
                    if current.chars().count() >= max_width {
                        out.push(ReportLine { text: current, heading });
                        current = format!("{prefix}  ");
                    }
                    current.push(ch);
                }
            } else {
                current.push_str(word);
            }
        }
        if !current.trim().is_empty() {
            out.push(ReportLine { text: current, heading });
        }
    }
    out
}

fn color_line(line: &ReportLine) -> String {
    if line.text.is_empty() { return String::new(); }
    let text = line.text.trim_start();
    let tone = if line.heading {
        theme::ACCENT
    } else if text.starts_with("CRÍTICO") || text.starts_with("CRITICO")
        || text.starts_with("ERROR") || text.starts_with("DENEGADO")
    {
        theme::BAD
    } else if text.starts_with("ADVERTENCIA") || text.starts_with("ATENCIÓN")
        || text.starts_with("ATENCION")
    {
        theme::WARN
    } else if text.starts_with("OK") || text.starts_with("CORRECTO") {
        theme::GOOD
    } else if text.contains(':') && !text.starts_with("http") {
        theme::LABEL
    } else {
        theme::TEXT
    };
    format!("{tone}{}{reset}", line.text, reset = theme::RESET)
}

pub fn show_report(
    factory: &dyn TerminalFactory,
    title: &str,
    command: &str,
    content: &str,
) -> Result<()> {
    let mut terminal = factory.alternate_screen()?;
    let mut scroll = 0usize;
    loop {
        let (cols, rows) = terminal.size()?;
        let width = usize::from(cols).max(28);
        let height = usize::from(rows).max(10);
        let page_size = height.saturating_sub(7).max(1);
        let lines = wrapped_lines(content, width.saturating_sub(6));
        let max_scroll = lines.len().saturating_sub(page_size);
        scroll = scroll.min(max_scroll);

        let mut view = format!(
            "  {accent}SST{reset}  {title_color}{title}{reset}  {muted}/  {command}{reset}\r\n",
            accent = theme::ACCENT, reset = theme::RESET,
            title_color = theme::TITLE, muted = theme::MUTED,
        );
        view.push_str(&format!(
            "  {label}VISTA DE HERRAMIENTA{reset}     {muted}Datos del comando, sin métricas simuladas{reset}\r\n\r\n",
            label = theme::LABEL, reset = theme::RESET, muted = theme::MUTED,
        ));

        for line in lines.iter().skip(scroll).take(page_size) {
            view.push_str("    ");
            view.push_str(&color_line(line));
            view.push_str("\r\n");
        }
        for _ in lines.iter().skip(scroll).take(page_size).count()..page_size {
            view.push_str("\r\n");
        }

        let last = (scroll + page_size).min(lines.len());
        view.push_str(&format!(
            "\r\n  {muted}LÍNEAS {first}–{last} / {total}     ↑↓ desplazarse  PgUp/PgDn página  Inicio/Fin  Esc/Q volver{reset}",
            first = if lines.is_empty() { 0 } else { scroll + 1 },
            total = lines.len(), muted = theme::MUTED, reset = theme::RESET,
        ));
        terminal.clear()?;
        terminal.set_panels(vec![TerminalPanel {
            start_row: 0,
            rows: u16::try_from(height.saturating_sub(2)).unwrap_or(u16::MAX),
        }])?;
        terminal.write(&view)?;
        terminal.flush()?;

        match terminal.poll_key(Duration::from_millis(180))? {
            Some(TerminalKey::Escape | TerminalKey::Char('q') | TerminalKey::Char('Q')) => break,
            Some(TerminalKey::Up) => scroll = scroll.saturating_sub(1),
            Some(TerminalKey::Down) => scroll = (scroll + 1).min(max_scroll),
            Some(TerminalKey::PageUp) => scroll = scroll.saturating_sub(page_size),
            Some(TerminalKey::PageDown | TerminalKey::Space) =>
                scroll = (scroll + page_size).min(max_scroll),
            Some(TerminalKey::Home) => scroll = 0,
            Some(TerminalKey::End) => scroll = max_scroll,
            _ => {}
        }
    }
    // TerminalSession's Drop exits alternate screen, restoring the entire PTY.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_is_wrapped_without_truncation_or_fake_rows() {
        let src = "SEGURIDAD\n------------------------------------------------------------\nRuta: C:\\Program Files\\Equipo\\Carpeta\\aplicacion.exe\n";
        let lines = wrapped_lines(src, 26);
        assert!(lines.iter().any(|row| row.heading && row.text == "SEGURIDAD"));
        assert!(!lines.iter().any(|row| is_rule(&row.text)));
        assert!(lines.iter().map(|r| r.text.replace(' ', "")).collect::<String>()
            .contains("C:\\ProgramFiles\\Equipo\\Carpeta\\aplicacion.exe"));
    }
}
