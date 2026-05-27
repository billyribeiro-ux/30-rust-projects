//! ICS (iCalendar) export.
//!
//! `GET /api/export/{calendar_id}.ics` — emits VCALENDAR with one VEVENT
//! per event row (including RRULE — calendar apps know how to expand it,
//! so we ship the spec value verbatim and let the client unroll).
//!
//! See project 12's export.rs for the line-by-line lesson on the format.

use axum::Router;
use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{Response, header};
use axum::routing::get;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    // axum 0.8 disallows a static suffix after a path param in the same
    // segment ("/{id}.ics" is rejected). We rely on the Content-Type and
    // Content-Disposition filename to make this a calendar-app-friendly URL.
    Router::new().route("/calendar/{calendar_id}", get(ics))
}

async fn ics(
    State(s): State<AppState>,
    user: AuthUser,
    Path(calendar_id): Path<Uuid>,
) -> AppResult<Response<Body>> {
    // Read access: owner OR any share.
    let cal = sqlx::query!(
        r#"SELECT c.name FROM calendars c
           LEFT JOIN calendar_shares cs ON cs.calendar_id = c.id AND cs.user_id = $1
           WHERE c.id = $2 AND (c.owner_id = $1 OR cs.user_id = $1)"#,
        user.id,
        calendar_id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let rows = sqlx::query!(
        r#"SELECT id, title, description, location, start_at, end_at,
                  tz, rrule, created_at
           FROM events WHERE calendar_id = $1
           ORDER BY start_at ASC"#,
        calendar_id,
    )
    .fetch_all(&s.pool)
    .await?;

    let mut buf = String::new();
    write_line(&mut buf, "BEGIN:VCALENDAR");
    write_line(&mut buf, "VERSION:2.0");
    write_line(
        &mut buf,
        "PRODID:-//30-rust-projects//Project 14 Calendar//EN",
    );
    write_line(&mut buf, "CALSCALE:GREGORIAN");
    write_line(&mut buf, "METHOD:PUBLISH");
    write_line(&mut buf, &format!("X-WR-CALNAME:{}", ics_escape(&cal.name)));

    for r in rows {
        write_line(&mut buf, "BEGIN:VEVENT");
        write_line(
            &mut buf,
            &format!("UID:{}-14-calendar@30-rust-projects.local", r.id),
        );
        write_line(&mut buf, &format!("DTSTAMP:{}", ics_dt(&r.created_at)));
        write_line(&mut buf, &format!("DTSTART:{}", ics_dt(&r.start_at)));
        write_line(&mut buf, &format!("DTEND:{}", ics_dt(&r.end_at)));
        write_line(&mut buf, &format!("SUMMARY:{}", ics_escape(&r.title)));
        if !r.description.is_empty() {
            write_line(
                &mut buf,
                &format!("DESCRIPTION:{}", ics_escape(&r.description)),
            );
        }
        if !r.location.is_empty() {
            write_line(&mut buf, &format!("LOCATION:{}", ics_escape(&r.location)));
        }
        if let Some(rrule) = r.rrule.as_deref() {
            write_line(&mut buf, rrule);
        }
        write_line(&mut buf, "END:VEVENT");
    }
    write_line(&mut buf, "END:VCALENDAR");

    let resp = Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, "text/calendar; charset=utf-8")
        .header(
            header::CONTENT_DISPOSITION,
            "attachment; filename=\"calendar.ics\"",
        )
        .body(Body::from(buf))
        .expect("response builds");
    Ok(resp)
}

fn write_line(buf: &mut String, line: &str) {
    buf.push_str(line);
    buf.push_str("\r\n");
}

fn ics_dt(t: &DateTime<Utc>) -> String {
    t.format("%Y%m%dT%H%M%SZ").to_string()
}

fn ics_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(',', "\\,")
        .replace(';', "\\;")
        .replace('\n', "\\n")
}
