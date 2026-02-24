/* app.js — Minimal UI interactions. This is the ONLY custom JS besides HTMX.
 * SRI-hash this file and add to CSP if you want belt-and-suspenders.
 * Total: ~30 lines. Fully auditable.
 */

// Sidebar toggle
document.getElementById('sidebar-toggle').addEventListener('click', function () {
    document.getElementById('sidebar').classList.toggle('collapsed');
});

// Theme toggle — uses CSS [data-theme] attribute, no localStorage (no fingerprinting)
// If you want persistence, the server can set a theme cookie instead.
document.getElementById('theme-toggle').addEventListener('click', function () {
    var html = document.documentElement;
    var next = html.getAttribute('data-theme') === 'dark' ? 'light' : 'dark';
    html.setAttribute('data-theme', next);
});

// Auto-dismiss error toasts after 5 seconds
document.body.addEventListener('htmx:afterSwap', function (e) {
    if (e.detail.target && e.detail.target.id === 'error-toast') {
        setTimeout(function () {
            e.detail.target.innerHTML = '';
        }, 5000);
    }
});

// Update CSRF token from response headers on every HTMX request
document.body.addEventListener('htmx:afterRequest', function (e) {
    var token = e.detail.xhr && e.detail.xhr.getResponseHeader('X-CSRF-Token');
    if (token) {
        // Update the hx-headers on body with the fresh CSRF token
        document.body.setAttribute('hx-headers', JSON.stringify({ 'X-CSRF-Token': token }));
    }
});

// SPA navigation — update sidebar active state and page title after content swap
function updateNavState() {
    var path = window.location.pathname;
    document.querySelectorAll('.sidebar-nav .nav-link').forEach(function (link) {
        link.classList.remove('active');
        if (link.getAttribute('href') === path) {
            link.classList.add('active');
        }
    });
    var titles = { '/': 'Home', '/demo': 'Demo', '/components': 'Components', '/security': 'Security', '/about': 'About' };
    document.title = (titles[path] || 'Page') + ' - Axum HTMX App';
}

// Forward navigation (HTMX push)
document.body.addEventListener('htmx:pushedIntoHistory', updateNavState);
// Back/forward button
window.addEventListener('popstate', function () {
    setTimeout(updateNavState, 10);
});
