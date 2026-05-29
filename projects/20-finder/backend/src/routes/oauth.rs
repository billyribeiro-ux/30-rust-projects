//! OAuth 2.0 + PKCE routes.
//!
//! Two endpoints per provider:
//!   * `GET /api/auth/oauth/:provider/start`
//!     Generates state + PKCE verifier + nonce, stashes them in
//!     short-lived HttpOnly cookies, then 303s to the provider's
//!     authorize URL.
//!   * `GET /api/auth/oauth/:provider/callback?code=&state=`
//!     Compares the returned `state` against the cookie (constant
//!     time), exchanges the code (with the PKCE verifier) for tokens,
//!     verifies the id_token nonce (Google only), fetches userinfo,
//!     upserts (user + oauth_account), creates a session, redirects
//!     to the SvelteKit frontend.

use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect};
use axum::routing::get;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::session;
use crate::error::{AppError, AppResult};
use crate::oauth::pkce;
use crate::oauth::provider::ProviderId;
use crate::state::AppState;

pub const STATE_COOKIE: &str = "oauth_state";
pub const VERIFIER_COOKIE: &str = "oauth_verifier";
pub const NONCE_COOKIE: &str = "oauth_nonce";
pub const PROVIDER_COOKIE: &str = "oauth_provider";

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{provider}/start", get(start))
        .route("/{provider}/callback", get(callback))
}

