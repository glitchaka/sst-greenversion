//! SST Green visual roles shared by human-readable terminal presentations.
//! Values are deliberately stable: command outputs provide data, not palettes.
//! The island and original SST logo are intentionally excluded from recoloring.
pub const RESET: &str = "\x1b[0m";
pub const TITLE: &str = "\x1b[1;38;2;235;219;178m"; // architectural cream
pub const ACCENT: &str = "\x1b[1;38;2;254;128;25m"; // heat marker
pub const TEXT: &str = "\x1b[38;2;235;219;178m";
pub const LABEL: &str = "\x1b[1;38;2;189;174;147m";
pub const MUTED: &str = "\x1b[38;2;168;153;132m";
pub const GOOD: &str = "\x1b[1;38;2;184;187;38m";
pub const WARN: &str = "\x1b[1;38;2;250;189;47m";
pub const BAD: &str = "\x1b[1;38;2;251;73;52m";
pub const OPTIMAL: &str = "\x1b[1;38;2;142;192;124m";
