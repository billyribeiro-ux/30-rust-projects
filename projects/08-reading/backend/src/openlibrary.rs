use moka::future::Cache;
use serde::Serialize;
use std::time::Duration;

use crate::error::{AppError, AppResult};

/// Normalized result we expose to the frontend. The Open Library response
/// shape is rich and inconsistent — we extract only the fields we care
/// about and shed everything else.
#[derive(Debug, Clone, Serialize)]
pub struct BookLookup {
    pub isbn: String,
    pub title: String,
    pub author: String,
    pub cover_url: Option<String>,
    pub pages: Option<i64>,
}

#[derive(Clone)]
pub struct OpenLibrary {
    client: reqwest::Client,
    base_url: String,
    cache: Cache<String, BookLookup>,
}

impl OpenLibrary {
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent("reading-tracker/0.1")
            .build()
            .expect("reqwest client builds");
        // moka cache: at most 1024 lookups, TTL 1 hour. Tradeoff: Open Library
        // data doesn't change for a given ISBN in practice, so we could keep
        // it longer — but bounding by time matches a "freshness-on-restart"
        // convention that tends to surprise users less.
        let cache = Cache::builder()
            .max_capacity(1024)
            .time_to_live(Duration::from_secs(60 * 60))
            .build();
        Self {
            client,
            base_url: base_url.into(),
            cache,
        }
    }

    /// Looks up a single ISBN via Open Library's `?bibkeys=ISBN:...` endpoint.
    /// Returns a normalized BookLookup; serves from the in-memory cache on
    /// repeat queries within the TTL.
    pub async fn lookup_isbn(&self, isbn: &str) -> AppResult<BookLookup> {
        let norm = normalize_isbn(isbn)?;
        if let Some(cached) = self.cache.get(&norm).await {
            return Ok(cached);
        }

        let url = format!(
            "{}/api/books?bibkeys=ISBN:{norm}&format=json&jscmd=data",
            self.base_url
        );
        let res = self.client.get(&url).send().await?;
        if !res.status().is_success() {
            return Err(AppError::Upstream(format!(
                "Open Library returned {}",
                res.status()
            )));
        }
        let body: serde_json::Value = res.json().await?;
        let parsed = parse_lookup(&norm, &body)?;
        self.cache.insert(norm.clone(), parsed.clone()).await;
        Ok(parsed)
    }
}

/// Normalize an ISBN string: strip hyphens/whitespace, uppercase X (the
/// ISBN-10 check digit) and validate length (10 or 13).
pub fn normalize_isbn(raw: &str) -> AppResult<String> {
    let cleaned: String = raw
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect();
    if cleaned.len() != 10 && cleaned.len() != 13 {
        return Err(AppError::Validation(
            "isbn must be 10 or 13 chars (after stripping hyphens/spaces)".into(),
        ));
    }
    if cleaned.len() == 13 && !cleaned.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::Validation("isbn-13 must be all digits".into()));
    }
    if cleaned.len() == 10
        && !cleaned
            .chars()
            .enumerate()
            .all(|(i, c)| c.is_ascii_digit() || (i == 9 && c == 'X'))
    {
        return Err(AppError::Validation(
            "isbn-10 may only contain digits, with X as the last check digit".into(),
        ));
    }
    Ok(cleaned)
}

/// Parse Open Library's JSON response into a BookLookup.
///
/// The shape we receive is `{ "ISBN:9780...": { title, authors, ... } }`.
/// If the key is missing entirely, the API found no match — return NotFound
/// so the route handler can map to a 404.
pub fn parse_lookup(isbn: &str, body: &serde_json::Value) -> AppResult<BookLookup> {
    let key = format!("ISBN:{}", isbn);
    let obj = body.get(&key).ok_or(AppError::NotFound)?;

    let title = obj
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if title.is_empty() {
        return Err(AppError::Upstream(
            "Open Library response missing title".into(),
        ));
    }

    // `authors` is an array of objects with a `name` field.
    let author = obj
        .get("authors")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|a| a.get("name").and_then(|n| n.as_str()))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    let cover_url = obj
        .get("cover")
        .and_then(|c| {
            c.get("medium")
                .or_else(|| c.get("large"))
                .or_else(|| c.get("small"))
        })
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let pages = obj.get("number_of_pages").and_then(|v| v.as_i64());

    Ok(BookLookup {
        isbn: isbn.to_string(),
        title,
        author,
        cover_url,
        pages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalize_isbn_strips_hyphens() {
        assert_eq!(
            normalize_isbn("978-0-13-468599-1").unwrap(),
            "9780134685991"
        );
    }

    #[test]
    fn normalize_isbn_uppercases_x() {
        assert_eq!(normalize_isbn("020161622x").unwrap(), "020161622X");
    }

    #[test]
    fn normalize_isbn_rejects_short() {
        assert!(matches!(
            normalize_isbn("123"),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn normalize_isbn_rejects_letters_in_13() {
        assert!(matches!(
            normalize_isbn("978013468599A"),
            Err(AppError::Validation(_))
        ));
    }

    #[test]
    fn parse_lookup_extracts_minimal() {
        let body = json!({
            "ISBN:9780134685991": {
                "title": "Effective Java",
                "authors": [{ "name": "Joshua Bloch" }],
                "number_of_pages": 412,
                "cover": { "small": "http://small.png", "medium": "http://medium.png" }
            }
        });
        let r = parse_lookup("9780134685991", &body).unwrap();
        assert_eq!(r.title, "Effective Java");
        assert_eq!(r.author, "Joshua Bloch");
        assert_eq!(r.pages, Some(412));
        assert_eq!(r.cover_url.as_deref(), Some("http://medium.png"));
    }

    #[test]
    fn parse_lookup_handles_multiple_authors() {
        let body = json!({
            "ISBN:9780201633610": {
                "title": "Design Patterns",
                "authors": [
                    { "name": "Erich Gamma" },
                    { "name": "Richard Helm" },
                    { "name": "Ralph Johnson" },
                    { "name": "John Vlissides" }
                ]
            }
        });
        let r = parse_lookup("9780201633610", &body).unwrap();
        assert_eq!(
            r.author,
            "Erich Gamma, Richard Helm, Ralph Johnson, John Vlissides"
        );
    }

    #[test]
    fn parse_lookup_falls_back_to_small_cover() {
        let body = json!({
            "ISBN:9780000000001": {
                "title": "X",
                "cover": { "small": "http://small.png" }
            }
        });
        let r = parse_lookup("9780000000001", &body).unwrap();
        assert_eq!(r.cover_url.as_deref(), Some("http://small.png"));
    }

    #[test]
    fn parse_lookup_returns_not_found_when_key_missing() {
        let body = json!({});
        assert!(matches!(
            parse_lookup("9780000000001", &body),
            Err(AppError::NotFound)
        ));
    }

    #[test]
    fn parse_lookup_rejects_missing_title() {
        let body = json!({ "ISBN:9780000000001": { "authors": [{ "name": "x" }] } });
        assert!(matches!(
            parse_lookup("9780000000001", &body),
            Err(AppError::Upstream(_))
        ));
    }
}
