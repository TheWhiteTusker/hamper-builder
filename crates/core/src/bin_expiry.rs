//! Recycle Bin retention (port of `bin.ts`).

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

pub const RETENTION_DAYS: i64 = 30;
const MS_PER_DAY: i128 = 24 * 60 * 60 * 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpiryInfo {
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
    pub days_remaining: i64,
    pub is_expired: bool,
}

/// When a binned item is purged (30 days after deletion) and how many days are left.
pub fn calculate_expiry(deleted_at: OffsetDateTime, now: OffsetDateTime) -> ExpiryInfo {
    let expires_at = deleted_at + Duration::days(RETENTION_DAYS);
    let ms_remaining = (expires_at - now).whole_milliseconds();
    // Ceiling division that also holds for negatives.
    let days = ms_remaining.div_euclid(MS_PER_DAY) + i128::from(ms_remaining.rem_euclid(MS_PER_DAY) != 0);
    ExpiryInfo {
        expires_at,
        days_remaining: i64::try_from(days.max(0)).unwrap_or(i64::MAX),
        is_expired: ms_remaining <= 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn fresh_item_has_the_full_window() {
        let now = datetime!(2026-09-24 12:00 UTC);
        let e = calculate_expiry(now, now);
        assert_eq!((e.days_remaining, e.is_expired), (RETENTION_DAYS, false));
        assert_eq!((e.expires_at - now).whole_days(), 30);
    }

    #[test]
    fn ten_day_old_item_has_twenty_left() {
        let now = datetime!(2026-09-24 12:00 UTC);
        let e = calculate_expiry(now - Duration::days(10), now);
        assert_eq!((e.days_remaining, e.is_expired), (20, false));
    }

    #[test]
    fn older_than_thirty_days_is_expired() {
        let now = datetime!(2026-09-24 12:00 UTC);
        let e = calculate_expiry(now - Duration::days(31), now);
        assert_eq!((e.days_remaining, e.is_expired), (0, true));
    }

    #[test]
    fn a_partial_day_counts_as_one() {
        let now = datetime!(2026-09-24 12:00 UTC);
        let e = calculate_expiry(now - Duration::days(29) - Duration::hours(23), now);
        assert_eq!(e.days_remaining, 1);
    }
}
