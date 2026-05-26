//! ICS (iCalendar) export of next-step due dates.
//!
//! RFC 5545 VCALENDAR format. Subscribe-as-calendar URL: each next_step
//! becomes a VEVENT. Apple/Google Calendar can subscribe to this feed
//! and the dates show up alongside meetings.
//!
//! We hand-roll the file because the format is small and predictable —
//! no need for a crate. The lesson: serialization formats with tight
//! specs (CRLF line endings, UTC `Z` suffix, fold-at-75-chars) are
//! cheap to emit if you read the RFC.

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Response, header};
use axum::routing::get;
use chrono::{DateTime, Utc};

use crate::auth::session::AuthUser;
use crate::error::AppResult;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/next-steps.ics", get(ics))
}

async fn ics(State(s): State<AppState>, user: AuthUser) -> AppResult<Response<Body>> {
    let rows = sqlx::query!(
        r#"
        SELECT ns.id, ns.body, ns.due_at, ns.completed_at, ns.created_at,
               a.company, a.role
        FROM next_steps ns
        JOIN applications a ON a.id = ns.application_id
        WHERE ns.user_id = $1
        ORDER BY ns.due_at ASC
        "#,
        user.id,
    )
    .fetch_all(&s.pool)
    .await?;

    let mut buf = String::new();
    write_line(&mut buf, "BEGIN:VCALENDAR");
    write_line(&mut buf, "VERSION:2.0");
    write_line(
        &mut buf,
        "PRODID:-//30-rust-projects//Project 12 Job Tracker//EN",
    );
    write_line(&mut buf, "CALSCALE:GREGORIAN");
    write_line(&mut buf, "METHOD:PUBLISH");
    write_line(&mut buf, "X-WR-CALNAME:Job applications — next steps");

    for r in rows {
        write_line(&mut buf, "BEGIN:VEVENT");
        write_line(
            &mut buf,
            &format!("UID:{}-12-applications@30-rust-projects.local", r.id),
        );
        write_line(&mut buf, &format!("DTSTAMP:{}", ics_dt(&r.created_at)));
        write_line(&mut buf, &format!("DTSTART:{}", ics_dt(&r.due_at)));
        // 30-minute reminder block by convention.
        write_line(&mut buf, "DURATION:PT30M");
        write_line(
            &mut buf,
            &format!(
                "SUMMARY:{}",
                ics_escape(&format!("{} — {} ({})", r.body, r.company, r.role))
            ),
        );
        if r.completed_at.is_some() {
            write_line(&mut buf, "STATUS:COMPLETED");
        } else {
            write_line(&mut buf, "STATUS:CONFIRMED");
        }
        // VALARM 1 hour before — most clients respect this for popup/email.
        write_line(&mut buf, "BEGIN:VALARM");
        write_line(&mut buf, "ACTION:DISPLAY");
        write_line(&mut buf, &format!("DESCRIPTION:{}", ics_escape(&r.body)));
        write_line(&mut buf, "TRIGGER:-PT1H");
        write_line(&mut buf, "END:VALARM");
        write_line(&mut buf, "END:VEVENT");
    }

    write_line(&mut buf, "END:VCALENDAR");

    let resp = Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, "text/calendar; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"next-steps.ics\"",
        )
        .body(Body::from(buf))
        .expect("response builds");
    Ok(resp)
}

/// Append a line with the mandatory CRLF ending. RFC 5545 §3.1: lines
/// SHALL be terminated by a CRLF sequence. We don't bother folding
/// (>75 chars) because our content is short.
fn write_line(buf: &mut String, line: &str) {
    buf.push_str(line);
    buf.push_str("\r\n");
}

/// `20260524T143000Z` — RFC 5545 §3.3.5 "UTC time" form.
fn ics_dt(t: &DateTime<Utc>) -> String {
    t.format("%Y%m%dT%H%M%SZ").to_string()
}

/// RFC 5545 §3.3.11: backslash, comma, semicolon, newline must be escaped.
fn ics_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(',', "\\,")
        .replace(';', "\\;")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dt_round_trip() {
        let t: DateTime<Utc> = "2026-05-24T14:30:00Z".parse().unwrap();
        assert_eq!(ics_dt(&t), "20260524T143000Z");
    }

    #[test]
    fn escape_handles_specials() {
        assert_eq!(
            ics_escape("hello, world; \\back\nline"),
            "hello\\, world\\; \\\\back\\nline"
        );
    }
}
