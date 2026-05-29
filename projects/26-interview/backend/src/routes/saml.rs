//! SAML 2.0 SP minimum surface.
//!
//! Ships the SP metadata XML so an enterprise customer's IdP can be
//! configured against this app. The ACS endpoint is documented but
//! not signature-verified here — the `samael` crate (commented in
//! Cargo.toml under "future work") is the right next step.

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::{get, post};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/metadata", get(metadata))
        .route("/acs", post(acs))
}

async fn metadata(State(s): State<AppState>) -> impl IntoResponse {
    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<md:EntityDescriptor xmlns:md="urn:oasis:names:tc:SAML:2.0:metadata"
                     entityID="{entity}">
  <md:SPSSODescriptor AuthnRequestsSigned="false"
                      WantAssertionsSigned="true"
                      protocolSupportEnumeration="urn:oasis:names:tc:SAML:2.0:protocol">
    <md:NameIDFormat>urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress</md:NameIDFormat>
    <md:AssertionConsumerService
        Binding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST"
        Location="{acs}"
        index="0"
        isDefault="true"/>
  </md:SPSSODescriptor>
</md:EntityDescriptor>
"#,
        entity = s.saml_entity_id,
        acs = s.saml_acs_url,
    );
    let mut h = HeaderMap::new();
    h.insert(
        header::CONTENT_TYPE,
        "application/xml; charset=utf-8".parse().unwrap(),
    );
    (StatusCode::OK, h, xml)
}

async fn acs(State(_s): State<AppState>) -> impl IntoResponse {
    // TODO: SAMLResponse verification via `samael` (future work). Today the
    // endpoint exists so the metadata is real (IdP configuration succeeds);
    // production deployments must implement the signature + assertion
    // consumption per the LESSON.
    StatusCode::NOT_IMPLEMENTED
}