fn short_cookie(name: &'static str, value: String, secure: bool) -> Cookie<'static> {
    Cookie::build((name, value))
        .http_only(true)
        .secure(secure)
        // Must be Lax (not Strict) so the cookie is sent when the
        // browser follows the provider's 302 back to our callback.
        .same_site(SameSite::Lax)
        .path("/api/auth/oauth")
        .max_age(time::Duration::minutes(10))
        .build()
}

fn clear_short(name: &'static str, secure: bool) -> Cookie<'static> {
    Cookie::build((name, ""))
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .path("/api/auth/oauth")
        .max_age(time::Duration::seconds(0))
        .build()
}

async fn start(
    State(s): State<AppState>,
    jar: CookieJar,
    Path(provider): Path<String>,
) -> AppResult<impl IntoResponse> {
    let pid = ProviderId::parse(&provider).ok_or(AppError::NotFound)?;
    let prov = s
        .oauth
        .get(pid)
        .ok_or_else(|| AppError::Internal(format!("provider {provider} not configured")))?;

    let state_tok = pkce::random_token();
    let verifier = pkce::random_token();
    let challenge = pkce::code_challenge_s256(&verifier);
    let nonce = pkce::random_token();

    let url = prov.authorize_url(&state_tok, &challenge, &nonce);

    let jar = jar
        .add(short_cookie(STATE_COOKIE, state_tok, s.secure_cookies))
        .add(short_cookie(VERIFIER_COOKIE, verifier, s.secure_cookies))
        .add(short_cookie(NONCE_COOKIE, nonce, s.secure_cookies))
        .add(short_cookie(
            PROVIDER_COOKIE,
            pid.as_str().to_string(),
            s.secure_cookies,
        ));

    Ok((jar, Redirect::to(&url)))
}

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

async fn callback(
    State(s): State<AppState>,
    jar: CookieJar,
    Path(provider): Path<String>,
    Query(q): Query<CallbackQuery>,
) -> AppResult<impl IntoResponse> {
    let pid = ProviderId::parse(&provider).ok_or(AppError::NotFound)?;
    let prov = s
        .oauth
        .get(pid)
        .ok_or_else(|| AppError::Internal(format!("provider {provider} not configured")))?;

    // If the provider returned ?error= just clear the temp cookies and
    // bounce to the failure page.
    if let Some(err) = q.error.as_deref() {
        let jar = clear_all(jar, s.secure_cookies);
        tracing::warn!(error = %err, "oauth provider returned error");
        return Ok((jar, Redirect::to(&s.oauth_failure_redirect)).into_response());
    }

    let code = q
        .code
        .as_deref()
        .ok_or_else(|| AppError::Validation("missing code".into()))?;
    let state_in = q
        .state
        .as_deref()
        .ok_or_else(|| AppError::Validation("missing state".into()))?;

    let cookie_state = jar.get(STATE_COOKIE).map(|c| c.value().to_string());
    let cookie_verifier = jar.get(VERIFIER_COOKIE).map(|c| c.value().to_string());
    let cookie_nonce = jar.get(NONCE_COOKIE).map(|c| c.value().to_string());

    let Some(expected_state) = cookie_state else {
        return Err(AppError::Validation("oauth state cookie missing".into()));
    };
    let Some(verifier) = cookie_verifier else {
        return Err(AppError::Validation("oauth verifier cookie missing".into()));
    };
    let nonce = cookie_nonce.unwrap_or_default();

    // CSRF defence: the state on the wire must match what we stored.
    // Constant-time compare so we don't leak it byte-by-byte.
    if !pkce::constant_eq(state_in, &expected_state) {
        return Err(AppError::Unauthorized);
    }

    let tokens = prov.exchange_code(code, &verifier).await?;

    // Google: verify the nonce in the id_token. GitHub: no-op.
    if pid == ProviderId::Google
        && let Some(id_token) = tokens.id_token.as_deref()
    {
        prov.verify_id_token_nonce(id_token, &nonce)?;
    }

    let info = prov.fetch_userinfo(&tokens.access_token).await?;

    // Upsert: by (provider, provider_user_id) first; fall back to
    // creating a user by email if it doesn't exist; otherwise link the
    // existing email-user to this provider.
    let user_id = upsert_oauth_user(&s.pool, pid, &info).await?;

    let (raw, _sid) = session::create(&s.pool, user_id, None, None).await?;
    let jar = clear_all(jar, s.secure_cookies).add(session::session_cookie(raw, s.secure_cookies));

    Ok((jar, Redirect::to(&s.oauth_success_redirect)).into_response())
}

fn clear_all(mut jar: CookieJar, secure: bool) -> CookieJar {
    for n in [STATE_COOKIE, VERIFIER_COOKIE, NONCE_COOKIE, PROVIDER_COOKIE] {
        jar = jar.add(clear_short(n, secure));
    }
    jar
}

pub async fn upsert_oauth_user(
    pool: &sqlx::PgPool,
    pid: ProviderId,
    info: &crate::oauth::provider::ProviderUserInfo,
) -> AppResult<Uuid> {
    let provider = pid.as_str();

    // 1) Already linked? Reuse the user.
    if let Some(row) = sqlx::query!(
        r#"SELECT user_id FROM oauth_accounts WHERE provider = $1 AND provider_user_id = $2"#,
        provider,
        info.provider_user_id,
    )
    .fetch_optional(pool)
    .await?
    {
        return Ok(row.user_id);
    }

    // 2) Same email already in users? Link without creating a new row.
    if let Some(email) = info.email.as_deref() {
        let email_norm = email.trim().to_lowercase();
        if let Some(u) = sqlx::query!(r#"SELECT id FROM users WHERE email = $1"#, email_norm)
            .fetch_optional(pool)
            .await?
        {
            sqlx::query!(
                r#"INSERT INTO oauth_accounts (user_id, provider, provider_user_id, email)
                   VALUES ($1, $2, $3, $4)
                   ON CONFLICT (provider, provider_user_id) DO NOTHING"#,
                u.id,
                provider,
                info.provider_user_id,
                info.email,
            )
            .execute(pool)
            .await?;
            return Ok(u.id);
        }
    }

    // 3) Brand-new user. Email may legitimately be absent (private GitHub
    //    profile); we synthesise a placeholder so the UNIQUE constraint
    //    is satisfied. The user can change it later.
    let id = Uuid::new_v4();
    let email = info
        .email
        .clone()
        .map(|e| e.trim().to_lowercase())
        .unwrap_or_else(|| format!("{}+{}@oauth.local", provider, info.provider_user_id));
    let name = info.name.clone().unwrap_or_default();

    sqlx::query!(
        r#"INSERT INTO users (id, email, password_hash, name)
           VALUES ($1, $2, NULL, $3)"#,
        id,
        email,
        name,
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        r#"INSERT INTO oauth_accounts (user_id, provider, provider_user_id, email)
           VALUES ($1, $2, $3, $4)"#,
        id,
        provider,
        info.provider_user_id,
        info.email,
    )
    .execute(pool)
    .await?;

    Ok(id)
}

// The router for oauth is mounted at /api/auth/oauth so the `start` and
// `callback` paths inherit that prefix. The "method must return at least
// one type" trick for callbacks: we return `axum::response::Response`
// directly via .into_response() so the two branches share a type.

// Re-export for tests/integration imports.
pub use crate::oauth::pkce::{code_challenge_s256, constant_eq, random_token};

#[allow(dead_code)]
fn _unused_status() -> StatusCode {
    StatusCode::OK
}
