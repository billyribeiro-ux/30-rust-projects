//! Admin routes — session-gated. Products CRUD + orders list + refund.
//!
//! For simplicity we accept a JSON payload for product file uploads that
//! contains a base64-encoded blob. Production would prefer a multipart
//! upload + pre-signed S3 URL (project 15 territory), but for the curriculum
//! the base64 path keeps the wire format the same as everywhere else.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/products", get(list_products).post(create_product))
        .route(
            "/products/{id}",
            get(get_product)
                .patch(update_product)
                .delete(delete_product),
        )
        .route("/products/{id}/file", post(upload_product_file))
        .route("/orders", get(list_orders))
        .route("/orders/{id}/refund", post(refund_order))
}

// ---------- products ----------

#[derive(Debug, Serialize)]
pub struct ProductAdmin {
    pub id: Uuid,
    pub sku: String,
    pub name: String,
    pub description: String,
    pub price_cents: i64,
    pub currency: String,
    pub file_path: Option<String>,
    pub file_name: Option<String>,
    pub file_size: Option<i64>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

async fn list_products(
    _user: AuthUser,
    State(s): State<AppState>,
) -> AppResult<Json<Vec<ProductAdmin>>> {
    let rows = sqlx::query_as!(
        ProductAdmin,
        r#"
        SELECT id, sku, name, description, price_cents, currency,
               file_path, file_name, file_size, active, created_at, updated_at
        FROM products
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(rows))
}

async fn get_product(
    _user: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ProductAdmin>> {
    let row = sqlx::query_as!(
        ProductAdmin,
        r#"
        SELECT id, sku, name, description, price_cents, currency,
               file_path, file_name, file_size, active, created_at, updated_at
        FROM products WHERE id = $1
        "#,
        id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}

#[derive(Debug, Deserialize)]
pub struct CreateProductInput {
    pub sku: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub price_cents: i64,
    #[serde(default = "default_currency")]
    pub currency: String,
}
fn default_currency() -> String {
    "usd".into()
}

async fn create_product(
    _user: AuthUser,
    State(s): State<AppState>,
    Json(input): Json<CreateProductInput>,
) -> AppResult<(StatusCode, Json<ProductAdmin>)> {
    validate_product(&input.sku, &input.name, input.price_cents, &input.currency)?;
    let id = Uuid::new_v4();
    let result = sqlx::query_as!(
        ProductAdmin,
        r#"
        INSERT INTO products (id, sku, name, description, price_cents, currency)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, sku, name, description, price_cents, currency,
                  file_path, file_name, file_size, active, created_at, updated_at
        "#,
        id,
        input.sku,
        input.name,
        input.description,
        input.price_cents,
        input.currency.to_lowercase(),
    )
    .fetch_one(&s.pool)
    .await;

    match result {
        Ok(row) => Ok((StatusCode::CREATED, Json(row))),
        Err(sqlx::Error::Database(dbe)) if dbe.constraint() == Some("products_sku_key") => {
            Err(AppError::Conflict("SKU already exists".into()))
        }
        Err(e) => Err(e.into()),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateProductInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub price_cents: Option<i64>,
    pub active: Option<bool>,
}

async fn update_product(
    _user: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateProductInput>,
) -> AppResult<Json<ProductAdmin>> {
    if let Some(p) = input.price_cents
        && p <= 0
    {
        return Err(AppError::Fields(vec![FieldError {
            field: "price_cents".into(),
            message: "must be > 0".into(),
        }]));
    }
    let row = sqlx::query_as!(
        ProductAdmin,
        r#"
        UPDATE products SET
            name        = COALESCE($2, name),
            description = COALESCE($3, description),
            price_cents = COALESCE($4, price_cents),
            active      = COALESCE($5, active)
        WHERE id = $1
        RETURNING id, sku, name, description, price_cents, currency,
                  file_path, file_name, file_size, active, created_at, updated_at
        "#,
        id,
        input.name,
        input.description,
        input.price_cents,
        input.active,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}

async fn delete_product(
    _user: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    // Soft delete via active=false to avoid breaking existing orders.
    let res = sqlx::query!("UPDATE products SET active = FALSE WHERE id = $1", id,)
        .execute(&s.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct UploadFileInput {
    pub file_name: String,
    /// Base64-encoded file contents.
    pub content_base64: String,
}

async fn upload_product_file(
    _user: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<UploadFileInput>,
) -> AppResult<Json<ProductAdmin>> {
    let bytes = STANDARD
        .decode(input.content_base64.as_bytes())
        .map_err(|_| AppError::Validation("content_base64 is not valid base64".into()))?;
    if bytes.is_empty() {
        return Err(AppError::Validation("file is empty".into()));
    }
    // Safe filename: <product_id>-<sanitised name>
    let safe_name = input
        .file_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    let on_disk = format!("{id}-{safe_name}");
    let full_path = s.product_files_dir.join(&on_disk);

    tokio::fs::create_dir_all(&s.product_files_dir).await?;
    tokio::fs::write(&full_path, &bytes).await?;

    let size = bytes.len() as i64;
    let row = sqlx::query_as!(
        ProductAdmin,
        r#"
        UPDATE products SET
            file_path = $2,
            file_name = $3,
            file_size = $4
        WHERE id = $1
        RETURNING id, sku, name, description, price_cents, currency,
                  file_path, file_name, file_size, active, created_at, updated_at
        "#,
        id,
        on_disk,
        input.file_name,
        size,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(row))
}

fn validate_product(sku: &str, name: &str, price_cents: i64, currency: &str) -> AppResult<()> {
    let mut errs = vec![];
    if sku.trim().is_empty() {
        errs.push(FieldError {
            field: "sku".into(),
            message: "required".into(),
        });
    }
    if name.trim().is_empty() {
        errs.push(FieldError {
            field: "name".into(),
            message: "required".into(),
        });
    }
    if price_cents <= 0 {
        errs.push(FieldError {
            field: "price_cents".into(),
            message: "must be > 0".into(),
        });
    }
    if currency.len() != 3 || !currency.chars().all(|c| c.is_ascii_alphabetic()) {
        errs.push(FieldError {
            field: "currency".into(),
            message: "must be a 3-letter ISO 4217 code".into(),
        });
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(AppError::Fields(errs))
    }
}

// ---------- orders ----------

#[derive(Debug, Serialize)]
pub struct OrderAdmin {
    pub id: Uuid,
    pub customer_email: String,
    pub stripe_session_id: String,
    pub stripe_payment_intent: Option<String>,
    pub status: String,
    pub amount_cents: i64,
    pub currency: String,
    pub refunded_at: Option<DateTime<Utc>>,
    pub fulfilled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

async fn list_orders(
    _user: AuthUser,
    State(s): State<AppState>,
) -> AppResult<Json<Vec<OrderAdmin>>> {
    let rows = sqlx::query_as!(
        OrderAdmin,
        r#"
        SELECT id, customer_email, stripe_session_id, stripe_payment_intent,
               status, amount_cents, currency, refunded_at, fulfilled_at, created_at
        FROM orders
        ORDER BY created_at DESC
        LIMIT 500
        "#,
    )
    .fetch_all(&s.pool)
    .await?;
    Ok(Json(rows))
}

async fn refund_order(
    _user: AuthUser,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<OrderAdmin>> {
    // Look up payment intent.
    let order = sqlx::query!(
        r#"
        SELECT id, status, stripe_payment_intent
        FROM orders WHERE id = $1
        "#,
        id,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    if order.status == "refunded" {
        return Err(AppError::Conflict("order already refunded".into()));
    }
    let pi = order
        .stripe_payment_intent
        .ok_or_else(|| AppError::Conflict("order has no payment intent to refund yet".into()))?;

    // Idempotency key per order id so a double-click doesn't refund twice.
    let idem = format!("rf_{}", order.id);
    let resp = s.stripe.refund_payment_intent(&pi, &idem).await?;
    tracing::info!(refund_id = %resp.id, status = %resp.status, order_id = %order.id, "stripe refund created");

    // The `charge.refunded` webhook will flip status, but we also flip
    // it here so the admin sees the change immediately.
    let mut tx = s.pool.begin().await?;
    sqlx::query!(
        "UPDATE orders SET status = 'refunded', refunded_at = now() WHERE id = $1",
        order.id,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE download_links SET revoked_at = now() WHERE order_id = $1 AND revoked_at IS NULL",
        order.id,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let updated = sqlx::query_as!(
        OrderAdmin,
        r#"
        SELECT id, customer_email, stripe_session_id, stripe_payment_intent,
               status, amount_cents, currency, refunded_at, fulfilled_at, created_at
        FROM orders WHERE id = $1
        "#,
        order.id,
    )
    .fetch_one(&s.pool)
    .await?;
    Ok(Json(updated))
}
