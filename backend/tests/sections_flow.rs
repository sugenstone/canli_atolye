//! Faz 2 integration testleri: recursive section ağacı, bulk üretici,
//! klonlama, RBAC ve workspace izolasyonu — gerçek SQLite üzerinde.

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
use tower::ServiceExt;

struct TestApp {
    state: AppState,
    #[allow(dead_code)]
    db_path: std::path::PathBuf,
}

impl TestApp {
    async fn new() -> Self {
        // DB hatalarını görebilmek için log subscriber (bir kez init yeter)
        let _ = tracing_subscriber::fmt()
            .with_env_filter("canli_atolye_backend=debug")
            .try_init();

        let db_path = std::env::temp_dir().join(format!(
            "atolye-sec-{}.db",
            uuid::Uuid::new_v4().simple()
        ));
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
        Self { state: AppState::new(
            pool,
            config,
            std::sync::Arc::new(LocalFileStorage::new(
                std::env::temp_dir().join(format!("atolye-st-{}", uuid::Uuid::new_v4().simple())),
            )),
        ), db_path }
    }

    async fn req(
        &self,
        method: &str,
        uri: &str,
        cookie: Option<&str>,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(cookie) = cookie {
            builder = builder.header(header::COOKIE, cookie);
        }
        let body = match body {
            Some(json) => {
                builder = builder.header(header::CONTENT_TYPE, "application/json");
                Body::from(json.to_string())
            }
            None => Body::empty(),
        };
        let response = router(self.state.clone())
            .oneshot(builder.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(Value::Null)
        };
        (status, json)
    }

