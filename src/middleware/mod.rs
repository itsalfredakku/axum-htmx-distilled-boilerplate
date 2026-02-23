//! Hardened HTTP Middleware
//!
//! Security-first middleware stack:
//! - Strict security headers (CSP with SRI, no external resources)
//! - CSRF validation on all state-changing requests
//! - Session management via HttpOnly cookies
//! - Request logging with timing (no sensitive data leaked)
//! - Server header stripping

use axum::{
    extract::Request,
    http::{header, Method, StatusCode},
    middleware::Next,
    response::{Html, IntoResponse, Response},
};

use crate::models::AppState;
use crate::services::session::SESSION_COOKIE;
use std::sync::Arc;

/// SRI hash for the vendored htmx.min.js — update if the file changes.
/// Generate with: openssl dgst -sha384 -binary static/js/htmx.min.js | openssl base64 -A
const HTMX_SRI_HASH: &str =
    "sha384-HGfztofotfshcF7+8n44JQL2oJmowVChPTg48S+jvZoztPfvwD79OC/LTtG6dMp+";

/// SRI hash for app.js — update if the file changes.
const APP_SRI_HASH: &str =
    "sha384-PMounJsLzecWPmGgUp+rmq81ao6CaK1vp02qhyBK66VebP1pIGgbYS+m14+AsFN5";

// ─── Security Headers ───────────────────────────────────────────────────────

/// Hardened security headers — strict CSP, no external resources, no leaks
pub async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let h = response.headers_mut();

    // Content Security Policy — only allow self + SRI-hashed JS files
    // No unsafe-inline, no unsafe-eval, no external origins
    h.insert(
        header::HeaderName::from_static("content-security-policy"),
        format!(
            "default-src 'self'; \
             script-src 'self' '{HTMX_SRI_HASH}' '{APP_SRI_HASH}'; \
             style-src 'self' 'unsafe-inline'; \
             img-src 'self' data:; \
             font-src 'self'; \
             connect-src 'self'; \
             frame-ancestors 'none'; \
             base-uri 'self'; \
             form-action 'self'; \
             object-src 'none'"
        )
        .parse()
        .unwrap(),
    );

    // Prevent MIME sniffing
    h.insert(
        header::HeaderName::from_static("x-content-type-options"),
        header::HeaderValue::from_static("nosniff"),
    );

    // Prevent framing (clickjacking)
    h.insert(
        header::HeaderName::from_static("x-frame-options"),
        header::HeaderValue::from_static("DENY"),
    );

    // XSS protection (legacy browsers)
    h.insert(
        header::HeaderName::from_static("x-xss-protection"),
        header::HeaderValue::from_static("1; mode=block"),
    );

    // No referrer leaks (critical for .onion / dark web)
    h.insert(
        header::HeaderName::from_static("referrer-policy"),
        header::HeaderValue::from_static("no-referrer"),
    );

    // Prevent DNS prefetch (prevents DNS leaks on Tor)
    h.insert(
        header::HeaderName::from_static("x-dns-prefetch-control"),
        header::HeaderValue::from_static("off"),
    );

    // Disable browser features that leak info
    h.insert(
        header::HeaderName::from_static("permissions-policy"),
        header::HeaderValue::from_static(
            "camera=(), microphone=(), geolocation=(), browsing-topics=()",
        ),
    );

    // Strip server identification
    h.remove(header::SERVER);
    h.insert(header::SERVER, header::HeaderValue::from_static(""));

    // Prevent caching of sensitive pages
    h.insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store, no-cache, must-revalidate"),
    );
    h.insert(header::PRAGMA, header::HeaderValue::from_static("no-cache"));

    // Cross-Origin policies
    h.insert(
        header::HeaderName::from_static("cross-origin-opener-policy"),
        header::HeaderValue::from_static("same-origin"),
    );
    h.insert(
        header::HeaderName::from_static("cross-origin-embedder-policy"),
        header::HeaderValue::from_static("require-corp"),
    );
    h.insert(
        header::HeaderName::from_static("cross-origin-resource-policy"),
        header::HeaderValue::from_static("same-origin"),
    );

    response
}

