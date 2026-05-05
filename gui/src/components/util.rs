pub fn humanize_duration(d: chrono::Duration) -> String {
    let secs = d.num_seconds().max(0);
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h{}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Strips ANSI escape sequences (CSI / OSC / single-char ESC) so a raw PTY
/// byte stream can be shown in a plain text widget without escape codes
/// rendering as garbage. Lossy (loses colours, cursor positioning) — meant
/// for a quick-glance preview, not full TUI rendering.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        match chars.next() {
            // CSI: ESC [ ... <letter>
            Some('[') => {
                for next in chars.by_ref() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            // OSC: ESC ] ... (BEL | ESC \)
            Some(']') => {
                while let Some(next) = chars.next() {
                    if next == '\x07' {
                        break;
                    }
                    if next == '\x1b' {
                        let _ = chars.next();
                        break;
                    }
                }
            }
            // Single-char ESC sequences (e.g. ESC = , ESC > )
            Some(_) | None => {}
        }
    }
    out
}
