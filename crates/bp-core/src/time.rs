//! Durations and times of day as people type them: "90", "1:30", "1:02:03".

/// "m:ss" below an hour, "h:mm:ss" above; whole seconds.
pub fn format_duration(secs: f64) -> String {
    let total = secs.max(0.0).round() as u64;
    let (h, m, s) = (total / 3600, (total / 60) % 60, total % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// Parse "90" (seconds), "1:30", "1:02:03", "1.5" or "1,5" (seconds).
/// `None` for anything else.
pub fn parse_duration(text: &str) -> Option<f64> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    let parts: Vec<&str> = t.split(':').collect();
    if parts.len() > 3 {
        return None;
    }
    let mut total = 0.0;
    for (i, part) in parts.iter().enumerate() {
        let last = i + 1 == parts.len();
        let v: f64 = if last {
            part.trim().replace(',', ".").parse().ok()?
        } else {
            part.trim().parse::<u64>().ok()? as f64
        };
        if !v.is_finite() || v < 0.0 || (i > 0 && v >= 60.0) {
            return None;
        }
        total = total * 60.0 + v;
    }
    Some(total)
}

/// A number typed with a dot or a comma ("606.125", "606,125").
pub fn parse_number(text: &str) -> Option<f64> {
    let v: f64 = text.trim().replace(',', ".").parse().ok()?;
    v.is_finite().then_some(v)
}

/// A number without trailing zeros ("606.125", "12").
pub fn format_number(v: f64) -> String {
    let s = format!("{v:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations() {
        assert_eq!(parse_duration("90"), Some(90.0));
        assert_eq!(parse_duration("1:30"), Some(90.0));
        assert_eq!(parse_duration("1:02:03"), Some(3723.0));
        assert_eq!(parse_duration("2,5"), Some(2.5));
        assert_eq!(parse_duration("1:75"), None);
        assert_eq!(parse_duration("x"), None);
        assert_eq!(parse_duration(""), None);
        assert_eq!(format_duration(90.0), "1:30");
        assert_eq!(format_duration(3723.0), "1:02:03");
        assert_eq!(format_number(606.125), "606.125");
        assert_eq!(format_number(12.0), "12");
        assert_eq!(parse_number("606,125"), Some(606.125));
    }
}
