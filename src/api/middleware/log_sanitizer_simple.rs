//! Simple log sanitizer
//! Remove sensitive info from logs

use std::sync::OnceLock;

use regex::Regex;

static PRIVATE_KEY_PATTERN: OnceLock<Regex> = OnceLock::new();

pub fn sanitize_log_content(content: &str) -> String {
    let pattern = PRIVATE_KEY_PATTERN.get_or_init(|| Regex::new(r"0x[0-9a-fA-F]{64}").unwrap());

    pattern.replace_all(content, "[REDACTED]").to_string()
}

pub fn init_global_log_sanitizer() {
    // Initialize patterns
    let _ = PRIVATE_KEY_PATTERN.get_or_init(|| Regex::new(r"0x[0-9a-fA-F]{64}").unwrap());
}
