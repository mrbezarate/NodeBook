//! Web Server and Mini App REST API for NodeBook OS.

use axum::{
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Json, Response},
    routing::{delete, get, post},
    Router,
};
use brain_analytics::LifeAnalyticsEngine;
use brain_core::engine::BrainEngine;
use brain_media_downloader::{downloader::MediaItem, MediaDownloader};
use brain_vault::VaultRegistry;
use ring::hmac;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path as StdPath, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTaskState {
    pub task_id: String,
    pub url: String,
    pub title: String,
    pub is_playlist: bool,
    pub total_tracks: usize,
    pub completed_tracks: usize,
    pub current_track: Option<String>,
    pub current_artist: Option<String>,
    pub percent: u8,
    pub status: String, // "queued" | "downloading" | "done" | "error"
    pub error: Option<String>,
    pub items: Vec<MediaItem>,
}

#[derive(Clone)]
pub struct AppState {
    pub engine: Arc<BrainEngine>,
    pub downloader: Arc<MediaDownloader>,
    pub analytics_engine: Arc<LifeAnalyticsEngine>,
    pub vault_registry: Arc<RwLock<VaultRegistry>>,
    pub download_tasks: Arc<RwLock<HashMap<String, DownloadTaskState>>>,
}

pub async fn start_web_server(state: AppState, port: u16) {
    let app = Router::new()
        // ── Static Web App ──────────────────────────────────────────────────
        .route("/", get(serve_index))
        .route("/app", get(serve_index))
        .route("/app.css", get(serve_css))
        .route("/app.js", get(serve_js))
        .route("/health", get(health_check))
        // ── Auth Verification Endpoint ──────────────────────────────────────
        .route("/api/auth/verify", get(verify_auth).post(verify_auth))
        // ── Music Player Endpoints ──────────────────────────────────────────
        .route("/api/player/tracks", get(get_audio_tracks))
        .route("/api/player/stream/{id}", get(stream_audio))
        .route("/api/player/cover/{id}", get(stream_cover))
        // ── Media & Video Hub Endpoints ─────────────────────────────────────
        .route("/api/media/videos", get(get_videos))
        .route("/api/media/pins", get(get_pins))
        .route("/api/media/stream/{id}", get(stream_video))
        .route("/api/media/thumb/{id}", get(stream_thumb))
        .route("/api/media/upload", post(upload_media))
        .route("/api/media/download", post(download_url))
        .route("/api/media/download/task/{id}", get(get_download_task_status))
        .route("/api/media/{id}", delete(delete_media))
        // ── Knowledge Vault Viewer Endpoints ────────────────────────────────
        .route("/api/vault/notes", get(get_vault_notes))
        .route("/api/vault/note/{id}", delete(delete_vault_note))
        .route("/api/vault/notes/{id}", delete(delete_vault_note))
        .route("/api/vault/note/{id}/properties", get(get_note_properties))
        // ── Playlists Endpoints ─────────────────────────────────────────────
        .route("/api/playlists", get(get_playlists).post(create_playlist))
        .route("/api/playlists/{id}", delete(delete_playlist))
        .route("/api/playlists/{id}/items", post(add_item_to_playlist))
        .route("/api/playlists/{id}/items/{item_id}", delete(remove_item_from_playlist))
        // ── Stats & AI Endpoints ────────────────────────────────────
        .route("/api/stats", get(get_stats))
        .route("/api/ai/insight", post(get_ai_insight))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => {
            info!("🚀 NodeBook Web OS & Mini App server running at http://{}", addr);
            if let Err(e) = axum::serve(listener, app).await {
                error!("Web server error: {}", e);
            }
        }
        Err(e) => {
            warn!("Could not bind web server on {}: {}", addr, e);
        }
    }
}

// ── Telegram Mini App Cryptographic Authentication ──────────────────────────

fn constant_time_eq_hex(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (&x, &y) in a.as_bytes().iter().zip(b.as_bytes().iter()) {
        diff |= x.to_ascii_lowercase() ^ y.to_ascii_lowercase();
    }
    diff == 0
}

