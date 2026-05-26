use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetInput {
    pub weight_minor: i64,
    pub reps: i64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PrInfo {
    pub volume_minor: i64,
    pub is_pr: bool,
}

/// Iterates sets in chronological order; marks each set as PR if its
/// volume (weight * reps) strictly exceeds the cumulative max volume.
/// Ties don't count as PRs (you have to BEAT the previous best, not match).
///
/// Input is assumed to be sorted ascending by timestamp; the caller is
/// responsible for that. Volume is computed as `weight_minor * reps` and
/// saturated (`saturating_mul`) so pathologically large inputs don't
/// overflow — proptest exercises this.
pub fn detect_prs(sets: &[SetInput]) -> Vec<PrInfo> {
    let mut out = Vec::with_capacity(sets.len());
    let mut best: i64 = 0;
    for s in sets {
        let volume = s.weight_minor.saturating_mul(s.reps);
        let is_pr = volume > best;
        if is_pr {
            best = volume;
        }
        out.push(PrInfo {
            volume_minor: volume,
            is_pr,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn ts(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).expect("valid epoch")
    }

    fn s(w: i64, r: i64, t: i64) -> SetInput {
        SetInput {
            weight_minor: w,
            reps: r,
            timestamp: ts(t),
        }
    }

    #[test]
    fn empty_in_empty_out() {
        assert!(detect_prs(&[]).is_empty());
    }

    #[test]
    fn first_set_is_pr() {
        let out = detect_prs(&[s(50_000, 5, 0)]);
        assert_eq!(out.len(), 1);
        assert!(out[0].is_pr);
        assert_eq!(out[0].volume_minor, 250_000);
    }

    #[test]
    fn ascending_volume_all_prs() {
        let out = detect_prs(&[s(50_000, 5, 0), s(60_000, 5, 1), s(70_000, 5, 2)]);
        assert!(out.iter().all(|p| p.is_pr));
    }

    #[test]
    fn equal_volume_does_not_count_as_pr() {
        // 50 kg x 10 reps = 500_000 minor·reps; 100 kg x 5 reps = 500_000.
        let out = detect_prs(&[s(50_000, 10, 0), s(100_000, 5, 1)]);
        assert!(out[0].is_pr);
        assert!(!out[1].is_pr, "ties don't count — you must BEAT the max");
    }

    #[test]
    fn strictly_greater_does_count() {
        let out = detect_prs(&[s(50_000, 10, 0), s(50_000, 11, 1)]);
        assert!(out[0].is_pr);
        assert!(out[1].is_pr);
    }

    #[test]
    fn regression_does_not_clear_pr_state() {
        // After a PR, a weaker set is not a PR; a subsequent stronger set IS.
        let out = detect_prs(&[
            s(100_000, 5, 0), // PR (500_000)
            s(80_000, 5, 1),  // not (400_000)
            s(110_000, 5, 2), // PR (550_000)
        ]);
        assert_eq!(
            out.iter().map(|p| p.is_pr).collect::<Vec<_>>(),
            vec![true, false, true]
        );
    }

    // -------- property tests --------

    fn arb_set() -> impl Strategy<Value = SetInput> {
        // weight_minor and reps strictly positive to mirror DB constraints;
        // bounds keep volumes well below i64::MAX.
        (1i64..=1_000_000, 1i64..=30, 0i64..=1_000_000).prop_map(|(w, r, t)| s(w, r, t))
    }

    fn arb_sets() -> impl Strategy<Value = Vec<SetInput>> {
        prop::collection::vec(arb_set(), 0..=50).prop_map(|mut v| {
            v.sort_by_key(|s| s.timestamp);
            v
        })
    }

    proptest! {
        // The volume sequence of PRs is strictly increasing — that's the
        // definitional property of a personal record.
        #[test]
        fn prs_are_monotone(sets in arb_sets()) {
            let out = detect_prs(&sets);
            let pr_volumes: Vec<i64> = out.iter().filter(|p| p.is_pr).map(|p| p.volume_minor).collect();
            for w in pr_volumes.windows(2) {
                prop_assert!(w[1] > w[0], "PRs must be strictly increasing in volume");
            }
        }

        // Any non-empty input has at least one PR — the very first set,
        // whose volume is > 0 (both weight and reps are > 0 by construction).
        #[test]
        fn first_set_is_always_pr(sets in arb_sets()) {
            let out = detect_prs(&sets);
            if let Some(first) = out.first() {
                prop_assert!(first.is_pr);
            }
        }

        // Bounded: number of PRs cannot exceed total sets.
        #[test]
        fn pr_count_leq_total(sets in arb_sets()) {
            let out = detect_prs(&sets);
            let pr_count = out.iter().filter(|p| p.is_pr).count();
            prop_assert!(pr_count <= sets.len());
        }

        // Output length is exactly the input length and preserves volume.
        #[test]
        fn output_aligned_with_input(sets in arb_sets()) {
            let out = detect_prs(&sets);
            prop_assert_eq!(out.len(), sets.len());
            for (s, p) in sets.iter().zip(out.iter()) {
                prop_assert_eq!(p.volume_minor, s.weight_minor.saturating_mul(s.reps));
            }
        }

        // Idempotency: re-running on the same slice yields the same result.
        #[test]
        fn deterministic(sets in arb_sets()) {
            let a = detect_prs(&sets);
            let b = detect_prs(&sets);
            prop_assert_eq!(a, b);
        }
    }
}
