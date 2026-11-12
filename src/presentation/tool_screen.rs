//! Full-client-area reports for SST Green.
//! Every value comes from the command itself. The UI never invents telemetry.
//! Alternate-screen exit restores the complete console and its scrollback.
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
    text.chars().count() >= 5 && text.chars().all(|ch| matches!(ch, '-' | '=' | '─'))
}

fn is_tabular_heading(text: &str) -> bool {
    // Titles of actual tables are not section names. Preserve their columns.
    let words = text.split_whitespace().count();
    words >= 3 && text.contains("  ")
}

fn is_heading(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty()
        && text.chars().count() <= 68
        && text.chars().any(char::is_alphabetic)
        && text.chars().filter(|ch| ch.is_alphabetic()).all(|ch| !ch.is_lowercase())
        && !text.contains(": ")
        && !is_tabular_heading(text)
}

fn wrap_original(original: &str, max_width: usize, heading: bool) -> Vec<ReportLine> {
    // No split_whitespace(): the original system/network table alignment is data.
    if original.chars().count() <= max_width {
        return vec![ReportLine { text: original.to_owned(), heading }];
    }
    let mut result = Vec::new();
    let mut remaining = original;
    let mut first = true;
    while remaining.chars().count() > max_width {
        let hard = remaining.char_indices().nth(max_width)
            .map(|(offset, _)| offset).unwrap_or(remaining.len());
        let split = remaining[..hard].rfind(char::is_whitespace)
            .filter(|at| *at >= hard / 2)
            .map(|at| at + remaining[*at..].chars().next().map(char::len_utf8).unwrap_or(0))
            .unwrap_or(hard);
        let (head, tail) = remaining.split_at(split);
        result.push(ReportLine {
            text: head.to_owned(),
            heading: first && heading,
        });
        remaining = tail;
        first = false;
    }
    if !remaining.is_empty() {
        result.push(ReportLine { text: remaining.to_owned(), heading: first && heading });
    }
    result
}

fn wrapped_lines(input: &str, max_width: usize) -> Vec<ReportLine> {
    let max_width = max_width.max(16);
    let mut lines = Vec::new();
    for original in input.lines() {
        if is_rule(original) { continue; }
        let text = original.trim_end_matches('\r');
        if text.trim().is_empty() {
            if !lines.last().is_some_and(|row: &ReportLine| row.text.is_empty()) {
                lines.push(ReportLine { text: String::new(), heading: false });
            }
            continue;
        }
        lines.extend(wrap_original(text, max_width, is_heading(text)));
    }
    lines
}

fn tone_for_verified_status(value: &str) -> Option<&'static str> {
    match value.trim().trim_end_matches(['.', ';']).to_uppercase().as_str() {
        "CRÍTICO" | "CRITICO" | "ERROR" | "FALLO"
        | "DESCONECTADO" | "BLOQUEADO" | "DOWN" | "OFFLINE" => Some(theme::BAD),
        "ADVERTENCIA" | "ATENCIÓN" | "ATENCION" | "DEGRADADO"
        | "PENDIENTE" | "WARNING" => Some(theme::WARN),
        "OPERATIVO" | "OPERATIVA" | "CONECTADO" | "CONECTADA"
        | "ACTIVO" | "ACTIVA" | "CORRECTO" | "OK" | "UP" => Some(theme::GOOD),
        _ => None,
    }
}

fn color_line(line: &ReportLine) -> String {
    if line.text.is_empty() { return String::new(); }
    if line.text.contains('\x1b') { return line.text.clone(); }
    let text = line.text.trim_start();
    if line.heading {
        return format!("{}{}{}", theme::ACCENT, line.text, theme::RESET);
    }
    if is_tabular_heading(text) && text.chars().any(char::is_alphabetic) {
        return format!("{}{}{}", theme::LABEL, line.text, theme::RESET);
    }
    // A warning in evidence is shown as a warning, not a false positive badge
    // inferred from a filename or a passing occurrence of the word "error".
    if text.starts_with("CRÍTICO ") || text.starts_with("CRITICO ")
        || text.starts_with("ERROR:") || text.starts_with("DENEGADO:")
    {
        return format!("{}{}{}", theme::BAD, line.text, theme::RESET);
    }
    if text.starts_with("ADVERTENCIA:") || text.starts_with("ATENCIÓN:")
        || text.starts_with("ATENCION:")
    {
        return format!("{}{}{}", theme::WARN, line.text, theme::RESET);
    }
    if let Some((label, value)) = line.text.split_once(':')
        && (2..=40).contains(&label.trim().chars().count())
        && label.trim().chars().any(char::is_alphabetic)
    {
        let semantic = matches!(
            label.trim().to_ascii_lowercase().as_str(),
            "estado" | "status" | "salud" | "health" | "resultado"
        );
        let value_tone = if semantic {
            tone_for_verified_status(value).unwrap_or(theme::TEXT)
        } else {
            theme::TEXT
        };
        return format!(
            "{}{}:{}{}{}{}",
            theme::LABEL, label, theme::RESET, value_tone, value, theme::RESET
        );
    }
    format!("{}{}{}", theme::TEXT, line.text, theme::RESET)
}

