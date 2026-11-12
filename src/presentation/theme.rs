//! SST Green's single semantic color contract for *all* terminal output.
//! Text and diagnostics use measured states; colors never imply a fake status.
pub const RESET: &str = "\x1b[0m";
pub const TITLE: &str = "\x1b[1;38;2;226;218;194m"; // architectural cream
pub const ACCENT: &str = "\x1b[1;38;2;243;145;32m"; // heat marker
pub const TEXT: &str = "\x1b[38;2;226;218;194m"; // architectural cream
pub const LABEL: &str = "\x1b[1;38;2;79;187;188m"; // industrial aqua
pub const MUTED: &str = "\x1b[38;2;168;153;132m"; // legible muted neutral
pub const GOOD: &str = "\x1b[1;38;2;173;215;148m"; // operational
pub const WARN: &str = "\x1b[1;38;2;243;145;32m"; // warning
pub const BAD: &str = "\x1b[1;38;2;236;70;36m"; // critical
