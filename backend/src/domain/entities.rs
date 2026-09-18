//! Faz 1 entity'leri (docs/architecture.md §11 şemasıyla birebir).
//! Faz 2+ Section/WorkItem/Process* entity'leri eklenecek.

use chrono::{DateTime, Utc};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Değer nesneleri — durum ve roller
// ---------------------------------------------------------------------------

macro_rules! text_enum {
    ($(#[$meta:meta])* $name:ident { $($(#[$vmeta:meta])* $variant:ident => $str:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum $name {
            $($(#[$vmeta])* $variant,)+
        }

        impl $name {
            pub fn as_str(&self) -> &'static str {
                match self {
                    $($name::$variant => $str,)+
                }
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl std::str::FromStr for $name {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($str => Ok($name::$variant),)+
                    other => Err(format!("geçersiz {}: {other}", stringify!($name))),
                }
            }
        }

        impl sqlx::Type<sqlx::Sqlite> for $name {
            fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
                                <String as sqlx::Type<sqlx::Sqlite>>::type_info()
            }
            fn compatible(ty: &sqlx::sqlite::SqliteTypeInfo) -> bool {
                <String as sqlx::Type<sqlx::Sqlite>>::compatible(ty)
            }
        }

        impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for $name {
            fn decode(
                value: sqlx::sqlite::SqliteValueRef<'r>,
            ) -> Result<Self, sqlx::error::BoxDynError> {
                let s = <&str as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
                Ok(<$name as std::str::FromStr>::from_str(s)?)
            }
        }

        impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for $name {
            fn encode_by_ref(
                &self,
                buf: &mut <sqlx::Sqlite as sqlx::Database>::ArgumentBuffer<'q>,
            ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
                <String as sqlx::Encode<'q, sqlx::Sqlite>>::encode_by_ref(
                    &self.as_str().to_string(),
                    buf,
                )
            }
        }
    };
}

text_enum! {
    /// Kullanıcı rolleri (MASTER PLAN §5). RBAC matrisi: domain::permissions
    Role {
        Admin => "ADMIN",
        ProjectManager => "PROJECT_MANAGER",
        TeamLeader => "TEAM_LEADER",
        Worker => "WORKER",
        Viewer => "VIEWER",
    }
}

text_enum! {
    /// Proje durumları (MASTER PLAN §6)
    ProjectStatus {
        Draft => "DRAFT",
        Active => "ACTIVE",
        Paused => "PAUSED",
        Completed => "COMPLETED",
        Cancelled => "CANCELLED",
        Archived => "ARCHIVED",
    }
}

text_enum! {
    /// Takım üyesi rolü
    TeamRole {
        Leader => "LEADER",
        Member => "MEMBER",
    }
}

text_enum! {
    /// WorkItem önceliği (MASTER PLAN §22)
    Priority {
        Low => "LOW",
        Normal => "NORMAL",
        High => "HIGH",
        Urgent => "URGENT",
    }
}

text_enum! {
    /// WorkItem durumu — Faz 4'te süreç aggregate'inden türetilir,
    /// Faz 3'te varsayılan PENDING.
    WorkItemStatus {
        Pending => "PENDING",
        Ready => "READY",
        InProgress => "IN_PROGRESS",
        Paused => "PAUSED",
        Blocked => "BLOCKED",
        Completed => "COMPLETED",
        Cancelled => "CANCELLED",
    }
}

text_enum! {
    /// Süreç durumu — state machine (MASTER PLAN §13, §14)
    ProcessStatus {
        Pending => "PENDING",
        Ready => "READY",
        InProgress => "IN_PROGRESS",
        Paused => "PAUSED",
        Blocked => "BLOCKED",
        Completed => "COMPLETED",
        Cancelled => "CANCELLED",
    }
}

text_enum! {
    /// ProcessEvent tipleri (MASTER PLAN §15) — append-only denetim izi
    EventType {
        Created => "CREATED",
        Ready => "READY",
        Assigned => "ASSIGNED",
        Started => "STARTED",
        Paused => "PAUSED",
        Resumed => "RESUMED",
        Blocked => "BLOCKED",
        Unblocked => "UNBLOCKED",
        Completed => "COMPLETED",
        Reopened => "REOPENED",
        Cancelled => "CANCELLED",
        NoteAdded => "NOTE_ADDED",
        FileAdded => "FILE_ADDED",
        AssigneeChanged => "ASSIGNEE_CHANGED",
        PlannedDateChanged => "PLANNED_DATE_CHANGED",
    }
}