fn truncate_chrome(text: &str, available: usize) -> String {
    let available = available.max(4);
    if text.chars().count() <= available { return text.to_owned(); }
    format!("{}…", text.chars().take(available - 1).collect::<String>())
}

/// A meter is shown only if the actual Windows report includes both values.
/// It never infers RAM usage from a made-up host, process or demo percentage.
fn memory_meter(report: &str, width: usize) -> Option<String> {
    fn mib(report: &str, key: &str) -> Option<u64> {
        report.lines().find_map(|line| {
            line.trim().strip_prefix(key)?
                .split_whitespace().next()?.parse().ok()
        })
    }
    let total = mib(report, "memory_total:")?;
    let used = mib(report, "memory_used:")?;
    if total == 0 { return None; }
    let percentage = used.min(total).saturating_mul(100) / total;
    let cells = if width >= 90 { 18 } else { 9 };
    let filled = (percentage * cells / 100) as usize;
    let empty = cells as usize - filled;
    let meter = format!(
        "{}{}{}{}{}{}",
        theme::ACCENT, "━".repeat(filled), theme::RESET,
        theme::MUTED, "─".repeat(empty), theme::RESET,
    );
    Some(format!(
        "  {}RAM{}  {meter}  {}{percentage:>3}%{}   {}{used} / {total} MiB{}",
        theme::LABEL, theme::RESET, theme::TITLE, theme::RESET,
        theme::MUTED, theme::RESET,
    ))
}

fn section_panels(visible: &[ReportLine], top: usize, bottom: usize) -> Vec<TerminalPanel> {
    if visible.is_empty() { return Vec::new(); }
    let mut boundaries = vec![0usize];
    for (index, line) in visible.iter().enumerate().skip(1) {
        if line.heading && boundaries.len() < 4 {
            boundaries.push(index);
        }
    }
    let mut panels = Vec::new();
    for (group, start) in boundaries.iter().enumerate() {
        let end = boundaries.get(group + 1).copied().unwrap_or(visible.len());
        let row = top.saturating_sub(1) + start;
        let last = (top + end).min(bottom);
        if last <= row { continue; }
        panels.push(TerminalPanel {
            start_row: row.min(u16::MAX as usize) as u16,
            rows: (last - row).min(u16::MAX as usize) as u16,
        });
    }
    panels
}

fn render_page(
    title: &str,
    command: &str,
    content: &str,
    lines: &[ReportLine],
    scroll: usize,
    width: usize,
    height: usize,
) -> (String, Vec<TerminalPanel>) {
    let body_top = 3;
    let body_rows = height.saturating_sub(6).max(1);
    let visible = lines.iter().skip(scroll).take(body_rows).cloned().collect::<Vec<_>>();
    let mut view = String::new();
    let brand = format!("SST  /  {}", title);
    view.push_str(&format!(
        "  {}{}{}\r\n",
        theme::TITLE, truncate_chrome(&brand, width.saturating_sub(4)), theme::RESET
    ));
    view.push_str(&format!(
        "  {}COMANDO{}  {}{}{}     {}ESC / Q  ·  CONSOLA{}",
        theme::LABEL, theme::RESET, theme::ACCENT,
        truncate_chrome(command, width.saturating_sub(31)), theme::RESET,
        theme::MUTED, theme::RESET,
    ));
    view.push_str("\r\n");
    if let Some(meter) = memory_meter(content, width) {
        view.push_str(&meter);
    }
    view.push_str("\r\n");

    for line in &visible {
        view.push_str("    ");
        view.push_str(&color_line(line));
        view.push_str("\r\n");
    }
    for _ in visible.len()..body_rows {
        view.push_str("\r\n");
    }
    view.push_str("\r\n");
    let last = (scroll + body_rows).min(lines.len());
    view.push_str(&format!(
        "  {}{first}–{last} / {total}{}   {}↑↓  PgUp/PgDn  Inicio/Fin  ·  Esc/Q salir{}",
        theme::LABEL, theme::RESET, theme::MUTED, theme::RESET,
        first = if lines.is_empty() { 0 } else { scroll + 1 },
        total = lines.len(),
    ));
    let frames = section_panels(&visible, body_top, height.saturating_sub(2));
    (view, frames)
}

