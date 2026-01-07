use chrono::{DateTime, TimeZone, Utc};

/// Format a Unix timestamp as relative time (e.g., "3 days ago")
pub fn format_relative(timestamp: i64) -> String {
    let dt = Utc.timestamp_opt(timestamp, 0).single();
    match dt {
        Some(dt) => format_relative_datetime(dt),
        None => "unknown".to_string(),
    }
}

/// Format a DateTime as relative time
pub fn format_relative_datetime(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(dt);
    let secs = duration.num_seconds();

    if secs < 60 {
        return "just now".to_string();
    }

    let units = [
        (365 * 24 * 60 * 60, "year", "years"),
        (30 * 24 * 60 * 60, "month", "months"),
        (7 * 24 * 60 * 60, "week", "weeks"),
        (24 * 60 * 60, "day", "days"),
        (60 * 60, "hour", "hours"),
        (60, "min", "mins"),
    ];

    for (unit_secs, singular, plural) in units {
        if secs >= unit_secs {
            let count = secs / unit_secs;
            let label = if count == 1 { singular } else { plural };
            return format!("{} {} ago", count, label);
        }
    }

    "just now".to_string()
}

/// Format a DateTime as short relative time (e.g., "3d ago" or "Jan 5" for older)
pub fn format_relative_short(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(dt);
    let secs = duration.num_seconds();

    if secs < 60 {
        return "now".to_string();
    }

    let units = [
        (7 * 24 * 60 * 60, "w"),
        (24 * 60 * 60, "d"),
        (60 * 60, "h"),
        (60, "m"),
    ];

    // For anything older than a week, use date format
    if secs >= 30 * 24 * 60 * 60 {
        return dt.format("%b %d").to_string();
    }

    for (unit_secs, suffix) in units {
        if secs >= unit_secs {
            let count = secs / unit_secs;
            return format!("{}{} ago", count, suffix);
        }
    }

    "now".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_format_relative_just_now() {
        let now = Utc::now();
        assert_eq!(format_relative_datetime(now), "just now");
    }

    #[test]
    fn test_format_relative_minutes() {
        let dt = Utc::now() - Duration::minutes(5);
        assert_eq!(format_relative_datetime(dt), "5 mins ago");
    }

    #[test]
    fn test_format_relative_hours() {
        let dt = Utc::now() - Duration::hours(2);
        assert_eq!(format_relative_datetime(dt), "2 hours ago");
    }

    #[test]
    fn test_format_relative_days() {
        let dt = Utc::now() - Duration::days(3);
        assert_eq!(format_relative_datetime(dt), "3 days ago");
    }

    #[test]
    fn test_format_relative_short() {
        let dt = Utc::now() - Duration::days(3);
        assert_eq!(format_relative_short(dt), "3d ago");
    }
}
