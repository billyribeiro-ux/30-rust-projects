//! Provider trait + Google/GitHub implementations.
//!
//! Each provider exposes:
//!   * authorize URL builder
//!   * token exchange (POST to token endpoint)
//!   * userinfo fetch
//!
//! `ProviderConfig` lets us point the endpoints at a wiremock server in
//! tests — production overrides the defaults via env.

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Deserialize;
use std::collections::HashMap;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderId {
    Google,
    Github,
}

impl ProviderId {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "google" => Some(Self::Google),
            "github" => Some(Self::Github),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Google => "google",
            Self::Github => "github",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub client_id: String,
    pub client_secret: String,
    pub authorize_url: String,
    pub token_url: String,
    pub userinfo_url: String,
    pub redirect_uri: String,
    /// `openid email profile` for Google; `read:user user:email` for GitHub.
    pub scope: &'static str,
}

#[derive(Debug, Clone)]
pub struct ProviderUserInfo {
    pub provider_user_id: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

/// What the provider's token endpoint returns. Both Google and GitHub
/// give us `access_token`; Google additionally returns `id_token` when
/// scope includes `openid`. We use `id_token` solely to verify the
/// `nonce` we sent on /authorize.
#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub id_token: Option<String>,
    #[allow(dead_code)]
    pub token_type: Option<String>,
    #[allow(dead_code)]
    pub expires_in: Option<i64>,
}

#[async_trait]
pub trait OAuthProvider: Send + Sync {
    fn id(&self) -> ProviderId;
    fn config(&self) -> &ProviderConfig;
    /// Build the authorize URL. Returns a URL the caller should redirect
    /// the browser to.
    fn authorize_url(&self, state: &str, challenge: &str, nonce: &str) -> String;
    /// Exchange the auth code (with the PKCE verifier) for tokens.
    async fn exchange_code(&self, code: &str, verifier: &str) -> AppResult<TokenResponse>;
    /// Fetch userinfo from the provider with the access token.
    async fn fetch_userinfo(&self, access_token: &str) -> AppResult<ProviderUserInfo>;
    /// Optional: verify the `nonce` claim in id_token. Only Google
    /// returns id_token; GitHub does not, so the default impl is a
    /// no-op.
    fn verify_id_token_nonce(&self, _id_token: &str, _expected_nonce: &str) -> AppResult<()> {
        Ok(())
    }
}

// ---------- Google ----------

pub struct GoogleProvider {
    cfg: ProviderConfig,
    http: reqwest::Client,
}

impl GoogleProvider {
    pub fn new(cfg: ProviderConfig, http: reqwest::Client) -> Self {
        Self { cfg, http }
    }
}

#[async_trait]
impl OAuthProvider for GoogleProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Google
    }
    fn config(&self) -> &ProviderConfig {
        &self.cfg
    }

    fn authorize_url(&self, state: &str, challenge: &str, nonce: &str) -> String {
        // Google requires `response_type=code`, `access_type=online`
        // (or `offline` if you want a refresh token — we don't), and
        // accepts `prompt=consent` to force re-consent. We pass
        // `code_challenge_method=S256` because PKCE plain is unsafe.
        let mut url = url::Url::parse(&self.cfg.authorize_url).expect("valid authorize url");
        url.query_pairs_mut()
            .append_pair("client_id", &self.cfg.client_id)
            .append_pair("redirect_uri", &self.cfg.redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", self.cfg.scope)
            .append_pair("state", state)
            .append_pair("code_challenge", challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("nonce", nonce)
            .append_pair("access_type", "online")
            .append_pair("include_granted_scopes", "true");
        url.into()
    }

    async fn exchange_code(&self, code: &str, verifier: &str) -> AppResult<TokenResponse> {
        let form = [
            ("code", code),
            ("client_id", &self.cfg.client_id),
            ("client_secret", &self.cfg.client_secret),
            ("redirect_uri", &self.cfg.redirect_uri),
            ("grant_type", "authorization_code"),
            ("code_verifier", verifier),
        ];
        let res = self
            .http
            .post(&self.cfg.token_url)
            .form(&form)
            .send()
            .await?;
        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(AppError::Upstream(format!(
                "google token exchange failed: {body}"
            )));
        }
        let token: TokenResponse = res.json().await?;
        Ok(token)
    }

    async fn fetch_userinfo(&self, access_token: &str) -> AppResult<ProviderUserInfo> {
        #[derive(Deserialize)]
        struct Body {
            sub: String,
            email: Option<String>,
            name: Option<String>,
        }
        let res = self
            .http
            .get(&self.cfg.userinfo_url)
            .bearer_auth(access_token)
            .send()
            .await?
            .error_for_status()?;
        let b: Body = res.json().await?;
        Ok(ProviderUserInfo {
            provider_user_id: b.sub,
            email: b.email,
            name: b.name,
        })
    }

    fn verify_id_token_nonce(&self, id_token: &str, expected_nonce: &str) -> AppResult<()> {
        // For the curriculum we decode the JWT payload without verifying
        // the signature (the token was just delivered over TLS to us by
        // Google's token endpoint, so we trust transport here). In
        // production you would verify against Google's JWKS.
        decode_jwt_claim(id_token, "nonce").and_then(|nonce| {
            if super::pkce::constant_eq(&nonce, expected_nonce) {
                Ok(())
            } else {
                Err(AppError::Upstream("id_token nonce mismatch".into()))
            }
        })
    }
}

