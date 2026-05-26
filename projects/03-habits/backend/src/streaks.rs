use chrono::{Duration, NaiveDate};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StreakInfo {
    pub current: u32,
    pub longest: u32,
    pub total: u32,
    pub last_completion: Option<NaiveDate>,
}

/// Computes streak info from a list of completion dates and "today".
///
/// `completions` MUST be sorted ascending and contain no duplicates.
/// The caller is responsible for this — we enforce it with `debug_assert!`
/// in debug builds but trust it in release for performance.
///
/// Streaks count consecutive days. A 1-day gap breaks a streak.
/// `current` is the length of the trailing run ending at today or yesterday;
/// 0 otherwise (a streak you broke two days ago is not "current").
pub fn compute(completions: &[NaiveDate], today: NaiveDate) -> StreakInfo {
    debug_assert!(
        completions.windows(2).all(|w| w[0] < w[1]),
        "completions must be sorted and unique"
    );

    if completions.is_empty() {
        return StreakInfo {
            current: 0,
            longest: 0,
            total: 0,
            last_completion: None,
        };
    }

    let mut longest: u32 = 1;
    let mut run: u32 = 1;
    for w in completions.windows(2) {
        if w[1] - w[0] == Duration::days(1) {
            run += 1;
            if run > longest {
                longest = run;
            }
        } else {
            run = 1;
        }
    }

    let last = *completions.last().expect("non-empty checked above");
    let yesterday = today - Duration::days(1);
    let current = if last == today || last == yesterday {
        run
    } else {
        0
    };

    StreakInfo {
        current,
        longest,
        total: completions.len() as u32,
        last_completion: Some(last),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("valid date")
    }

    #[test]
    fn empty_is_zero() {
        let info = compute(&[], d(2026, 5, 24));
        assert_eq!(info.current, 0);
        assert_eq!(info.longest, 0);
        assert_eq!(info.total, 0);
        assert!(info.last_completion.is_none());
    }

    #[test]
    fn single_completion_today() {
        let today = d(2026, 5, 24);
        let info = compute(&[today], today);
        assert_eq!(info.current, 1);
        assert_eq!(info.longest, 1);
        assert_eq!(info.total, 1);
    }

    #[test]
    fn single_completion_yesterday_still_current() {
        let today = d(2026, 5, 24);
        let info = compute(&[d(2026, 5, 23)], today);
        assert_eq!(info.current, 1);
    }

    #[test]
    fn single_completion_two_days_ago_is_broken() {
        let today = d(2026, 5, 24);
        let info = compute(&[d(2026, 5, 22)], today);
        assert_eq!(info.current, 0);
        assert_eq!(info.longest, 1);
    }

    #[test]
    fn week_run_ending_today() {
        let today = d(2026, 5, 24);
        let dates: Vec<NaiveDate> = (0..7).map(|n| today - Duration::days(6 - n)).collect();
        let info = compute(&dates, today);
        assert_eq!(info.current, 7);
        assert_eq!(info.longest, 7);
    }

    #[test]
    fn old_long_run_then_short_recent_run() {
        let today = d(2026, 5, 24);
        // 10-day run ending 2026-04-01, then a 2-day run ending today
        let mut dates: Vec<NaiveDate> = (0..10)
            .map(|n| d(2026, 3, 23) + Duration::days(n))
            .collect();
        dates.push(d(2026, 5, 23));
        dates.push(today);
        let info = compute(&dates, today);
        assert_eq!(info.current, 2);
        assert_eq!(info.longest, 10);
        assert_eq!(info.total, 12);
    }

    // -------- property tests --------

    fn arb_date() -> impl Strategy<Value = NaiveDate> {
        // bounded so dates stay in a sane range (chrono can produce out-of-range)
        (2020i32..=2030, 1u32..=12, 1u32..=28)
            .prop_map(|(y, m, d)| NaiveDate::from_ymd_opt(y, m, d).expect("clamped to valid"))
    }

    // Build a set of unique sorted dates by sampling offsets from a base date.
    fn arb_completions() -> impl Strategy<Value = Vec<NaiveDate>> {
        prop::collection::hash_set(0i64..=1000, 0..=200).prop_map(|set| {
            let base = NaiveDate::from_ymd_opt(2024, 1, 1).expect("valid");
            let mut v: Vec<NaiveDate> = set.into_iter().map(|n| base + Duration::days(n)).collect();
            v.sort();
            v
        })
    }

    proptest! {
        #[test]
        fn longest_never_exceeds_total(dates in arb_completions(), today in arb_date()) {
            let info = compute(&dates, today);
            prop_assert!(info.longest as usize <= info.total as usize);
        }

        #[test]
        fn longest_at_least_current(dates in arb_completions(), today in arb_date()) {
            let info = compute(&dates, today);
            prop_assert!(info.longest >= info.current);
        }

        #[test]
        fn empty_means_zero(today in arb_date()) {
            let info = compute(&[], today);
            prop_assert_eq!(info.current, 0);
            prop_assert_eq!(info.longest, 0);
            prop_assert_eq!(info.total, 0);
            prop_assert!(info.last_completion.is_none());
        }

        #[test]
        fn current_zero_unless_recent(dates in arb_completions(), today in arb_date()) {
            let info = compute(&dates, today);
            if let Some(last) = info.last_completion {
                let yesterday = today - Duration::days(1);
                if last != today && last != yesterday {
                    prop_assert_eq!(info.current, 0);
                }
            }
        }

        #[test]
        fn current_equals_trailing_run_when_recent(dates in arb_completions(), today in arb_date()) {
            let info = compute(&dates, today);
            if let Some(last) = info.last_completion {
                let yesterday = today - Duration::days(1);
                if last == today || last == yesterday {
                    // current must equal the length of the trailing consecutive run
                    let mut run: u32 = 1;
                    for w in dates.windows(2).rev() {
                        if w[1] - w[0] == Duration::days(1) {
                            run += 1;
                        } else {
                            break;
                        }
                    }
                    prop_assert_eq!(info.current, run);
                }
            }
        }

        #[test]
        fn total_equals_input_length(dates in arb_completions(), today in arb_date()) {
            let info = compute(&dates, today);
            prop_assert_eq!(info.total as usize, dates.len());
        }

        #[test]
        fn last_is_max(dates in arb_completions(), today in arb_date()) {
            let info = compute(&dates, today);
            prop_assert_eq!(info.last_completion, dates.iter().max().copied());
        }
    }
}
