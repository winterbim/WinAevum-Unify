//! Minimal ISO-8601 (second precision) helpers — no chrono dependency required
//! for the core graph so local-first builds stay lean.

/// Parse `YYYY-MM-DDTHH:MM:SS[.frac][+TZ|Z]` into seconds since Unix epoch.
pub fn parse_iso_to_seconds(s: &str) -> Option<u64> {
    let mut body = s.trim();
    if let Some(stripped) = body.strip_suffix('Z').or_else(|| body.strip_suffix('z')) {
        body = stripped;
    } else if let Some(tpos) = body.find('T') {
        // Strip trailing ±HH:MM / ±HHMM offset after the time component.
        if let Some(rel) = body[tpos + 1..].rfind(['+', '-']) {
            let abs = tpos + 1 + rel;
            // Offset must look like +00:00 or -0500 (digit after sign).
            if body
                .as_bytes()
                .get(abs + 1)
                .is_some_and(|c| c.is_ascii_digit())
            {
                body = &body[..abs];
            }
        }
    }

    let parts: Vec<&str> = body.split('T').collect();
    if parts.len() != 2 {
        return None;
    }
    let date: Vec<u32> = parts[0].split('-').filter_map(|x| x.parse().ok()).collect();
    let time: Vec<u32> = parts[1]
        .split(':')
        .filter_map(|x| x.split_terminator('.').next()?.parse().ok())
        .collect();
    if date.len() != 3 || time.len() < 3 {
        return None;
    }
    let days_from_epoch = days_since_1970(date[0], date[1], date[2])?;
    Some(
        days_from_epoch * 86_400
            + (time[0] as u64) * 3600
            + (time[1] as u64) * 60
            + (time[2] as u64),
    )
}

fn days_since_1970(y: u32, m: u32, d: u32) -> Option<u64> {
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let mut days: u64 = 0;
    for yy in 1970..y {
        days += if is_leap(yy) { 366 } else { 365 };
    }
    let months = [
        31u64,
        if is_leap(y) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    for mm in 1..m {
        days += months[(mm - 1) as usize];
    }
    days += (d - 1) as u64;
    Some(days)
}

fn is_leap(y: u32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

/// True when `point` is inside `[valid_at, invalid_at)`.
/// Missing `invalid_at` means still valid.
pub fn is_valid_at(valid_at: &str, invalid_at: Option<&str>, point: &str) -> bool {
    let Some(p) = parse_iso_to_seconds(point) else {
        return false;
    };
    let Some(v) = parse_iso_to_seconds(valid_at) else {
        return false;
    };
    if p < v {
        return false;
    }
    if let Some(inv) = invalid_at {
        let Some(i) = parse_iso_to_seconds(inv) else {
            return false;
        };
        return p < i;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_utc_z() {
        assert_eq!(
            parse_iso_to_seconds("2026-08-02T10:00:00Z"),
            parse_iso_to_seconds("2026-08-02T10:00:00+00:00")
        );
        assert!(parse_iso_to_seconds("2026-08-02T10:00:00Z").is_some());
    }

    #[test]
    fn bi_temporal_window() {
        assert!(is_valid_at(
            "2026-08-02T10:00:00Z",
            Some("2026-08-02T12:00:00Z"),
            "2026-08-02T11:00:00Z"
        ));
        assert!(!is_valid_at(
            "2026-08-02T10:00:00Z",
            Some("2026-08-02T12:00:00Z"),
            "2026-08-02T12:00:00Z"
        ));
        assert!(is_valid_at(
            "2026-08-02T10:00:00Z",
            None,
            "2099-01-01T00:00:00Z"
        ));
    }
}