pub fn authenticate_request(
    headers: &HeaderMap,
    query_params: Option<&HashMap<String, String>>,
    bot_token: &str,
    allowed_users: &[u64],
) -> Result<u64, (StatusCode, Json<serde_json::Value>)> {
    if allowed_users.is_empty() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "Access Forbidden: No allowed_users configured in system."
            })),
        ));
    }

    // Check header first (X-Telegram-Init-Data or Authorization: tma <data>), then query string ?initData=...
    let mut init_data_str = headers
        .get("x-telegram-init-data")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| {
            headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.strip_prefix("tma "))
                .map(|s| s.to_string())
        });

    if init_data_str.is_none() || init_data_str.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
        if let Some(qp) = query_params {
            if let Some(param) = qp.get("initData").or_else(|| qp.get("tgWebAppData")) {
                init_data_str = Some(param.clone());
            }
        }
    }

    let init_data = match init_data_str {
        Some(s) if !s.trim().is_empty() => s,
        _ => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": "Unauthorized: Telegram initData is required to access NodeBook OS."
                })),
            ));
        }
    };

    // Parse initData key=value
    let mut params = Vec::new();
    let mut raw_params = Vec::new();
    let mut received_hash = String::new();

    for pair in init_data.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            let key = urlencoding::decode(k).unwrap_or(std::borrow::Cow::Borrowed(k)).into_owned();
            let val = urlencoding::decode(v).unwrap_or(std::borrow::Cow::Borrowed(v)).into_owned();
            if key == "hash" {
                received_hash = val;
            } else {
                params.push((key, val));
                raw_params.push((k, v));
            }
        }
    }

    // Cryptographic token or direct ID check for personal desktop browser bookmark (e.g. initData=desktop_<uid> or initData=desktop_<uid>_<signature>)
    if let Some(rest) = init_data.strip_prefix("desktop_") {
        let (uid_str, maybe_sig) = match rest.split_once('_') {
            Some((u, s)) => (u, Some(s)),
            None => (rest, None),
        };
        if let Ok(uid) = uid_str.parse::<u64>() {
            if allowed_users.contains(&uid) {
                if let Some(sig) = maybe_sig {
                    let secret_key = hmac::Key::new(hmac::HMAC_SHA256, bot_token.as_bytes());
                    let calculated_tag = hmac::sign(&secret_key, format!("desktop_{}", uid).as_bytes());
                    let calculated_hex: String = calculated_tag
                        .as_ref()
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect();
                    if constant_time_eq_hex(&calculated_hex, sig) {
                        return Ok(uid);
                    }
                } else {
                    return Ok(uid);
                }
            }
        }
    }

    if received_hash.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Unauthorized: Missing cryptographic hash in initData." })),
        ));
    }

    // secret_key = HMAC_SHA256("WebAppData", bot_token)
    let web_app_data_key = hmac::Key::new(hmac::HMAC_SHA256, b"WebAppData");
    let secret_key_tag = hmac::sign(&web_app_data_key, bot_token.as_bytes());
    let secret_key = hmac::Key::new(hmac::HMAC_SHA256, secret_key_tag.as_ref());

    let mut check_passed = false;

    // 1. Try with decoded values
    params.sort_by(|a, b| a.0.cmp(&b.0));
    let data_check_string = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("\n");

    let calculated_tag = hmac::sign(&secret_key, data_check_string.as_bytes());
    let calculated_hex: String = calculated_tag
        .as_ref()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();

    if constant_time_eq_hex(&calculated_hex, &received_hash) {
        check_passed = true;
    }

    // 2. Try with raw values (standard Telegram Desktop format)
    if !check_passed {
        raw_params.sort_by(|a, b| a.0.cmp(b.0));
        let raw_check_string = raw_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("\n");

        let raw_calculated_tag = hmac::sign(&secret_key, raw_check_string.as_bytes());
        let raw_calculated_hex: String = raw_calculated_tag
            .as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();

        if constant_time_eq_hex(&raw_calculated_hex, &received_hash) {
            check_passed = true;
        }
    }

    if !check_passed {
        warn!("🚫 Forged or mismatch Telegram initData signature detected!");
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "Unauthorized: Invalid cryptographic signature." })),
        ));
    }

    // Extract user ID
    let mut user_id: Option<u64> = None;
    for (k, v) in &params {
        if k == "user" {
            if let Ok(user_json) = serde_json::from_str::<serde_json::Value>(v) {
                user_id = user_json.get("id").and_then(|id| id.as_u64());
            }
        }
    }

    let uid = match user_id {
        Some(id) => id,
        None => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "Unauthorized: Missing user identity in initData." })),
            ));
        }
    };

    if !allowed_users.contains(&uid) {
        warn!("🚫 Unauthorized Telegram user_id {} attempted access to NodeBook Web App!", uid);
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": format!("Forbidden: User ID {} is not authorized to access this private NodeBook OS instance.", uid)
            })),
        ));
    }

    Ok(uid)
}

fn auth_check(
    state: &AppState,
    headers: &HeaderMap,
    query: Option<&HashMap<String, String>>,
) -> Result<u64, Response> {
    authenticate_request(
        headers,
        query,
        &state.engine.config.telegram.bot_token,
        &state.engine.config.telegram.allowed_users,
    )
    .map_err(|(code, json)| (code, json).into_response())
}

async fn verify_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    match authenticate_request(
        &headers,
        Some(&query),
        &state.engine.config.telegram.bot_token,
        &state.engine.config.telegram.allowed_users,
    ) {
        Ok(uid) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "authenticated": true,
                "user_id": uid,
                "status": "authorized"
            })),
        )
            .into_response(),
        Err((code, err)) => (code, err).into_response(),
    }
}

// ── Static Asset Handlers ───────────────────────────────────────────────────

async fn serve_index() -> impl IntoResponse {
    let content = tokio::fs::read_to_string("apps/telegram-bot/static/index.html")
        .await
        .unwrap_or_else(|_| include_str!("../static/index.html").to_string());
    Html(content)
}

async fn serve_css() -> impl IntoResponse {
    let content = tokio::fs::read_to_string("apps/telegram-bot/static/app.css")
        .await
        .unwrap_or_else(|_| include_str!("../static/app.css").to_string());
    ([(header::CONTENT_TYPE, "text/css; charset=utf-8")], content)
}

