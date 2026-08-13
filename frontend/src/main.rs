use axum::{body::Body, extract::Path, http::Request, routing::get, Router};
use leptos::prelude::*;
use leptos::{component, view, IntoView};
use leptos_axum::render_app_to_stream;
use tower_http::services::ServeDir;

#[derive(Clone, Copy, PartialEq)]
enum Route {
    Browse,
    Detail,
}

#[component]
fn App(route: Route, name: String) -> impl IntoView {
    view! {
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <title>"LXS Registry — Linux Services"</title>
                <link rel="preconnect" href="https://fonts.googleapis.com" />
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="true" />
                <link href="https://fonts.googleapis.com/css2?family=DM+Mono:wght@400;500&family=Manrope:wght@400;500;600;700;800&display=swap" rel="stylesheet" />
                <link rel="stylesheet" href="/static/style.css" />
            </head>
            <body>
                <header class="top">
                    <div class="top-inner">
                        <a class="brand" href="/">
                            <span class="brand-mark" aria-hidden="true">"L"</span>
                            <span class="brand-name">"LXS Registry"</span>
                        </a>
                        <nav class="nav" aria-label="Primary">
                            <a href="/">"Browse"</a>
                            <a href="https://github.com/getecosphere/lxs-registry">"GitHub"</a>
                        </nav>
                    </div>
                </header>
                {match route {
                    Route::Browse => view! { <BrowsePage /> }.into_any(),
                    Route::Detail => view! { <DetailPage name={name.clone()} /> }.into_any(),
                }}
                <footer class="foot"><div class="foot-inner"><p>"LXS Registry — versioned Linux Services, composed into Estates."</p></div></footer>
            </body>
        </html>
    }
}

#[component]
fn BrowsePage() -> impl IntoView {
    let js = r##"(function () {
      var root = document.getElementById("lxs-root");
      if (!root) return;
      var chips = document.getElementById("category-chips");
      var search = document.getElementById("lxs-search");
      var state = { category: "", q: "" };
      var esc = function (v) { return String(v || "").replace(/[&<>"']/g, function (c) { return {"&":"&amp;","<":"&lt;",">":"&gt;","\"":"&quot;","'":"&#039;"}[c]; }); };
      var api = (location.hostname === "127.0.0.1" || location.hostname === "localhost") ? "http://127.0.0.1:8260/api/lxs" : "/api/lxs";
      function load() {
        root.innerHTML = '<p class="empty">Loading LXS…</p>';
        var url = api + "?category=" + encodeURIComponent(state.category) + "&q=" + encodeURIComponent(state.q);
        fetch(url).then(function (r) { return r.json(); }).then(function (data) {
          if (!data.lxs || !data.lxs.length) { root.innerHTML = '<p class="empty">No LXS match that filter.</p>'; return; }
          root.innerHTML = data.lxs.map(function (m) {
            var archs = (m.targets || []).map(function (a) { return '<i>' + esc(a) + '</i>'; }).join("");
            return '<article class="lxs-card">' +
              '<div class="card-head"><h3><a href="/lxs/' + esc(m.name) + '">' + esc(m.name) + '</a></h3>' +
              '<span class="ver">v' + esc(m.version) + '</span><span class="cat">' + esc(m.category) + '</span>' +
              '<span class="status ' + esc(m.status) + '">' + esc(m.status) + '</span>' +
              (m.docs_available ? '<span class="status docs">docs</span>' : "") + '</div>' +
              '<p class="sum">' + esc(m.summary) + '</p>' +
              '<p class="meta">' + esc(m.runtime || "self-contained") + ' · ' + esc(m.publisher || "") + ' · ' + archs + '</p>' +
              (m.source ? '<a class="src" href="' + esc(m.source.replace(".git","")) + '" target="_blank" rel="noopener">View source ↗</a>' : "") +
              '</article>';
          }).join("");
        }).catch(function () { root.innerHTML = '<p class="empty">Could not reach the LXS registry API.</p>'; });
      }
      function loadCategories() {
        var curl = (location.hostname === "127.0.0.1" || location.hostname === "localhost") ? "http://127.0.0.1:8260/api/lxs/categories" : "/api/lxs/categories";
        fetch(curl).then(function (r) { return r.json(); }).then(function (d) {
          if (!d.categories || !chips) return;
          var all = '<button type="button" data-cat="" class="chip' + (state.category === "" ? " active" : "") + '">All</button>';
          chips.innerHTML = all + d.categories.map(function (c) {
            return '<button type="button" data-cat="' + esc(c.name) + '" class="chip' + (state.category === c.name ? " active" : "") + '">' + esc(c.name) + ' <small>' + c.count + '</small></button>';
          }).join("");
          chips.querySelectorAll("button").forEach(function (b) {
            b.addEventListener("click", function () { state.category = b.getAttribute("data-cat"); load(); loadCategories(); });
          });
        }).catch(function () {});
      }
      if (search) search.addEventListener("input", function () { state.q = search.value; load(); });
      loadCategories();
      load();
    })();"##;
    view! {
        <main class="shell">
            <section class="hero">
                <p class="kicker">"LXS · LINUX SERVICE"</p>
                <h1>"Browse the LXS Registry."</h1>
                <p class="lede">"Versioned, tested Linux Services — compose them into an Estate with ecompose.yml. Every capability is traceable to its source."</p>
                <input id="lxs-search" type="search" placeholder="Search auth, storage, notifications…" aria-label="Search LXS" />
            </section>
            <div id="category-chips" class="chips" aria-label="Filter by category"></div>
            <div id="lxs-root" class="grid"><p class="empty">"Loading LXS…"</p></div>
        </main>
        <script>{js}</script>
    }
}

