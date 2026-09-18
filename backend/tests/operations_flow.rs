//! Faz 5 integration testleri: nedenli bloke, çözme, notlar ve dosya ekleri.

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
    storage_root: std::path::PathBuf,
}

impl TestApp {
    async fn new() -> Self {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("canli_atolye_backend=error")
            .try_init();
        let tag = uuid::Uuid::new_v4().simple().to_string();
        let db_path = std::env::temp_dir().join(format!("atolye-op-{tag}.db"));
        let storage_root = std::env::temp_dir().join(format!("atolye-up-{tag}"));
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
        let storage = Arc::new(LocalFileStorage::new(&storage_root));
        Self { state: AppState::new(pool, config, storage), storage_root }
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

    /// Multipart dosya yükleme isteği.
    async fn upload(&self, cookie: &str, entity_type: &str, entity_id: &str, file_name: &str, mime: &str, content: Vec<u8>) -> (StatusCode, Value) {
        let boundary = "----testboundary123";
        let mut body = Vec::new();
        for (name, value) in [("entity_type", entity_type.to_string()), ("entity_id", entity_id.to_string())] {
            body.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n").as_bytes());
        }
        body.extend_from_slice(format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{file_name}\"\r\nContent-Type: {mime}\r\n\r\n"
        ).as_bytes());
        body.extend_from_slice(&content);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

        let response = router(self.state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/attachments")
                    .header(header::COOKIE, cookie)
                    .header(header::CONTENT_TYPE, format!("multipart/form-data; boundary={boundary}"))
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
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

    async fn create_worker(&self, admin: &str) -> (String, String) {
        let workspace_id = {
            let (_, me) = self.req("GET", "/api/v1/auth/me", admin, None).await;
            me["workspace"]["id"].as_str().unwrap().to_string()
        };
        let (_, u) = self
            .req(
                "POST",
                &format!("/api/v1/workspaces/{workspace_id}/users"),
                admin,
                Some(json!({ "email": "mehmet@test.local", "password": "worker-pass-123", "full_name": "Mehmet", "role": "WORKER" })),
            )
            .await;
        (u["id"].as_str().unwrap().to_string(), self.login("mehmet@test.local", "worker-pass-123").await)
    }

    /// Kesim→İmalat grubu + item + atama; Kesim execution id döner.
    async fn setup_ready_execution(&self, admin: &str) -> (String, String) {
        let mut template_ids = Vec::new();
        for (name, code) in [("Kesim", "KSM"), ("İmalat", "IML")] {
            let (_, t) = self.req("POST", "/api/v1/process-templates", admin, Some(json!({ "name": name, "code": code }))).await;
            template_ids.push(t["id"].as_str().unwrap().to_string());
        }
        let (_, g) = self
            .req(
                "POST",
                "/api/v1/process-groups",
                admin,
                Some(json!({
                    "name": "Lineer",
                    "steps": template_ids.iter().map(|id| json!({ "template_id": id })).collect::<Vec<_>>(),
                    "dependencies": [{ "step": 1, "depends_on": 0 }]
                })),
            )
            .await;
        let group_id = g["id"].as_str().unwrap().to_string();

        let (_, p) = self.req("POST", "/api/v1/projects", admin, Some(json!({ "name": "G", "code": "G" }))).await;
        let pid = p["id"].as_str().unwrap().to_string();
        let (_, s) = self.req("POST", &format!("/api/v1/projects/{pid}/sections"), admin, Some(json!({ "name": "Daire 1" }))).await;
        let (_, t) = self.req("POST", "/api/v1/work-item-types", admin, Some(json!({ "name": "Tezgah", "code": "TZ" }))).await;
        let (_, i) = self
            .req(
                "POST",
                &format!("/api/v1/projects/{pid}/work-items"),
                admin,
                Some(json!({ "section_id": s["id"], "work_item_type_id": t["id"] })),
            )
            .await;
        let item_id = i["id"].as_str().unwrap().to_string();
        self.req(
            "POST",
            &format!("/api/v1/work-items/{item_id}/assign-process-group"),
            admin,
            Some(json!({ "process_group_id": group_id })),
        )
        .await;
        let (_, list) = self
            .req("GET", &format!("/api/v1/process-executions?work_item_id={item_id}"), admin, None)
            .await;
        let kesim = list["executions"][0]["id"].as_str().unwrap().to_string();
        (item_id, kesim)
    }
}

#[tokio::test]
async fn block_requires_reason_and_unblock_resolves() {
    let app = TestApp::new().await;
    let admin = app.login("admin@test.local", "admin-pass-123").await;
    let (_worker_id, worker) = app.create_worker(&admin).await;
    let (_item, kesim) = app.setup_ready_execution(&admin).await;

    // Neden kataloğu: admin oluşturur, worker görür ama oluşturamaz
    let (status, reason) = app
        .req("POST", "/api/v1/block-reasons", &admin, Some(json!({ "name": "Malzeme eksik" })))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let reason_id = reason["id"].as_str().unwrap().to_string();

    let (status, _) = app.req("POST", "/api/v1/block-reasons", &worker, Some(json!({ "name": "X" }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, list) = app.req("GET", "/api/v1/block-reasons", &worker, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);

    // Reasonsuz bloke → 422 (zorunlu alan)
    let (status, _) = app.req("POST", &format!("/api/v1/process-executions/{kesim}/block"), &admin, Some(json!({}))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

    // Geçersiz reason → 400
    let fake = uuid::Uuid::new_v4().to_string();
    let (status, _) = app
        .req("POST", &format!("/api/v1/process-executions/{kesim}/block"), &admin, Some(json!({ "reason_id": fake })))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Nedenli bloke → BLOCKED + kayıt + olay
    let (status, exec) = app
        .req(
            "POST",
            &format!("/api/v1/process-executions/{kesim}/block"),
            &admin,
            Some(json!({ "reason_id": reason_id, "description": "Lamar stokta yok" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(exec["status"], "BLOCKED");
    assert_eq!(exec["status_before_block"], "READY");

    let (_, blocks) = app.req("GET", &format!("/api/v1/process-executions/{kesim}/blocks"), &admin, None).await;
    assert_eq!(blocks.as_array().unwrap().len(), 1);
    assert_eq!(blocks[0]["resolved_at"], Value::Null, "henüz açık");

    // Olayda not = neden adı
    let (_, events) = app.req("GET", &format!("/api/v1/process-executions/{kesim}/events"), &admin, None).await;
    let blocked = events.as_array().unwrap().iter().find(|e| e["event_type"] == "BLOCKED").unwrap();
    assert_eq!(blocked["note"], "Malzeme eksik");

    // Worker unblock edemez (yalnız ADMIN/PM)
    let (status, _) = app.req("POST", &format!("/api/v1/process-executions/{kesim}/unblock"), &worker, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Admin unblock → READY'e döner + kayıt çözülür
    let (status, exec2) = app
        .req(
            "POST",
            &format!("/api/v1/process-executions/{kesim}/unblock"),
            &admin,
            Some(json!({ "resolution_note": "Malzeme geldi" })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(exec2["status"], "READY", "kaydedilen duruma döner");

    let (_, blocks2) = app.req("GET", &format!("/api/v1/process-executions/{kesim}/blocks"), &admin, None).await;
    assert!(blocks2[0]["resolved_at"].is_string(), "bloke kaydı çözüldü");
    assert_eq!(blocks2[0]["resolution_note"], "Malzeme geldi");
}

#[tokio::test]
async fn notes_are_immutable_events() {
    let app = TestApp::new().await;
    let admin = app.login("admin@test.local", "admin-pass-123").await;
    let (_item, kesim) = app.setup_ready_execution(&admin).await;

    let (status, _) = app
        .req("POST", &format!("/api/v1/process-executions/{kesim}/notes"), &admin, Some(json!({ "note": "Testere değişimi planlandı" })))
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, _) = app
        .req("POST", &format!("/api/v1/process-executions/{kesim}/notes"), &admin, Some(json!({ "note": "   " })))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "boş not reddedilir");

    let (_, events) = app.req("GET", &format!("/api/v1/process-executions/{kesim}/events"), &admin, None).await;
    let note_event = events.as_array().unwrap().iter().find(|e| e["event_type"] == "NOTE_ADDED").unwrap();
    assert_eq!(note_event["note"], "Testere değişimi planlandı");
}

#[tokio::test]
async fn attachment_upload_download_and_validation() {
    let app = TestApp::new().await;
    let admin = app.login("admin@test.local", "admin-pass-123").await;
    let (item_id, _kesim) = app.setup_ready_execution(&admin).await;

    // Geçerli PNG (küçük sahte içerik)
    let content: Vec<u8> = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3, 4];
    let (status, att) = app
        .upload(&admin, "WORK_ITEM", &item_id, "tezgah fotoğrafı.png", "image/png", content.clone())
        .await;
    assert_eq!(status, StatusCode::CREATED, "yükledi: {att}");
    assert_eq!(att["file_name"], "tezgah_fotografi.png", "isim sanitize edilir");
    assert_eq!(att["size"], content.len() as i64);
    let att_id = att["id"].as_str().unwrap().to_string();

    // Liste
    let (status, list) = app
        .req("GET", &format!("/api/v1/attachments?entity_type=WORK_ITEM&entity_id={item_id}"), &admin, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);

    // İndir: içerik birebir
    let response = router(app.state.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/attachments/{att_id}/download"))
                .header(header::COOKIE, &admin)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers().get(header::CONTENT_TYPE).unwrap(), "image/png");
    let downloaded = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(downloaded.to_vec(), content);

    // Diskte gerçekten var
    let key = att["storage_key"].as_str().unwrap();
    assert!(app.storage_root.join(key).exists(), "storage anahtarı diskte");

    // Redler: exe uzantısı, boyut aşımı, geçersiz entity
    let (status, _) = app.upload(&admin, "WORK_ITEM", &item_id, "script.exe", "application/x-msdownload", vec![1]).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let big = vec![0u8; 11 * 1024 * 1024];
    let (status, _) = app.upload(&admin, "WORK_ITEM", &item_id, "buyuk.png", "image/png", big).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let fake_entity = uuid::Uuid::new_v4().to_string();
    let (status, _) = app.upload(&admin, "WORK_ITEM", &fake_entity, "f.png", "image/png", vec![1]).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "başka workspace/varlık izolasyonu");

    // Sil → listeden düşer (soft)
    let (status, _) = app.req("DELETE", &format!("/api/v1/attachments/{att_id}"), &admin, None).await;
    assert_eq!(status, StatusCode::OK);
    let (_, list2) = app
        .req("GET", &format!("/api/v1/attachments?entity_type=WORK_ITEM&entity_id={item_id}"), &admin, None)
        .await;
    assert_eq!(list2.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn execution_attachment_produces_file_added_event() {
    let app = TestApp::new().await;
    let admin = app.login("admin@test.local", "admin-pass-123").await;
    let (_item, kesim) = app.setup_ready_execution(&admin).await;

    let (status, _) = app
        .upload(&admin, "PROCESS_EXECUTION", &kesim, "talimat.pdf", "application/pdf", vec![1, 2, 3])
        .await;
    assert_eq!(status, StatusCode::CREATED);

    let (_, events) = app.req("GET", &format!("/api/v1/process-executions/{kesim}/events"), &admin, None).await;
    assert!(
        events.as_array().unwrap().iter().any(|e| e["event_type"] == "FILE_ADDED"),
        "dosya eki olay üretir"
    );
}
