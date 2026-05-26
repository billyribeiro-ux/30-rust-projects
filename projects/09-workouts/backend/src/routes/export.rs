use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::IntoResponse;
use std::fmt::Write as _;

use crate::error::AppResult;
use crate::state::AppState;

pub async fn export_csv(State(s): State<AppState>) -> AppResult<impl IntoResponse> {
    let rows = sqlx::query!(
        r#"
        SELECT w.performed_at, w.name AS workout_name, e.name AS exercise_name,
               e.muscle_group, s.weight_minor, s.reps, s.rir, s.set_order
        FROM sets s
        JOIN workouts w ON w.id = s.workout_id
        JOIN exercises e ON e.id = s.exercise_id
        ORDER BY w.performed_at ASC, s.set_order ASC
        "#,
    )
    .fetch_all(&s.pool)
    .await?;

    let mut body = String::with_capacity(rows.len() * 80 + 80);
    // Header row — explicit, machine-friendly column names.
    body.push_str(
        "performed_at,workout_name,exercise_name,muscle_group,weight_minor,reps,rir,set_order\n",
    );
    for r in rows {
        let _ = writeln!(
            body,
            "{},{},{},{},{},{},{},{}",
            csv_escape(&r.performed_at),
            csv_escape(&r.workout_name),
            csv_escape(&r.exercise_name),
            csv_escape(&r.muscle_group),
            r.weight_minor,
            r.reps,
            r.rir,
            r.set_order,
        );
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"workouts.csv\""),
    );
    Ok((headers, body))
}

/// RFC 4180-ish escaping: wrap in quotes if the field contains comma, quote,
/// CR or LF; double up embedded quotes.
fn csv_escape(raw: &str) -> String {
    let needs_quoting = raw.contains([',', '"', '\n', '\r']);
    if !needs_quoting {
        return raw.to_string();
    }
    let escaped = raw.replace('"', "\"\"");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_plain() {
        assert_eq!(csv_escape("hello"), "hello");
    }

    #[test]
    fn escape_with_comma() {
        assert_eq!(csv_escape("a, b"), "\"a, b\"");
    }

    #[test]
    fn escape_with_quotes() {
        assert_eq!(csv_escape("a \"b\" c"), "\"a \"\"b\"\" c\"");
    }

    #[test]
    fn escape_with_newline() {
        assert_eq!(csv_escape("line1\nline2"), "\"line1\nline2\"");
    }
}
