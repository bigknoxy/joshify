//! A tiny stand-in for api.spotify.com, for tests.
//!
//! It answers just enough of `GET /playlists/{id}/items` and
//! `GET /albums/{id}/tracks` to exercise the real pagination code, and it
//! enforces the one rule that bit 0.8.3: a `limit` above the endpoint's
//! documented maximum is rejected with 400, not clamped. Every request is
//! recorded so a test can assert what was actually sent.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use http_body_util::Full;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

/// One request the fake saw: the path and its parsed query parameters.
#[derive(Debug, Clone)]
pub struct Hit {
    pub path: String,
    pub query: HashMap<String, String>,
}

impl Hit {
    fn param_u32(&self, key: &str) -> Option<u32> {
        self.query.get(key).and_then(|v| v.parse().ok())
    }
}

/// How big the fake pretends each collection is, and where (if anywhere) it
/// should start failing.
#[derive(Debug, Clone, Copy)]
pub struct Catalog {
    pub playlist_total: u32,
    pub album_total: u32,
    /// Offsets at or beyond this fail with 500. `None` means never.
    pub fail_from_offset: Option<u32>,
}

pub struct FakeSpotify {
    pub base_url: String,
    hits: Arc<Mutex<Vec<Hit>>>,
}

impl FakeSpotify {
    pub async fn start(catalog: Catalog) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind fake spotify");
        let addr = listener.local_addr().expect("local addr");
        let hits: Arc<Mutex<Vec<Hit>>> = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&hits);

        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    return;
                };
                let recorded = Arc::clone(&recorded);
                let service = service_fn(move |req: Request<hyper::body::Incoming>| {
                    let recorded = Arc::clone(&recorded);
                    let hit = Hit {
                        path: req.uri().path().to_string(),
                        query: parse_query(req.uri().query().unwrap_or("")),
                    };
                    async move {
                        let (status, body) = respond(&hit, catalog);
                        recorded.lock().expect("hits lock").push(hit);
                        Ok::<_, hyper::Error>(
                            Response::builder()
                                .status(status)
                                .header("Content-Type", "application/json")
                                .body(Full::new(Bytes::from(body)))
                                .expect("static response builder should never fail"),
                        )
                    }
                });
                tokio::spawn(async move {
                    // The client closes the connection when it is done; an
                    // error here is that, not a test failure.
                    let _ = http1::Builder::new()
                        .serve_connection(TokioIo::new(stream), service)
                        .await;
                });
            }
        });

        Self {
            base_url: format!("http://{addr}/"),
            hits,
        }
    }

    pub fn hits(&self) -> Vec<Hit> {
        self.hits.lock().expect("hits lock").clone()
    }
}

fn parse_query(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter(|kv| !kv.is_empty())
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn respond(hit: &Hit, catalog: Catalog) -> (StatusCode, String) {
    let segments: Vec<&str> = hit.path.trim_matches('/').split('/').collect();
    match segments.as_slice() {
        ["playlists", _, "items"] | ["playlists", _, "tracks"] => {
            page(hit, catalog, catalog.playlist_total, 50, playlist_item)
        }
        ["albums", _, "tracks"] => page(hit, catalog, catalog.album_total, 50, simplified_track),
        _ => (
            StatusCode::NOT_FOUND,
            r#"{"error":{"status":404,"message":"Not found"}}"#.to_string(),
        ),
    }
}

/// Spotify's paging rules: `limit` defaults to 20 and must be 1..=max, the
/// request fails outright otherwise; `next` is null on the final page.
fn page(
    hit: &Hit,
    catalog: Catalog,
    total: u32,
    max_limit: u32,
    item: fn(u32) -> String,
) -> (StatusCode, String) {
    let limit = hit.param_u32("limit").unwrap_or(20);
    let offset = hit.param_u32("offset").unwrap_or(0);
    if limit < 1 || limit > max_limit {
        return (
            StatusCode::BAD_REQUEST,
            r#"{"error":{"status":400,"message":"Invalid limit"}}"#.to_string(),
        );
    }
    if let Some(fail_from) = catalog.fail_from_offset {
        if offset >= fail_from {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                r#"{"error":{"status":500,"message":"Server error"}}"#.to_string(),
            );
        }
    }
    let end = (offset + limit).min(total);
    let items: Vec<String> = (offset..end).map(item).collect();
    let next = if end < total {
        format!(r#""{}?offset={end}&limit={limit}""#, hit.path)
    } else {
        "null".to_string()
    };
    (
        StatusCode::OK,
        format!(
            r#"{{"href":"{}","items":[{}],"limit":{limit},"next":{next},"offset":{offset},"previous":null,"total":{total}}}"#,
            hit.path,
            items.join(",")
        ),
    )
}

fn full_track(n: u32) -> String {
    format!(
        r#"{{
          "album": {{
            "album_type": "album", "artists": [], "available_markets": [],
            "external_urls": {{}}, "href": "h", "id": "alb1", "images": [],
            "name": "An Album", "release_date": "2020-01-01",
            "release_date_precision": "day", "type": "album",
            "uri": "spotify:album:alb1"
          }},
          "artists": [{{
            "external_urls": {{}}, "href": "h", "id": "art1",
            "name": "An Artist", "type": "artist", "uri": "spotify:artist:art1"
          }}],
          "available_markets": [], "disc_number": 1, "duration_ms": 1000,
          "explicit": false, "external_ids": {{}}, "external_urls": {{}},
          "href": "h", "id": "{id}", "is_local": false, "name": "Track {n}",
          "popularity": 1, "preview_url": null, "track_number": {tn},
          "type": "track", "uri": "spotify:track:{id}"
        }}"#,
        id = track_id(n),
        tn = n + 1,
    )
}

fn playlist_item(n: u32) -> String {
    format!(
        r#"{{"added_at":"2020-01-01T00:00:00Z","added_by":null,"is_local":false,"item":{}}}"#,
        full_track(n)
    )
}

fn simplified_track(n: u32) -> String {
    format!(
        r#"{{
          "artists": [{{
            "external_urls": {{}}, "href": "h", "id": "art1",
            "name": "An Artist", "type": "artist", "uri": "spotify:artist:art1"
          }}],
          "available_markets": [], "disc_number": 1, "duration_ms": 1000,
          "explicit": false, "external_urls": {{}}, "href": "h",
          "id": "{id}", "is_local": false, "name": "Track {n}",
          "preview_url": null, "track_number": {tn}, "type": "track",
          "uri": "spotify:track:{id}"
        }}"#,
        id = track_id(n),
        tn = n + 1,
    )
}

/// A valid-looking 22-character base62 id that encodes `n`.
pub fn track_id(n: u32) -> String {
    format!("{:0>22}", format!("t{n}"))
}
