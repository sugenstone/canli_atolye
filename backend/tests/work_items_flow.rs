//! Faz 3 integration testleri: iş kalemi tipleri, iş kalemleri, bulk
//! (yaprak bölümlere) ve dinamik özellikler — gerçek SQLite üzerinde.

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
}

impl TestApp {
    async fn new() -> Self {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("canli_atolye_backend=error")
            .try_init();
        let db_path = std::env::temp_dir().join(format!(
            "atolye-wi-{}.db",
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
        ) }
    }

    async fn req(&self, method: &str, uri: &str, cookie: &str, body: Option<Value>) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(uri).header(header::COOKIE, cookie);
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
        let json: Value = if bytes.is_empty() { Value::Null } else { serde_json::from_slice(&bytes).unwrap_or(Value::Null) };
        (status, json)
    }

    async fn login(&self, email: &str, password: &str) -> String {
        let response = router(self.state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "email": email, "password": password }).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        response
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .expect("Set-Cookie")
            .split(';')
            .next()
            .unwrap()
            .to_string()
    }

    async fn create_worker(&self, admin: &str) -> String {
        let workspace_id = {
            let (_, me) = self.req("GET", "/api/v1/auth/me", admin, None).await;
            me["workspace"]["id"].as_str().unwrap().to_string()
        };
        self.req(
            "POST",
            &format!("/api/v1/workspaces/{workspace_id}/users"),
            admin,
            Some(json!({ "email": "mehmet@test.local", "password": "worker-pass-123", "full_name": "Mehmet", "role": "WORKER" })),
        )
        .await;
        self.login("mehmet@test.local", "worker-pass-123").await
    }

    /// Blok → kat → N daire kur, (blok_id, kat_id) döndür.
    async fn setup_structure(&self, admin: &str, apartments: i64) -> (String, String, String) {
        let project_id = {
            let (_, p) = self.req("POST", "/api/v1/projects", admin, Some(json!({ "name": "Gardenia", "code": "GRD" }))).await;
            p["id"].as_str().unwrap().to_string()
        };
        let block_id = {
            let (_, s) = self.req("POST", &format!("/api/v1/projects/{project_id}/sections"), admin, Some(json!({ "name": "A Blok" }))).await;
            s["id"].as_str().unwrap().to_string()
        };
        let floor_id = {
            let (_, s) = self.req("POST", &format!("/api/v1/projects/{project_id}/sections"), admin, Some(json!({ "name": "1. Kat", "parent_id": block_id }))).await;
            s["id"].as_str().unwrap().to_string()
        };
        self.req(
            "POST",
            &format!("/api/v1/projects/{project_id}/sections/bulk"),
            admin,
            Some(json!({ "parent_id": floor_id, "start_index": 1, "count": apartments, "name_format": "Daire {n}" })),
        )
        .await;
        (project_id, block_id, floor_id)
    }
}

#[tokio::test]
async fn types_catalog_and_definitions_admin_only() {
    let app = TestApp::new().await;
    let admin = app.login("admin@test.local", "admin-pass-123").await;
    let worker = app.create_worker(&admin).await;

    // ADMIN tip oluşturur
    let (status, t) = app
        .req("POST", "/api/v1/work-item-types", &admin, Some(json!({ "name": "Mutfak Tezgahı", "code": "mt" })))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(t["code"], "MT", "kod büyük harfe normalize edilir");

    // WORKER tip oluşturamaz
    let (status, body) = app
        .req("POST", "/api/v1/work-item-types", &worker, Some(json!({ "name": "X", "code": "x" })))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "AUTH_FORBIDDEN");

    // WORKER listeyi görebilir
    let (status, list) = app.req("GET", "/api/v1/work-item-types", &worker, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);

    // Özellik tanımı: SELECT options'lı + NUMBER'lı
    let (status, def_sel) = app
        .req("POST", "/api/v1/property-definitions", &admin, Some(json!({
            "name": "Malzeme", "key": "malzeme", "data_type": "SELECT",
            "options": ["Lamar Moon White", "Arma Bej", "Quartz"]
        })))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let (status, def_num) = app
        .req("POST", "/api/v1/property-definitions", &admin, Some(json!({
            "name": "Metraj", "data_type": "NUMBER", "unit": "m", "required": true
        })))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(def_num["key"], "metraj", "key boşsa isimden türetilir");

    // SELECT options'sız reddedilir
    let (status, _) = app
        .req("POST", "/api/v1/property-definitions", &admin, Some(json!({ "name": "Hatalı", "data_type": "SELECT" })))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let _ = (def_sel, def_num);
}

