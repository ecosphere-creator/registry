use axum::{body::Body, extract::{Path, Query, State}, http::Request, routing::get, Router};
use leptos::prelude::*;
use leptos::{component, view, IntoView};
use leptos_axum::render_app_to_stream;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path as FsPath, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tower_http::services::ServeDir;

const REGISTRY_URL: &str = "https://github.com/getecosphere/lxs-registry.git";
const REFRESH_INTERVAL: Duration = Duration::from_secs(600);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct LxsManifest {
    name: String,
    #[serde(default)]
    domain: String,
    version: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    publisher: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    targets: Vec<String>,
    #[serde(default)]
    contract: Contract,
    #[serde(default)]
    runtime: Runtime,
    #[serde(default)]
    provenance: Provenance,
    #[serde(default)]
    release: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Contract {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    api: String,
    #[serde(default)]
    db: String,
    #[serde(default)]
    env: Env,
    #[serde(default)]
    network: Network,
    #[serde(default)]
    resources: Resources,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Env {
    #[serde(default)]
    required: Vec<String>,
    #[serde(default)]
    optional: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Network {
    #[serde(default)]
    inbound: Vec<String>,
    #[serde(default)]
    outbound: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Resources {
    #[serde(default)]
    memory: String,
    #[serde(default)]
    disk: String,
    #[serde(default)]
    startup_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Runtime {
    #[serde(default)]
    base: String,
    #[serde(default)]
    libc: String,
    #[serde(default)]
    dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Provenance {
    #[serde(default)]
    source: String,
    #[serde(default)]
    commit: String,
    #[serde(default)]
    built_by: String,
    #[serde(default)]
    built_at: String,
}

#[derive(Clone)]
struct AppState {
    cache: PathBuf,
    manifests: Arc<Mutex<Vec<LxsManifest>>>,
    last_refresh: Arc<Mutex<SystemTime>>,
}

fn run(cmd: &str, args: &[&str]) -> Result<(), String> {
    let out = Command::new(cmd).args(args).output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn scan_manifests(cache: &FsPath) -> Vec<LxsManifest> {
    let mut out = Vec::new();
    let Ok(names) = std::fs::read_dir(cache) else { return out };
    for name in names.flatten() {
        let name_dir = name.path();
        if !name_dir.is_dir() || name_dir.file_name().map(|n| n == ".git").unwrap_or(false) {
            continue;
        }
        let Ok(versions) = std::fs::read_dir(&name_dir) else { continue };
        for version in versions.flatten() {
            let manifest_path = version.path().join("lxs.yml");
            if !manifest_path.is_file() {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&manifest_path) {
                if let Ok(m) = serde_yaml::from_str::<LxsManifest>(&text) {
                    out.push(m);
                }
            }
        }
    }
    out.sort_by(|a, b| format!("{}-{}", a.name, a.version).cmp(&format!("{}-{}", b.name, b.version)));
    out
}

fn refresh_registry(state: &AppState) -> Result<(), String> {
    let cache = &state.cache;
    if !cache.join(".git").exists() {
        let _ = std::fs::remove_dir_all(cache);
        std::fs::create_dir_all(cache.parent().unwrap_or(FsPath::new("."))).map_err(|e| e.to_string())?;
        run("git", &["clone", "--depth", "1", REGISTRY_URL, &cache.display().to_string()])?;
    } else {
        let _ = run("git", &["-C", &cache.display().to_string(), "pull", "--ff-only"]);
    }
    let manifests = scan_manifests(cache);
    *state.manifests.lock().unwrap() = manifests;
    *state.last_refresh.lock().unwrap() = SystemTime::now();
    Ok(())
}

fn maybe_refresh(state: &AppState) {
    let due = match state.last_refresh.lock() {
        Ok(ts) => ts.elapsed().map(|d| d >= REFRESH_INTERVAL).unwrap_or(true),
        Err(_) => true,
    };
    if due {
        let _ = refresh_registry(state);
    }
}

#[derive(Serialize)]
struct CategoryCount {
    name: String,
    count: usize,
}

#[derive(Serialize)]
struct LxsCard {
    name: String,
    version: String,
    category: String,
    status: String,
    publisher: String,
    summary: String,
    runtime: String,
    targets: Vec<String>,
    source: String,
    commit: String,
    docs_available: bool,
}

#[derive(Serialize)]
struct LxsDocs {
    files: Vec<String>,
    has_openapi: bool,
    index: String,
    api: String,
    examples: String,
    changelog: String,
    gotchas: String,
}

#[derive(Serialize)]
struct LxsDetail {
    name: String,
    domain: String,
    version: String,
    category: String,
    status: String,
    publisher: String,
    summary: String,
    targets: Vec<String>,
    contract: Contract,
    runtime: Runtime,
    provenance: Provenance,
    release: Vec<String>,
    versions: Vec<String>,
    docs: Option<LxsDocs>,
}

fn load_docs(cache: &FsPath, name: &str, version: &str) -> Option<LxsDocs> {
    let dir = cache.join(name).join(version).join("docs");
    if !dir.is_dir() {
        return None;
    }
    let read = |f: &str| std::fs::read_to_string(dir.join(f)).unwrap_or_default();
    let mut files: Vec<String> = std::fs::read_dir(&dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    files.sort();
    Some(LxsDocs {
        has_openapi: dir.join("openapi.json").is_file(),
        index: read("README.md"),
        api: read("api.md"),
        examples: read("examples.sh"),
        changelog: read("changelog.md"),
        gotchas: read("gotchas.md"),
        files,
    })
}

fn latest_by_name(manifests: &[LxsManifest]) -> HashMap<String, &LxsManifest> {
    let mut latest: HashMap<String, &LxsManifest> = HashMap::new();
    for m in manifests {
        let entry = latest.entry(m.name.clone()).or_insert(m);
        if semver_gt(&m.version, &entry.version) {
            *entry = m;
        }
    }
    latest
}

fn semver_gt(a: &str, b: &str) -> bool {
    let pa = parse_semver(a);
    let pb = parse_semver(b);
    (pa.0, pa.1, pa.2) > (pb.0, pb.1, pb.2)
}

fn parse_semver(v: &str) -> (u64, u64, u64) {
    let mut it = v.trim_start_matches('v').split('.').map(|p| p.parse::<u64>().unwrap_or(0));
    (it.next().unwrap_or(0), it.next().unwrap_or(0), it.next().unwrap_or(0))
}

async fn api_categories(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    maybe_refresh(&state);
    let manifests = state.manifests.lock().unwrap().clone();
    let latest = latest_by_name(&manifests);
    let mut counts: HashMap<String, usize> = HashMap::new();
    for m in latest.values() {
        let cat = if m.category.is_empty() { "Uncategorized".to_string() } else { m.category.clone() };
        *counts.entry(cat).or_insert(0) += 1;
    }
    let mut list: Vec<CategoryCount> = counts.into_iter().map(|(name, count)| CategoryCount { name, count }).collect();
    list.sort_by(|a, b| b.count.cmp(&a.count).then(a.name.cmp(&b.name)));
    axum::Json(serde_json::json!({ "categories": list }))
}

async fn api_list_lxs(State(state): State<AppState>, Query(query): Query<HashMap<String, String>>) -> axum::Json<serde_json::Value> {
    maybe_refresh(&state);
    let manifests = state.manifests.lock().unwrap().clone();
    let latest = latest_by_name(&manifests);
    let category = query.get("category").cloned().unwrap_or_default();
    let q = query.get("q").cloned().unwrap_or_default().to_lowercase();
    let mut cards: Vec<LxsCard> = latest
        .values()
        .filter(|m| {
            let cat_ok = category.is_empty() || m.category.eq_ignore_ascii_case(&category);
            let q_ok = q.is_empty() || m.name.to_lowercase().contains(&q) || m.summary.to_lowercase().contains(&q) || m.domain.to_lowercase().contains(&q);
            cat_ok && q_ok
        })
        .map(|m| LxsCard {
            name: m.name.clone(),
            version: m.version.clone(),
            category: if m.category.is_empty() { "Uncategorized".to_string() } else { m.category.clone() },
            status: m.status.clone(),
            publisher: m.publisher.clone(),
            summary: m.summary.clone(),
            runtime: m.runtime.base.clone(),
            targets: m.targets.clone(),
            source: m.provenance.source.clone(),
            commit: m.provenance.commit.clone(),
            docs_available: state.cache.join(&m.name).join(&m.version).join("docs").is_dir(),
        })
        .collect();
    cards.sort_by(|a, b| a.name.cmp(&b.name));
    axum::Json(serde_json::json!({ "lxs": cards, "count": cards.len() }))
}

async fn api_lxs_detail(State(state): State<AppState>, Path(name): Path<String>) -> axum::response::Response {
    maybe_refresh(&state);
    let manifests = state.manifests.lock().unwrap().clone();
    let mut versions: Vec<&LxsManifest> = manifests.iter().filter(|m| m.name == name).collect();
    if versions.is_empty() {
        return axum::response::IntoResponse::into_response(axum::http::StatusCode::NOT_FOUND);
    }
    versions.sort_by(|a, b| {
        if semver_gt(&a.version, &b.version) {
            std::cmp::Ordering::Less
        } else if semver_gt(&b.version, &a.version) {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });
    let latest = versions[0];
    let release: Vec<String> = latest.release.iter().cloned().collect();
    let all_versions: Vec<String> = manifests.iter().filter(|m| m.name == name).map(|m| m.version.clone()).collect();
    let detail = LxsDetail {
        name: latest.name.clone(),
        domain: latest.domain.clone(),
        version: latest.version.clone(),
        category: latest.category.clone(),
        status: latest.status.clone(),
        publisher: latest.publisher.clone(),
        summary: latest.summary.clone(),
        targets: latest.targets.clone(),
        contract: latest.contract.clone(),
        runtime: latest.runtime.clone(),
        provenance: latest.provenance.clone(),
        release,
        versions: all_versions,
        docs: load_docs(&state.cache, &name, &latest.version),
    };
    axum::response::IntoResponse::into_response(axum::Json(detail))
}

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
    let port: u16 = std::env::var("SERVER_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .or_else(|| std::env::var("PORT").ok().and_then(|p| p.parse().ok()))
        .unwrap_or(8261);
    let cache = PathBuf::from(
        std::env::var("LXS_REGISTRY_CACHE").ok().filter(|v| !v.is_empty()).unwrap_or_else(|| "/var/lib/lxs-registry".to_string()),
    );
    let state = AppState {
        cache,
        manifests: Arc::new(Mutex::new(Vec::new())),
        last_refresh: Arc::new(Mutex::new(SystemTime::UNIX_EPOCH)),
    };
    let _ = refresh_registry(&state);
    let app = Router::new()
        .route("/", get(render_app_to_stream(|| view! { <App route=Route::Browse name=String::new() /> })))
        .route("/lxs/:name", get(render_detail))
        .route("/api/lxs", get(api_list_lxs))
        .route("/api/lxs/categories", get(api_categories))
        .route("/api/lxs/:name", get(api_lxs_detail))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await.unwrap();
    println!("[registry-frontend] listening on :{port}");
    axum::serve(listener, app).await.unwrap();
}
