use std::fmt::Write;

pub(super) fn escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            '\t' | '\n' | '\r' => {
                let _ = write!(escaped, "&#{};", u32::from(c));
            }
            c if c >= ' ' && !matches!(c, '\u{fffe}' | '\u{ffff}') => escaped.push(c),
            _ => {}
        }
    }
    escaped
}
