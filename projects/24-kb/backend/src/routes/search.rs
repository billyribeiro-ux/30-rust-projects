//! Hybrid search: Postgres FTS (BM25 + `ts_headline` snippets) merged
//! with Meili (typo-tolerant + optional semantic) via **Reciprocal Rank
//! Fusion**. If Meili isn't configured, we serve FTS-only results and
//! a small pg_trgm typo-fuzz fallback.
//!
//! RRF math:
//!     score(d) = sum_i 1 / (k + rank_i(d))
//! …with `k = 60` (the value Cormack/Clarke recommended; not tunable
//! here on purpose — read the LESSON for why).

use axum::Json;
use axum::Router;
use axum::extract::{Query, State};
use axum::routing::get;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::AppResult;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(search))
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub locale: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct SearchHit {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub snippet: String,
    pub score: f64,
}

const RRF_K: f64 = 60.0;

async fn search(
    State(s): State<AppState>,
    Query(input): Query<SearchQuery>,
) -> AppResult<Json<Vec<SearchHit>>> {
    let q = input.q.trim();
    if q.is_empty() {
        return Ok(Json(vec![]));
    }
    let locale = input.locale.unwrap_or_else(|| "en".into());

    // 1) Postgres FTS — primary signal.
    let fts = fts_search(&s.pool, q, &locale).await?;

    // 2) pg_trgm typo fallback — only consulted if FTS returned nothing.
    let trgm = if fts.is_empty() {
        trgm_search(&s.pool, q, &locale).await?
    } else {
        vec![]
    };

    // 3) Meili — optional, gracefully skipped if unconfigured.
    let meili = match (s.meili_url.as_deref(), s.meili_key.as_deref()) {
        (Some(url), Some(key)) => meili_search(url, key, q).await.unwrap_or_default(),
        _ => vec![],
    };

    // RRF merge (Postgres FTS + Meili). pg_trgm is treated as a
    // "promote when FTS was empty" — added with its own rank scale.
    let mut score: HashMap<Uuid, f64> = HashMap::new();
    for (i, h) in fts.iter().enumerate() {
        *score.entry(h.id).or_default() += 1.0 / (RRF_K + (i as f64 + 1.0));
    }
    for (i, h) in meili.iter().enumerate() {
        *score.entry(h.id).or_default() += 1.0 / (RRF_K + (i as f64 + 1.0));
    }
    for (i, h) in trgm.iter().enumerate() {
        *score.entry(h.id).or_default() += 0.5 / (RRF_K + (i as f64 + 1.0));
    }

    // Resolve back to hits with snippets — prefer the FTS snippet (best),
    // fall back to Meili-provided, then trgm title-only.
    let mut by_id: HashMap<Uuid, SearchHit> = HashMap::new();
    for h in fts
        .into_iter()
        .chain(meili.into_iter())
        .chain(trgm.into_iter())
    {
        by_id.entry(h.id).or_insert(h);
    }
    let mut hits: Vec<SearchHit> = by_id
        .into_iter()
        .map(|(id, mut h)| {
            h.score = *score.get(&id).unwrap_or(&0.0);
            h
        })
        .collect();
    hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    hits.truncate(50);
    Ok(Json(hits))
}

async fn fts_search(pool: &sqlx::PgPool, q: &str, _locale: &str) -> AppResult<Vec<SearchHit>> {
    // `plainto_tsquery` is forgiving of user-entered queries (no quoting).
    let rows = sqlx::query!(
        r#"
        SELECT
            id,
            slug,
            title,
            ts_headline(
                'english',
                COALESCE(summary, body_md),
                plainto_tsquery('english', $1),
                'MaxFragments=2, MaxWords=18, MinWords=6, StartSel=<mark>, StopSel=</mark>'
            ) AS "snippet!",
            ts_rank_cd(tsv, plainto_tsquery('english', $1))::float8 AS "rank_score!: f64"
        FROM articles
        WHERE published_at IS NOT NULL
          AND tsv @@ plainto_tsquery('english', $1)
        ORDER BY ts_rank_cd(tsv, plainto_tsquery('english', $1)) DESC
        LIMIT 50
        "#,
        q
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| SearchHit {
            id: r.id,
            slug: r.slug,
            title: r.title,
            snippet: r.snippet,
            score: r.rank_score,
        })
        .collect())
}

async fn trgm_search(pool: &sqlx::PgPool, q: &str, _locale: &str) -> AppResult<Vec<SearchHit>> {
    // pg_trgm similarity > 0.2 is the conventional "looks like a typo" floor.
    let rows = sqlx::query!(
        r#"
        SELECT id, slug, title, summary,
               similarity(title, $1)::float8 AS "sim_score!: f64"
        FROM articles
        WHERE published_at IS NOT NULL
          AND similarity(title, $1) > 0.2
        ORDER BY similarity(title, $1) DESC
        LIMIT 20
        "#,
        q
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| SearchHit {
            id: r.id,
            slug: r.slug,
            title: r.title,
            snippet: r.summary,
            score: r.sim_score,
        })
        .collect())
}

async fn meili_search(base_url: &str, key: &str, q: &str) -> AppResult<Vec<SearchHit>> {
    // Best-effort. Returns Ok([]) on any error; the caller treats Meili as
    // a recommender, not a source of truth.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| crate::error::AppError::Internal(format!("meili client: {e}")))?;
    let body = serde_json::json!({
        "q": q,
        "limit": 50,
        "attributesToHighlight": ["title", "summary"]
    });
    let res = client
        .post(format!("{base_url}/indexes/articles/search"))
        .bearer_auth(key)
        .json(&body)
        .send()
        .await;
    let res = match res {
        Ok(r) if r.status().is_success() => r,
        _ => return Ok(vec![]),
    };
    let body: serde_json::Value = res.json().await.unwrap_or(serde_json::Value::Null);
    let hits = body
        .get("hits")
        .and_then(|h| h.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(hits
        .into_iter()
        .filter_map(|h| {
            let id = h.get("id").and_then(|v| v.as_str())?.parse().ok()?;
            let slug = h.get("slug").and_then(|v| v.as_str())?.to_string();
            let title = h.get("title").and_then(|v| v.as_str())?.to_string();
            let snippet = h
                .get("_formatted")
                .and_then(|f| f.get("summary"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Some(SearchHit {
                id,
                slug,
                title,
                snippet,
                score: 0.0,
            })
        })
        .collect())
}
