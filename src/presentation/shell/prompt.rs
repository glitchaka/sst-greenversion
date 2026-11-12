use std::{env, path::Path};

use crate::support::path;

/// Compact native SST prompt. ANSI colors express the industrial theme
/// without changing Bash commands or pretending this is a Linux host.
pub fn render(cwd: &Path) -> String {
    let user = env::var("USERNAME").unwrap_or_else(|_| "usuario".to_owned());
    let host = env::var("COMPUTERNAME").unwrap_or_else(|_| "windows".to_owned());
    let cwd = path::display(cwd);

    format!(
        "\x1b[38;2;226;218;194m╭─ \u{f007} {user}@{host}\x1b[0m  \x1b[38;2;79;187;188m\u{f07c} {cwd}\x1b[0m\r\n\x1b[1;38;2;254;130;84m╰─ \u{f120} ❯\x1b[0m "
    )
}

pub fn banner() -> &'static str {
    // The permanent Slint chrome contains the logo, version and real host
    // metrics; don't repeat a text logo or add fictitious startup data.
    "\r\n\x1b[1;38;2;254;130;84m  SHELL SHOCK TOOL  /  GREEN\x1b[0m\r\n\x1b[38;2;226;218;194m  Administración y diagnóstico nativo de Windows.\x1b[0m\r\n\x1b[38;2;189;174;147m  help  ·  sys  ·  net monitor  ·  triage  ·  instp  ·  sigsour\x1b[0m\r\n\r\n"
}
