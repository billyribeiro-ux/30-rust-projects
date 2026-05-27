//! Customer-facing routes that don't require a login.
//!
//!  GET  /api/products              — list active products
//!  GET  /api/products/{id}         — single product
//!  POST /api/checkout              — start a checkout session, returns Stripe URL

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult, FieldError};
use crate::routes::auth::normalize_email;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/products", get(list_products))
        .route("/products/{id}", get(get_product))
        .route("/checkout", post(create_checkout))
}

#[derive(Debug, Serialize)]
pub struct ProductPublic {
    pub id: Uuid,
    pub sku: String,
    pub name: String,
    pub description: String,
    pub price_cents: i64,
    pub currency: String,
}

async fn list_products(State(s): State<AppState>) -> AppResult<Json<Vec<ProductPublic>>> {
    let rows = sqlx::query!(
        r#"
        SELECT id, sku, name, description, price_cents, currency
        FROM products
        WHERE active = TRUE
        ORDER BY name ASC
        "#,
    )
    .fetch_all(&s.pool)
    .await?;

    let out = rows
        .into_iter()
        .map(|r| ProductPublic {
            id: r.id,
            sku: r.sku,
            name: r.name,
            description: r.description,
            price_cents: r.price_cents,
            currency: r.currency,
        })
        .collect();
    Ok(Json(out))
}

async fn get_product(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ProductPublic>> {
    let r = sqlx::query!(
        r#"
        SELECT id, sku, name, description, price_cents, currency
        FROM products
        WHERE id = $1 AND active = TRUE
        "#,
        id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(ProductPublic {
        id: r.id,
        sku: r.sku,
        name: r.name,
        description: r.description,
        price_cents: r.price_cents,
        currency: r.currency,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CheckoutInput {
    pub email: String,
    pub product_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct CheckoutOutput {
    pub url: String,
    pub session_id: String,
    pub order_id: Uuid,
}

async fn create_checkout(
    State(s): State<AppState>,
    Json(input): Json<CheckoutInput>,
) -> AppResult<Json<CheckoutOutput>> {
    let email = normalize_email(&input.email)?;

    let product = sqlx::query!(
        r#"
        SELECT id, name, price_cents, currency, file_path
        FROM products
        WHERE id = $1 AND active = TRUE
        "#,
        input.product_id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    // Refuse to start a checkout for a product that has no file yet:
    // the customer would pay and we'd have nothing to deliver.
    if product.file_path.is_none() {
        return Err(AppError::Fields(vec![FieldError {
            field: "product_id".into(),
            message: "product has no file uploaded yet".into(),
        }]));
    }

    // Idempotency key: deterministic for (email, product_id, hour) so a
    // double-click in the same minute doesn't create two Stripe sessions.
    // The actual robustness comes from passing this as the Idempotency-Key
    // header to Stripe; even if a customer clicks ten times, Stripe returns
    // the same session id.
    let idem = format!(
        "co_{}_{}_{}",
        email,
        input.product_id,
        chrono::Utc::now().format("%Y%m%d%H")
    );

    let order_id = Uuid::new_v4();
    let success_url = format!(
        "{}/success?session_id={{CHECKOUT_SESSION_ID}}",
        s.public_url.trim_end_matches('/')
    );
    let cancel_url = format!("{}/cancel", s.public_url.trim_end_matches('/'));

    let mut metadata = std::collections::HashMap::new();
    metadata.insert("order_id".into(), order_id.to_string());
    metadata.insert("product_id".into(), product.id.to_string());

    let session = s
        .stripe
        .create_checkout_session(
            &email,
            &product.name,
            product.price_cents,
            &product.currency,
            &success_url,
            &cancel_url,
            &idem,
            &metadata,
        )
        .await?;

    // Record the pending order. The webhook will flip status to 'paid' /
    // 'fulfilled' once Stripe confirms.
    let mut tx = s.pool.begin().await?;
    sqlx::query!(
        r#"
        INSERT INTO orders (
            id, customer_email, stripe_session_id, status,
            amount_cents, currency, idempotency_key
        )
        VALUES ($1, $2, $3, 'pending', $4, $5, $6)
        ON CONFLICT (idempotency_key) DO UPDATE
            SET stripe_session_id = EXCLUDED.stripe_session_id
        "#,
        order_id,
        email,
        session.id,
        product.price_cents,
        product.currency,
        idem,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        r#"
        INSERT INTO order_items (order_id, product_id, quantity, unit_price_cents)
        VALUES ($1, $2, 1, $3)
        ON CONFLICT (order_id, product_id) DO NOTHING
        "#,
        order_id,
        product.id,
        product.price_cents,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(CheckoutOutput {
        url: session.url,
        session_id: session.id,
        order_id,
    }))
}