    /// Giriş yapıp session cookie string'ini döndürür.
    async fn login(&self, email: &str, password: &str) -> String {
        let response = router(self.state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({ "email": email, "password": password }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let set_cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .expect("Set-Cookie").to_string();
        set_cookie.split(';').next().unwrap().to_string()
    }

    async fn create_worker(&self, admin_cookie: &str) -> String {
        let workspace_id = {
            let (_, me) = self.req("GET", "/api/v1/auth/me", Some(admin_cookie), None).await;
            me["workspace"]["id"].as_str().unwrap().to_string()
        };
        self.req(
            "POST",
            &format!("/api/v1/workspaces/{workspace_id}/users"),
            Some(admin_cookie),
            Some(json!({
                "email": "mehmet@test.local",
                "password": "worker-pass-123",
                "full_name": "Mehmet",
                "role": "WORKER"
            })),
        )
        .await;
        self.login("mehmet@test.local", "worker-pass-123").await
    }

    async fn create_project(&self, admin_cookie: &str) -> String {
        let (status, project) = self
            .req(
                "POST",
                "/api/v1/projects",
                Some(admin_cookie),
                Some(json!({ "name": "Gardenia", "code": "GRD" })),
            )
            .await;
        assert_eq!(status, StatusCode::CREATED);
        project["id"].as_str().unwrap().to_string()
    }
}

#[tokio::test]
async fn builds_recursive_tree_and_bulk_generates_apartments() {
    let app = TestApp::new().await;
    let admin = app.login("admin@test.local", "admin-pass-123").await;
    let project_id = app.create_project(&admin).await;

    // A Blok (root)
    let (status, block) = app
        .req(
            "POST",
            &format!("/api/v1/projects/{project_id}/sections"),
            Some(&admin),
            Some(json!({ "name": "A Blok", "code": "A", "type": "BLOCK" })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let block_id = block["id"].as_str().unwrap().to_string();

    // 1. Kat (A Blok altında)
    let (status, floor) = app
        .req(
            "POST",
            &format!("/api/v1/projects/{project_id}/sections"),
            Some(&admin),
            Some(json!({ "name": "1. Kat", "code": "K01", "type": "FLOOR", "parent_id": block_id })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let floor_id = floor["id"].as_str().unwrap().to_string();

    // Bulk: 1. Kat altında Daire 1..Daire 10
    let (status, created) = app
        .req(
            "POST",
            &format!("/api/v1/projects/{project_id}/sections/bulk"),
            Some(&admin),
            Some(json!({
                "parent_id": floor_id,
                "start_index": 1,
                "count": 10,
                "name_format": "Daire {n}",
                "code_format": "D{n:2}",
                "type": "APARTMENT"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created.as_array().unwrap().len(), 10);
    assert_eq!(created[0]["name"], "Daire 1");
    assert_eq!(created[0]["code"], "D01");
    assert_eq!(created[9]["name"], "Daire 10");

    // Ağaç: A Blok → 1. Kat → 10 daire
    let (status, tree) = app
        .req(
            "GET",
            &format!("/api/v1/projects/{project_id}/sections/tree"),
            Some(&admin),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(tree["total"], 12, "blok + kat + 10 daire");
    assert_eq!(tree["nodes"].as_array().unwrap().len(), 1);
    let block_node = &tree["nodes"][0];
    assert_eq!(block_node["name"], "A Blok");
    assert_eq!(block_node["type"], "BLOCK");
    let floor_node = &block_node["children"][0];
    assert_eq!(floor_node["name"], "1. Kat");
    assert_eq!(floor_node["children"].as_array().unwrap().len(), 10);
    assert_eq!(floor_node["children"][0]["name"], "Daire 1");

    // floor_id döndür (sonraki testler için değil, assert bitti)
    let _ = floor_id;
}

#[tokio::test]
async fn clone_subtree_multiplies_floor_with_apartments() {
    let app = TestApp::new().await;
    let admin = app.login("admin@test.local", "admin-pass-123").await;
    let project_id = app.create_project(&admin).await;

    // Kurulum: A Blok → 1. Kat → 10 daire
    let (_, block) = app
        .req(
            "POST",
            &format!("/api/v1/projects/{project_id}/sections"),
            Some(&admin),
            Some(json!({ "name": "A Blok" })),
        )
        .await;
    let block_id = block["id"].as_str().unwrap().to_string();
    let (_, floor) = app
        .req(
            "POST",
            &format!("/api/v1/projects/{project_id}/sections"),
            Some(&admin),
            Some(json!({ "name": "1. Kat", "parent_id": block_id })),
        )
        .await;
    let floor_id = floor["id"].as_str().unwrap().to_string();
    app.req(
        "POST",
        &format!("/api/v1/projects/{project_id}/sections/bulk"),
        Some(&admin),
        Some(json!({ "parent_id": floor_id, "start_index": 1, "count": 10, "name_format": "Daire {n}" })),
    )
    .await;

    // "Katı çoğalt ×14": adı otomatik türet (1. Kat → 2..15. Kat)
    let (status, created) = app
        .req(
            "POST",
            &format!("/api/v1/sections/{floor_id}/clone"),
            Some(&admin),
            Some(json!({ "count": 14 })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    // 14 kopya × (kat + 10 daire) = 154 satır; BFS sırası:
    // [0] kopya kökü, [1..10] daireler, [11] sonraki kopya kökü...
    assert_eq!(created.as_array().unwrap().len(), 154);
    assert_eq!(created[0]["name"], "2. Kat");
    assert_eq!(created[1]["name"], "Daire 1");
    assert_eq!(created[10]["name"], "Daire 10");
    assert_eq!(created[11]["name"], "3. Kat");

    // Ağaç toplam: blok(1) + kat(1) + 10 daire + 14 kat + 140 daire = 166
    let (_, tree) = app
        .req("GET", &format!("/api/v1/projects/{project_id}/sections/tree"), Some(&admin), None)
        .await;
    assert_eq!(tree["total"], 166);
    let block_node = &tree["nodes"][0];
    assert_eq!(block_node["children"].as_array().unwrap().len(), 15, "1 orijinal + 14 klon kat");
}

#[tokio::test]
async fn section_validation_and_delete_rules() {
    let app = TestApp::new().await;
    let admin = app.login("admin@test.local", "admin-pass-123").await;
    let project_id = app.create_project(&admin).await;

    // Boş ad reddi
    let (status, body) = app
        .req(
            "POST",
            &format!("/api/v1/projects/{project_id}/sections"),
            Some(&admin),
            Some(json!({ "name": "  " })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_FAILED");

    // Format without {n} rejection
    let (status, body) = app
        .req(
            "POST",
            &format!("/api/v1/projects/{project_id}/sections/bulk"),
            Some(&admin),
            Some(json!({ "start_index": 1, "count": 5, "name_format": "Daire" })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_FAILED");

    // Adet limiti
    let (status, _) = app
        .req(
            "POST",
            &format!("/api/v1/projects/{project_id}/sections/bulk"),
            Some(&admin),
            Some(json!({ "start_index": 1, "count": 500, "name_format": "X {n}" })),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Parent restriction: subsection with children cannot be deleted
    let (_, parent) = app
        .req(
            "POST",
            &format!("/api/v1/projects/{project_id}/sections"),
            Some(&admin),
            Some(json!({ "name": "Blok B" })),
        )
        .await;
    let parent_id = parent["id"].as_str().unwrap().to_string();
    let (_, child) = app
        .req(
            "POST",
            &format!("/api/v1/projects/{project_id}/sections"),
            Some(&admin),
            Some(json!({ "name": "Kat 1", "parent_id": parent_id })),
        )
        .await;
    let child_id = child["id"].as_str().unwrap().to_string();

    let (status, body) = app
        .req("DELETE", &format!("/api/v1/sections/{parent_id}"), Some(&admin), None)
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "CONFLICT");

    // First the child, then the parent → both can be deleted (soft)
    let (status, _) = app
        .req("DELETE", &format!("/api/v1/sections/{child_id}"), Some(&admin), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app
        .req("DELETE", &format!("/api/v1/sections/{parent_id}"), Some(&admin), None)
        .await;
    assert_eq!(status, StatusCode::OK);

    // Tree is now empty
    let (_, tree) = app
        .req("GET", &format!("/api/v1/projects/{project_id}/sections/tree"), Some(&admin), None)
        .await;
    assert_eq!(tree["total"], 0);

    // Update: rename
    let (_, sec) = app
        .req(
            "POST",
            &format!("/api/v1/projects/{project_id}/sections"),
            Some(&admin),
            Some(json!({ "name": "Eski Ad" })),
        )
        .await;
    let sec_id = sec["id"].as_str().unwrap().to_string();
    let (status, updated) = app
        .req(
            "PATCH",
            &format!("/api/v1/sections/{sec_id}"),
            Some(&admin),
            Some(json!({ "name": "Yeni Ad", "code": "YN" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["name"], "Yeni Ad");
    assert_eq!(updated["code"], "YN");
}

#[tokio::test]
async fn section_rbac_and_isolation() {
    let app = TestApp::new().await;
    let admin = app.login("admin@test.local", "admin-pass-123").await;
    let project_id = app.create_project(&admin).await;

    // WORKER: can see the tree but cannot create a section
    let worker = app.create_worker(&admin).await;
    let (status, _) = app
        .req(
            "GET",
            &format!("/api/v1/projects/{project_id}/sections/tree"),
            Some(&worker),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK, "worker can see the tree");

    let (status, body) = app
        .req(
            "POST",
            &format!("/api/v1/projects/{project_id}/sections"),
            Some(&worker),
            Some(json!({ "name": "X" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "AUTH_FORBIDDEN");

    // Workspace isolation: a section belonging to a non-existent project → 404
    let fake_project = uuid::Uuid::new_v4().to_string();
    let (status, _) = app
        .req(
            "GET",
            &format!("/api/v1/projects/{fake_project}/sections/tree"),
            Some(&admin),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Cloning a non-existent section → 404
    let fake_section = uuid::Uuid::new_v4().to_string();
    let (status, _) = app
        .req(
            "POST",
            &format!("/api/v1/sections/{fake_section}/clone"),
            Some(&admin),
            Some(json!({ "count": 1 })),
        )
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
