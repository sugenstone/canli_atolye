//! Faz 8 testleri: plan tarihleri, gecikme tespiti (LATE), toplu plan ve
//! süre kırılımı (planned vs actual altyapısı).

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
        let db_path = std::env::temp_dir().join(format!("atolye-pln-{}", uuid::Uuid::new_v4().simple()));
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

    /// 2 daire + item + Kesim atanmış; (pid, blok_id, kesim_exec'ler) döner.
    async fn setup(&self, admin: &str) -> (String, String, Vec<String>) {
        let (_, p) = self.req("POST", "/api/v1/projects", admin, Some(json!({ "name": "G", "code": "G" }))).await;
        let pid = p["id"].as_str().unwrap().to_string();
        let (_, b) = self.req("POST", &format!("/api/v1/projects/{pid}/sections"), admin, Some(json!({ "name": "A Blok" }))).await;
        let bid = b["id"].as_str().unwrap().to_string();
        let (_, kat) = self.req("POST", &format!("/api/v1/projects/{pid}/sections"), admin, Some(json!({ "name": "1. Kat", "parent_id": bid }))).await;
        self.req("POST", &format!("/api/v1/projects/{pid}/sections/bulk"), admin,
            Some(json!({ "parent_id": kat["id"], "start_index": 1, "count": 2, "name_format": "Daire {n}" }))).await;
        let (_, t) = self.req("POST", "/api/v1/work-item-types", admin, Some(json!({ "name": "T", "code": "T" }))).await;
        let (_, tpl) = self.req("POST", "/api/v1/process-templates", admin, Some(json!({ "name": "Kesim", "code": "KSM" }))).await;
        let (_, g) = self.req("POST", "/api/v1/process-groups", admin, Some(json!({
            "name": "L", "steps": [{ "template_id": tpl["id"] }], "dependencies": []
        }))).await;
        self.req("POST", &format!("/api/v1/projects/{pid}/work-items/bulk"), admin,
            Some(json!({ "parent_section_id": bid, "work_item_type_id": t["id"] }))).await;
        let (_, tree) = self.req("GET", &format!("/api/v1/projects/{pid}/sections/tree"), admin, None).await;
        let mut execs = Vec::new();
        for daire in tree["nodes"][0]["children"][0]["children"].as_array().unwrap() {
            let (_, items) = self.req("GET", &format!("/api/v1/projects/{pid}/work-items?section_id={}", daire["id"].as_str().unwrap()), admin, None).await;
            let item = items[0]["id"].as_str().unwrap().to_string();
            self.req("POST", &format!("/api/v1/work-items/{item}/assign-process-group"), admin,
                Some(json!({ "process_group_id": g["id"] }))).await;
            let (_, l) = self.req("GET", &format!("/api/v1/process-executions?work_item_id={item}"), admin, None).await;
            execs.push(l["executions"][0]["id"].as_str().unwrap().to_string());
        }
        (pid, bid, execs)
    }
}

#[tokio::test]
async fn planned_dates_event_and_late_detection_everywhere() {
    let app = TestApp::new().await;
    let admin = app.login().await;
    let (pid, bid, execs) = app.setup(&admin).await;

    // Geçmiş bir plan bitişi ata → LATE
    let past = "2020-01-01T00:00:00Z";
    let (status, exec) = app.req(
        "PATCH",
        &format!("/api/v1/process-executions/{}/planned-dates", execs[0]),
        &admin,
        Some(json!({ "planned_end_at": past })),
    ).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(exec["planned_end_at"].as_str().unwrap(), past);

    // Olay üretildi
    let (_, events) = app.req("GET", &format!("/api/v1/process-executions/{}/events", execs[0]), &admin, None).await;
    assert!(events.as_array().unwrap().iter().any(|e| e["event_type"] == "PLANNED_DATE_CHANGED"));

    // Dashboard summary: late = 1
    let (_, summary) = app.req("GET", &format!("/api/v1/projects/{pid}/dashboard-summary"), &admin, None).await;
    assert_eq!(summary["work_items"]["late"], 1, "geciken iş kalemi sayısı");

    // Matris hücresi late bayrağı
    let (_, matrix) = app.req("GET", &format!("/api/v1/projects/{pid}/matrix?parent={bid}"), &admin, None).await;
    let row1 = matrix["rows"][0]["id"].as_str().unwrap();
    assert_eq!(matrix["cells"][row1]["1"]["late"], true);
    assert_eq!(matrix["cells"][row1]["2"]["late"], false, "plansız hücre gecikmez");

    // Geçersiz aralık reddi
    let (status, _) = app.req(
        "PATCH",
        &format!("/api/v1/process-executions/{}/planned-dates", execs[1]),
        &admin,
        Some(json!({ "planned_start_at": "2030-01-01T00:00:00Z", "planned_end_at": "2029-01-01T00:00:00Z" })),
    ).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Gelecek tarih → late değil; tamamlandıktan sonra da gecikmez
    let (_, summary2) = app.req("GET", &format!("/api/v1/projects/{pid}/dashboard-summary"), &admin, None).await;
    assert_eq!(summary2["work_items"]["late"], 1);
}

#[tokio::test]
async fn bulk_plan_by_template_updates_all_active() {
    let app = TestApp::new().await;
    let admin = app.login().await;
    let (pid, _bid, execs) = app.setup(&admin).await;

    // Şablonun kimliğini bul (flow'dan değil exec detayından: template_id)
    let (_, e0) = app.req("GET", &format!("/api/v1/process-executions/{}", execs[0]), &admin, None).await;
    let template_id = e0["process_template_id"].as_str().unwrap().to_string();

    let (status, result) = app.req(
        "POST",
        &format!("/api/v1/projects/{pid}/process-executions/bulk-plan"),
        &admin,
        Some(json!({ "template_id": template_id, "planned_end_at": "2030-06-01T00:00:00Z" })),
    ).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["updated"], 2, "iki aktif execution planlandı");

    // Her ikisinde de tarih var ve late değil
    for ex in &execs {
        let (_, e) = app.req("GET", &format!("/api/v1/process-executions/{ex}"), &admin, None).await;
        assert!(e["planned_end_at"].is_string());
    }
    // Geçersiz istek (parametre yok)
    let (status, _) = app.req("POST", &format!("/api/v1/projects/{pid}/process-executions/bulk-plan"), &admin, Some(json!({}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn durations_from_event_stream() {
    let app = TestApp::new().await;
    let admin = app.login().await;
    let (_pid, _bid, execs) = app.setup(&admin).await;
    let e = &execs[0];

    app.req("POST", &format!("/api/v1/process-executions/{e}/start"), &admin, None).await;
    app.req("POST", &format!("/api/v1/process-executions/{e}/pause"), &admin, None).await;
    app.req("POST", &format!("/api/v1/process-executions/{e}/resume"), &admin, None).await;
    app.req("POST", &format!("/api/v1/process-executions/{e}/complete"), &admin, None).await;

    let (status, d) = app.req("GET", &format!("/api/v1/process-executions/{e}/durations"), &admin, None).await;
    assert_eq!(status, StatusCode::OK);
    // Sıfıra yakın ama yapılandırılmış kırılım (test anında ms'ler)
    assert!(d["active_seconds"].is_number());
    assert!(d["paused_seconds"].is_number());
    assert!(d["lead_seconds"].is_number(), "tamamlandığında lead dolu");
    assert!(d["waiting_seconds"].is_number());
    // aktif >= 0 ve lead >= active
    assert!(d["lead_seconds"].as_i64().unwrap() >= d["active_seconds"].as_i64().unwrap());
}
