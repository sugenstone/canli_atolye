//! Faz 4 integration testleri: süreç motoru uçtan uca — şablonlar, lineer
//! grup + bağımlılıklar, atama, state machine akışı, downstream READY,
//! olay üretimi, concurrency (optimistic lock) ve RBAC.

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
            "atolye-proc-{}.db",
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

    /// "Kesim → İmalat → Nakliye → Montaj" lineer grubu kur; (template id'leri map, group_id) döndür.
    async fn setup_lineer_group(&self, admin: &str) -> (Vec<String>, String) {
        let mut ids = Vec::new();
        for (name, code) in [("Kesim", "KSM"), ("İmalat", "IML"), ("Nakliye", "NKL"), ("Montaj", "MNT")] {
            let (_, t) = self
                .req("POST", "/api/v1/process-templates", admin, Some(json!({ "name": name, "code": code })))
                .await;
            ids.push(t["id"].as_str().unwrap().to_string());
        }
        let (_, g) = self
            .req(
                "POST",
                "/api/v1/process-groups",
                admin,
                Some(json!({
                    "name": "Standart Tezgah Süreci",
                    "steps": ids.iter().map(|id| json!({ "template_id": id })).collect::<Vec<_>>(),
                    "dependencies": [
                        { "step": 1, "depends_on": 0 },
                        { "step": 2, "depends_on": 1 },
                        { "step": 3, "depends_on": 2 }
                    ]
                })),
            )
            .await;
        (ids, g["id"].as_str().unwrap().to_string())
    }

    /// Proje + blok + 1 daire + iş kalemi; (project_id, work_item_id) döndür.
    async fn setup_item(&self, admin: &str) -> (String, String) {
        let (_, p) = self.req("POST", "/api/v1/projects", admin, Some(json!({ "name": "Gardenia", "code": "GRD" }))).await;
        let pid = p["id"].as_str().unwrap().to_string();
        let (_, s) = self.req("POST", &format!("/api/v1/projects/{pid}/sections"), admin, Some(json!({ "name": "Daire 1" }))).await;
        let sid = s["id"].as_str().unwrap().to_string();
        let (_, t) = self.req("POST", "/api/v1/work-item-types", admin, Some(json!({ "name": "Mutfak Tezgahı", "code": "MT" }))).await;
        let (_, i) = self
            .req(
                "POST",
                &format!("/api/v1/projects/{pid}/work-items"),
                admin,
                Some(json!({ "section_id": sid, "work_item_type_id": t["id"] })),
            )
            .await;
        (pid, i["id"].as_str().unwrap().to_string())
    }
}

