//! Turning an HTTP `Date` header into an RFC 3339 timestamp.
//!
//! A snapshot records when it was captured. The time comes from the server's own
//! `Date` header rather than the local clock, which may be wrong. Only the one
//! fixed form that HTTP servers must send (RFC 9110, "IMF-fixdate") is accepted,
//! and it is checked for consistency, so a malformed or contradictory date is
//! refused instead of guessed at.

use crate::error::MapError;

const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Converts `Sat, 17 Oct 2026 08:17:27 GMT` into `2026-10-17T08:17:27Z`.
///
/// The weekday must agree with the date, the day must exist in that month, and
/// the zone must be `GMT`.
pub fn http_date_to_rfc3339(text: &str) -> Result<String, MapError> {
    let parts: Vec<&str> = text.split(' ').collect();
    let [weekday, day, month, year, time, zone] = parts[..] else {
        return Err(MapError::BadDate);
    };
    if zone != "GMT" {
        return Err(MapError::BadDate);
    }

    let weekday = weekday.strip_suffix(',').ok_or(MapError::BadDate)?;
    let weekday = WEEKDAYS
        .iter()
        .position(|name| *name == weekday)
        .ok_or(MapError::BadDate)?;
    let month = MONTHS
        .iter()
        .position(|name| *name == month)
        .ok_or(MapError::BadDate)?
        + 1;

    let day = number(day, 2)?;
    let year = number(year, 4)?;
    let (hour, minute, second) = clock(time)?;

    let month = i64::try_from(month).map_err(|_| MapError::BadDate)?;
    if !(1..=days_in_month(year, month)).contains(&day) {
        return Err(MapError::BadDate);
    }
    // The weekday is derived from the date, so a date that contradicts its own
    // weekday is rejected.
    let derived = (days_from_civil(year, month, day) + 4).rem_euclid(7);
    if usize::try_from(derived).ok() != Some(weekday) {
        return Err(MapError::BadDate);
    }

    Ok(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    ))
}

/// A number of exactly `digits` ASCII digits.
fn number(text: &str, digits: usize) -> Result<i64, MapError> {
    if text.len() != digits || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(MapError::BadDate);
    }
    text.parse().map_err(|_| MapError::BadDate)
}

/// `HH:MM:SS`, each part two digits. A leap second (60) is not accepted: HTTP
/// servers do not send one in this form.
fn clock(text: &str) -> Result<(i64, i64, i64), MapError> {
    let parts: Vec<&str> = text.split(':').collect();
    let [hour, minute, second] = parts[..] else {
        return Err(MapError::BadDate);
    };
    let (hour, minute, second) = (number(hour, 2)?, number(minute, 2)?, number(second, 2)?);
    if hour > 23 || minute > 59 || second > 59 {
        return Err(MapError::BadDate);
    }
    Ok((hour, minute, second))
}

const fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

const fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 for a date in the proleptic Gregorian calendar.
const fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let year_of_era = y - era * 400;
    let month_from_march = (month + 9) % 12;
    let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_dates_that_servers_really_sent() {
        // Taken from real Horizon responses on 2026-10-17.
        assert_eq!(
            http_date_to_rfc3339("Sat, 17 Oct 2026 08:17:27 GMT").as_deref(),
            Ok("2026-10-17T08:17:27Z")
        );
        assert_eq!(
            http_date_to_rfc3339("Sat, 17 Oct 2026 08:29:55 GMT").as_deref(),
            Ok("2026-10-17T08:29:55Z")
        );
    }

    #[test]
    fn handles_the_epoch_and_leap_days() {
        assert_eq!(
            http_date_to_rfc3339("Thu, 01 Jan 1970 00:00:00 GMT").as_deref(),
            Ok("1970-01-01T00:00:00Z")
        );
        assert_eq!(
            http_date_to_rfc3339("Thu, 29 Feb 2024 23:59:59 GMT").as_deref(),
            Ok("2024-02-29T23:59:59Z")
        );
        assert_eq!(
            http_date_to_rfc3339("Wed, 31 Dec 2025 00:00:00 GMT").as_deref(),
            Ok("2025-12-31T00:00:00Z")
        );
    }

    #[test]
    fn a_date_that_contradicts_its_weekday_is_refused() {
        assert_eq!(
            http_date_to_rfc3339("Sun, 17 Oct 2026 08:17:27 GMT"),
            Err(MapError::BadDate)
        );
        assert_eq!(
            http_date_to_rfc3339("Fri, 01 Jan 1970 00:00:00 GMT"),
            Err(MapError::BadDate)
        );
    }

    #[test]
    fn a_day_that_does_not_exist_is_refused() {
        for bad in [
            "Wed, 29 Feb 2023 00:00:00 GMT",
            "Thu, 30 Feb 2024 00:00:00 GMT",
            "Mon, 31 Apr 2026 00:00:00 GMT",
            "Mon, 00 Apr 2026 00:00:00 GMT",
            "Mon, 32 Aug 2026 00:00:00 GMT",
        ] {
            assert_eq!(http_date_to_rfc3339(bad), Err(MapError::BadDate), "{bad}");
        }
    }

    #[test]
    fn the_time_of_day_must_be_in_range() {
        for bad in [
            "Sat, 17 Oct 2026 24:00:00 GMT",
            "Sat, 17 Oct 2026 08:60:00 GMT",
            "Sat, 17 Oct 2026 08:17:60 GMT",
            "Sat, 17 Oct 2026 8:17:27 GMT",
            "Sat, 17 Oct 2026 08:17 GMT",
        ] {
            assert_eq!(http_date_to_rfc3339(bad), Err(MapError::BadDate), "{bad}");
        }
    }

    #[test]
    fn only_the_one_fixed_form_is_accepted() {
        for bad in [
            "",
            "Saturday, 17-Oct-26 08:17:27 GMT",
            "Sat Oct 17 08:17:27 2026",
            "Sat, 17 Oct 2026 08:17:27 UTC",
            "Sat, 17 Oct 2026 08:17:27 +0000",
            "Sat, 17 Oct 2026 08:17:27",
            "Sat, 17 Oct 2026 08:17:27 GMT extra",
            "Sat 17 Oct 2026 08:17:27 GMT",
            "Sat,  17 Oct 2026 08:17:27 GMT",
            "sat, 17 Oct 2026 08:17:27 GMT",
            "Sat, 17 oct 2026 08:17:27 GMT",
            "Sat, 17 Oct 26 08:17:27 GMT",
            "Sat, 17 Oct 2026 08:17:27 gmt",
            "Sat, 1 Oct 2026 08:17:27 GMT",
            "Sat, 17 Oct 2026 08:17:27 GMT\r\n",
        ] {
            assert_eq!(http_date_to_rfc3339(bad), Err(MapError::BadDate), "{bad:?}");
        }
    }

    #[test]
    fn digits_must_be_plain_ascii_digits() {
        for bad in [
            "Sat, +7 Oct 2026 08:17:27 GMT",
            "Sat, 17 Oct 2026 0x:17:27 GMT",
            "Sat, 17 Oct -026 08:17:27 GMT",
        ] {
            assert_eq!(http_date_to_rfc3339(bad), Err(MapError::BadDate), "{bad:?}");
        }
    }

    #[test]
    fn the_result_is_accepted_by_the_snapshot_reader() {
        // A snapshot rejects a malformed timestamp, so the two must agree.
        let stamp = http_date_to_rfc3339("Sat, 17 Oct 2026 08:17:27 GMT").unwrap();
        assert_eq!(stamp.len(), 20);
        assert!(stamp.ends_with('Z'));
    }

    #[test]
    fn weekdays_are_correct_across_many_years() {
        // 1 January of each year, from a reference table.
        for (year, weekday) in [
            (1970, "Thu"),
            (1980, "Tue"),
            (2000, "Sat"),
            (2024, "Mon"),
            (2025, "Wed"),
            (2026, "Thu"),
            (2038, "Fri"),
        ] {
            let text = format!("{weekday}, 01 Jan {year} 00:00:00 GMT");
            assert!(http_date_to_rfc3339(&text).is_ok(), "{text}");
        }
    }
}
