use actix_web::{get, web, App, HttpServer, HttpResponse};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

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
    artifacts: HashMap<String, serde_json::Value>,
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

struct AppState {
    cache: PathBuf,
    manifests: Mutex<Vec<LxsManifest>>,
    last_refresh: Mutex<SystemTime>,
}

fn run(cmd: &str, args: &[&str]) -> Result<(), String> {
    let out = Command::new(cmd).args(args).output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn scan_manifests(cache: &Path) -> Vec<LxsManifest> {
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
        std::fs::create_dir_all(cache.parent().unwrap_or(Path::new("."))).map_err(|e| e.to_string())?;
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

#[get("/api/lxs/categories")]
async fn categories(state: web::Data<AppState>) -> HttpResponse {
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
    HttpResponse::Ok().json(serde_json::json!({ "categories": list }))
}

#[get("/api/lxs")]
async fn list_lxs(state: web::Data<AppState>, query: web::Query<HashMap<String, String>>) -> HttpResponse {
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
        })
        .collect();
    cards.sort_by(|a, b| a.name.cmp(&b.name));
    HttpResponse::Ok().json(serde_json::json!({ "lxs": cards, "count": cards.len() }))
}

#[get("/api/lxs/{name}")]
async fn detail(state: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    maybe_refresh(&state);
    let name = path.into_inner();
    let manifests = state.manifests.lock().unwrap().clone();
    let mut versions: Vec<&LxsManifest> = manifests.iter().filter(|m| m.name == name).collect();
    if versions.is_empty() {
        return HttpResponse::NotFound().json(serde_json::json!({ "error": format!("LXS {name} not found") }));
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
    };
    HttpResponse::Ok().json(detail)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let cache = PathBuf::from(std::env::var("LXS_REGISTRY_CACHE").unwrap_or_else(|_| "/var/lib/lxs-registry".to_string()));
    let state = web::Data::new(AppState {
        cache,
        manifests: Mutex::new(Vec::new()),
        last_refresh: Mutex::new(SystemTime::UNIX_EPOCH),
    });
    if let Err(e) = refresh_registry(&state) {
        eprintln!("[registry] initial refresh failed: {e}");
    } else {
        println!("[registry] loaded {} LXS manifests", state.manifests.lock().unwrap().len());
    }
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8260);
    println!("[registry] listening on :{port}");
    HttpServer::new(move || App::new().app_data(state.clone()).service(categories).service(list_lxs).service(detail))
        .bind(("0.0.0.0", port))?
        .run()
        .await
}
