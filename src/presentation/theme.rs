//! SST Green semantic terminal output.
//! The 16 ANSI colors are loaded from config/green-theme.toml at runtime.
//! SECTION is a non-presentation marker consumed by the native compositor.
pub const RESET: &str = "\x1b[0m";
pub const TITLE: &str = "\x1b[1;97m";
pub const SECTION: &str = "\x1b[1;38;2;213;196;161m";
pub const ACCENT: &str = "\x1b[1;33m";
pub const TEXT: &str = "\x1b[97m";
pub const LABEL: &str = "\x1b[1;36m";
pub const MUTED: &str = "\x1b[37m";
pub const GOOD: &str = "\x1b[1;92m";
pub const WARN: &str = "\x1b[1;93m";
pub const BAD: &str = "\x1b[1;91m";
