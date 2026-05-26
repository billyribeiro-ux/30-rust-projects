use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult, FieldError};

/// One leg of a double-entry transaction.
/// `amount_minor` is the minor-unit amount (e.g., cents). Positive = debit,
/// negative = credit. The convention "debit = positive" is arbitrary and we
/// use it consistently throughout — it lets us validate "sum to zero" without
/// distinguishing direction at the type level.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PostingInput {
    pub account_id: String,
    pub amount_minor: i64,
}

/// Validates that a set of postings forms a balanced transaction:
/// - At least two postings
/// - Sum of `amount_minor` exactly zero
/// - No single posting is zero (a zero posting is a no-op and signals a bug)
/// - No duplicate account_id (would represent two separate accounting events)
pub fn validate_postings(postings: &[PostingInput]) -> AppResult<()> {
    if postings.len() < 2 {
        return Err(AppError::Fields(vec![FieldError {
            field: "postings".into(),
            message: "a transaction must have at least two postings".into(),
        }]));
    }

    let mut seen = std::collections::HashSet::new();
    let mut sum: i128 = 0;
    for (i, p) in postings.iter().enumerate() {
        if p.amount_minor == 0 {
            return Err(AppError::Fields(vec![FieldError {
                field: format!("postings.{i}.amount_minor"),
                message: "posting amount must not be zero".into(),
            }]));
        }
        if !seen.insert(p.account_id.clone()) {
            return Err(AppError::Fields(vec![FieldError {
                field: "postings".into(),
                message: format!("duplicate account in transaction: {}", p.account_id),
            }]));
        }
        sum = sum
            .checked_add(p.amount_minor as i128)
            .ok_or_else(|| AppError::Validation("posting sum overflows".into()))?;
    }
    if sum != 0 {
        return Err(AppError::Fields(vec![FieldError {
            field: "postings".into(),
            message: format!(
                "debits must equal credits (sum: {} minor units, must be 0)",
                sum
            ),
        }]));
    }
    Ok(())
}

/// Convert a `Decimal` (e.g., `12.34`) to minor units (cents = 1234).
/// Rejects values with more than 2 decimal places to avoid silent rounding.
pub fn decimal_to_minor(d: Decimal) -> Result<i64, String> {
    if d.scale() > 2 {
        return Err(format!("amount has more than 2 decimal places: {}", d));
    }
    let minor = d * Decimal::new(100, 0);
    let int = minor.trunc().mantissa();
    if minor.fract() != Decimal::ZERO {
        return Err("amount must be whole cents".into());
    }
    i64::try_from(int).map_err(|_| "amount overflows i64 minor units".into())
}

/// Convert minor units back to a Decimal for display: 1234 → 12.34.
#[allow(dead_code)] // Used in tests + reserved for downstream tooling.
pub fn minor_to_decimal(minor: i64) -> Decimal {
    Decimal::new(minor, 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn p(id: &str, amount: i64) -> PostingInput {
        PostingInput {
            account_id: id.into(),
            amount_minor: amount,
        }
    }

    #[test]
    fn balanced_pair_ok() {
        assert!(validate_postings(&[p("a", 100), p("b", -100)]).is_ok());
    }

    #[test]
    fn three_way_split_ok() {
        assert!(validate_postings(&[p("a", 100), p("b", 50), p("c", -150)]).is_ok());
    }

    #[test]
    fn unbalanced_rejected() {
        assert!(matches!(
            validate_postings(&[p("a", 100), p("b", -50)]),
            Err(AppError::Fields(_))
        ));
    }

    #[test]
    fn single_posting_rejected() {
        assert!(matches!(
            validate_postings(&[p("a", 0)]),
            Err(AppError::Fields(_))
        ));
    }

    #[test]
    fn zero_posting_rejected() {
        assert!(matches!(
            validate_postings(&[p("a", 0), p("b", 0)]),
            Err(AppError::Fields(_))
        ));
    }

    #[test]
    fn duplicate_account_rejected() {
        assert!(matches!(
            validate_postings(&[p("a", 100), p("a", -100)]),
            Err(AppError::Fields(_))
        ));
    }

    #[test]
    fn decimal_to_minor_round_trip() {
        let d = "12.34".parse::<Decimal>().unwrap();
        let m = decimal_to_minor(d).unwrap();
        assert_eq!(m, 1234);
        assert_eq!(minor_to_decimal(m), d);
    }

    #[test]
    fn decimal_to_minor_rejects_three_decimal_places() {
        let d = "12.345".parse::<Decimal>().unwrap();
        assert!(decimal_to_minor(d).is_err());
    }

    // ---------- properties ----------

    proptest! {
        // Any vec of postings whose amounts sum to zero (and no dups / no zeros
        // / >= 2 entries) must validate. We construct such a vec by sampling
        // N-1 amounts and setting the Nth to the negative sum.
        #[test]
        fn balanced_always_validates(
            mut amounts in prop::collection::vec(-1_000_000i64..=1_000_000, 1..=10),
        ) {
            // Filter zeros
            amounts.retain(|&a| a != 0);
            prop_assume!(!amounts.is_empty());

            // Add a balancing entry
            let s: i128 = amounts.iter().map(|&a| a as i128).sum();
            prop_assume!(s != 0); // need a non-zero balance entry
            let balance = -s;
            prop_assume!(i64::try_from(balance).is_ok());
            amounts.push(balance as i64);

            let postings: Vec<PostingInput> = amounts.iter().enumerate()
                .map(|(i, &a)| PostingInput { account_id: format!("acc{i}"), amount_minor: a })
                .collect();
            prop_assert!(validate_postings(&postings).is_ok());
        }

        // Any vec that doesn't sum to zero must NOT validate.
        #[test]
        fn unbalanced_never_validates(
            amounts in prop::collection::vec(1i64..=1_000_000, 2..=10),
        ) {
            // All positive amounts → sum > 0 → unbalanced
            let postings: Vec<PostingInput> = amounts.iter().enumerate()
                .map(|(i, &a)| PostingInput { account_id: format!("acc{i}"), amount_minor: a })
                .collect();
            prop_assert!(validate_postings(&postings).is_err());
        }
    }
}
