//! Faz 9 testleri: raporlama — süreç performansı, darboğaz, takım performansı.

use canli_atolye_backend::api::router;
use canli_atolye_backend::api::state::AppState;
use canli_atolye_backend::application::services::seed;
use canli_atolye_backend::config::Config;
use canli_atolye_backend::infrastructure::db;
use canli_atolye_backend::infrastructure::storage::LocalFileStorage;
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

struct TestApp {
    state: AppState,
}

impl TestApp {
    async fn new() -> Self {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("canli_atolye_backend=error")
            .try_init();
        let db_path = std::env::temp_dir().join(format!("atolye-rep-{}", uuid::Uuid::new_v4().simple()));
        let url = format!("sqlite://{}?mode=rwc", db_path.display());
        let pool = db::init_pool(&url).await.expect("pool");
        db::migrate(&pool).await.expect("migrate");
        let config = Config {
            database_url: url,
            host: "127.0.0.1".into(),
            port: 0,
            cors_origin: None,
            cookie_secure: false,
            session_ttl_hours: 24 * 7,
            seed_workspace_name: "Test Atölye".into(),
            seed_admin_email: "admin@test.local".into(),
            seed_admin_password: "admin-pass-123".into(),
        };
        seed::ensure_seed(&pool, &config).await.expect("seed");
        let storage = Arc::new(LocalFileStorage::new(
            std::env::temp_dir().join(format!("atolye-st-{}", uuid::Uuid::new_v4().simple())),
        ));
        Self { state: AppState::new(pool, config, storage) }
    }

    async fn req(&self, method: &str, uri: &str, cookie: &str, body: Option<Value>) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(uri).header(header::COOKIE, cookie);
        let body = match body {
            Some(j) => {
                builder = builder.header(header::CONTENT_TYPE, "application/json");
                Body::from(j.to_string())
            }
            None => Body::empty(),
        };
        let response = router(self.state.clone()).oneshot(builder.body(body).unwrap()).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap_or(Value::Null) };
        (status, json)
    }

    async fn login(&self) -> String {
        let response = router(self.state.clone())
            .oneshot(
                Request::builder()
                    .method("POST").uri("/api/v1/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "email": "admin@test.local", "password": "admin-pass-123" }).to_string()))
                    .unwrap(),
            ).await.unwrap();
        response.headers().get(header::SET_COOKIE).and_then(|v| v.to_str().ok()).expect("cookie")
            .split(';').next().unwrap().to_string()
    }
}