// ─── CSRF Protection ────────────────────────────────────────────────────────

/// CSRF middleware — validates token on all state-changing requests.
/// The token must be sent as `X-CSRF-Token` header (HTMX sends this automatically
/// via `hx-headers` attribute on the body tag).
pub async fn csrf_protection(request: Request, next: Next) -> Response {
    let method = request.method().clone();

    // Only validate on state-changing methods
    if matches!(method, Method::GET | Method::HEAD | Method::OPTIONS) {
        return next.run(request).await;
    }

    // Extract state and session cookie
    let state = request.extensions().get::<Arc<AppState>>().cloned();
    let csrf_header = request
        .headers()
        .get("x-csrf-token")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let session_id = request
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|c| {
                let c = c.trim();
                c.strip_prefix(&format!("{}=", SESSION_COOKIE))
                    .map(|v| v.to_string())
            })
        });

    match (state, csrf_header, session_id) {
        (Some(state), Some(token), Some(sid)) => {
            // Verify session exists
            if state.services.sessions.get(&sid).is_none() {
                return csrf_error("Invalid session");
            }
            // Verify CSRF token
            if !state.services.csrf.validate_token(&token, &sid) {
                return csrf_error("Invalid CSRF token");
            }
            next.run(request).await
        }
        _ => csrf_error("Missing CSRF token or session"),
    }
}

fn csrf_error(msg: &str) -> Response {
    let body = format!(
        r#"<div class="alert alert-danger" role="alert">
    <div class="alert-title"><i class="bi bi-shield-x"></i> <strong>Forbidden</strong></div>
    <div class="alert-body">{}</div>
</div>"#,
        msg
    );
    (StatusCode::FORBIDDEN, Html(body)).into_response()
}

// ─── Session Middleware ─────────────────────────────────────────────────────

/// Session middleware — ensures every request has a valid session.
/// Creates a new session if none exists or if the session has expired.
/// Injects CSRF token into response for HTMX to pick up.
pub async fn session_middleware(request: Request, next: Next) -> Response {
    let state = match request.extensions().get::<Arc<AppState>>().cloned() {
        Some(s) => s,
        None => return next.run(request).await,
    };

    // Try to extract existing session ID from cookie
    let existing_sid = request
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|c| {
                let c = c.trim();
                c.strip_prefix(&format!("{}=", SESSION_COOKIE))
                    .map(|v| v.to_string())
            })
        });

    // Validate or create session
    let (session, _is_new) = match existing_sid {
        Some(ref sid) => {
            match state.services.sessions.get(sid) {
                Some(session) => {
                    state.services.sessions.touch(sid);
                    (session, false)
                }
                None => (state.services.sessions.create(), true), // Expired or invalid
            }
        }
        None => (state.services.sessions.create(), true),
    };

    // Generate CSRF token for this session
    let csrf_token = state.services.csrf.generate_token(&session.id);
    state
        .services
        .sessions
        .update_csrf(&session.id, &csrf_token);

    let mut response = next.run(request).await;

    // Set session cookie (always — refreshes expiry)
    let cookie_value = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Strict; Max-Age=3600",
        SESSION_COOKIE, session.id
    );
    response
        .headers_mut()
        .append(header::SET_COOKIE, cookie_value.parse().unwrap());

    // Inject CSRF token as a response header for HTMX to read
    response.headers_mut().insert(
        header::HeaderName::from_static("x-csrf-token"),
        csrf_token.parse().unwrap(),
    );

    // Periodically cleanup expired sessions (every ~100th request)
    if rand::random::<u8>() < 3 {
        state.services.sessions.cleanup_expired();
    }

    response
}

// ─── Request Logging ────────────────────────────────────────────────────────

/// Request logging middleware — logs method, path, status and duration.
/// Does NOT log query strings, headers, or bodies (no data leaks).
pub async fn request_logger(request: Request, next: Next) -> Response {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let start = std::time::Instant::now();

    let response = next.run(request).await;

    let duration = start.elapsed();
    tracing::info!(
        method = %method,
        path = %path,
        status = response.status().as_u16(),
        duration_ms = duration.as_millis() as u64,
        "request"
    );

    response
}