pub fn show_report(
    factory: &dyn TerminalFactory,
    title: &str,
    command: &str,
    content: &str,
) -> Result<()> {
    let mut terminal = factory.alternate_screen()?;
    let mut scroll = 0usize;
    let mut last_drawn: Option<(u16, u16, usize)> = None;
    loop {
        let (cols, rows) = terminal.size()?;
        let width = usize::from(cols).max(28);
        let height = usize::from(rows).max(10);
        let body_rows = height.saturating_sub(6).max(1);
        let lines = wrapped_lines(content, width.saturating_sub(7));
        let max_scroll = lines.len().saturating_sub(body_rows);
        scroll = scroll.min(max_scroll);

        // A static tool only repaints after a key-navigation or viewport resize.
        if last_drawn != Some((cols, rows, scroll)) {
            let (view, panels) = render_page(title, command, content, &lines, scroll, width, height);
            terminal.clear()?;
            terminal.set_panels(panels)?;
            terminal.write(&view)?;
            terminal.flush()?;
            last_drawn = Some((cols, rows, scroll));
        }

        match terminal.poll_key(Duration::from_millis(180))? {
            Some(TerminalKey::Escape | TerminalKey::Char('q') | TerminalKey::Char('Q')) => break,
            Some(TerminalKey::Up) => scroll = scroll.saturating_sub(1),
            Some(TerminalKey::Down) => scroll = (scroll + 1).min(max_scroll),
            Some(TerminalKey::PageUp) => scroll = scroll.saturating_sub(body_rows),
            Some(TerminalKey::PageDown | TerminalKey::Space) =>
                scroll = (scroll + body_rows).min(max_scroll),
            Some(TerminalKey::Home) => scroll = 0,
            Some(TerminalKey::End) => scroll = max_scroll,
            _ => {}
        }
    }
    // The session's Drop restores the original console, not a recreated prompt.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_values_have_distinct_visual_roles() {
        let row = ReportLine { text: "Estado: OPERATIVO".to_owned(), heading: false };
        let text = color_line(&row);
        assert!(text.contains(theme::LABEL));
        assert!(text.contains(theme::GOOD));
        let row = ReportLine { text: "Hostname: PC-01".to_owned(), heading: false };
        assert!(color_line(&row).contains(theme::TEXT));
    }

    #[test]
    fn table_spacing_is_preserved() {
        let src = "DISCOS\nMOUNT                 TOTAL GiB   USED GiB   FREE GiB   USE%   FS\nC:\\                  256.0      123.1       132.9  48.1%   NTFS\n";
        let lines = wrapped_lines(src, 100);
        assert!(lines.iter().any(|r| r.text.starts_with("MOUNT                 TOTAL")));
        assert!(lines.iter().any(|r| r.text.contains("256.0      123.1")));
        assert!(!lines.iter().any(|r| r.heading && r.text.starts_with("MOUNT")));
    }

    #[test]
    fn long_evidence_is_never_truncated() {
        let src = "SEGURIDAD\n------------------------------------------------------------\nRuta: C:\\Program Files\\Equipo\\Carpeta\\aplicacion.exe\n";
        let lines = wrapped_lines(src, 26);
        assert!(lines.iter().any(|r| r.heading && r.text == "SEGURIDAD"));
        assert!(!lines.iter().any(|r| is_rule(&r.text)));
        assert!(lines.iter().map(|r| r.text.replace(' ', "")).collect::<String>()
            .contains("C:\\ProgramFiles\\Equipo\\Carpeta\\aplicacion.exe"));
    }

    #[test]
    fn memory_meter_uses_only_measured_values() {
        assert!(memory_meter("memoria: desconocida", 100).is_none());
        let meter = memory_meter("memory_total: 1000 MiB\nmemory_used: 250 MiB", 100).unwrap();
        assert!(meter.contains("25%"));
        assert!(meter.contains("250 / 1000 MiB"));
    }

    #[test]
    fn sections_get_individual_native_frames() {
        let lines = wrapped_lines("SISTEMA\nhostname: TEST\n\nMEMORIA\nmemory_total: 1000 MiB\n", 110);
        let (_, frames) = render_page("ESTADO DEL SISTEMA", "sys", "", &lines, 0, 110, 30);
        assert_eq!(frames.len(), 2);
        assert!(frames[0].start_row < frames[1].start_row);
        assert!(frames.iter().all(|frame| frame.rows >= 2));
    }
}