text_enum! {
    /// Dinamik özellik veri tipi (MASTER PLAN §8)
    PropertyDataType {
        Text => "TEXT",
        LongText => "LONG_TEXT",
        Number => "NUMBER",
        Decimal => "DECIMAL",
        Boolean => "BOOLEAN",
        Date => "DATE",
        Datetime => "DATETIME",
        Select => "SELECT",
        MultiSelect => "MULTI_SELECT",
    }
}

// ---------------------------------------------------------------------------
// Entity'ler — FromRow ile SQLx sorgularına direkt map edilir
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub email: String,
    /// Asla API yanıtına serileştirilmez; DTO katmanı bunu dışlar.
    pub password_hash: String,
    pub full_name: String,
    pub role: Role,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Session {
    /// Cookie token'ının SHA-256 hash'i — token'ın kendisi saklanmaz.
    pub id: String,
    pub user_id: Uuid,
    pub workspace_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub user_agent: Option<String>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct Team {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct TeamMember {
    pub team_id: Uuid,
    pub user_id: Uuid,
    pub role: TeamRole,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct Project {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub status: ProjectStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_start_date: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_end_date: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_start_date: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_end_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Sınırsuz iç içe bölüm (MASTER PLAN §6). `type` sabit bir enum DEĞİLDİR;
/// yeni tipler kullanıcı tarafından serbestçe tanımlanabilir.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct Section {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub project_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Uuid>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Opsiyonel etiket: BLOCK / FLOOR / APARTMENT / ... / serbest metin
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub section_type: Option<String>,
    pub sort_order: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
}

/// İş kalemi tipi — "Mutfak Tezgahı", "Banyo Tezgahı" gibi workspace
/// seviyesinde tanımlanan katalog (kodda hard-code edilmez).
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct WorkItemType {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub code: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Takip edilen esas iş (MASTER PLAN §7).
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct WorkItem {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub project_id: Uuid,
    pub section_id: Uuid,
    pub work_item_type_id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub status: WorkItemStatus,
    pub priority: Priority,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_start_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_end_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Dinamik özellik tanımı — workspace seviyesinde şema (MASTER PLAN §8).
/// `options`: SELECT/MULTI_SELECT için JSON array string'i.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct PropertyDefinition {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub key: String,
    pub data_type: PropertyDataType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Bir iş kalemine bağlı özellik değeri — typed kolonlardan biri dolar.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PropertyValue {
    pub id: Uuid,
    pub work_item_id: Uuid,
    pub property_definition_id: Uuid,
    pub value_text: Option<String>,
    pub value_number: Option<f64>,
    pub value_boolean: Option<bool>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Süreç motoru (Faz 4)
// ---------------------------------------------------------------------------

/// Süreç tanımı: "Kesim", "İmalat"... — workspace kataloğu, kodda hard-code edilmez.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ProcessTemplate {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_duration: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Bir iş kalemine uygulanabilir süreç dizisi (MASTER PLAN §10).
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ProcessGroup {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Gruptaki sıralı adım; template referansı + bağımlılıklar.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ProcessGroupStep {
    pub id: Uuid,
    pub process_group_id: Uuid,
    pub process_template_id: Uuid,
    pub sort_order: i64,
    pub required: bool,
}

/// Çoklu bağımlılık: step, depends_on step'in required_status'e ulaşmasını bekler.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ProcessDependency {
    pub id: Uuid,
    pub process_group_step_id: Uuid,
    pub depends_on_process_group_step_id: Uuid,
    pub required_status: ProcessStatus,
}

/// Bir iş kaleminin bir adım somut çalıştırması — state machine sahibi.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ProcessExecution {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub work_item_id: Uuid,
    pub process_template_id: Uuid,
    pub process_group_step_id: Uuid,
    pub status: ProcessStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_before_block: Option<ProcessStatus>,
    pub version: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_user_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_team_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_start_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_end_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ready_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    pub revision_no: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_execution_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Append-only olay kaydı (MASTER PLAN §15). UPDATE/DELETE yapılmaz.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ProcessEvent {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub process_execution_id: Uuid,
    pub event_type: EventType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_status: Option<ProcessStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_status: Option<ProcessStatus>,
    pub user_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team_id: Option<Uuid>,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

/// Bloke nedeni — admin tarafından yönetilen katalog (MASTER PLAN §18).
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct BlockReason {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub active: bool,
    pub sort_order: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Bir bloke olayının kaydı: neden + açıklama + çözüm bilgisi.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct ProcessBlock {
    pub id: Uuid,
    pub process_execution_id: Uuid,
    pub reason_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_by: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution_note: Option<String>,
}

/// Polymorphic dosya eki — storage anahtarı üzerinden erişilir.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct Attachment {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub file_name: String,
    pub storage_key: String,
    pub mime_type: String,
    pub size: i64,
    pub uploaded_by: Uuid,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
}
