//! API DTO'ları (docs/architecture.md §91).
//! Domain entity ile dış API modeli ayrıdır: `password_hash` gibi alanlar
//! asla response'ta yer almaz.

use crate::domain::entities::{Priority, ProjectStatus, Role, TeamRole, User, Workspace};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// --- Auth ---

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct UserDto {
    pub id: Uuid,
    pub email: String,
    pub full_name: String,
    pub role: Role,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserDto {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            email: u.email,
            full_name: u.full_name,
            role: u.role,
            active: u.active,
            created_at: u.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct WorkspaceDto {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
}

impl From<Workspace> for WorkspaceDto {
    fn from(w: Workspace) -> Self {
        Self {
            id: w.id,
            name: w.name,
            slug: w.slug,
            description: w.description,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub user: UserDto,
    pub workspace: WorkspaceDto,
}

// --- Kullanıcı yönetimi ---

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub full_name: String,
    pub role: Role,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub full_name: Option<String>,
    pub role: Option<Role>,
    pub active: Option<bool>,
}

// --- Takımlar ---

#[derive(Debug, Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTeamRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: Uuid,
    #[serde(default)]
    pub role: Option<TeamRole>,
}

// --- Bölümler (recursive sections) ---

#[derive(Debug, Deserialize)]
pub struct CreateSectionRequest {
    pub name: String,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(rename = "type", default)]
    pub section_type: Option<String>,
    #[serde(default)]
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSectionRequest {
    pub name: Option<String>,
    /// `null` gönderilirse kod/type temizlenir; alan yoksa dokunulmaz.
    pub code: Option<Option<String>>,
    #[serde(rename = "type")]
    pub section_type: Option<Option<String>>,
    pub sort_order: Option<i64>,
}

/// Bulk generator (MASTER PLAN §24): `startIndex`'ten `count` adet,
/// `nameFormat` içindeki `{n}` sıra numarasıyla değiştirilir.
/// Örn: { "startIndex": 1, "count": 10, "nameFormat": "Daire {n}" }
#[derive(Debug, Deserialize)]
pub struct BulkSectionRequest {
    #[serde(default)]
    pub parent_id: Option<Uuid>,
    pub start_index: i64,
    pub count: i64,
    pub name_format: String,
    #[serde(default)]
    pub code_format: Option<String>,
    #[serde(rename = "type", default)]
    pub section_type: Option<String>,
}

/// Alt ağaç klonlama: kaynağın tüm alt ağacı `count` kez kopyalanır.
/// `name_format` verilmezse kaynak adındaki öndeki sayı otomatik artırılır
/// ("1. Kat" → "2. Kat", "3. Kat", ...), sayı yoksa "Ad (kopya N)".
#[derive(Debug, Deserialize)]
pub struct CloneSectionRequest {
    pub count: i64,
    #[serde(default)]
    pub name_format: Option<String>,
    #[serde(default)]
    pub code_format: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SectionDto {
    pub id: Uuid,
    pub project_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub code: Option<String>,
    #[serde(rename = "type")]
    pub section_type: Option<String>,
    pub sort_order: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<crate::domain::entities::Section> for SectionDto {
    fn from(s: crate::domain::entities::Section) -> Self {
        Self {
            id: s.id,
            project_id: s.project_id,
            parent_id: s.parent_id,
            name: s.name,
            code: s.code,
            section_type: s.section_type,
            sort_order: s.sort_order,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}

/// Ağaç düğümü — service katmanında kurulur, istemciye nested döner.
#[derive(Debug, Serialize)]
pub struct SectionTreeNode {
    #[serde(flatten)]
    pub section: SectionDto,
    pub children: Vec<SectionTreeNode>,
}

#[derive(Debug, Serialize)]
pub struct SectionTreeResponse {
    pub project_id: Uuid,
    pub total: usize,
    pub nodes: Vec<SectionTreeNode>,
}

// --- İş kalemleri (Faz 3) ---

#[derive(Debug, Deserialize)]
pub struct CreateWorkItemTypeRequest {
    pub name: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkItemRequest {
    pub section_id: Uuid,
    pub work_item_type_id: Uuid,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub priority: Option<Priority>,
}

/// Toplu: bir üst bölümün altındaki tüm YAPRAK bölümlere aynı tipte
/// iş kalemi ekler ("A Blok altındaki 30 daireye Mutfak Tezgahı").
#[derive(Debug, Deserialize)]
pub struct BulkWorkItemRequest {
    pub parent_section_id: Uuid,
    pub work_item_type_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkItemRequest {
    pub name: Option<String>,
    pub priority: Option<Priority>,
    pub planned_start_at: Option<DateTime<Utc>>,
    pub planned_end_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct WorkItemDetailDto {
    pub item: crate::domain::entities::WorkItem,
    pub properties: Vec<crate::infrastructure::repositories::work_item_repository::PropertyValueRow>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePropertyDefinitionRequest {
    pub name: String,
    /// Boş/verilmezse isimden otomatik türetilir (slug).
    #[serde(default)]
    pub key: Option<String>,
    pub data_type: crate::domain::entities::PropertyDataType,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub options: Option<Vec<String>>,
}

/// Tek özellik değeri ataması; `value: null` alanı temizler.
#[derive(Debug, Deserialize)]
pub struct SetPropertyValueRequest {
    pub property_definition_id: Uuid,
    pub value: Option<serde_json::Value>,
}

// --- Projeler ---

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub code: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<ProjectStatus>,
    pub planned_start_date: Option<DateTime<Utc>>,
    pub planned_end_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct ProjectDto {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
    pub planned_start_date: Option<DateTime<Utc>>,
    pub planned_end_date: Option<DateTime<Utc>>,
    pub actual_start_date: Option<DateTime<Utc>>,
    pub actual_end_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<crate::domain::entities::Project> for ProjectDto {
    fn from(p: crate::domain::entities::Project) -> Self {
        Self {
            id: p.id,
            workspace_id: p.workspace_id,
            name: p.name,
            code: p.code,
            description: p.description,
            status: p.status,
            planned_start_date: p.planned_start_date,
            planned_end_date: p.planned_end_date,
            actual_start_date: p.actual_start_date,
            actual_end_date: p.actual_end_date,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}
