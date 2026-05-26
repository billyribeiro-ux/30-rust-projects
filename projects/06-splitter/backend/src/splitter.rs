use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult, FieldError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SplitKind {
    Equal,
    Exact,
    Percent,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShareInput {
    pub member_id: String,
    /// For `Exact`: cents share. For `Percent`: basis points (0..=10_000).
    /// Ignored for `Equal`.
    #[serde(default)]
    pub value: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ShareOutput {
    pub member_id: String,
    pub share_cents: i64,
}

/// Computes per-member shares given total + kind + inputs.
///
/// Invariants enforced:
/// - Every share_cents >= 0
/// - Sum of share_cents == amount_cents (exactly)
/// - No duplicate member ids in the input
/// - At least one share (for Equal); `value` constraints per kind
pub fn split(
    amount_cents: i64,
    kind: SplitKind,
    shares: &[ShareInput],
) -> AppResult<Vec<ShareOutput>> {
    if amount_cents <= 0 {
        return Err(AppError::Fields(vec![FieldError {
            field: "amount_cents".into(),
            message: "must be greater than 0".into(),
        }]));
    }
    if shares.is_empty() {
        return Err(AppError::Fields(vec![FieldError {
            field: "shares".into(),
            message: "at least one member required".into(),
        }]));
    }
    let mut seen = std::collections::HashSet::new();
    for s in shares {
        if !seen.insert(&s.member_id) {
            return Err(AppError::Fields(vec![FieldError {
                field: "shares".into(),
                message: format!("duplicate member: {}", s.member_id),
            }]));
        }
    }

    match kind {
        SplitKind::Equal => split_equal(amount_cents, shares),
        SplitKind::Exact => split_exact(amount_cents, shares),
        SplitKind::Percent => split_percent(amount_cents, shares),
    }
}

fn split_equal(amount_cents: i64, shares: &[ShareInput]) -> AppResult<Vec<ShareOutput>> {
    let n = shares.len() as i64;
    let base = amount_cents / n;
    let remainder = amount_cents - base * n; // in [0, n)
    Ok(shares
        .iter()
        .enumerate()
        .map(|(i, s)| ShareOutput {
            member_id: s.member_id.clone(),
            // The first `remainder` members each pay one extra cent so the
            // total adds up exactly without resorting to fractional cents.
            share_cents: base + if (i as i64) < remainder { 1 } else { 0 },
        })
        .collect())
}

fn split_exact(amount_cents: i64, shares: &[ShareInput]) -> AppResult<Vec<ShareOutput>> {
    let mut sum = 0i64;
    for s in shares {
        if s.value < 0 {
            return Err(AppError::Fields(vec![FieldError {
                field: format!("shares.{}", s.member_id),
                message: "share must be >= 0".into(),
            }]));
        }
        sum = sum
            .checked_add(s.value)
            .ok_or_else(|| AppError::Validation("share sum overflows i64".into()))?;
    }
    if sum != amount_cents {
        return Err(AppError::Fields(vec![FieldError {
            field: "shares".into(),
            message: format!("shares must sum to {} cents (got {})", amount_cents, sum),
        }]));
    }
    Ok(shares
        .iter()
        .map(|s| ShareOutput {
            member_id: s.member_id.clone(),
            share_cents: s.value,
        })
        .collect())
}

fn split_percent(amount_cents: i64, shares: &[ShareInput]) -> AppResult<Vec<ShareOutput>> {
    // value is in basis points (1/100 of a percent). 10_000 bp == 100%.
    let mut sum_bp: i64 = 0;
    for s in shares {
        if s.value < 0 || s.value > 10_000 {
            return Err(AppError::Fields(vec![FieldError {
                field: format!("shares.{}", s.member_id),
                message: "percent must be between 0 and 100".into(),
            }]));
        }
        sum_bp = sum_bp
            .checked_add(s.value)
            .ok_or_else(|| AppError::Validation("basis points sum overflows".into()))?;
    }
    if sum_bp != 10_000 {
        return Err(AppError::Fields(vec![FieldError {
            field: "shares".into(),
            message: format!(
                "percentages must sum to 100% (got {:.2}%)",
                sum_bp as f64 / 100.0
            ),
        }]));
    }

    // Truncating integer math first; then distribute the leftover cents to the
    // members with the largest fractional remainder. This is the standard
    // "largest remainder method" used to make percent splits sum exactly.
    let mut out: Vec<(usize, i64, i64)> = shares
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let exact = amount_cents.saturating_mul(s.value);
            let cents = exact / 10_000;
            let remainder = exact - cents * 10_000;
            (i, cents, remainder)
        })
        .collect();

    let assigned: i64 = out.iter().map(|(_, c, _)| *c).sum();
    let mut leftover = amount_cents - assigned;

    // Distribute leftover cents one at a time to the largest fractional
    // remainders, breaking ties by index (stable).
    let mut order: Vec<usize> = (0..out.len()).collect();
    order.sort_by(|&a, &b| out[b].2.cmp(&out[a].2).then(out[a].0.cmp(&out[b].0)));
    let mut k = 0;
    while leftover > 0 {
        let pos = order[k % order.len()];
        out[pos].1 += 1;
        leftover -= 1;
        k += 1;
    }

    Ok(out
        .into_iter()
        .map(|(i, cents, _)| ShareOutput {
            member_id: shares[i].member_id.clone(),
            share_cents: cents,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn s(id: &str, value: i64) -> ShareInput {
        ShareInput {
            member_id: id.to_string(),
            value,
        }
    }

    fn sum(out: &[ShareOutput]) -> i64 {
        out.iter().map(|o| o.share_cents).sum()
    }

    #[test]
    fn equal_divides_remainder() {
        // 100 cents / 3 members = [34, 33, 33] (sum = 100)
        let out = split(100, SplitKind::Equal, &[s("a", 0), s("b", 0), s("c", 0)]).unwrap();
        assert_eq!(sum(&out), 100);
        assert_eq!(out[0].share_cents, 34);
        assert_eq!(out[1].share_cents, 33);
        assert_eq!(out[2].share_cents, 33);
    }

    #[test]
    fn exact_must_sum_to_amount() {
        // sum mismatch: 30 + 60 != 100
        let err = split(100, SplitKind::Exact, &[s("a", 30), s("b", 60)]).unwrap_err();
        assert!(matches!(err, AppError::Fields(_)));
    }

    #[test]
    fn exact_ok_when_sum_matches() {
        let out = split(100, SplitKind::Exact, &[s("a", 30), s("b", 70)]).unwrap();
        assert_eq!(out[0].share_cents, 30);
        assert_eq!(out[1].share_cents, 70);
    }

    #[test]
    fn percent_basis_points_must_sum_to_100() {
        let err = split(100, SplitKind::Percent, &[s("a", 5000), s("b", 4000)]).unwrap_err();
        assert!(matches!(err, AppError::Fields(_)));
    }

    #[test]
    fn percent_distributes_leftover_cent() {
        // 100 cents @ 33.33% + 33.33% + 33.34% (bp 3333,3333,3334)
        let out = split(
            100,
            SplitKind::Percent,
            &[s("a", 3333), s("b", 3333), s("c", 3334)],
        )
        .unwrap();
        assert_eq!(sum(&out), 100);
    }

    #[test]
    fn rejects_zero_amount() {
        let err = split(0, SplitKind::Equal, &[s("a", 0)]).unwrap_err();
        assert!(matches!(err, AppError::Fields(_)));
    }

    #[test]
    fn rejects_duplicate_members() {
        let err = split(100, SplitKind::Equal, &[s("a", 0), s("a", 0)]).unwrap_err();
        assert!(matches!(err, AppError::Fields(_)));
    }

    // ------- property tests -------

    fn arb_amount() -> impl Strategy<Value = i64> {
        1i64..=100_000_000 // up to $1M
    }

    fn arb_member_ids() -> impl Strategy<Value = Vec<String>> {
        (1usize..=12).prop_map(|n| (0..n).map(|i| format!("m{i}")).collect())
    }

    proptest! {
        #[test]
        fn equal_always_sums_to_amount(amount in arb_amount(), ids in arb_member_ids()) {
            let inputs: Vec<ShareInput> = ids.iter().map(|id| s(id, 0)).collect();
            let out = split(amount, SplitKind::Equal, &inputs).unwrap();
            prop_assert_eq!(sum(&out), amount);
            for o in &out {
                prop_assert!(o.share_cents >= 0);
            }
        }

        #[test]
        fn equal_max_difference_is_one_cent(amount in arb_amount(), ids in arb_member_ids()) {
            let inputs: Vec<ShareInput> = ids.iter().map(|id| s(id, 0)).collect();
            let out = split(amount, SplitKind::Equal, &inputs).unwrap();
            let max = out.iter().map(|o| o.share_cents).max().unwrap();
            let min = out.iter().map(|o| o.share_cents).min().unwrap();
            prop_assert!(max - min <= 1);
        }

        #[test]
        fn percent_always_sums_to_amount(amount in arb_amount(), ids in arb_member_ids()) {
            // Build a percent split where the first member gets the remainder so
            // basis points sum to exactly 10_000.
            let n = ids.len() as i64;
            let per = 10_000 / n;
            let mut bps: Vec<i64> = vec![per; ids.len()];
            let used = per * n;
            bps[0] += 10_000 - used;
            let inputs: Vec<ShareInput> = ids
                .iter()
                .zip(bps.iter())
                .map(|(id, &bp)| s(id, bp))
                .collect();
            let out = split(amount, SplitKind::Percent, &inputs).unwrap();
            prop_assert_eq!(sum(&out), amount);
        }
    }
}