async fn serve_js() -> impl IntoResponse {
    let content = tokio::fs::read_to_string("apps/telegram-bot/static/app.js")
        .await
        .unwrap_or_else(|_| include_str!("../static/app.js").to_string());
    ([(header::CONTENT_TYPE, "application/javascript; charset=utf-8")], content)
}

async fn health_check() -> (StatusCode, &'static str) {
    (StatusCode::OK, "OK")
}

// ── Music Player Handlers ───────────────────────────────────────────────────

fn guess_media_mime(path: &StdPath, default_mime: &'static str) -> &'static str {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        match ext.to_ascii_lowercase().as_str() {
            "mp3" => "audio/mpeg",
            "m4a" | "aac" => "audio/mp4",
            "ogg" | "opus" => "audio/ogg",
            "wav" => "audio/wav",
            "flac" => "audio/flac",
            "mp4" | "m4v" => "video/mp4",
            "webm" => "video/webm",
            "mov" => "video/quicktime",
            "mkv" => "video/x-matroska",
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            _ => default_mime,
        }
    } else {
        default_mime
    }
}

async fn get_audio_tracks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, Response> {
    auth_check(&state, &headers, Some(&query))?;
    let lib = state.downloader.get_library().await;
    let audio_tracks: Vec<MediaItem> = lib.into_iter().filter(|i| i.media_type == "audio").collect();
    Ok(Json(audio_tracks))
}

async fn stream_audio(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    if let Err(res) = auth_check(&state, &headers, Some(&query)) {
        return res;
    }
    let lib = state.downloader.get_library().await;
    if let Some(item) = lib.into_iter().find(|i| i.id == id) {
        let file_path = state.downloader.download_dir().join(&item.file_name);
        let mime = guess_media_mime(&file_path, "audio/mpeg");
        return stream_file_with_range(file_path, mime, headers).await;
    }
    (StatusCode::NOT_FOUND, "Track not found").into_response()
}

async fn stream_cover(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    if id == "default" {
        return serve_default_cover();
    }
    if let Err(res) = auth_check(&state, &headers, Some(&query)) {
        return res;
    }

    let lib = state.downloader.get_library().await;
    if let Some(item) = lib.into_iter().find(|i| i.id == id) {
        if let Some(ref cover) = item.cover_file {
            let path = state.downloader.download_dir().join(cover);
            if path.exists() {
                if let Ok(bytes) = tokio::fs::read(&path).await {
                    let mime = guess_media_mime(&path, "image/jpeg");
                    return (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, mime)],
                        bytes,
                    )
                        .into_response();
                }
            }
        }
    }
    serve_default_cover()
}

fn serve_default_cover() -> Response {
    let svg = concat!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"300\" height=\"300\" viewBox=\"0 0 300 300\">",
        "<rect width=\"300\" height=\"300\" fill=\"#181818\"/>",
        "<circle cx=\"150\" cy=\"150\" r=\"80\" fill=\"#242424\"/>",
        "<circle cx=\"150\" cy=\"150\" r=\"30\" fill=\"#1db954\"/>",
        "<circle cx=\"150\" cy=\"150\" r=\"8\" fill=\"#121212\"/>",
        "<path d=\"M140 120 L170 150 L140 180 Z\" fill=\"#ffffff\" opacity=\"0.8\"/>",
        "</svg>"
    );
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/svg+xml")],
        svg,
    )
        .into_response()
}

// ── Media & Video Hub Handlers ──────────────────────────────────────────────

async fn get_videos(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, Response> {
    auth_check(&state, &headers, Some(&query))?;
    let lib = state.downloader.get_library().await;
    let videos: Vec<MediaItem> = lib.into_iter().filter(|i| i.media_type == "video").collect();
    Ok(Json(videos))
}

async fn get_pins(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, Response> {
    auth_check(&state, &headers, Some(&query))?;
    let lib = state.downloader.get_library().await;
    let pins: Vec<MediaItem> = lib.into_iter().filter(|i| i.media_type == "photo").collect();
    Ok(Json(pins))
}

async fn stream_video(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    if let Err(res) = auth_check(&state, &headers, Some(&query)) {
        return res;
    }
    let lib = state.downloader.get_library().await;
    if let Some(item) = lib.into_iter().find(|i| i.id == id) {
        let file_path = state.downloader.download_dir().join(&item.file_name);
        let mime = guess_media_mime(&file_path, "video/mp4");
        return stream_file_with_range(file_path, mime, headers).await;
    }
    (StatusCode::NOT_FOUND, "Video not found").into_response()
}

async fn stream_thumb(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    if let Err(res) = auth_check(&state, &headers, Some(&query)) {
        return res;
    }
    let lib = state.downloader.get_library().await;
    if let Some(item) = lib.into_iter().find(|i| i.id == id) {
        if let Some(ref cover) = item.cover_file {
            let path = state.downloader.download_dir().join(cover);
            if path.exists() {
                if let Ok(bytes) = tokio::fs::read(&path).await {
                    let mime = guess_media_mime(&path, "image/jpeg");
                    return (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, mime)],
                        bytes,
                    )
                        .into_response();
                }
            }
        }
    }
    serve_default_cover()
}

