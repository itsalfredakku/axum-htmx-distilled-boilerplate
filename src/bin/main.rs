use std::sync::Arc;
use std::time::SystemTime;

use axum::{middleware, routing::get, Router};
use tower::ServiceBuilder;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing::info;

use app::{
    config::AppConfig,
    db,
    handlers::{partials, templates},
    middleware as mw,
    models::AppState,
    services::Services,
    utils::logging,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load config
    let config = AppConfig::load().unwrap_or_else(|e| {
        eprintln!("Config error: {}, using defaults", e);
        AppConfig::default()
    });

    // Init logging
    logging::init_logging(&config.logging.level)?;

    info!("Starting axum-htmx-app v{}", env!("CARGO_PKG_VERSION"));

    // Initialize database pool and run migrations
    let db = db::init_pool(&config.database.url)
        .await
        .expect("Failed to initialize database");

    // Initialize services (includes CSRF secret + session store)
    let services = Services::new_with_db(SystemTime::now(), db.clone());

    // Shared state with services
    let state = Arc::new(AppState::new(services, db));

    // ── Routes ──────────────────────────────────────────────────────────
    // No JSON API. No Swagger. No CORS.
    // Every route returns HTML — full pages or HTMX partials.

    // HTMX partial routes (HTML fragments)
    let partial_routes = Router::new()
        .route("/partials/status-card", get(partials::status_card))
        .route("/partials/item-list", get(partials::item_list))
        .route("/partials/greeting", get(partials::greeting));

    // Health check (no middleware — used by Docker HEALTHCHECK)
    let health_route = Router::new().route("/healthz", get(app::handlers::healthz));

    // Page routes (full HTML)
    let app = Router::new()
        .route("/", get(templates::home_page))
        .route("/about", get(templates::about_page))
        .route("/demo", get(templates::demo_page))
        .route("/components", get(templates::components_page))
        .merge(partial_routes)
        .merge(health_route)
        // Static files (vendored CSS, JS, fonts — no external CDN)
        .nest_service("/static", ServeDir::new("static"))
        // Inject shared state into extensions for middleware access
        .layer(axum::Extension(state.clone()))
        .with_state(state.clone())
        // ── Middleware (applied bottom-up) ───────────────────────────────
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(middleware::from_fn(mw::request_logger))
                .layer(middleware::from_fn(mw::security_headers))
                .layer(middleware::from_fn(mw::session_middleware))
                .layer(middleware::from_fn(mw::csrf_protection)),
        );

    // ── Start ───────────────────────────────────────────────────────────

    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("Listening on http://{}", addr);
    info!("Security: CSP + CSRF + HttpOnly sessions + SRI + no external deps");

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            info!("Shutting down...");
        })
        .await?;

    Ok(())
}