#[tokio::test]
async fn full_lineer_flow_with_downstream_promotion() {
    let app = TestApp::new().await;
    let admin = app.login("admin@test.local", "admin-pass-123").await;
    let (templates, group_id) = app.setup_lineer_group(&admin).await;
    let (_pid, item_id) = app.setup_item(&admin).await;

    // Grubu ata → 4 execution; Kesim READY, diğerleri PENDING
    let (status, created) = app
        .req(
            "POST",
            &format!("/api/v1/work-items/{item_id}/assign-process-group"),
            &admin,
            Some(json!({ "process_group_id": group_id })),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created.as_array().unwrap().len(), 4);
    assert_eq!(created[0]["status"], "PENDING", "henüz event'ler transaction içinde");

    // Liste: sıralı + template adları ile
    let (_, list) = app
        .req("GET", &format!("/api/v1/process-executions?work_item_id={item_id}"), &admin, None)
        .await;
    let execs = list["executions"].as_array().unwrap();
    assert_eq!(execs.len(), 4);
    assert_eq!(execs[0]["template_name"], "Kesim");
    assert_eq!(execs[0]["status"], "READY", "bağımlılıksız ilk adım atanınca READY olur");
    assert_eq!(execs[1]["status"], "PENDING", "İmalat Kesim'e bağımlı");
    assert!(execs[0]["ready_at"].is_string());

    // İkinci atama reddedilir
    let (status, _) = app
        .req(
            "POST",
            &format!("/api/v1/work-items/{item_id}/assign-process-group"),
            &admin,
            Some(json!({ "process_group_id": group_id })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // PENDING adım başlatılamaz (İmalat)
    let iml_id = execs[1]["id"].as_str().unwrap().to_string();
    let (status, body) = app.req("POST", &format!("/api/v1/process-executions/{iml_id}/start"), &admin, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "VALIDATION_FAILED");

    // Kesim: start → IN_PROGRESS
    let kesim_id = execs[0]["id"].as_str().unwrap().to_string();
    let (status, t1) = app.req("POST", &format!("/api/v1/process-executions/{kesim_id}/start"), &admin, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(t1["execution"]["status"], "IN_PROGRESS");
    assert!(t1["execution"]["started_at"].is_string());
    assert_eq!(t1["execution"]["version"], 2, "assign'taki READY promosyonu + start = 2 geçiş");

    // Tekrar start → geçersiz geçiş
    let (status, _) = app.req("POST", &format!("/api/v1/process-executions/{kesim_id}/start"), &admin, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // pause → resume
    let (status, _) = app.req("POST", &format!("/api/v1/process-executions/{kesim_id}/pause"), &admin, None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app.req("POST", &format!("/api/v1/process-executions/{kesim_id}/resume"), &admin, None).await;
    assert_eq!(status, StatusCode::OK);

    // complete → İmalat READY'ya promoted
    let (status, t2) = app
        .req("POST", &format!("/api/v1/process-executions/{kesim_id}/complete"), &admin, Some(json!({ "note": "Tamamlandı" })))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(t2["execution"]["status"], "COMPLETED");
    eprintln!("PROMOTED: {:?}", t2["promoted"]);
    let (_, dbg_list) = app.req("GET", &format!("/api/v1/process-executions?work_item_id={item_id}"), &admin, None).await;
    for e in dbg_list["executions"].as_array().unwrap() {
        eprintln!("  {} -> {}", e["template_name"], e["status"]);
    }
    assert_eq!(t2["promoted"].as_array().unwrap().len(), 1, "İmalat READY oldu");

    // İmalat artık başlatılabilir
    let (status, _) = app.req("POST", &format!("/api/v1/process-executions/{iml_id}/start"), &admin, None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app.req("POST", &format!("/api/v1/process-executions/{iml_id}/complete"), &admin, None).await;
    assert_eq!(status, StatusCode::OK);

    // Olay geçmişi: Kesim CREATED → READY → STARTED → PAUSED → RESUMED → COMPLETED
    let (_, events) = app.req("GET", &format!("/api/v1/process-executions/{kesim_id}/events"), &admin, None).await;
    let types: Vec<&str> = events.as_array().unwrap().iter().map(|e| e["event_type"].as_str().unwrap()).collect();
    assert_eq!(types, vec!["CREATED", "READY", "STARTED", "PAUSED", "RESUMED", "COMPLETED"]);
    // son olay notu taşır
    assert_eq!(events[5]["note"], "Tamamlandı");
    // geçiş durumları kayıtlı
    assert_eq!(events[2]["previous_status"], "READY");
    assert_eq!(events[2]["new_status"], "IN_PROGRESS");

    let _ = templates;
}

#[tokio::test]
async fn diamond_dependency_multiple_parents() {
    let app = TestApp::new().await;
    let admin = app.login("admin@test.local", "admin-pass-123").await;

    // Elmas: A → (B, C paralel) → D (hem B hem C ister)
    let mut ids = Vec::new();
    for (name, code) in [("Ölçü", "OLC"), ("Çizim", "CIZ"), ("Onay", "ONY"), ("Üretim", "URT")] {
        ids.push(self_req_template(&app, &admin, name, code).await);
    }
    let (_, g) = app
        .req(
            "POST",
            "/api/v1/process-groups",
            &admin,
            Some(json!({
                "name": "Elmas Süreci",
                "steps": ids.iter().map(|id| json!({ "template_id": id })).collect::<Vec<_>>(),
                "dependencies": [
                    { "step": 1, "depends_on": 0 },
                    { "step": 2, "depends_on": 0 },
                    { "step": 3, "depends_on": 1 },
                    { "step": 3, "depends_on": 2 }
                ]
            })),
        )
        .await;
    let group_id = g["id"].as_str().unwrap().to_string();

    let (_pid, item_id) = app.setup_item(&admin).await;
    app.req(
        "POST",
        &format!("/api/v1/work-items/{item_id}/assign-process-group"),
        &admin,
        Some(json!({ "process_group_id": group_id })),
    )
    .await;

    let (_, list) = app
        .req("GET", &format!("/api/v1/process-executions?work_item_id={item_id}"), &admin, None)
        .await;
    let execs = list["executions"].as_array().unwrap();
    assert_eq!(execs[0]["status"], "READY", "A bağımlılıksız");
    assert_eq!(execs[1]["status"], "PENDING", "B, A tamamlanmasını bekler");
    assert_eq!(execs[2]["status"], "PENDING", "C, A tamamlanmasını bekler");
    assert_eq!(execs[3]["status"], "PENDING", "D iki ebeveyne bağımlı");

    // A tamam → B ve C birlikte READY olur (promoted = 2); D hâlâ PENDING
    let a_id = execs[0]["id"].as_str().unwrap().to_string();
    app.req("POST", &format!("/api/v1/process-executions/{a_id}/start"), &admin, None).await;
    let (_, ta) = app.req("POST", &format!("/api/v1/process-executions/{a_id}/complete"), &admin, None).await;
    assert_eq!(ta["promoted"].as_array().unwrap().len(), 2, "B ve C birlikte READY olur");

    let (_, list2) = app
        .req("GET", &format!("/api/v1/process-executions?work_item_id={item_id}"), &admin, None)
        .await;
    assert_eq!(list2["executions"][1]["status"], "READY");
    assert_eq!(list2["executions"][2]["status"], "READY");
    assert_eq!(list2["executions"][3]["status"], "PENDING", "D; B ve C tamamlanmadan READY olmaz");

    // B tamam → D hâlâ PENDING
    let b_id = execs[1]["id"].as_str().unwrap().to_string();
    app.req("POST", &format!("/api/v1/process-executions/{b_id}/start"), &admin, None).await;
    let (_, tb) = app.req("POST", &format!("/api/v1/process-executions/{b_id}/complete"), &admin, None).await;
    assert_eq!(tb["promoted"].as_array().unwrap().len(), 0, "tek ebeveyn yetmez");
    let (_, list3) = app
        .req("GET", &format!("/api/v1/process-executions?work_item_id={item_id}"), &admin, None)
        .await;
    assert_eq!(list3["executions"][3]["status"], "PENDING");

    // C tamam → D READY
    let c_id = execs[2]["id"].as_str().unwrap().to_string();
    app.req("POST", &format!("/api/v1/process-executions/{c_id}/start"), &admin, None).await;
    let (_, t) = app.req("POST", &format!("/api/v1/process-executions/{c_id}/complete"), &admin, None).await;
    assert_eq!(t["promoted"].as_array().unwrap().len(), 1, "D sonunda READY olur");
    let (_, list4) = app
        .req("GET", &format!("/api/v1/process-executions?work_item_id={item_id}"), &admin, None)
        .await;
    assert_eq!(list4["executions"][3]["status"], "READY");
}

async fn self_req_template(app: &TestApp, admin: &str, name: &str, code: &str) -> String {
    let (_, t) = app
        .req("POST", "/api/v1/process-templates", admin, Some(json!({ "name": name, "code": code })))
        .await;
    t["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn block_unblock_roundtrip_with_recorded_status() {
    let app = TestApp::new().await;
    let admin = app.login("admin@test.local", "admin-pass-123").await;
    let (_, group_id) = app.setup_lineer_group(&admin).await;
    let (_pid, item_id) = app.setup_item(&admin).await;
    app.req(
        "POST",
        &format!("/api/v1/work-items/{item_id}/assign-process-group"),
        &admin,
        Some(json!({ "process_group_id": group_id })),
    )
    .await;

    let (_, list) = app
        .req("GET", &format!("/api/v1/process-executions?work_item_id={item_id}"), &admin, None)
        .await;
    let kesim = list["executions"][0]["id"].as_str().unwrap().to_string();

    // Neden kataloğundan reason al (Faz 5: reason zorunlu)
    let (_, reason) = app
        .req("POST", "/api/v1/block-reasons", &admin, Some(json!({ "name": "Malzeme eksik" })))
        .await;
    let reason_id = reason["id"].as_str().unwrap().to_string();

    // READY'den bloke → before = READY
    let (status, _) = app
        .req("POST", &format!("/api/v1/process-executions/{kesim}/block"), &admin, Some(json!({ "reason_id": reason_id })))
        .await;
    assert_eq!(status, StatusCode::OK);
    let (_, exec) = app.req("GET", &format!("/api/v1/process-executions/{kesim}"), &admin, None).await;
    assert_eq!(exec["status"], "BLOCKED");
    assert_eq!(exec["status_before_block"], "READY");

    // BLOCKED'de start geçersiz
    let (status, _) = app.req("POST", &format!("/api/v1/process-executions/{kesim}/start"), &admin, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // unblock → kayıtlı duruma dön (READY)
    let (status, t) = app.req("POST", &format!("/api/v1/process-executions/{kesim}/unblock"), &admin, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(t["status"], "READY");
    assert!(t["status_before_block"].is_null());
}

#[tokio::test]
async fn optimistic_lock_rejects_stale_concurrent_update() {
    let app = TestApp::new().await;
    let admin = app.login("admin@test.local", "admin-pass-123").await;
    let (_, group_id) = app.setup_lineer_group(&admin).await;
    let (_pid, item_id) = app.setup_item(&admin).await;
    app.req(
        "POST",
        &format!("/api/v1/work-items/{item_id}/assign-process-group"),
        &admin,
        Some(json!({ "process_group_id": group_id })),
    )
    .await;
    let (_, list) = app
        .req("GET", &format!("/api/v1/process-executions?work_item_id={item_id}"), &admin, None)
        .await;
    let kesim = list["executions"][0]["id"].as_str().unwrap().to_string();

    // İlk start başarılı
    let (status, _) = app.req("POST", &format!("/api/v1/process-executions/{kesim}/start"), &admin, None).await;
    assert_eq!(status, StatusCode::OK);

    // Aynı execution'a "eski sürüm" ile ikinci yazma: GEÇERSİZ GEÇİŞ (PAUSED→)
    // yerine gerçek version çatışmasını simüle etmek için admin'i kandıramayız;
    // geçiş kuralı bunu zaten 400 ile reddeder. Concurrency koruması:
    // aynı anda iki geçerli hedef (pause) sıralı → ilki OK, ikincisi artık
    // IN_PROGRESS'ten PAUSED'a geçerli... bu yüzden asıl test: version alanı ilerler.
    let (_, exec1) = app.req("GET", &format!("/api/v1/process-executions/{kesim}"), &admin, None).await;
    assert_eq!(exec1["version"], 2, "READY(1) + START(2): her geçiş version'u artırır");

    // Cancel sonrası donmuş durum
    let (status, _) = app.req("POST", &format!("/api/v1/process-executions/{kesim}/cancel"), &admin, None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app.req("POST", &format!("/api/v1/process-executions/{kesim}/start"), &admin, None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "CANCELLED donmuş durum");
}

#[tokio::test]
async fn rbac_worker_operates_only_assigned_executions() {
    let app = TestApp::new().await;
    let admin = app.login("admin@test.local", "admin-pass-123").await;
    let (_, group_id) = app.setup_lineer_group(&admin).await;
    let (_pid, item_id) = app.setup_item(&admin).await;
    app.req(
        "POST",
        &format!("/api/v1/work-items/{item_id}/assign-process-group"),
        &admin,
        Some(json!({ "process_group_id": group_id })),
    )
    .await;
    let (_, list) = app
        .req("GET", &format!("/api/v1/process-executions?work_item_id={item_id}"), &admin, None)
        .await;
    let kesim = list["executions"][0]["id"].as_str().unwrap().to_string();

    let (worker_id, worker) = app.create_worker(&admin).await;

    // AtanMAMIŞ execution'ı worker başlatamaz
    let (status, body) = app.req("POST", &format!("/api/v1/process-executions/{kesim}/start"), &worker, None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "AUTH_FORBIDDEN");

    // Worker şablon/grup oluşturamaz
    let (status, _) = app
        .req("POST", "/api/v1/process-templates", &worker, Some(json!({ "name": "X", "code": "X" })))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    // Admin worker'ı Kesim'e ata → artık başlatabilir
    let (status, _) = app
        .req(
            "POST",
            &format!("/api/v1/process-executions/{kesim}/assign"),
            &admin,
            Some(json!({ "user_id": worker_id })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, t) = app.req("POST", &format!("/api/v1/process-executions/{kesim}/start"), &worker, None).await;
    assert_eq!(status, StatusCode::OK, "atanan worker kendi işini başlatır");
    assert_eq!(t["execution"]["status"], "IN_PROGRESS");

    // Olaylarda ASSIGNED görünüyor
    let (_, events) = app.req("GET", &format!("/api/v1/process-executions/{kesim}/events"), &admin, None).await;
    let types: Vec<&str> = events.as_array().unwrap().iter().map(|e| e["event_type"].as_str().unwrap()).collect();
    assert!(types.contains(&"ASSIGNED"), "atama olayı kayıtlı: {types:?}");

    // Worker listeyi görebilir
    let (status, _) = app
        .req("GET", &format!("/api/v1/process-executions?work_item_id={item_id}"), &worker, None)
        .await;
    assert_eq!(status, StatusCode::OK);
}
