//! Faz 7 testleri: my-work — kullanıcı ve takım atamalı işlerin görünürlüğü.

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
        let db_path = std::env::temp_dir().join(format!("atolye-mw-{}.db", uuid::Uuid::new_v4().simple()));
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

    async fn login(&self, email: &str, password: &str) -> String {
        let response = router(self.state.clone())
            .oneshot(
                Request::builder()
                    .method("POST").uri("/api/v1/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "email": email, "password": password }).to_string()))
                    .unwrap(),
            ).await.unwrap();
        response.headers().get(header::SET_COOKIE).and_then(|v| v.to_str().ok()).expect("cookie")
            .split(';').next().unwrap().to_string()
    }
}

#[tokio::test]
async fn my_work_lists_user_and_team_assignments_only() {
    let app = TestApp::new().await;
    let admin = app.login("admin@test.local", "admin-pass-123").await;

    // Kullanıcılar: worker1 (kullanıcı ataması + takım üyesi), worker2 (takım dışı)
    let workspace_id = {
        let (_, me) = app.req("GET", "/api/v1/auth/me", &admin, None).await;
        me["workspace"]["id"].as_str().unwrap().to_string()
    };
    let mut users = std::collections::HashMap::new();
    for (email, name) in [("ahmet@test.local", "Ahmet"), ("mehmet@test.local", "Mehmet")] {
        let (_, u) = app.req(
            "POST", &format!("/api/v1/workspaces/{workspace_id}/users"), &admin,
            Some(json!({ "email": email, "password": "worker-pass-123", "full_name": name, "role": "WORKER" })),
        ).await;
        users.insert(name.to_string(), u["id"].as_str().unwrap().to_string());
    }
    // Takım: Kesim Ekibi — Ahmet üye
    let (_, team) = app.req("POST", "/api/v1/teams", &admin, Some(json!({ "name": "Kesim Ekibi" }))).await;
    let team_id = team["id"].as_str().unwrap().to_string();
    app.req("POST", &format!("/api/v1/teams/{team_id}/members/{}", users["Ahmet"]), &admin,
        Some(json!({ "user_id": users["Ahmet"] }))).await;

    let ahmet = app.login("ahmet@test.local", "worker-pass-123").await;
    let mehmet = app.login("mehmet@test.local", "worker-pass-123").await;

    // Yapı: 2 daire, 2 item, her birine Kesim→Montaj ata
    let (_, p) = app.req("POST", "/api/v1/projects", &admin, Some(json!({ "name": "G", "code": "G" }))).await;
    let pid = p["id"].as_str().unwrap().to_string();
    let (_, s1) = app.req("POST", &format!("/api/v1/projects/{pid}/sections"), &admin, Some(json!({ "name": "Daire 1" }))).await;
    let (_, s2) = app.req("POST", &format!("/api/v1/projects/{pid}/sections"), &admin, Some(json!({ "name": "Daire 2" }))).await;
    let (_, t) = app.req("POST", "/api/v1/work-item-types", &admin, Some(json!({ "name": "Tezgah", "code": "TZ" }))).await;
    let (_, ktpl) = app.req("POST", "/api/v1/process-templates", &admin, Some(json!({ "name": "Kesim", "code": "KSM" }))).await;
    let (_, g) = app.req("POST", "/api/v1/process-groups", &admin, Some(json!({
        "name": "L", "steps": [{ "template_id": ktpl["id"] }], "dependencies": []
    }))).await;
    let mut items = Vec::new();
    for s in [&s1, &s2] {
        let (_, i) = app.req("POST", &format!("/api/v1/projects/{pid}/work-items"), &admin,
            Some(json!({ "section_id": s["id"], "work_item_type_id": t["id"] }))).await;
        items.push(i["id"].as_str().unwrap().to_string());
    }
    let mut kesim_execs = Vec::new();
    for it in &items {
        app.req("POST", &format!("/api/v1/work-items/{it}/assign-process-group"), &admin,
            Some(json!({ "process_group_id": g["id"] }))).await;
        let (_, l) = app.req("GET", &format!("/api/v1/process-executions?work_item_id={it}"), &admin, None).await;
        kesim_execs.push(l["executions"][0]["id"].as_str().unwrap().to_string());
    }

    // Daire 1 Kesim → Ahmet'e (kullanıcı); Daire 2 Kesim → takıma
    app.req("POST", &format!("/api/v1/process-executions/{}/assign", kesim_execs[0]), &admin,
        Some(json!({ "user_id": users["Ahmet"] }))).await;
    app.req("POST", &format!("/api/v1/process-executions/{}/assign", kesim_execs[1]), &admin,
        Some(json!({ "team_id": team_id }))).await;

    // Ahmet: İKİ işi de görür (kullanıcı + takım)
    let (status, my) = app.req("GET", "/api/v1/my-work", &ahmet, None).await;
    assert_eq!(status, StatusCode::OK);
    let cards = my.as_array().unwrap();
    assert_eq!(cards.len(), 2, "kullanıcı + takım ataması");
    assert!(cards.iter().any(|c| c["assignment_kind"] == "user"));
    assert!(cards.iter().any(|c| c["assignment_kind"] == "team"));
    assert!(cards[0]["section_path"].as_str().unwrap().contains("Daire"));

    // Ahmet başlatır → my-work'ta IN_PROGRESS en üstte
    app.req("POST", &format!("/api/v1/process-executions/{}/start", kesim_execs[0]), &ahmet, None).await;
    let (_, my2) = app.req("GET", "/api/v1/my-work", &ahmet, None).await;
    assert_eq!(my2[0]["status"], "IN_PROGRESS", "aktif iş en üstte");
    assert!(my2[0]["started_at"].is_string());

    // Mehmet: hiçbir iş görmez
    let (_, my3) = app.req("GET", "/api/v1/my-work", &mehmet, None).await;
    assert_eq!(my3.as_array().unwrap().len(), 0, "atanmamış worker boş liste");

    // Bitirilince listeden düşer
    app.req("POST", &format!("/api/v1/process-executions/{}/complete", kesim_execs[0]), &ahmet, None).await;
    let (_, my4) = app.req("GET", "/api/v1/my-work", &ahmet, None).await;
    assert_eq!(my4.as_array().unwrap().len(), 1, "tamamlanan iş kalkar");
}
