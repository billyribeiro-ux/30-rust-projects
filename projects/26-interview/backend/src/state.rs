use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::broadcast;
use uuid::Uuid;

/// One broadcaster per interview-in-flight. The first WebSocket
/// connection allocates it; the last to disconnect drops it.
pub type Hubs = Arc<Mutex<HashMap<Uuid, broadcast::Sender<String>>>>;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub hubs: Hubs,
    pub public_url: String,
    pub saml_entity_id: String,
    pub saml_acs_url: String,
    pub secure_cookies: bool,
}
