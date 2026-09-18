//! Faz 6 integration testleri: dashboard aggregate, matris hücre özetleri,
//! üretim akışı ve aktivite.

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
        let db_path = std::env::temp_dir().join(format!(
            "atolye-ins-{}.db",
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
        let storage = Arc::new(LocalFileStorage::new(
            std::env::temp_dir().join(format!("atolye-st-{}", uuid::Uuid::new_v4().simple())),
        ));
        Self { state: AppState::new(pool, config, storage) }
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

    async fn login(&self) -> String {
        let response = router(self.state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(json!({ "email": "admin@test.local", "password": "admin-pass-123" }).to_string()))
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

    /// A Blok → 2 kat × 3 daire; her daireye tezgah; lineer süreç ata.
    /// Farklı durumlara çevirir. (blok_id, kesim_exec_map[daire], item_map[daire]) döndürür.
    async fn setup_matrix_project(&self, admin: &str) -> (String, String, Value, Vec<String>) {
        // yapı
        let (_, p) = self.req("POST", "/api/v1/projects", admin, Some(json!({ "name": "Gardenia", "code": "GRD" }))).await;
        let pid = p["id"].as_str().unwrap().to_string();
        let (_, b) = self.req("POST", &format!("/api/v1/projects/{pid}/sections"), admin, Some(json!({ "name": "A Blok" }))).await;
        let bid = b["id"].as_str().unwrap().to_string();
        let mut floors = Vec::new();
        for k in 1..=2 {
            let (_, f) = self.req("POST", &format!("/api/v1/projects/{pid}/sections"), admin, Some(json!({ "name": format!("{k}. Kat"), "parent_id": bid }))).await;
            floors.push(f["id"].as_str().unwrap().to_string());
        }
        for fid in &floors {
            self.req("POST", &format!("/api/v1/projects/{pid}/sections/bulk"), admin,
                Some(json!({ "parent_id": fid, "start_index": 1, "count": 3, "name_format": "Daire {n}" }))).await;
        }

        // tip + item'lar + grup + atama
        let (_, t) = self.req("POST", "/api/v1/work-item-types", admin, Some(json!({ "name": "Tezgah", "code": "TZ" }))).await;
        let (_, tpl1) = self.req("POST", "/api/v1/process-templates", admin, Some(json!({ "name": "Kesim", "code": "KSM" }))).await;
        let (_, tpl2) = self.req("POST", "/api/v1/process-templates", admin, Some(json!({ "name": "Montaj", "code": "MNT" }))).await;
        let (_, g) = self.req("POST", "/api/v1/process-groups", admin, Some(json!({
            "name": "Lineer",
            "steps": [{ "template_id": tpl1["id"] }, { "template_id": tpl2["id"] }],
            "dependencies": [{ "step": 1, "depends_on": 0 }]
        }))).await;
        let _bulk = self.req("POST", &format!("/api/v1/projects/{pid}/work-items/bulk"), admin,
            Some(json!({ "parent_section_id": bid, "work_item_type_id": t["id"] }))).await;

        // her daireye süreç ata + execution haritası
        let (_, tree) = self.req("GET", &format!("/api/v1/projects/{pid}/sections/tree"), admin, None).await;
        let mut item_map = serde_json::Map::new();
        for floor in tree["nodes"][0]["children"].as_array().unwrap() {
            for daire in floor["children"].as_array().unwrap() {
                let (_, items) = self.req(
                    "GET",
                    &format!("/api/v1/projects/{pid}/work-items?section_id={}", daire["id"].as_str().unwrap()),
                    admin, None,
                ).await;
                let item_id = items[0]["id"].as_str().unwrap().to_string();
                self.req("POST", &format!("/api/v1/work-items/{item_id}/assign-process-group"), admin,
                    Some(json!({ "process_group_id": g["id"] }))).await;
                item_map.insert(daire["id"].as_str().unwrap().to_string(), Value::String(item_id));
            }
        }
        let ordered: Vec<String> = {
            let (_, tree) = self.req("GET", &format!("/api/v1/projects/{pid}/sections/tree"), admin, None).await;
            let mut v = Vec::new();
            for floor in tree["nodes"][0]["children"].as_array().unwrap() {
                for daire in floor["children"].as_array().unwrap() {
                    v.push(item_map[daire["id"].as_str().unwrap()].as_str().unwrap().to_string());
                }
            }
            v
        };
        (pid, bid, Value::Object(item_map), ordered)
    }

    /// item'ın ilk (Kesim) execution id'si.
    async fn first_exec(&self, admin: &str, item_id: &str) -> String {
        let (_, l) = self.req("GET", &format!("/api/v1/process-executions?work_item_id={item_id}"), admin, None).await;
        l["executions"][0]["id"].as_str().unwrap().to_string()
    }
}

#[tokio::test]
async fn matrix_cells_summarize_process_states() {
    let app = TestApp::new().await;
    let admin = app.login().await;
    let (pid, bid, _item_map, items) = app.setup_matrix_project(&admin).await;

    // Kesim execution id'leri (sıra: 1.Kat 3 daire + 2.Kat 3 daire)
    let mut execs = Vec::new();
    for it in &items {
        execs.push(app.first_exec(&admin, it).await);
    }

    // Durum senaryosu:
    //  - 1.Kat/Daire1: kesim tamam → COMPLETED + montaj READY → özet IN_PROGRESS? Hayır:
    //    özet: [COMPLETED, READY] → READY (bloklu/aktif yok, ready var) → READY
    let e1 = &execs[0];
    app.req("POST", &format!("/api/v1/process-executions/{e1}/start"), &admin, None).await;
    app.req("POST", &format!("/api/v1/process-executions/{e1}/complete"), &admin, None).await;

    //  - 1.Kat/Daire2: kesim başladı → özet IN_PROGRESS
    let e2 = &execs[1];
    app.req("POST", &format!("/api/v1/process-executions/{e2}/start"), &admin, None).await;

    //  - 2.Kat/Daire1: kesim bloke → özet BLOCKED
    let e4 = &execs[3];
    let (_, reason) = app.req("POST", "/api/v1/block-reasons", &admin, Some(json!({ "name": "Malzeme" }))).await;
    app.req("POST", &format!("/api/v1/process-executions/{e4}/block"), &admin,
        Some(json!({ "reason_id": reason["id"] }))).await;

    // Matris
    let (status, matrix) = app.req("GET", &format!("/api/v1/projects/{pid}/matrix?parent={bid}"), &admin, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(matrix["rows"].as_array().unwrap().len(), 2, "satır = katlar");
    assert_eq!(matrix["cols"].as_array().unwrap().len(), 3, "sütun = daireler");

    let row1 = matrix["rows"][0]["id"].as_str().unwrap();
    let row2 = matrix["rows"][1]["id"].as_str().unwrap();

    assert_eq!(matrix["cols"][0]["label"], "Daire 1", "sütun etiketi en sık addan gelir");
    assert_eq!(matrix["cells"][row1]["1"]["status"], "READY", "kesim bitti, montaj hazır");
    assert_eq!(matrix["cells"][row1]["2"]["status"], "IN_PROGRESS", "kesim sürüyor");
    assert_eq!(matrix["cells"][row2]["1"]["status"], "BLOCKED", "kesim blokeli");
    assert_eq!(matrix["cells"][row1]["3"]["status"], "READY", "atanan süreçte ilk adım atanınca hazır olur");
    assert_eq!(matrix["cells"][row1]["1"]["label"], "Tezgah");
    assert_eq!(matrix["cells"][row1]["1"]["item_count"], 1);
    assert!(matrix["cells"][row1]["1"]["section_id"].is_string(), "hücre section kimliği taşır");
}

#[tokio::test]
async fn dashboard_summary_and_activity_and_flow() {
    let app = TestApp::new().await;
    let admin = app.login().await;
    let (pid, _bid, _item_map, items) = app.setup_matrix_project(&admin).await;

    let e1 = app.first_exec(&admin, &items[0]).await;
    app.req("POST", &format!("/api/v1/process-executions/{e1}/start"), &admin, None).await;

    // Dashboard özeti
    let (status, summary) = app.req("GET", &format!("/api/v1/projects/{pid}/dashboard-summary"), &admin, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(summary["work_items"]["total"], 6);
    assert_eq!(summary["work_items"]["in_progress"], 1);
    assert_eq!(summary["processes"].as_array().unwrap().len(), 2, "Kesim + Montaj");
    let kesim = summary["processes"].as_array().unwrap().iter().find(|p| p["template_name"] == "Kesim").unwrap();
    assert_eq!(kesim["total"], 6);
    assert_eq!(kesim["in_progress"], 1);

    // Üretim akışı: yalnızca aktif kartlar (COMPLETED hariç)
    let (_, flow) = app.req("GET", &format!("/api/v1/projects/{pid}/flow"), &admin, None).await;
    let cards = flow.as_array().unwrap();
    assert!(cards.len() >= 6, "kesim READY'ler + başlatılan + montaj PENDING'ler");
    assert!(cards.iter().all(|c| c["status"] != "COMPLETED"));

    // Aktivite: son olaylar kullanıcı adıyla
    let (_, activity) = app.req("GET", &format!("/api/v1/projects/{pid}/activity?limit=10"), &admin, None).await;
    let events = activity.as_array().unwrap();
    assert!(!events.is_empty());
    assert_eq!(events[0]["user_name"], "Sistem Yöneticisi");
    assert!(events[0]["section_path"].as_str().unwrap().contains("/"), "yol iki seviyeli");

    // İzolasyon: sahte proje → 404
    let fake = uuid::Uuid::new_v4().to_string();
    let (status, _) = app.req("GET", &format!("/api/v1/projects/{fake}/dashboard-summary"), &admin, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
