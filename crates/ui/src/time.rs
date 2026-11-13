//! Time formatting for the log.

/// Formats seconds since 1970-01-01 00:00 UTC as `YYYY-MM-DD HH:MM:SS` in UTC.
#[must_use]
pub fn format_utc(seconds: u64) -> String {
    let days: u64 = seconds / 86_400;
    let rest: u64 = seconds % 86_400;
    let (year, month, day): (u64, u64, u64) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// Converts days since 1970-01-01 to a (year, month, day) date.
/// Algorithm by Howard Hinnant, restricted to dates from 1970 on.
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z: u64 = days + 719_468;
    let era: u64 = z / 146_097;
    let day_of_era: u64 = z - era * 146_097;
    let year_of_era: u64 =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year: u64 = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index: u64 = (5 * day_of_year + 2) / 153;
    let day: u64 = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month: u64 = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year: u64 = year_of_era + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::format_utc;

    #[test]
    fn epoch() {
        assert_eq!(format_utc(0), "1970-01-01 00:00:00");
    }

    #[test]
    fn known_dates() {
        assert_eq!(format_utc(951_782_400), "2000-02-29 00:00:00");
        assert_eq!(format_utc(1_709_251_199), "2024-02-29 23:59:59");
        assert_eq!(format_utc(1_798_761_600), "2027-01-01 00:00:00");
        assert_eq!(format_utc(1_763_000_000), "2025-11-13 02:13:20");
    }
}
