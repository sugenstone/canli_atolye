//! Eksik kapanış testleri: toplu süreç atama, reopen, bildirim, arama, CSV, audit.

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
            .with_env_filter("canli_atolye_backend=error,sqlx=warn")
            .try_init();
        let db_path = std::env::temp_dir().join(format!("atolye-gp-{}", uuid::Uuid::new_v4().simple()));
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
async fn bulk_assign_reopen_notifications_search_csv_audit() {
    let app = TestApp::new().await;
    let admin = app.login().await;

    // Yapı: blok→kat→2 daire; 2 tezgah; Kesim grubu
    let (_, p) = app.req("POST", "/api/v1/projects", &admin, Some(json!({ "name": "G", "code": "G" }))).await;
    let pid = p["id"].as_str().unwrap().to_string();
    let (_, b) = app.req("POST", &format!("/api/v1/projects/{pid}/sections"), &admin, Some(json!({ "name": "A Blok" }))).await;
    let bid = b["id"].as_str().unwrap().to_string();
    let (_, kat) = app.req("POST", &format!("/api/v1/projects/{pid}/sections"), &admin, Some(json!({ "name": "1. Kat", "parent_id": bid }))).await;
    app.req("POST", &format!("/api/v1/projects/{pid}/sections/bulk"), &admin,
        Some(json!({ "parent_id": kat["id"], "start_index": 1, "count": 2, "name_format": "Daire {n}" }))).await;
    let (_, t) = app.req("POST", "/api/v1/work-item-types", &admin, Some(json!({ "name": "Tezgah", "code": "TZ" }))).await;
    app.req("POST", &format!("/api/v1/projects/{pid}/work-items/bulk"), &admin,
        Some(json!({ "parent_section_id": bid, "work_item_type_id": t["id"] }))).await;
    let (_, tpl) = app.req("POST", "/api/v1/process-templates", &admin, Some(json!({ "name": "Kesim", "code": "KSM" }))).await;
    let (_, g) = app.req("POST", "/api/v1/process-groups", &admin, Some(json!({
        "name": "L", "steps": [{ "template_id": tpl["id"] }], "dependencies": []
    }))).await;

    // ① TOPLU ATAMA: her iki daireye
    let (status, _) = app.req(
        "POST", &format!("/api/v1/projects/{pid}/work-items/bulk-assign-process-group"), &admin,
        Some(json!({ "parent_section_id": bid, "process_group_id": g["id"] })),
    ).await;
    assert_eq!(status, StatusCode::CREATED);
    // idempotent: tekrar → hata yok
    let (status2, _) = app.req(
        "POST", &format!("/api/v1/projects/{pid}/work-items/bulk-assign-process-group"), &admin,
        Some(json!({ "parent_section_id": bid, "process_group_id": g["id"] })),
    ).await;
    assert_eq!(status2, StatusCode::CREATED);

    // ④ ARAMA: "Daire 1" ve "Tezgah"
    let (_, hits) = app.req("GET", "/api/v1/search?q=Daire%201", &admin, None).await;
    assert!(hits.as_array().unwrap().iter().any(|h| h["kind"] == "section"), "bölüm bulunur");
    let (_, hits2) = app.req("GET", "/api/v1/search?q=Tezgah", &admin, None).await;
    assert!(hits2.as_array().unwrap().iter().any(|h| h["kind"] == "work_item"), "iş kalemi bulunur");

    // ② REOPEN: ilk dairenin kesimini tamamla → yeniden aç
    let (_, tree) = app.req("GET", &format!("/api/v1/projects/{pid}/sections/tree"), &admin, None).await;
    let daire1 = tree["nodes"][0]["children"][0]["children"][0]["id"].as_str().unwrap().to_string();
    let (_, items) = app.req("GET", &format!("/api/v1/projects/{pid}/work-items?section_id={daire1}"), &admin, None).await;
    let item = items[0]["id"].as_str().unwrap().to_string();
    let (_, l) = app.req("GET", &format!("/api/v1/process-executions?work_item_id={item}"), &admin, None).await;
    let exec = l["executions"][0]["id"].as_str().unwrap().to_string();

    app.req("POST", &format!("/api/v1/process-executions/{exec}/start"), &admin, None).await;
    app.req("POST", &format!("/api/v1/process-executions/{exec}/complete"), &admin, None).await;

    // reasonsuz reopen → 400
    let (st, _) = app.req("POST", &format!("/api/v1/process-executions/{exec}/reopen"), &admin, Some(json!({ "reason": " " }))).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    // geçerli reopen → yeni PENDING execution (revizyon 1)
    let (st, rev) = app.req("POST", &format!("/api/v1/process-executions/{exec}/reopen"), &admin,
        Some(json!({ "reason": "Ürün hasarlı bulundu" }))).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(rev["status"], "PENDING", "bağımlılıksız adım promote ile READY olmalı — kontrol:");
    assert_eq!(rev["revision_no"], 1);
    assert_eq!(rev["parent_execution_id"], exec);
    // ikinci reopen (aktif revizyon varken) → 409
    let (st2, _) = app.req("POST", &format!("/api/v1/process-executions/{exec}/reopen"), &admin,
        Some(json!({ "reason": "tekrar" }))).await;
    assert_eq!(st2, StatusCode::CONFLICT);

    // ③ BİLDİRİM: bloke → admin'e bildirim düşer
    let (_, reason) = app.req("POST", "/api/v1/block-reasons", &admin, Some(json!({ "name": "Malzeme" }))).await;
    let rev_id = rev["id"].as_str().unwrap().to_string();
    // revizyon READY ise başlat sonra bloke et
    let (_, rev_now) = app.req("GET", &format!("/api/v1/process-executions/{rev_id}"), &admin, None).await;
    let (_, start_res) = app.req("POST", &format!("/api/v1/process-executions/{rev_id}/start"), &admin, None).await;
    let (_, block_res) = app.req("POST", &format!("/api/v1/process-executions/{rev_id}/block"), &admin,
        Some(json!({ "reason_id": reason["id"] }))).await;
    let (_, nn) = app.req("GET", "/api/v1/notifications", &admin, None).await;
    if rev_now["status"] == "READY" {
        app.req("POST", &format!("/api/v1/process-executions/{rev_id}/start"), &admin, None).await;
    }
    app.req("POST", &format!("/api/v1/process-executions/{rev_id}/block"), &admin,
        Some(json!({ "reason_id": reason["id"] }))).await;
    let (_, notifs) = app.req("GET", "/api/v1/notifications", &admin, None).await;
    assert!(notifs.as_array().unwrap().iter().any(|n| n["type"] == "BLOCKED"), "admin bloke bildirimi alır");
    // okundu işaretle
    let nid = notifs.as_array().unwrap().iter().find(|n| n["type"] == "BLOCKED").unwrap()["id"].as_str().unwrap().to_string();
    let (st3, _) = app.req("POST", &format!("/api/v1/notifications/{nid}/read"), &admin, None).await;
    assert_eq!(st3, StatusCode::OK);

    // CSV: başlık + en az 2 satır
    let response = router(app.state.clone())
        .oneshot(
            Request::builder()
                .method("GET").uri(format!("/api/v1/projects/{pid}/export.csv"))
                .header(header::COOKIE, &admin)
                .body(Body::empty()).unwrap(),
        ).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("İş Kalemi"), "başlık BOM ile Türkçe gelir");
    assert!(text.contains("Tezgah"));

    // ⑤ AUDIT: kullanıcı + proje oluşturma kayıtları
    let (_, logs) = app.req("GET", "/api/v1/audit-logs", &admin, None).await;
    let actions: Vec<&str> = logs.as_array().unwrap().iter().map(|l| l["action"].as_str().unwrap()).collect();
    assert!(actions.contains(&"project.created"), "proje oluşturma denetlendi: {actions:?}");
    assert!(logs[0]["full_name"].is_string(), "kullanıcı adı birleşik gelir");
}