#[tokio::test]
async fn bulk_creates_items_on_all_leaf_sections_idempotently() {
    let app = TestApp::new().await;
    let admin = app.login("admin@test.local", "admin-pass-123").await;
    let (project_id, block_id, floor_id) = app.setup_structure(&admin, 10).await;

    let type_id = {
        let (_, t) = app.req("POST", "/api/v1/work-item-types", &admin, Some(json!({ "name": "Mutfak Tezgahı", "code": "MT" }))).await;
        t["id"].as_str().unwrap().to_string()
    };

    // Bulk: A Blok altındaki tüm yapraklara (10 daire) tezgah ekle
    let (status, created) = app
        .req(
            "POST",
            &format!("/api/v1/projects/{project_id}/work-items/bulk"),
            &admin,
            Some(json!({ "parent_section_id": block_id, "work_item_type_id": type_id })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED, "bulk yapraklara ekler");
    assert_eq!(created.as_array().unwrap().len(), 10, "10 daire = 10 iş kalemi");
    assert_eq!(created[0]["name"], "Mutfak Tezgahı", "isim tip adından gelir");
    assert_eq!(created[0]["status"], "PENDING");
    assert_eq!(created[0]["priority"], "NORMAL");

    // Kat bölümü yaprak değil → blok seçilince kat'a item EKLENMEZ (sadece daireler)
    let section_check: Vec<&str> = created
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v["section_id"].as_str())
        .collect();
    assert!(!section_check.contains(&floor_id.as_str()), "ara (yaprak olmayan) bölümlere eklenmez");

    // Tekrar çalıştır → hepsi zaten var, 0 yeni
    let (status, again) = app
        .req(
            "POST",
            &format!("/api/v1/projects/{project_id}/work-items/bulk"),
            &admin,
            Some(json!({ "parent_section_id": block_id, "work_item_type_id": type_id })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(again.as_array().unwrap().len(), 0, "idempotent: mevcut olanlar atlanır");
}

#[tokio::test]
async fn property_values_validate_by_data_type() {
    let app = TestApp::new().await;
    let admin = app.login("admin@test.local", "admin-pass-123").await;
    let (project_id, _block_id, _floor_id) = app.setup_structure(&admin, 2).await;

    let type_id = {
        let (_, t) = app.req("POST", "/api/v1/work-item-types", &admin, Some(json!({ "name": "Mutfak Tezgahı", "code": "MT" }))).await;
        t["id"].as_str().unwrap().to_string()
    };
    // Daire 1'in section id'si
    let section_id = {
        let (_, tree) = app.req("GET", &format!("/api/v1/projects/{project_id}/sections/tree"), &admin, None).await;
        let floor = &tree["nodes"][0]["children"][0];
        floor["children"][0]["id"].as_str().unwrap().to_string()
    };

    let item_id = {
        let (_, item) = app
            .req(
                "POST",
                &format!("/api/v1/projects/{project_id}/work-items"),
                &admin,
                Some(json!({ "section_id": section_id, "work_item_type_id": type_id, "priority": "HIGH" })),
            )
            .await;
        assert_eq!(item["priority"], "HIGH");
        item["id"].as_str().unwrap().to_string()
    };

    let (def_sel, def_num) = {
        let (_, s) = app.req("POST", "/api/v1/property-definitions", &admin, Some(json!({
            "name": "Malzeme", "key": "malzeme", "data_type": "SELECT",
            "options": ["Lamar Moon White", "Arma Bej"]
        })))
        .await;
        let (_, n) = app.req("POST", "/api/v1/property-definitions", &admin, Some(json!({
            "name": "Metraj", "key": "metraj", "data_type": "NUMBER", "unit": "m"
        })))
        .await;
        (s["id"].as_str().unwrap().to_string(), n["id"].as_str().unwrap().to_string())
    };

    // Geçerli değerler
    let (status, _) = app
        .req(
            "PUT",
            &format!("/api/v1/work-items/{item_id}/property-values"),
            &admin,
            Some(json!([
                { "property_definition_id": def_sel, "value": "Lamar Moon White" },
                { "property_definition_id": def_num, "value": 5.82 }
            ])),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    // Detay: değerler tanımla birlikte döner
    let (status, detail) = app.req("GET", &format!("/api/v1/work-items/{item_id}"), &admin, None).await;
    assert_eq!(status, StatusCode::OK);
    let props = detail["properties"].as_array().unwrap();
    assert_eq!(props.len(), 2);
    let metraj = props.iter().find(|p| p["definition_key"] == "metraj").unwrap();
    assert_eq!(metraj["value_number"], 5.82);
    let malzeme = props.iter().find(|p| p["definition_key"] == "malzeme").unwrap();
    assert_eq!(malzeme["value_text"], "Lamar Moon White");

    // Geçersiz SELECT → 400
    let (status, body) = app
        .req(
            "PUT",
            &format!("/api/v1/work-items/{item_id}/property-values"),
            &admin,
            Some(json!([{ "property_definition_id": def_sel, "value": "Bilinmeyen Taş" }])),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_FAILED");

    // Geçersiz NUMBER → 400
    let (status, _) = app
        .req(
            "PUT",
            &format!("/api/v1/work-items/{item_id}/property-values"),
            &admin,
            Some(json!([{ "property_definition_id": def_num, "value": "beş nokta seksen iki" }])),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // WORKER değer atayamaz
    let worker = app.create_worker(&admin).await;
    let (status, _) = app
        .req(
            "PUT",
            &format!("/api/v1/work-items/{item_id}/property-values"),
            &worker,
            Some(json!([{ "property_definition_id": def_num, "value": 1.0 }])),
        )
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // WORKER görebilir
    let (status, _) = app.req("GET", &format!("/api/v1/work-items/{item_id}"), &worker, None).await;
    assert_eq!(status, StatusCode::OK);

    // null ile temizleme → detayda metraj kaybolur
    let (status, _) = app
        .req(
            "PUT",
            &format!("/api/v1/work-items/{item_id}/property-values"),
            &admin,
            Some(json!([{ "property_definition_id": def_num, "value": null }])),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, detail) = app.req("GET", &format!("/api/v1/work-items/{item_id}"), &admin, None).await;
    let metraj = detail["properties"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["definition_key"] == "metraj")
        .unwrap();
    assert!(metraj["value_number"].is_null(), "null değer alanı temizler");
}

#[tokio::test]
async fn work_item_update_delete_and_isolation() {
    let app = TestApp::new().await;
    let admin = app.login("admin@test.local", "admin-pass-123").await;
    let (project_id, _b, _f) = app.setup_structure(&admin, 1).await;

    let type_id = {
        let (_, t) = app.req("POST", "/api/v1/work-item-types", &admin, Some(json!({ "name": "Banyo Tezgahı", "code": "BT" }))).await;
        t["id"].as_str().unwrap().to_string()
    };
    let section_id = {
        let (_, tree) = app.req("GET", &format!("/api/v1/projects/{project_id}/sections/tree"), &admin, None).await;
        tree["nodes"][0]["children"][0]["children"][0]["id"].as_str().unwrap().to_string()
    };
    let item_id = {
        let (_, i) = app
            .req("POST", &format!("/api/v1/projects/{project_id}/work-items"), &admin, Some(json!({ "section_id": section_id, "work_item_type_id": type_id })))
            .await;
        i["id"].as_str().unwrap().to_string()
    };

    // Güncelle
    let (status, updated) = app
        .req("PATCH", &format!("/api/v1/work-items/{item_id}"), &admin, Some(json!({ "name": "Banyo Tezgahı (Premium)", "priority": "URGENT" })))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["name"], "Banyo Tezgahı (Premium)");
    assert_eq!(updated["priority"], "URGENT");

    // Section listesi
    let (status, list) = app
        .req("GET", &format!("/api/v1/projects/{project_id}/work-items?section_id={section_id}"), &admin, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);

    // Olmayan tip → 404
    let fake_type = uuid::Uuid::new_v4().to_string();
    let (status, _) = app
        .req("POST", &format!("/api/v1/projects/{project_id}/work-items"), &admin, Some(json!({ "section_id": section_id, "work_item_type_id": fake_type })))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Sil → listeden düşer
    let (status, _) = app.req("DELETE", &format!("/api/v1/work-items/{item_id}"), &admin, None).await;
    assert_eq!(status, StatusCode::OK);
    let (_, list) = app
        .req("GET", &format!("/api/v1/projects/{project_id}/work-items?section_id={section_id}"), &admin, None)
        .await;
    assert_eq!(list.as_array().unwrap().len(), 0);
}
