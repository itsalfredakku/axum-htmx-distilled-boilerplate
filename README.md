# Axum + HTMX Boilerplate

A full-stack Rust web application boilerplate that delivers SPA-like interactivity using server-rendered HTML — no JavaScript framework needed.

## Stack

| Layer | Tech | Role |
|---|---|---|
| Runtime | [Tokio](https://tokio.rs) | Async runtime |
| Framework | [Axum](https://github.com/tokio-rs/axum) 0.7 | HTTP routing, middleware, state |
| Interactivity | [HTMX](https://htmx.org) | Swap HTML fragments without page reloads |
| Templates (dev) | [MiniJinja](https://github.com/mitsuhiko/minijinja) | Hot-reload from disk |
| Templates (release) | [Askama](https://github.com/djc/askama) | Compiled into the binary at build time |
| Styling | Custom CSS (~5 KB) | Dark-mode-ready design system, no framework |
| API Docs | [utoipa](https://github.com/juhaku/utoipa) + Swagger UI | Auto-generated OpenAPI spec |
| Error Handling | [thiserror](https://github.com/dtolnay/thiserror) + [anyhow](https://github.com/dtolnay/anyhow) | Typed errors with HTMX-aware HTML responses |

## Quick Start

```bash
cargo run
```

Open [http://localhost:8000](http://localhost:8000) for the app and [http://localhost:8000/api-docs/](http://localhost:8000/api-docs/) for Swagger UI.

## How It Works

```
Browser                 Server
  │                       │
  │  GET /about           │   full HTML page (server-rendered)
  │──────────────────────▶│
  │◀──────────────────────│
  │                       │
  │  GET /partials/       │   HTML fragment (HTMX swap)
  │  status-card          │
  │──────────────────────▶│
  │◀──────────────────────│
  │                       │
  │  GET /api/health      │   JSON response (REST API)
  │──────────────────────▶│
  │◀──────────────────────│
```

Three response modes from the same server:

1. **Pages** — full HTML documents served on navigation (`/`, `/about`, `/demo`).
2. **Partials** — HTML fragments fetched by HTMX and swapped into the DOM (`/partials/status-card`, `/partials/item-list`, `/partials/greeting`).
3. **API** — JSON endpoints for programmatic access (`/api/health`).

Templates hot-reload during development (`cargo run`) and compile into the binary in release builds (`cargo build --release`).

## Project Layout

```
├── config/
│   └── app.toml                 # Server, logging & env settings
├── src/
│   ├── bin/main.rs              # Entry point: router, middleware, server
│   ├── lib.rs                   # Crate root, module declarations
│   ├── config.rs                # TOML config loader (env override support)
│   ├── error.rs                 # AppError — HTMX-aware error responses
│   ├── render.rs                # define_page! / define_partial! macros
│   ├── handlers/
│   │   ├── templates.rs         # Full-page route handlers
│   │   ├── partials.rs          # HTMX fragment handlers
│   │   └── api/health.rs        # JSON REST endpoints
│   ├── services/
│   │   ├── mod.rs               # Service container (DI via Arc<dyn Trait>)
│   │   ├── health.rs            # Health check service
│   │   └── items.rs             # Item CRUD (in-memory, DB-ready)
│   ├── middleware/mod.rs        # Security headers, request logging
│   ├── models/mod.rs            # Shared AppState
│   └── utils/
│       ├── logging.rs           # tracing/tracing-subscriber init
│       └── templates.rs         # MiniJinja hot-reload helper
├── templates/
│   ├── base.html                # Root layout (sidebar, header, theme toggle)
│   ├── pages/                   # Full-page templates
│   ├── partials/                # Fragment templates
│   └── components/              # Reusable design tokens
└── static/
    ├── css/app.css              # ~5 KB design system
    ├── css/bootstrap-icons.*    # Icon font styles
    ├── fonts/                   # Bootstrap Icons woff/woff2
    └── js/htmx.min.js           # HTMX (~14 KB gzip)
```

## Configuration

Default settings live in `config/app.toml`:

```toml
[server]
host = "0.0.0.0"
port = 8000

[logging]
level = "info"
```

Override any value with environment variables using the `APP__` prefix and `__` as the nesting separator:

```bash
APP__SERVER__PORT=8080 cargo run
```

## Adding a Page

1. Create a template at `templates/pages/mypage.html` (extend `base.html`).
2. Define the struct and handler in `src/handlers/templates.rs`:

```rust
crate::define_page!(MyPage, "pages/mypage.html", { current_page: &'static str });

pub async fn my_page() -> impl IntoResponse {
    MyPage { current_page: "mypage" }.render_response()
}
```

3. Register the route in `src/bin/main.rs`:

```rust
.route("/mypage", get(templates::my_page))
```

## Adding a Partial

1. Create a template at `templates/partials/widget.html`.
2. Define the struct and handler in `src/handlers/partials.rs`:

```rust
crate::define_partial!(Widget, "partials/widget.html", { label: String });

pub async fn widget() -> impl IntoResponse {
    Widget { label: "hello".into() }.render_response()
}
```

3. Register the route in `src/bin/main.rs`:

```rust
.route("/partials/widget", get(partials::widget))
```

4. Trigger from any template:

```html
<div hx-get="/partials/widget" hx-swap="innerHTML"></div>
```

## Key Design Decisions

- **`define_page!` / `define_partial!` macros** — a single declaration generates both the Askama compiled template (release) and the MiniJinja hot-reloading template (debug), eliminating boilerplate.
- **Trait-based service layer** — services are injected as `Arc<dyn Trait>`, making it straightforward to swap in-memory implementations for database-backed ones or test doubles.
- **HTMX-aware error handling** — `AppError` renders HTML fragments with `HX-Retarget` and `HX-Reswap` headers so errors automatically appear in a toast/notification area.
- **Security middleware** — every response includes `X-Content-Type-Options`, `X-Frame-Options`, `Content-Security-Policy`, and other hardening headers out of the box.
- **Minimal JS footprint** — the only JavaScript dependency is HTMX (~14 KB gzipped). No build step, no bundler.

## Optional Features

Enable SQLite support via the `database` feature flag:

```bash
cargo run --features database
```

This pulls in [SQLx](https://github.com/launchbadge/sqlx) with the `runtime-tokio` and `sqlite` drivers.

## License

MIT
