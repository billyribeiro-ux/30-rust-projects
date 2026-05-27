//! Outbound email via SMTP — same pattern as project 14.
//!
//! For project 16 the customer-facing email contains the signed download
//! link. We deliberately do NOT include sensitive content (no card data,
//! no full order details an attacker could enumerate from a forwarded
//! email). Just product name + link + expiry.

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
            tracing::warn!(err = %e, "failed to send email; customer can re-request from receipt page");
        }
    }
}

/// "Your download is ready" email. `link` is the full signed URL.
pub fn download_ready_body(
    public_url: &str,
    product_name: &str,
    link: &str,
    expires_hours: i64,
) -> (String, String) {
    let public_url = public_url.trim_end_matches('/');
    let subject = format!("Your {product_name} download is ready");
    let body = format!(
        "Thanks for your purchase!\n\n\
         Your download link for \"{product_name}\":\n\n\
         {link}\n\n\
         The link expires in {expires_hours} hours. If it expires, reply \
         to this email and we'll re-issue.\n\n\
         — {public_url}\n"
    );
    (subject, body)
}
