//! Hand-rolled OAuth 2.0 Authorization Code + PKCE flow.
//!
//! We deliberately avoid `openidconnect` here so the reader sees every
//! field on the wire: `state`, `code_verifier`/`code_challenge`, `nonce`.
//!
//! Two providers are wired in:
//!   * Google (OpenID Connect — id_token returned; we verify the `nonce`)
//!   * GitHub (plain OAuth2 — no id_token; userinfo via REST)

pub mod pkce;
pub mod provider;

use std::collections::HashMap;

pub use provider::{OAuthProvider, ProviderConfig, ProviderId, ProviderUserInfo};

/// Indexed by `ProviderId`. Built once at startup from env vars.
pub struct OAuthRegistry {
    providers: HashMap<ProviderId, Box<dyn OAuthProvider>>,
}

impl OAuthRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    pub fn insert(&mut self, p: Box<dyn OAuthProvider>) {
        self.providers.insert(p.id(), p);
    }

    pub fn get(&self, id: ProviderId) -> Option<&dyn OAuthProvider> {
        self.providers.get(&id).map(|b| b.as_ref())
    }

    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}

impl Default for OAuthRegistry {
    fn default() -> Self {
        Self::new()
    }
}