// ---------- GitHub ----------

pub struct GithubProvider {
    cfg: ProviderConfig,
    http: reqwest::Client,
}

impl GithubProvider {
    pub fn new(cfg: ProviderConfig, http: reqwest::Client) -> Self {
        Self { cfg, http }
    }
}

#[async_trait]
impl OAuthProvider for GithubProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Github
    }
    fn config(&self) -> &ProviderConfig {
        &self.cfg
    }

    fn authorize_url(&self, state: &str, challenge: &str, _nonce: &str) -> String {
        // GitHub supports PKCE since 2022 but ignores `nonce` (no OIDC
        // id_token). We send PKCE + state.
        let mut url = url::Url::parse(&self.cfg.authorize_url).expect("valid authorize url");
        url.query_pairs_mut()
            .append_pair("client_id", &self.cfg.client_id)
            .append_pair("redirect_uri", &self.cfg.redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", self.cfg.scope)
            .append_pair("state", state)
            .append_pair("code_challenge", challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("allow_signup", "true");
        url.into()
    }

    async fn exchange_code(&self, code: &str, verifier: &str) -> AppResult<TokenResponse> {
        let form = [
            ("code", code),
            ("client_id", &self.cfg.client_id),
            ("client_secret", &self.cfg.client_secret),
            ("redirect_uri", &self.cfg.redirect_uri),
            ("grant_type", "authorization_code"),
            ("code_verifier", verifier),
        ];
        let res = self
            .http
            .post(&self.cfg.token_url)
            .header("accept", "application/json")
            .form(&form)
            .send()
            .await?;
        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(AppError::Upstream(format!(
                "github token exchange failed: {body}"
            )));
        }
        let token: TokenResponse = res.json().await?;
        Ok(token)
    }

    async fn fetch_userinfo(&self, access_token: &str) -> AppResult<ProviderUserInfo> {
        #[derive(Deserialize)]
        struct UserBody {
            id: i64,
            login: String,
            name: Option<String>,
            email: Option<String>,
        }
        let user: UserBody = self
            .http
            .get(&self.cfg.userinfo_url)
            .bearer_auth(access_token)
            .header("user-agent", "finder-backend/0.1")
            .header("accept", "application/vnd.github+json")
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(ProviderUserInfo {
            provider_user_id: user.id.to_string(),
            email: user.email,
            name: user.name.or(Some(user.login)),
        })
    }
}

// ---------- helpers ----------

/// Decode a base64url JWT claim without verifying signature. Returns
/// the value of `claim` as a string (or an Upstream error if absent or
/// malformed).
fn decode_jwt_claim(token: &str, claim: &str) -> AppResult<String> {
    let mut parts = token.split('.');
    let _header = parts.next();
    let payload = parts
        .next()
        .ok_or_else(|| AppError::Upstream("id_token missing payload".into()))?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| AppError::Upstream("id_token payload not base64url".into()))?;
    let claims: HashMap<String, serde_json::Value> = serde_json::from_slice(&bytes)
        .map_err(|_| AppError::Upstream("id_token payload not JSON".into()))?;
    match claims.get(claim) {
        Some(serde_json::Value::String(s)) => Ok(s.clone()),
        _ => Err(AppError::Upstream(format!(
            "id_token missing string claim {claim}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(token_url: &str) -> ProviderConfig {
        ProviderConfig {
            client_id: "cid".into(),
            client_secret: "secret".into(),
            authorize_url: "https://example.test/authorize".into(),
            token_url: token_url.into(),
            userinfo_url: "https://example.test/userinfo".into(),
            redirect_uri: "http://localhost:3019/api/auth/oauth/google/callback".into(),
            scope: "openid email profile",
        }
    }

    #[test]
    fn google_authorize_url_includes_pkce_state_nonce() {
        let p = GoogleProvider::new(cfg("https://example.test/token"), reqwest::Client::new());
        let u = p.authorize_url("STATE", "CHAL", "NONCE");
        assert!(u.contains("state=STATE"));
        assert!(u.contains("code_challenge=CHAL"));
        assert!(u.contains("code_challenge_method=S256"));
        assert!(u.contains("nonce=NONCE"));
        assert!(u.contains("response_type=code"));
    }

    #[test]
    fn github_authorize_url_includes_pkce_state_no_nonce() {
        let mut c = cfg("https://example.test/token");
        c.scope = "read:user user:email";
        let p = GithubProvider::new(c, reqwest::Client::new());
        let u = p.authorize_url("STATE", "CHAL", "NONCE-IGNORED");
        assert!(u.contains("state=STATE"));
        assert!(u.contains("code_challenge=CHAL"));
        assert!(u.contains("code_challenge_method=S256"));
        assert!(!u.contains("nonce="));
    }
}