#[tokio::test]
async fn reports_process_performance_and_bottleneck() {
    let app = TestApp::new().await;
    let admin = app.login().await;

    // Yapı: blok → kat → 4 daire; Kesim→Montaj lineer grup; her daireye tezgah + süreç
    let (_, p) = app.req("POST", "/api/v1/projects", admin.as_str(), Some(json!({ "name": "G", "code": "G" }))).await;
    let pid = p["id"].as_str().unwrap().to_string();
    let (_, b) = app.req("POST", &format!("/api/v1/projects/{pid}/sections"), &admin, Some(json!({ "name": "A Blok" }))).await;
    let bid = b["id"].as_str().unwrap().to_string();
    let (_, kat) = app.req("POST", &format!("/api/v1/projects/{pid}/sections"), &admin, Some(json!({ "name": "1. Kat", "parent_id": bid }))).await;
    app.req("POST", &format!("/api/v1/projects/{pid}/sections/bulk"), &admin,
        Some(json!({ "parent_id": kat["id"], "start_index": 1, "count": 4, "name_format": "Daire {n}" }))).await;
    let (_, t) = app.req("POST", "/api/v1/work-item-types", &admin, Some(json!({ "name": "T", "code": "T" }))).await;
    let (_, ktpl) = app.req("POST", "/api/v1/process-templates", &admin, Some(json!({ "name": "Kesim", "code": "KSM" }))).await;
    let (_, mtpl) = app.req("POST", "/api/v1/process-templates", &admin, Some(json!({ "name": "Montaj", "code": "MNT" }))).await;
    let (_, g) = app.req("POST", "/api/v1/process-groups", &admin, Some(json!({
        "name": "L", "steps": [{ "template_id": ktpl["id"] }, { "template_id": mtpl["id"] }],
        "dependencies": [{ "step": 1, "depends_on": 0 }]
    }))).await;
    app.req("POST", &format!("/api/v1/projects/{pid}/work-items/bulk"), &admin,
        Some(json!({ "parent_section_id": bid, "work_item_type_id": t["id"] }))).await;

    // item + execution haritası
    let (_, tree) = app.req("GET", &format!("/api/v1/projects/{pid}/sections/tree"), &admin, None).await;
    let mut kesim_execs = Vec::new();
    let mut montaj_execs = Vec::new();
    for daire in tree["nodes"][0]["children"][0]["children"].as_array().unwrap() {
        let (_, items) = app.req("GET", &format!("/api/v1/projects/{pid}/work-items?section_id={}", daire["id"].as_str().unwrap()), &admin, None).await;
        let item = items[0]["id"].as_str().unwrap().to_string();
        app.req("POST", &format!("/api/v1/work-items/{item}/assign-process-group"), &admin,
            Some(json!({ "process_group_id": g["id"] }))).await;
        let (_, l) = app.req("GET", &format!("/api/v1/process-executions?work_item_id={item}"), &admin, None).await;
        kesim_execs.push(l["executions"][0]["id"].as_str().unwrap().to_string());
        montaj_execs.push(l["executions"][1]["id"].as_str().unwrap().to_string());
    }

    // 2 dairede TÜM akış bitmiş olsun (kesim+montaj); 2 dairede kesim bitmiş, montaj READY'de yığılsın
    for idx in 0..2 {
        for e in [&kesim_execs[idx], &montaj_execs[idx]] {
            app.req("POST", &format!("/api/v1/process-executions/{e}/start"), &admin, None).await;
            app.req("POST", &format!("/api/v1/process-executions/{e}/complete"), &admin, None).await;
        }
    }
    for e in &kesim_execs[2..] {
        app.req("POST", &format!("/api/v1/process-executions/{e}/start"), &admin, None).await;
        app.req("POST", &format!("/api/v1/process-executions/{e}/complete"), &admin, None).await;
    }

    let (status, report) = app.req("GET", &format!("/api/v1/projects/{pid}/reports"), &admin, None).await;
    assert_eq!(status, StatusCode::OK);

    // Genel özet
    assert_eq!(report["work_items"]["total"], 4);
    assert_eq!(report["work_items"]["completed"], 2);

    // Süreç performansı: Kesim 2 tamamlanmış, ortalama aktif süre >= 0
    let kesim = report["process_performance"].as_array().unwrap().iter()
        .find(|r| r["template_name"] == "Kesim").unwrap();
    assert_eq!(kesim["completed_count"], 4, "tüm kesimler bitti");
    let montaj_perf = report["process_performance"].as_array().unwrap().iter()
        .find(|r| r["template_name"] == "Montaj").unwrap();
    assert_eq!(montaj_perf["completed_count"], 2);
    assert!(kesim["avg_active_seconds"].is_number());
    assert!(kesim["avg_waiting_seconds"].is_number());
    // Montaj: tamamlanan yok → listede yok (sadece tamamlananlar yer alır)
    

    // Darboğaz: Montaj 2 READY bekliyor (bağımlılık karşılandı)
    let montaj_bn = report["bottlenecks"].as_array().unwrap().iter()
        .find(|r| r["template_name"] == "Montaj").unwrap();
    assert_eq!(montaj_bn["ready_count"], 2);

    // Takım performansı: atanan takım işi yok → boş (ya da atarsak dolu)
    assert!(report["team_performance"].as_array().unwrap().is_empty());

    // İzolasyon: sahte proje
    let fake = uuid::Uuid::new_v4().to_string();
    let (status, _) = app.req("GET", &format!("/api/v1/projects/{fake}/reports"), &admin, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn team_performance_counts_team_completions() {
    let app = TestApp::new().await;
    let admin = app.login().await;

    // Minimal: 1 daire, 1 item, Kesim atayıp takıma ata, tamamla
    let (_, p) = app.req("POST", "/api/v1/projects", &admin, Some(json!({ "name": "G2", "code": "G2" }))).await;
    let pid = p["id"].as_str().unwrap().to_string();
    let (_, s) = app.req("POST", &format!("/api/v1/projects/{pid}/sections"), &admin, Some(json!({ "name": "D1" }))).await;
    let (_, t) = app.req("POST", "/api/v1/work-item-types", &admin, Some(json!({ "name": "T", "code": "T" }))).await;
    let (_, tpl) = app.req("POST", "/api/v1/process-templates", &admin, Some(json!({ "name": "Kesim", "code": "KSM" }))).await;
    let (_, g) = app.req("POST", "/api/v1/process-groups", &admin, Some(json!({
        "name": "L", "steps": [{ "template_id": tpl["id"] }], "dependencies": []
    }))).await;
    let (_, team) = app.req("POST", "/api/v1/teams", &admin, Some(json!({ "name": "Kesim Ekibi" }))).await;

    let (_, i) = app.req("POST", &format!("/api/v1/projects/{pid}/work-items"), &admin,
        Some(json!({ "section_id": s["id"], "work_item_type_id": t["id"] }))).await;
    app.req("POST", &format!("/api/v1/work-items/{}/assign-process-group", i["id"].as_str().unwrap()), &admin,
        Some(json!({ "process_group_id": g["id"] }))).await;
    let (_, l) = app.req("GET", &format!("/api/v1/process-executions?work_item_id={}", i["id"].as_str().unwrap()), &admin, None).await;
    let exec = l["executions"][0]["id"].as_str().unwrap().to_string();

    app.req("POST", &format!("/api/v1/process-executions/{exec}/assign"), &admin,
        Some(json!({ "team_id": team["id"] }))).await;
    app.req("POST", &format!("/api/v1/process-executions/{exec}/start"), &admin, None).await;
    app.req("POST", &format!("/api/v1/process-executions/{exec}/complete"), &admin, None).await;

    let (_, report) = app.req("GET", &format!("/api/v1/projects/{pid}/reports"), &admin, None).await;
    let teams = report["team_performance"].as_array().unwrap();
    assert_eq!(teams.len(), 1);
    assert_eq!(teams[0]["team_name"], "Kesim Ekibi");
    assert_eq!(teams[0]["completed_count"], 1);
}
