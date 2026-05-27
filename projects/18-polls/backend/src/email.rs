//! Outbound email via SMTP — uses `lettre`.
//!
//! In dev we point at MailHog (smtp://localhost:1025) which catches everything
//! into a web UI at http://localhost:8025. In prod you'd swap the SMTP_URL
//! for Resend, Postmark, AWS SES, etc.
//!
//! The Mailer is `Send + Sync + 'static`, held in AppState behind Arc.
//! Errors in send are logged but NOT bubbled to the user — losing one
//! reset email shouldn't 500 the password-reset endpoint. (The user can
//! always request another.)

use lettre::AsyncTransport;
use lettre::Tokio1Executor;
use lettre::message::{Mailbox, MessageBuilder};
use lettre::transport::smtp::AsyncSmtpTransport;

pub struct Mailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl Mailer {
    pub fn new(smtp_url: &str, from: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let transport = if let Some(rest) = smtp_url.strip_prefix("smtp://") {
            // Plain SMTP (MailHog dev). lettre requires explicit relay+port.
            let (host, port) = match rest.split_once(':') {
                Some((h, p)) => (h.to_string(), p.parse::<u16>().unwrap_or(25)),
                None => (rest.to_string(), 25),
            };
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
                .port(port)
                .build()
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::from_url(smtp_url)?.build()
        };
        let from: Mailbox = from.parse()?;
        Ok(Self { transport, from })
    }

    pub async fn send(&self, to: &str, subject: &str, body: String) {
        let to: Mailbox = match to.parse() {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(err = %e, to, "invalid recipient address; skipping send");
                return;
            }
        };
        let msg = match MessageBuilder::new()
            .from(self.from.clone())
            .to(to)
            .subject(subject)
            .body(body)
        {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(err = %e, "failed to build message");
                return;
            }
        };
        if let Err(e) = self.transport.send(msg).await {
            tracing::warn!(err = %e, "failed to send email; user can request another");
        }
    }
}

pub fn verify_email_body(public_url: &str, name: &str, token: &str) -> (String, String) {
    let link = format!("{}/verify/{}", public_url.trim_end_matches('/'), token);
    let subject = "Verify your email".to_string();
    let body = format!(
        "Hi {name},\n\n\
         Click the link below to verify your email address:\n\n\
         {link}\n\n\
         The link expires in 24 hours. If you didn't sign up, ignore this email."
    );
    (subject, body)
}

pub fn password_reset_body(public_url: &str, name: &str, token: &str) -> (String, String) {
    let link = format!("{}/reset/{}", public_url.trim_end_matches('/'), token);
    let subject = "Reset your password".to_string();
    let body = format!(
        "Hi {name},\n\n\
         Someone requested a password reset for your account. If that was you, \
         click the link below within the next hour:\n\n\
         {link}\n\n\
         If it wasn't you, ignore this email and consider changing your password."
    );
    (subject, body)
}
