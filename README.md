# Axum + HTMX (Distilled) Boilerplate

A starter template for building server-rendered Rust web applications with [Axum](https://github.com/tokio-rs/axum) and [HTMX](https://htmx.org). Zero JavaScript frameworks, zero CDN dependencies, fully self-contained.

## Features

- **Server-side rendering** with compile-time templates ([Askama](https://github.com/djc/askama)) and hot-reload in dev ([MiniJinja](https://github.com/mitsuhiko/minijinja))
- **HTMX-powered interactivity** — swap HTML fragments without writing JavaScript
- **Security hardened** — strict CSP, CSRF protection, SRI on all scripts, HttpOnly sessions
- **Dual-macro template system** — `define_page!` / `define_partial!` generate both compiled and hot-reload templates from one declaration
- **Trait-based service layer** — dependency injection via `Arc<dyn Trait>`, easy to test or swap implementations
- **HTMX-aware error handling** — errors render as HTML fragments with `HX-Retarget`/`HX-Reswap` headers
- **Docker-ready** — multi-stage Dockerfile and docker-compose included
- **Optional SQLite** — add `--features database` to pull in SQLx

## Tech Stack

| Component | Technology |
|---|---|
| Runtime | [Tokio](https://tokio.rs) |
| Framework | [Axum](https://github.com/tokio-rs/axum) 0.7 |
| Interactivity | [HTMX](https://htmx.org) (vendored, SRI-pinned) |
| Templates (dev) | [MiniJinja](https://github.com/mitsuhiko/minijinja) — hot-reload from disk |
| Templates (release) | [Askama](https://github.com/djc/askama) — compiled into the binary |
| Styling | Custom CSS (~5 KB), dark-mode support |
| Error handling | [thiserror](https://github.com/dtolnay/thiserror) + [anyhow](https://github.com/dtolnay/anyhow) |

## Quick Start

```bash
cargo run
# → http://localhost:8000
```

With Docker:

```bash
docker compose up --build
```

## Security

| Threat | Mitigation |
|---|---|
| XSS | Strict CSP, no inline scripts, SRI on all JS |
| CSRF | Per-session HMAC-SHA256 tokens, auto-sent via HTMX headers |
| Clickjacking | `X-Frame-Options: DENY`, `frame-ancestors 'none'` |
| Supply chain | All assets vendored locally — zero npm, zero CDN |
| Session theft | HttpOnly + SameSite=Strict cookies, server-side sessions |
| Fingerprinting | No server header, no referrer, no DNS prefetch |

## How It Works

The app serves two kinds of responses:

1. **Pages** — full HTML documents for navigation routes (`/`, `/about`, `/demo`)
2. **Partials** — HTML fragments fetched by HTMX and swapped into the DOM (`/partials/status-card`, `/partials/item-list`)

```
Browser                 Server
  │  GET /about           │  → full HTML page + session cookie + CSRF token
  │──────────────────────▶│
  │◀──────────────────────│
  │                       │
  │  GET /partials/...    │  → HTML fragment (HTMX swap)
  │──────────────────────▶│
  │◀──────────────────────│
  │                       │
  │  POST /submit         │  → CSRF validated, responds with fragment
  │  X-CSRF-Token: ...    │
  │──────────────────────▶│
  │◀──────────────────────│
```

## Project Structure

```
config/app.toml               # Server, logging & environment settings
src/
├── bin/main.rs                # Entry point — router, middleware, server
├── lib.rs                     # Crate root
├── config.rs                  # TOML config loader with env override
├── error.rs                   # AppError — HTMX-aware error responses
├── render.rs                  # define_page! / define_partial! macros
├── handlers/
│   ├── templates.rs           # Full-page route handlers
│   └── partials.rs            # HTMX fragment handlers
├── services/
│   ├── mod.rs                 # Service container (DI)
│   ├── csrf.rs                # CSRF token generation + validation
│   ├── session.rs             # Server-side session management
│   ├── health.rs              # Health check
│   └── items.rs               # Item CRUD (in-memory, DB-ready)
├── middleware/mod.rs          # Security headers, CSRF, sessions, logging
├── models/mod.rs              # Shared AppState
└── utils/
    ├── logging.rs             # tracing init
    └── templates.rs           # MiniJinja hot-reload helper
templates/
├── base.html                  # Root layout
├── pages/                     # Full-page templates
├── partials/                  # Fragment templates
└── components/                # Reusable tokens
static/
├── css/                       # App styles + vendored Bootstrap Icons CSS
├── fonts/                     # Vendored icon fonts
└── js/                        # Vendored HTMX + minimal app.js (both SRI-pinned)
```

## Configuration

Defaults live in `config/app.toml`. Override with environment variables using the `APP__` prefix:

```bash
APP__SERVER__PORT=9000 APP__LOGGING__LEVEL=debug cargo run
```

## Adding a Page

1. Create `templates/pages/mypage.html` (extend `base.html`).
2. Define the handler in `src/handlers/templates.rs`:

```rust
crate::define_page!(MyPage, "pages/mypage.html", {
    current_page: &'static str,
    csrf_token: String,
});

pub async fn my_page(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let sid = get_session_id(&headers).unwrap_or_default();
    let csrf_token = state.services.csrf.generate_token(&sid);
    MyPage { current_page: "mypage", csrf_token }.render_response()
}
```

3. Register the route in `src/bin/main.rs`:

```rust
.route("/mypage", get(templates::my_page))
```

## Adding a Partial

1. Create `templates/partials/widget.html`.
2. Define the handler in `src/handlers/partials.rs`:

```rust
crate::define_partial!(Widget, "partials/widget.html", { label: String });

pub async fn widget() -> impl IntoResponse {
    Widget { label: "hello".into() }.render_response()
}
```

3. Register the route and trigger it from any template:

```rust
.route("/partials/widget", get(partials::widget))
```

```html
<div hx-get="/partials/widget" hx-swap="innerHTML"></div>
```

## Tor / Air-Gapped Deployment

The app makes zero external requests — no CDN, no remote fonts, no analytics. This makes it suitable for Tor hidden services or fully offline environments.

```
# torrc
HiddenServiceDir /var/lib/tor/myapp/
HiddenServicePort 80 127.0.0.1:8000
```

## License

MIT

