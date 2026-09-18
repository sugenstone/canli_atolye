//! Faz 1 integration testleri: login → session → RBAC akışı gerçek SQLite üzerinde.
//! Her test kendi geçici veritabanını kurar (migrate + seed).

use canli_atolye_backend::api::state::AppState;
use canli_atolye_backend::api::router;
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
        let db_path = std::env::temp_dir().join(format!(
            "atolye-test-{}.db",
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

        Self {
            state: AppState::new(
            pool,
            config,
            std::sync::Arc::new(LocalFileStorage::new(
                std::env::temp_dir().join(format!("atolye-st-{}", uuid::Uuid::new_v4().simple())),
            )),
        ),
            db_path,
        }
    }

    async fn request(&self, method: &str, uri: &str, cookie: Option<&str>, body: Option<Value>) -> (StatusCode, Option<String>, Value) {
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
        let set_cookie = response
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let json: Value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(Value::Null)
        };
        (status, set_cookie, json)
    }

    fn extract_session_cookie(set_cookie: &str) -> String {
        set_cookie
            .split(';')
            .next()
            .expect("cookie çifti")
            .to_string()
    }
}

#[tokio::test]
async fn login_with_valid_credentials_sets_http_only_cookie() {
    let app = TestApp::new().await;

    let (status, set_cookie, _) = app
        .request(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(json!({ "email": "admin@test.local", "password": "admin-pass-123" })),
        )
        .await;

    assert_eq!(status, StatusCode::OK, "geçerli girişte 200 beklenir");
    let set_cookie = set_cookie.expect("Set-Cookie header");
    assert!(set_cookie.contains("HttpOnly"), "cookie HttpOnly olmalı: {set_cookie}");
    assert!(set_cookie.contains("atolye_session="));
}

#[tokio::test]
async fn login_with_wrong_password_is_unauthorized() {
    let app = TestApp::new().await;

    let (status, _, body) = app
        .request(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(json!({ "email": "admin@test.local", "password": "wrong" })),
        )
        .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], "AUTH_INVALID_CREDENTIALS");
}

#[tokio::test]
async fn me_endpoint_requires_and_accepts_session() {
    let app = TestApp::new().await;

    // Oturumsuz → 401
    let (status, _, _) = app.request("GET", "/api/v1/auth/me", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Login → cookie
    let (_, set_cookie, _) = app
        .request(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(json!({ "email": "admin@test.local", "password": "admin-pass-123" })),
        )
        .await;
    let cookie = TestApp::extract_session_cookie(&set_cookie.unwrap());

    // Cookie ile → kullanıcı + workspace
    let (status, _, me) = app.request("GET", "/api/v1/auth/me", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["user"]["email"], "admin@test.local");
    assert_eq!(me["user"]["role"], "ADMIN");
    assert_eq!(me["workspace"]["slug"], "test-atolye");
    assert!(me["user"].get("password_hash").is_none(), "hash sızmamalı");
}

#[tokio::test]
async fn logout_invalidates_session() {
    let app = TestApp::new().await;

    let (_, set_cookie, _) = app
        .request(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(json!({ "email": "admin@test.local", "password": "admin-pass-123" })),
        )
        .await;
    let cookie = TestApp::extract_session_cookie(&set_cookie.unwrap());

    let (status, _, _) = app.request("POST", "/api/v1/auth/logout", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);

    // Aynı cookie artık geçersiz
    let (status, _, _) = app.request("GET", "/api/v1/auth/me", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn rbac_worker_cannot_create_projects_or_users() {
    let app = TestApp::new().await;

    // Admin login
    let (_, set_cookie, _) = app
        .request(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(json!({ "email": "admin@test.local", "password": "admin-pass-123" })),
        )
        .await;
    let admin_cookie = TestApp::extract_session_cookie(&set_cookie.unwrap());

    // Admin WORKER kullanıcı oluştur
    let workspace_id = {
        let (_, _, me) = app.request("GET", "/api/v1/auth/me", Some(&admin_cookie), None).await;
        me["workspace"]["id"].as_str().unwrap().to_string()
    };
    let (status, _, worker) = app
        .request(
            "POST",
            &format!("/api/v1/workspaces/{workspace_id}/users"),
            Some(&admin_cookie),
            Some(json!({
                "email": "mehmet@test.local",
                "password": "worker-pass-123",
                "full_name": "Mehmet",
                "role": "WORKER"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "admin kullanıcı oluşturabilmeli");

    let _ = worker;

    // Worker login
    let (status, set_cookie, _) = app
        .request(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(json!({ "email": "mehmet@test.local", "password": "worker-pass-123" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let worker_cookie = TestApp::extract_session_cookie(&set_cookie.unwrap());

    // WORKER proje oluşturamaz → 403
    let (status, _, body) = app
        .request(
            "POST",
            "/api/v1/projects",
            Some(&worker_cookie),
            Some(json!({ "name": "Gardenia", "code": "GRD" })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "AUTH_FORBIDDEN");

    // WORKER proje listesi görebilir → 200
    let (status, _, _) = app.request("GET", "/api/v1/projects", Some(&worker_cookie), None).await;
    assert_eq!(status, StatusCode::OK);

    // WORKER başka kullanıcı oluşturamaz → 403
    let (status, _, _) = app
        .request(
            "POST",
            &format!("/api/v1/workspaces/{workspace_id}/users"),
            Some(&worker_cookie),
            Some(json!({
                "email": "x@test.local",
                "password": "password123",
                "full_name": "X",
                "role": "VIEWER"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_project_crud_roundtrip() {
    let app = TestApp::new().await;

    let (_, set_cookie, _) = app
        .request(
            "POST",
            "/api/v1/auth/login",
            None,
            Some(json!({ "email": "admin@test.local", "password": "admin-pass-123" })),
        )
        .await;
    let cookie = TestApp::extract_session_cookie(&set_cookie.unwrap());

    // Oluştur
    let (status, _, project) = app
        .request(
            "POST",
            "/api/v1/projects",
            Some(&cookie),
            Some(json!({ "name": "Gardenia", "code": "grd", "description": "Demo projesi" })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(project["status"], "DRAFT");
    assert_eq!(project["code"], "GRD", "kod büyük harfe normalize edilir");
    let project_id = project["id"].as_str().unwrap().to_string();

    // Listele
    let (status, _, list) = app.request("GET", "/api/v1/projects", Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);

    // Güncelle
    let (status, _, updated) = app
        .request(
            "PATCH",
            &format!("/api/v1/projects/{project_id}"),
            Some(&cookie),
            Some(json!({ "status": "ACTIVE" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["status"], "ACTIVE");

    // Durum filtresi
    let (status, _, active) = app
        .request("GET", "/api/v1/projects?status=ACTIVE", Some(&cookie), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(active.as_array().unwrap().len(), 1);

    // Arşivle → listeden düşer
    let (status, _, _) = app
        .request("POST", &format!("/api/v1/projects/{project_id}/archive"), Some(&cookie), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, _, list) = app.request("GET", "/api/v1/projects", Some(&cookie), None).await;
    assert_eq!(list.as_array().unwrap().len(), 0, "arşivlenen proje listeden düşmeli");
}