#[derive(Deserialize)]
struct DownloadRequest {
    url: String,
    is_audio: bool,
}

async fn download_url(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<DownloadRequest>,
) -> Result<impl IntoResponse, Response> {
    auth_check(&state, &headers, None)?;

    let task_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let url = payload.url.trim().to_string();
    let is_audio = payload.is_audio;

    let title_preview = url.replace("https://", "").replace("http://", "").chars().take(35).collect();
    let is_playlist_guess = url.contains("/playlist/") || url.contains("/album/") || url.contains("list=");

    let init_state = DownloadTaskState {
        task_id: task_id.clone(),
        url: url.clone(),
        title: title_preview,
        is_playlist: is_playlist_guess,
        total_tracks: 1,
        completed_tracks: 0,
        current_track: None,
        current_artist: None,
        percent: 5,
        status: "queued".to_string(),
        error: None,
        items: Vec::new(),
    };

    {
        let mut tasks = state.download_tasks.write().await;
        // Evict older completed/errored tasks if capacity exceeds 100
        if tasks.len() > 100 {
            let keys_to_remove: Vec<String> = tasks
                .iter()
                .filter(|(_, t)| t.status == "done" || t.status == "error")
                .map(|(k, _)| k.clone())
                .take(tasks.len().saturating_sub(80))
                .collect();
            for k in keys_to_remove {
                tasks.remove(&k);
            }
        }
        tasks.insert(task_id.clone(), init_state);
    }

    let state_clone = state.clone();
    let task_id_clone = task_id.clone();
    let url_clone = url.clone();

    tokio::spawn(async move {
        let is_playlist = url_clone.contains("/playlist/") || url_clone.contains("/album/") || url_clone.contains("list=");

        if is_playlist && (url_clone.contains("spotify.com") || url_clone.contains("open.spotify.com")) {
            if let Some(entity) = MediaDownloader::extract_spotify_entity(&url_clone).await {
                let total = entity.tracks.len();
                {
                    let mut tasks = state_clone.download_tasks.write().await;
                    if let Some(t) = tasks.get_mut(&task_id_clone) {
                        t.title = entity.title.clone();
                        t.total_tracks = total;
                        t.is_playlist = true;
                        t.status = "downloading".to_string();
                        t.percent = 10;
                    }
                }

                let mut downloaded_items = Vec::new();
                for (idx, track) in entity.tracks.iter().enumerate() {
                    {
                        let mut tasks = state_clone.download_tasks.write().await;
                        if let Some(t) = tasks.get_mut(&task_id_clone) {
                            t.current_track = Some(track.title.clone());
                            t.current_artist = if !track.artist.is_empty() { Some(track.artist.clone()) } else { None };
                            t.percent = ((idx * 85) / total.max(1) + 10) as u8;
                        }
                    }

                    let track_cover = track.cover_url.as_deref();
                    match state_clone.downloader.download_spotify_track_direct(
                        &track.title,
                        &track.artist,
                        track_cover,
                        &track.url,
                    ).await {
                        Ok((_, item)) => {
                            downloaded_items.push(item);
                            let mut tasks = state_clone.download_tasks.write().await;
                            if let Some(t) = tasks.get_mut(&task_id_clone) {
                                t.completed_tracks += 1;
                            }
                        }
                        Err(e) => {
                            warn!("Download error for track {} - {}: {}", track.artist, track.title, e);
                        }
                    }
                }

                let mut tasks = state_clone.download_tasks.write().await;
                if let Some(t) = tasks.get_mut(&task_id_clone) {
                    if downloaded_items.is_empty() {
                        t.status = "error".to_string();
                        t.error = Some("Не удалось загрузить треки из плейлиста".to_string());
                        t.percent = 100;
                    } else {
                        t.status = "done".to_string();
                        t.percent = 100;
                        t.items = downloaded_items;
                    }
                }
                return;
            }
        }

        if is_playlist {
            {
                let mut tasks = state_clone.download_tasks.write().await;
                if let Some(t) = tasks.get_mut(&task_id_clone) {
                    t.status = "downloading".to_string();
                    t.percent = 15;
                }
            }

            match state_clone.downloader.download_playlist(&url_clone, is_audio).await {
                Ok(items) => {
                    let mut tasks = state_clone.download_tasks.write().await;
                    if let Some(t) = tasks.get_mut(&task_id_clone) {
                        t.status = "done".to_string();
                        t.percent = 100;
                        t.completed_tracks = items.len();
                        t.total_tracks = items.len();
                        t.items = items.into_iter().map(|(_, item)| item).collect();
                    }
                }
                Err(e) => {
                    let mut tasks = state_clone.download_tasks.write().await;
                    if let Some(t) = tasks.get_mut(&task_id_clone) {
                        t.status = "error".to_string();
                        t.error = Some(e.to_string());
                    }
                }
            }
        } else {
            {
                let mut tasks = state_clone.download_tasks.write().await;
                if let Some(t) = tasks.get_mut(&task_id_clone) {
                    t.status = "downloading".to_string();
                    t.percent = 25;
                }
            }

            match state_clone.downloader.download_auto(&url_clone, is_audio).await {
                Ok((_, item)) => {
                    let mut tasks = state_clone.download_tasks.write().await;
                    if let Some(t) = tasks.get_mut(&task_id_clone) {
                        t.title = item.title.clone();
                        t.status = "done".to_string();
                        t.percent = 100;
                        t.items = vec![item];
                    }
                }
                Err(e) => {
                    let mut tasks = state_clone.download_tasks.write().await;
                    if let Some(t) = tasks.get_mut(&task_id_clone) {
                        t.status = "error".to_string();
                        t.error = Some(e.to_string());
                    }
                }
            }
        }
    });

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "task_id": task_id,
            "status": "queued"
        })),
    ))
}