#[component]
fn DetailPage(name: String) -> impl IntoView {
    let js = r##"(function () {
      var root = document.getElementById("lxs-detail");
      if (!root) return;
      var name = document.getElementById("lxs-detail").getAttribute("data-name");
      var api = (location.hostname === "127.0.0.1" || location.hostname === "localhost") ? "http://127.0.0.1:8260/api/lxs/" + name : "/api/lxs/" + name;
      var esc = function (v) { return String(v || "").replace(/[&<>"']/g, function (c) { return {"&":"&amp;","<":"&lt;",">":"&gt;","\"":"&quot;","'":"&#039;"}[c]; }); };
      fetch(api).then(function (r) { return r.ok ? r.json() : null; }).then(function (d) {
        if (!d) { root.innerHTML = '<p class="empty">LXS not found.</p>'; return; }
        var docs = d.docs || null;
        var hasDocs = docs && docs.files && docs.files.length > 0;
        var reqEnv = (d.contract && d.contract.env && d.contract.env.required || []).map(esc).join(", ");
        var compose = '# ecompose.yml\nservices:\n  ' + (d.name || "svc") + '-backend:\n    lxs: ' + esc(d.name || "") + '@' + esc(d.version || "") + '\n    grants:\n      secrets: [' + reqEnv + ']   # must cover contract.env.required';
        var docsHtml = hasDocs ?
          '<div class="docs">' +
            '<h3>Docs bundle</h3>' +
            '<p class="doc-meta">Ships with this version: ' + docs.files.map(esc).join(" · ") + (docs.has_openapi ? ' · openapi.json' : '') + '</p>' +
            '<div class="compose"><b>Compose</b><pre>' + esc(compose) + '</pre></div>' +
            '<details open><summary>Overview — README.md</summary><pre>' + esc(docs.index || "") + '</pre></details>' +
            '<details><summary>API reference — api.md</summary><pre>' + esc(docs.api || "") + '</pre></details>' +
            '<details><summary>Examples — examples.sh</summary><pre>' + esc(docs.examples || "") + '</pre></details>' +
            '<details><summary>Changelog</summary><pre>' + esc(docs.changelog || "") + '</pre></details>' +
            '<details><summary>Gotchas</summary><pre>' + esc(docs.gotchas || "") + '</pre></details>' +
            '<p class="doc-note">For AI agents: this LXS ships as a <b>binary only</b> — these docs are the entire interface. Run <code>examples.sh</code> against a pulled binary before trusting behavior.</p>' +
          '</div>' :
          '<p class="empty">No docs bundle for this version yet.</p>';
        root.innerHTML =
          '<p class="kicker">' + esc(d.category || "Uncategorized") + ' · v' + esc(d.version) + ' · ' + esc(d.status || "unverified") + '</p>' +
          '<h1>' + esc(d.name) + '</h1>' +
          '<p class="lede">' + esc(d.summary) + '</p>' +
          (d.source ? '<a class="text-link" href="' + esc(d.source.replace(".git","")) + '" target="_blank" rel="noopener">View source ↗</a>' : "") +
          '<div class="spec">' +
          '<h3>Contract</h3>' +
          '<p><b>Database:</b> ' + esc(d.contract && d.contract.db || "none") + '</p>' +
          '<p><b>API:</b> ' + esc(d.contract && d.contract.api || "") + '</p>' +
          '<p><b>Required env:</b> ' + ((d.contract && d.contract.env && d.contract.env.required || []).map(esc).join(", ") || "—") + '</p>' +
          '<h3>Runtime</h3>' +
          '<p><b>Base:</b> ' + esc(d.runtime && d.runtime.base || "") + ' · libc ' + esc(d.runtime && d.runtime.libc || "") + '</p>' +
          '<h3>Provenance</h3>' +
          '<p><b>Commit:</b> <code>' + esc(d.provenance && d.provenance.commit || "") + '</code></p>' +
          '<p><b>Built by:</b> ' + esc(d.provenance && d.provenance.built_by || "") + ' · ' + esc(d.provenance && d.provenance.built_at || "") + '</p>' +
          '<p><b>Versions:</b> ' + (d.versions || []).map(function (v) { return esc(v); }).join(", ") + '</p>' +
          '</div>' +
          docsHtml;
      }).catch(function () { root.innerHTML = '<p class="empty">Could not reach the LXS registry API.</p>'; });
    })();"##;
    view! {
        <main class="shell"><div id="lxs-detail" class="detail" data-name={name.clone()}><p class="empty">"Loading…"</p></div></main>
        <script>{js}</script>
    }
}

async fn render_detail(Path(name): Path<String>) -> axum::response::Response<Body> {
    render_app_to_stream(move || view! { <App route=Route::Detail name={name.clone()} /> })(Request::new(Body::empty())).await
}

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8261);
    let app = Router::new()
        .route("/", get(render_app_to_stream(|| view! { <App route=Route::Browse name=String::new() /> })))
        .route("/lxs/:name", get(render_detail))
        .nest_service("/static", ServeDir::new("static"));
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await.unwrap();
    println!("[registry-frontend] listening on :{port}");
    axum::serve(listener, app).await.unwrap();
}
