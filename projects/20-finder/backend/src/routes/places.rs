//! Places search + detail.
//!
//! `GET /api/places?lat=&lng=&radius_m=&q=&cuisine=`
//!   * If lat/lng provided: ST_DWithin filter, ordered by ST_Distance.
//!   * If not: lexicographic ordering by name (admin/dev fallback).
//!   * `q` is a case-insensitive ILIKE on name+address.
//!
//! `GET /api/places/:id` returns the place + recent reviews +
//! distance from query lat/lng (if supplied).

use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, response::IntoResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::session::AuthUser;
use crate::error::{AppError, AppResult, FieldError};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(detail))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    /// Defaults to 2000m. Clamped to [50, 50_000].
    pub radius_m: Option<f64>,
    pub q: Option<String>,
    pub cuisine: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PlaceRow {
    pub id: Uuid,
    pub name: String,
    pub cuisine: String,
    pub address: String,
    pub lat: f64,
    pub lng: f64,
    /// Metres from the query lat/lng if supplied; null otherwise.
    pub distance_m: Option<f64>,
    pub avg_rating: Option<f64>,
    pub review_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

async fn list(
    State(s): State<AppState>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<Vec<PlaceRow>>> {
    let radius = q.radius_m.unwrap_or(2000.0).clamp(50.0, 50_000.0);
    let q_text =
        q.q.as_deref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
    let cuisine = q
        .cuisine
        .as_deref()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty());

    // Two queries — one geo-filtered, one general — keep them separate
    // because the WHERE on `geom` requires ST_MakePoint params we don't
    // want to bind for the no-coord path.
    let rows = match (q.lat, q.lng) {
        (Some(lat), Some(lng)) => {
            sqlx::query!(
                r#"
                SELECT
                    p.id          AS "id!: Uuid",
                    p.name        AS "name!",
                    p.cuisine     AS "cuisine!",
                    p.address     AS "address!",
                    ST_Y(p.geom::geometry) AS "lat!: f64",
                    ST_X(p.geom::geometry) AS "lng!: f64",
                    ST_Distance(p.geom, ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography) AS "distance_m!: f64",
                    (SELECT AVG(rating)::float8 FROM reviews r WHERE r.place_id = p.id) AS "avg_rating: f64",
                    COALESCE((SELECT COUNT(*) FROM reviews r WHERE r.place_id = p.id), 0) AS "review_count!: i64",
                    p.created_at  AS "created_at!: DateTime<Utc>",
                    p.updated_at  AS "updated_at!: DateTime<Utc>"
                FROM places p
                WHERE ST_DWithin(
                        p.geom,
                        ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography,
                        $3
                      )
                  AND ($4::text IS NULL OR p.cuisine = $4)
                  AND ($5::text IS NULL OR (p.name ILIKE '%' || $5 || '%' OR p.address ILIKE '%' || $5 || '%'))
                ORDER BY p.geom <-> ST_SetSRID(ST_MakePoint($2, $1), 4326)::geography
                LIMIT 100
                "#,
                lat,
                lng,
                radius,
                cuisine.as_deref(),
                q_text.as_deref(),
            )
            .fetch_all(&s.pool)
            .await?
            .into_iter()
            .map(|r| PlaceRow {
                id: r.id,
                name: r.name,
                cuisine: r.cuisine,
                address: r.address,
                lat: r.lat,
                lng: r.lng,
                distance_m: Some(r.distance_m),
                avg_rating: r.avg_rating,
                review_count: r.review_count,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect()
        }
        _ => sqlx::query!(
            r#"
            SELECT
                p.id          AS "id!: Uuid",
                p.name        AS "name!",
                p.cuisine     AS "cuisine!",
                p.address     AS "address!",
                ST_Y(p.geom::geometry) AS "lat!: f64",
                ST_X(p.geom::geometry) AS "lng!: f64",
                (SELECT AVG(rating)::float8 FROM reviews r WHERE r.place_id = p.id) AS "avg_rating: f64",
                COALESCE((SELECT COUNT(*) FROM reviews r WHERE r.place_id = p.id), 0) AS "review_count!: i64",
                p.created_at  AS "created_at!: DateTime<Utc>",
                p.updated_at  AS "updated_at!: DateTime<Utc>"
            FROM places p
            WHERE ($1::text IS NULL OR p.cuisine = $1)
              AND ($2::text IS NULL OR (p.name ILIKE '%' || $2 || '%' OR p.address ILIKE '%' || $2 || '%'))
            ORDER BY p.name
            LIMIT 100
            "#,
            cuisine.as_deref(),
            q_text.as_deref(),
        )
        .fetch_all(&s.pool)
        .await?
        .into_iter()
        .map(|r| PlaceRow {
            id: r.id,
            name: r.name,
            cuisine: r.cuisine,
            address: r.address,
            lat: r.lat,
            lng: r.lng,
            distance_m: None,
            avg_rating: r.avg_rating,
            review_count: r.review_count,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
        .collect(),
    };

    Ok(Json(rows))
}

#[derive(Debug, Serialize)]
pub struct ReviewRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub rating: i16,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct PlaceDetail {
    #[serde(flatten)]
    pub place: PlaceRow,
    pub reviews: Vec<ReviewRow>,
}

#[derive(Debug, Deserialize)]
pub struct DetailQuery {
    pub lat: Option<f64>,
    pub lng: Option<f64>,
}

async fn detail(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<DetailQuery>,
) -> AppResult<Json<PlaceDetail>> {
    let place_row = sqlx::query!(
        r#"
        SELECT
            p.id          AS "id!: Uuid",
            p.name        AS "name!",
            p.cuisine     AS "cuisine!",
            p.address     AS "address!",
            ST_Y(p.geom::geometry) AS "lat!: f64",
            ST_X(p.geom::geometry) AS "lng!: f64",
            CASE
                WHEN $2::float8 IS NOT NULL AND $3::float8 IS NOT NULL
                THEN ST_Distance(p.geom, ST_SetSRID(ST_MakePoint($3, $2), 4326)::geography)
                ELSE NULL
            END AS "distance_m: f64",
            (SELECT AVG(rating)::float8 FROM reviews r WHERE r.place_id = p.id) AS "avg_rating: f64",
            COALESCE((SELECT COUNT(*) FROM reviews r WHERE r.place_id = p.id), 0) AS "review_count!: i64",
            p.created_at  AS "created_at!: DateTime<Utc>",
            p.updated_at  AS "updated_at!: DateTime<Utc>"
        FROM places p
        WHERE p.id = $1
        "#,
        id,
        q.lat,
        q.lng,
    )
    .fetch_optional(&s.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let reviews = sqlx::query!(
        r#"
        SELECT r.id AS "id!: Uuid",
               r.user_id AS "user_id!: Uuid",
               u.name    AS "user_name!",
               r.rating  AS "rating!: i16",
               r.body    AS "body!",
               r.created_at AS "created_at!: DateTime<Utc>"
        FROM reviews r
        JOIN users u ON u.id = r.user_id
        WHERE r.place_id = $1
        ORDER BY r.created_at DESC
        LIMIT 50
        "#,
        id,
    )
    .fetch_all(&s.pool)
    .await?
    .into_iter()
    .map(|r| ReviewRow {
        id: r.id,
        user_id: r.user_id,
        user_name: r.user_name,
        rating: r.rating,
        body: r.body,
        created_at: r.created_at,
    })
    .collect();

    let place = PlaceRow {
        id: place_row.id,
        name: place_row.name,
        cuisine: place_row.cuisine,
        address: place_row.address,
        lat: place_row.lat,
        lng: place_row.lng,
        distance_m: place_row.distance_m,
        avg_rating: place_row.avg_rating,
        review_count: place_row.review_count,
        created_at: place_row.created_at,
        updated_at: place_row.updated_at,
    };

    Ok(Json(PlaceDetail { place, reviews }))
}

#[derive(Debug, Deserialize)]
pub struct CreateInput {
    pub name: String,
    pub cuisine: Option<String>,
    pub address: Option<String>,
    pub lat: f64,
    pub lng: f64,
}

async fn create(
    State(s): State<AppState>,
    _user: AuthUser,
    Json(input): Json<CreateInput>,
) -> AppResult<impl IntoResponse> {
    let name = input.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 200 {
        return Err(AppError::Fields(vec![FieldError {
            field: "name".into(),
            message: "1-200 characters required".into(),
        }]));
    }
    if !(-90.0..=90.0).contains(&input.lat) || !(-180.0..=180.0).contains(&input.lng) {
        return Err(AppError::Fields(vec![FieldError {
            field: "coords".into(),
            message: "lat must be in [-90,90] and lng in [-180,180]".into(),
        }]));
    }

    let cuisine = input.cuisine.unwrap_or_default().trim().to_lowercase();
    let address = input.address.unwrap_or_default().trim().to_string();
    let id = Uuid::new_v4();

    sqlx::query!(
        r#"
        INSERT INTO places (id, name, cuisine, address, geom)
        VALUES ($1, $2, $3, $4, ST_SetSRID(ST_MakePoint($6, $5), 4326)::geography)
        "#,
        id,
        name,
        cuisine,
        address,
        input.lat,
        input.lng,
    )
    .execute(&s.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(serde_json::json!({ "id": id }))))
}