async fn get_download_task_status(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, Response> {
    auth_check(&state, &headers, None)?;

    let tasks = state.download_tasks.read().await;
    if let Some(task) = tasks.get(&id) {
        Ok((StatusCode::OK, Json(serde_json::to_value(task).unwrap_or_default())))
    } else {
        Ok((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Task not found" }))))
    }
}

async fn delete_media(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, Response> {
    auth_check(&state, &headers, None)?;
    let deleted = state.downloader.delete_media_item(&id).await;
    if deleted {
        // Also clean up playlists to prevent dangling IDs
        let mut playlists = load_playlists(state.downloader.download_dir()).await;
        let mut changed = false;
        for pl in &mut playlists {
            let before = pl.item_ids.len();
            pl.item_ids.retain(|item_id| item_id != &id);
            if pl.item_ids.len() != before {
                changed = true;
            }
        }
        if changed {
            save_playlists(state.downloader.download_dir(), &playlists).await;
        }
        Ok((StatusCode::OK, Json(serde_json::json!({ "status": "deleted" }))))
    } else {
        Ok((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Item not found" }))))
    }
}

async fn upload_media(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, Response> {
    auth_check(&state, &headers, None)?;
    let file_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    const MAX_UPLOAD_BYTES: usize = 200 * 1024 * 1024; // 200 MB limit

    while let Ok(Some(field)) = multipart.next_field().await {
        let raw_name = field.file_name().unwrap_or("upload.mp4");
        let file_name = StdPath::new(raw_name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("upload.mp4")
            .to_string();
        let ext = StdPath::new(&file_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("mp4")
            .to_lowercase();
        let is_audio = ext == "mp3" || ext == "wav" || ext == "ogg" || ext == "m4a" || ext == "flac";
        let target_name = format!("{}.{}", file_id, ext);
        let target_path = state.downloader.download_dir().join(&target_name);

        if let Ok(data) = field.bytes().await {
            if data.len() > MAX_UPLOAD_BYTES {
                return Err((
                    StatusCode::PAYLOAD_TOO_LARGE,
                    Json(serde_json::json!({ "error": "File exceeds maximum size limit (200MB)" })),
                ).into_response());
            }
            if tokio::fs::write(&target_path, data).await.is_ok() {
                let item = MediaItem {
                    id: file_id.clone(),
                    title: file_name.replace(&format!(".{}", ext), ""),
                    uploader: Some("Локальная загрузка".to_string()),
                    media_type: if is_audio { "audio".to_string() } else { "video".to_string() },
                    file_name: target_name,
                    cover_file: None,
                    duration_secs: None,
                    source_url: "local_upload".to_string(),
                    created_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
                };
                state.downloader.save_media_item(item.clone()).await;
                return Ok((StatusCode::OK, Json(serde_json::to_value(item).unwrap_or_default())));
            }
        }
    }

    Ok((
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({ "error": "No file uploaded" })),
    ))
}

// ── Knowledge Vault Viewer Handlers ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultNoteView {
    pub id: String,
    pub title: String,
    pub para_category: String,
    pub area: Option<String>,
    pub clean_text: String,
    pub ai_summary: Option<String>,
    pub tags: Vec<String>,
    pub entities: Vec<String>,
    pub created_at: String,
    pub file_path: String,
}

async fn fetch_vault_notes(vault_path: &str) -> Vec<VaultNoteView> {
    let mut notes = Vec::new();

    if let Ok(_entries) = tokio::fs::read_dir(vault_path).await {
        let mut subdirs = vec![PathBuf::from(vault_path)];
        
        while let Some(dir) = subdirs.pop() {
            if let Ok(mut read_dir) = tokio::fs::read_dir(&dir).await {
                while let Ok(Some(entry)) = read_dir.next_entry().await {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        if !name.starts_with('.') && name != "target" && name != "node_modules" {
                            subdirs.push(path);
                        }
                    } else if path.extension().is_some_and(|e| e == "md") {
                        if let Ok(content) = tokio::fs::read_to_string(&path).await {
                            let note = parse_vault_note_clean(&path, &content, vault_path);
                            notes.push(note);
                        }
                    }
                }
            }
        }
    }

    notes.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    notes
}

async fn get_vault_notes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, Response> {
    auth_check(&state, &headers, Some(&query))?;
    let vault_path = state.vault_registry.read().await.get_active_path();
    let notes = fetch_vault_notes(&vault_path).await;
    Ok(Json(notes))
}

fn parse_vault_note_clean(path: &StdPath, content: &str, vault_root: &str) -> VaultNoteView {
    let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Note");
    let rel_path = path.strip_prefix(vault_root).unwrap_or(path).to_string_lossy().to_string();

    let mut para_category = "Daily".to_string();
    let mut area: Option<String> = None;

    let parts: Vec<&str> = rel_path.split('/').collect();
    if parts.len() > 1 {
        let first = parts[0];
        if first.starts_with("Projects") || first.starts_with("01_Projects") || first.starts_with("01 Projects") {
            para_category = "Projects".to_string();
        } else if first.starts_with("Areas") || first.starts_with("02_Areas") || first.starts_with("02 Areas") {
            para_category = "Areas".to_string();
        } else if first.starts_with("Resources") || first.starts_with("03_Resources") || first.starts_with("03 Resources") {
            para_category = "Resources".to_string();
        } else if first.starts_with("Archive") || first.starts_with("04_Archive") || first.starts_with("04 Archive") {
            para_category = "Archive".to_string();
        } else if first.starts_with("Daily") || first.starts_with("001 Daily") || first.starts_with("00_Daily") || first.starts_with("Journal") {
            para_category = "Daily".to_string();
        } else {
            para_category = first.to_string();
        }

        if parts.len() > 2 {
            area = Some(parts[1].to_string());
        }
    }

    // Extract frontmatter
    let mut clean_text = content.to_string();
    let mut tags = Vec::new();
    let entities = Vec::new();
    let mut ai_summary = None;
    let mut created_at = chrono::Local::now().format("%Y-%m-%d").to_string();

    if let Some(fm) = brain_vault::frontmatter::Frontmatter::parse_from_markdown(content) {
        if !fm.para.is_empty() {
            para_category = fm.para;
        }
        if !fm.area.is_empty() {
            area = Some(fm.area);
        }
        if !fm.tags.is_empty() {
            tags = fm.tags;
        }
        if !fm.created.is_empty() {
            created_at = fm.created;
        }
        if let Some(sum) = fm.summary {
            if !sum.trim().is_empty() {
                ai_summary = Some(sum);
            }
        }
        if let Some(stripped) = content.strip_prefix("---") {
            if let Some((_fm, body)) = stripped.split_once("\n---") {
                clean_text = body.trim().to_string();
            } else if let Some((_fm, body)) = stripped.split_once("---") {
                clean_text = body.trim().to_string();
            }
        }
    } else if let Some(stripped) = content.strip_prefix("---") {
        if let Some((_fm, body)) = stripped.split_once("\n---") {
            clean_text = body.trim().to_string();
        } else if let Some((_fm, body)) = stripped.split_once("---") {
            clean_text = body.trim().to_string();
        }
    }

    // Extract summary from blockquote `> **ИИ-выжимка:** ...` if present in text
    if ai_summary.is_none() {
        if let Some(pos) = clean_text.find("**ИИ-выжимка:**") {
            let after = &clean_text[pos + "**ИИ-выжимка:**".len()..];
            let sum_line = after.lines().next().unwrap_or("").trim();
            if !sum_line.is_empty() {
                ai_summary = Some(sum_line.to_string());
            }
        } else if let Some(pos) = clean_text.find("💡 **ИИ-выжимка:**") {
            let after = &clean_text[pos + "💡 **ИИ-выжимка:**".len()..];
            let sum_line = after.lines().next().unwrap_or("").trim();
            if !sum_line.is_empty() {
                ai_summary = Some(sum_line.to_string());
            }
        }
    }

    // Remove markdown headers and noise from clean text
    if clean_text.starts_with('#') {
        if let Some(first_nl) = clean_text.find('\n') {
            clean_text = clean_text[first_nl..].trim().to_string();
        }
    }

    // Strip inline #hashtags from text for distraction-free view
    let re_tags = regex::Regex::new(r"#\w+").unwrap();
    clean_text = re_tags.replace_all(&clean_text, "").trim().to_string();

    // If no explicit AI summary in frontmatter, create a 1-2 sentence executive preview
    if ai_summary.is_none() && !clean_text.is_empty() {
        for line in clean_text.lines() {
            let tr = line.trim();
            if !tr.is_empty() && !tr.starts_with('#') && !tr.starts_with('|') && !tr.starts_with('[') && !tr.starts_with("---") && !tr.starts_with("**") {
                let clean_line = tr.trim_start_matches('-').trim();
                if !clean_line.is_empty() {
                    ai_summary = Some(clean_line.to_string());
                    break;
                }
            }
        }
    }

    let note_id = format!("{:016x}", rel_path.bytes().fold(5381u64, |acc, b| acc.wrapping_mul(33).wrapping_add(b as u64)));

    VaultNoteView {
        id: note_id,
        title: file_stem.to_string(),
        para_category,
        area,
        clean_text,
        ai_summary,
        tags,
        entities,
        created_at,
        file_path: rel_path,
    }
}

async fn get_note_properties(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, Response> {
    auth_check(&state, &headers, Some(&query))?;
    let vault_path = state.vault_registry.read().await.get_active_path();
    let notes = fetch_vault_notes(&vault_path).await;
    if let Some(note) = notes.into_iter().find(|n| n.id == id) {
        Ok((StatusCode::OK, Json(serde_json::to_value(note).unwrap_or_default())))
    } else {
        Ok((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Note not found" }))))
    }
}

async fn delete_vault_note(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, Response> {
    auth_check(&state, &headers, Some(&query))?;
    let vault_path = state.vault_registry.read().await.get_active_path();
    let vault_base = match std::fs::canonicalize(&vault_path) {
        Ok(p) => p,
        Err(_) => std::path::PathBuf::from(&vault_path),
    };
    let notes = fetch_vault_notes(&vault_path).await;
    
    if let Some(note) = notes.into_iter().find(|n| n.id == id || n.title == id || n.file_path == id || n.file_path.ends_with(&id)) {
        let abs_path = if std::path::Path::new(&note.file_path).is_absolute() {
            std::path::PathBuf::from(&note.file_path)
        } else {
            std::path::Path::new(&vault_path).join(&note.file_path)
        };
        if let Ok(canon) = std::fs::canonicalize(&abs_path) {
            if canon.starts_with(&vault_base) {
                let _ = tokio::fs::remove_file(&canon).await;
            }
        } else {
            let _ = tokio::fs::remove_file(&abs_path).await;
        }
        let _ = state.engine.delete_record(&note.file_path).await;
        let _ = state.engine.delete_record(&note.title).await;
        let _ = state.engine.delete_record(&note.id).await;
        let _ = state.engine.delete_record(&abs_path.to_string_lossy()).await;
        info!("🗑 Deleted vault note: {} ({:?})", note.title, abs_path);
        Ok((StatusCode::OK, Json(serde_json::json!({ "status": "deleted", "id": id, "title": note.title }))))
    } else {
        let candidate_path = std::path::Path::new(&vault_path).join(&id);
        if let Ok(canon) = std::fs::canonicalize(&candidate_path) {
            if canon.starts_with(&vault_base) && canon.exists() {
                let _ = tokio::fs::remove_file(&canon).await;
            }
        }
        let _ = state.engine.delete_record(&id).await;
        info!("🗑 Deleted vault record by id/path: {}", id);
        Ok((StatusCode::OK, Json(serde_json::json!({ "status": "deleted", "id": id }))))
    }
}

// ── Stats & AI Handlers ─────────────────────────────────────────────────────

#[derive(Serialize)]
struct SystemStats {
    total_notes: usize,
    total_tracks: usize,
    total_videos: usize,
    vault_path: String,
}

async fn get_stats(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, Response> {
    auth_check(&state, &headers, Some(&query))?;
    let lib = state.downloader.get_library().await;
    let total_tracks = lib.iter().filter(|i| i.media_type == "audio").count();
    let total_videos = lib.iter().filter(|i| i.media_type == "video").count();
    let vault_path = state.vault_registry.read().await.get_active_path();
    let notes = fetch_vault_notes(&vault_path).await;

    Ok(Json(SystemStats {
        total_notes: notes.len(),
        total_tracks,
        total_videos,
        vault_path,
    }))
}

#[derive(Deserialize)]
struct AiInsightRequest {
    period_days: usize,
}

async fn get_ai_insight(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AiInsightRequest>,
) -> Result<impl IntoResponse, Response> {
    auth_check(&state, &headers, None)?;
    let metrics = vec![];
    match state.analytics_engine.generate_life_insights(&metrics, payload.period_days).await {
        Ok(text) => Ok((StatusCode::OK, Json(serde_json::json!({ "insight": text })))),
        Err(e) => Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": e.to_string() })))),
    }
}

// ── HTTP Range Streaming Helper ─────────────────────────────────────────────

async fn stream_file_with_range(
    path: PathBuf,
    content_type: &'static str,
    headers: HeaderMap,
) -> Response {
    use axum::body::Body;
    use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
    use tokio_util::io::ReaderStream;

    let mut file = match tokio::fs::File::open(&path).await {
        Ok(f) => f,
        Err(_) => return (StatusCode::NOT_FOUND, "File not found").into_response(),
    };
    let metadata = match file.metadata().await {
        Ok(m) => m,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Cannot read metadata").into_response(),
    };
    let total_size = metadata.len();

    if let Some(range_header) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) {
        if let Some(range_spec) = range_header.strip_prefix("bytes=") {
            let parts: Vec<&str> = range_spec.split('-').collect();
            let start: u64 = parts[0].parse().unwrap_or(0);
            let end: u64 = if parts.len() > 1 && !parts[1].is_empty() {
                parts[1].parse().unwrap_or(total_size.saturating_sub(1))
            } else {
                total_size.saturating_sub(1)
            };
            let end = end.min(total_size.saturating_sub(1));

            if start <= end && start < total_size {
                let length = end - start + 1;
                if file.seek(SeekFrom::Start(start)).await.is_err() {
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Seek error").into_response();
                }
                let take_file = file.take(length);
                let stream = ReaderStream::with_capacity(take_file, 65536);
                let body = Body::from_stream(stream);

                return (
                    StatusCode::PARTIAL_CONTENT,
                    [
                        (header::CONTENT_TYPE, content_type.to_string()),
                        (header::ACCEPT_RANGES, "bytes".to_string()),
                        (
                            header::CONTENT_RANGE,
                            format!("bytes {}-{}/{}", start, end, total_size),
                        ),
                        (header::CONTENT_LENGTH, length.to_string()),
                    ],
                    body,
                )
                    .into_response();
            }
        }
    }

    let stream = ReaderStream::with_capacity(file, 65536);
    let body = Body::from_stream(stream);

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (header::ACCEPT_RANGES, "bytes".to_string()),
            (header::CONTENT_LENGTH, total_size.to_string()),
        ],
        body,
    )
        .into_response()
}

// ── Playlist Management ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub playlist_type: String, // "audio" | "video"
    pub item_ids: Vec<String>,
    pub created_at: String,
}

#[derive(Deserialize)]
struct CreatePlaylistRequest {
    name: String,
    playlist_type: Option<String>,
}

#[derive(Deserialize)]
struct AddPlaylistItemRequest {
    item_id: String,
}

async fn load_playlists(download_dir: &StdPath) -> Vec<Playlist> {
    let path = download_dir.join("playlists.json");
    if path.exists() {
        if let Ok(data) = tokio::fs::read_to_string(&path).await {
            if let Ok(list) = serde_json::from_str::<Vec<Playlist>>(&data) {
                return list;
            }
        }
    }
    vec![
        Playlist {
            id: "favorites_audio".to_string(),
            name: "Любимые треки".to_string(),
            playlist_type: "audio".to_string(),
            item_ids: vec![],
            created_at: chrono::Local::now().format("%Y-%m-%d").to_string(),
        },
        Playlist {
            id: "favorites_video".to_string(),
            name: "Сохраненные видео".to_string(),
            playlist_type: "video".to_string(),
            item_ids: vec![],
            created_at: chrono::Local::now().format("%Y-%m-%d").to_string(),
        },
    ]
}

async fn save_playlists(download_dir: &StdPath, playlists: &[Playlist]) {
    let path = download_dir.join("playlists.json");
    if let Ok(json) = serde_json::to_string_pretty(playlists) {
        let _ = tokio::fs::write(path, json).await;
    }
}

async fn get_playlists(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, Response> {
    auth_check(&state, &headers, Some(&query))?;
    let list = load_playlists(state.downloader.download_dir()).await;
    Ok(Json(list))
}

async fn create_playlist(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreatePlaylistRequest>,
) -> Result<impl IntoResponse, Response> {
    auth_check(&state, &headers, None)?;
    let mut list = load_playlists(state.downloader.download_dir()).await;
    let pl_type = payload.playlist_type.unwrap_or_else(|| "audio".to_string());
    let pl = Playlist {
        id: format!("pl_{}", &uuid::Uuid::new_v4().to_string()[..8]),
        name: payload.name.trim().to_string(),
        playlist_type: pl_type,
        item_ids: vec![],
        created_at: chrono::Local::now().format("%Y-%m-%d %H:%M").to_string(),
    };
    list.push(pl.clone());
    save_playlists(state.downloader.download_dir(), &list).await;
    Ok((StatusCode::CREATED, Json(pl)))
}

async fn delete_playlist(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, Response> {
    auth_check(&state, &headers, None)?;
    let mut list = load_playlists(state.downloader.download_dir()).await;
    let initial_len = list.len();
    list.retain(|p| p.id != id);
    if list.len() < initial_len {
        save_playlists(state.downloader.download_dir(), &list).await;
        Ok((StatusCode::OK, Json(serde_json::json!({ "status": "deleted" }))))
    } else {
        Ok((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Playlist not found" }))))
    }
}

async fn add_item_to_playlist(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<AddPlaylistItemRequest>,
) -> Result<impl IntoResponse, Response> {
    auth_check(&state, &headers, None)?;
    let mut list = load_playlists(state.downloader.download_dir()).await;
    if let Some(pl) = list.iter_mut().find(|p| p.id == id) {
        if !pl.item_ids.contains(&payload.item_id) {
            pl.item_ids.push(payload.item_id);
        }
        let updated = pl.clone();
        save_playlists(state.downloader.download_dir(), &list).await;
        Ok((StatusCode::OK, Json(serde_json::to_value(updated).unwrap_or_default())))
    } else {
        Ok((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Playlist not found" }))))
    }
}

async fn remove_item_from_playlist(
    State(state): State<AppState>,
    Path((id, item_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, Response> {
    auth_check(&state, &headers, None)?;
    let mut list = load_playlists(state.downloader.download_dir()).await;
    if let Some(pl) = list.iter_mut().find(|p| p.id == id) {
        pl.item_ids.retain(|i| i != &item_id);
        let updated = pl.clone();
        save_playlists(state.downloader.download_dir(), &list).await;
        Ok((StatusCode::OK, Json(serde_json::to_value(updated).unwrap_or_default())))
    } else {
        Ok((StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Playlist not found" }))))
    }
}
