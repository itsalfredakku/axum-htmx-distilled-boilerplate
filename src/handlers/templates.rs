//! Page Handlers — serve full HTML pages
//!
//! Uses the define_page! macro for zero-cost dual-mode rendering:
//! - Debug: minijinja hot-reloads templates from disk  
//! - Release: askama compiles templates into the binary

use axum::{extract::State, http::header, response::IntoResponse};
use std::sync::Arc;

use crate::models::AppState;
use crate::services::session::SESSION_COOKIE;

// Define pages using the macro — one line per page instead of ~20!
crate::define_page!(HomePage, "pages/home.html", { current_page: &'static str, csrf_token: String });
crate::define_page!(AboutPage, "pages/about.html", { current_page: &'static str, csrf_token: String });
crate::define_page!(DemoPage, "pages/demo.html", { current_page: &'static str, csrf_token: String });
crate::define_page!(ComponentsPage, "pages/components.html", { current_page: &'static str, csrf_token: String });

/// Extract session ID from request cookies
fn get_session_id(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|c| {
                let c = c.trim();
                c.strip_prefix(&format!("{}=", SESSION_COOKIE))
                    .map(|v| v.to_string())
            })
        })
}

// =============================================================================
// Page Handlers — thin wrappers that delegate to templates
// =============================================================================

pub async fn home_page(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let sid = get_session_id(&headers).unwrap_or_default();
    let csrf_token = state.services.csrf.generate_token(&sid);
    HomePage {
        current_page: "home",
        csrf_token,
    }
    .render_response()
}

pub async fn about_page(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let sid = get_session_id(&headers).unwrap_or_default();
    let csrf_token = state.services.csrf.generate_token(&sid);
    AboutPage {
        current_page: "about",
        csrf_token,
    }
    .render_response()
}

pub async fn demo_page(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let sid = get_session_id(&headers).unwrap_or_default();
    let csrf_token = state.services.csrf.generate_token(&sid);
    DemoPage {
        current_page: "demo",
        csrf_token,
    }
    .render_response()
}

pub async fn components_page(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let sid = get_session_id(&headers).unwrap_or_default();
    let csrf_token = state.services.csrf.generate_token(&sid);
    ComponentsPage {
        current_page: "components",
        csrf_token,
    }
    .render_response()
}
