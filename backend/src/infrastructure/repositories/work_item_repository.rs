//! Work item repository: tipler, iş kalemleri ve dinamik özellik değerleri.

use crate::domain::entities::{
    Priority, PropertyDefinition, Section, WorkItem, WorkItemStatus, WorkItemType,
};
use crate::domain::errors::DomainError;
use crate::shared::{new_id, now};
use sqlx::{Sqlite, SqliteConnection, SqlitePool};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Tipler
// ---------------------------------------------------------------------------

pub struct WorkItemTypeRepository;

impl WorkItemTypeRepository {
    pub async fn insert(
        pool: &SqlitePool,
        workspace_id: Uuid,
        name: &str,
        code: &str,
    ) -> Result<WorkItemType, DomainError> {
        let t = WorkItemType {
            id: new_id(),
            workspace_id,
            name: name.to_string(),
            code: code.to_uppercase(),
            active: true,
            created_at: now(),
            updated_at: now(),
        };
        let result = sqlx::query(
            "INSERT INTO work_item_types (id, workspace_id, name, code, active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(t.id)
        .bind(t.workspace_id)
        .bind(&t.name)
        .bind(&t.code)
        .bind(t.active)
        .bind(t.created_at)
        .bind(t.updated_at)
        .execute(pool)
        .await;
        match result {
            Ok(_) => Ok(t),
            Err(sqlx::Error::Database(db))
                if db.message().contains("UNIQUE") =>
            {
                Err(DomainError::Conflict {
                    message: "Bu kodla bir iş kalemi tipi zaten var.".into(),
                })
            }
            Err(e) => Err(DomainError::Database(e)),
        }
    }

    pub async fn list_by_workspace(
        pool: &SqlitePool,
        workspace_id: Uuid,
    ) -> Result<Vec<WorkItemType>, DomainError> {
        Ok(sqlx::query_as::<_, WorkItemType>(
            "SELECT * FROM work_item_types WHERE workspace_id = ?1 AND active = 1 ORDER BY name",
        )
        .bind(workspace_id)
        .fetch_all(pool)
        .await?)
    }

    pub async fn find_in_workspace(
        pool: &SqlitePool,
        workspace_id: Uuid,
        type_id: Uuid,
    ) -> Result<WorkItemType, DomainError> {
        sqlx::query_as::<_, WorkItemType>(
            "SELECT * FROM work_item_types WHERE id = ?1 AND workspace_id = ?2 AND active = 1",
        )
        .bind(type_id)
        .bind(workspace_id)
        .fetch_optional(pool)
        .await?
        .ok_or(DomainError::NotFound)
    }
}

// ---------------------------------------------------------------------------
// İş kalemleri
// ---------------------------------------------------------------------------

pub struct NewWorkItem {
    pub project_id: Uuid,
    pub workspace_id: Uuid,
    pub section_id: Uuid,
    pub work_item_type_id: Uuid,
    pub name: String,
    pub code: Option<String>,
    pub priority: Priority,
}

pub struct WorkItemPatch {
    pub name: Option<String>,
    pub priority: Option<Priority>,
    pub planned_start_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
    pub planned_end_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
}

pub struct WorkItemRepository;

impl WorkItemRepository {
    async fn insert_row(
        executor: impl sqlx::Executor<'_, Database = Sqlite>,
        row: &NewWorkItem,
    ) -> Result<WorkItem, DomainError> {
        let item = WorkItem {
            id: new_id(),
            workspace_id: row.workspace_id,
            project_id: row.project_id,
            section_id: row.section_id,
            work_item_type_id: row.work_item_type_id,
            name: row.name.clone(),
            code: row.code.clone(),
            status: WorkItemStatus::Pending,
            priority: row.priority,
            planned_start_at: None,
            planned_end_at: None,
            created_at: now(),
            updated_at: now(),
            deleted_at: None,
        };
        sqlx::query(
            "INSERT INTO work_items (id, workspace_id, project_id, section_id, work_item_type_id, name, code, status, priority, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        )
        .bind(item.id)
        .bind(item.workspace_id)
        .bind(item.project_id)
        .bind(item.section_id)
        .bind(item.work_item_type_id)
        .bind(&item.name)
        .bind(&item.code)
        .bind(item.status)
        .bind(item.priority)
        .bind(item.created_at)
        .bind(item.updated_at)
        .execute(executor)
        .await?;
        Ok(item)
    }

    pub async fn insert(pool: &SqlitePool, row: NewWorkItem) -> Result<WorkItem, DomainError> {
        Self::insert_row(pool, &row).await
    }

    /// Toplu ekleme (bulk: yaprak bölümlere) — tek transaction.
    pub async fn insert_many(
        tx: &mut SqliteConnection,
        rows: Vec<NewWorkItem>,
    ) -> Result<Vec<WorkItem>, DomainError> {
        let mut created = Vec::with_capacity(rows.len());
        for row in rows {
            created.push(Self::insert_row(&mut *tx, &row).await?);
        }
        Ok(created)
    }

    pub async fn find_in_workspace(
        pool: &SqlitePool,
        workspace_id: Uuid,
        item_id: Uuid,
    ) -> Result<WorkItem, DomainError> {
        sqlx::query_as::<_, WorkItem>(
            "SELECT * FROM work_items WHERE id = ?1 AND workspace_id = ?2 AND deleted_at IS NULL",
        )
        .bind(item_id)
        .bind(workspace_id)
        .fetch_optional(pool)
        .await?
        .ok_or(DomainError::NotFound)
    }

    pub async fn list_by_section(
        pool: &SqlitePool,
        workspace_id: Uuid,
        section_id: Uuid,
    ) -> Result<Vec<WorkItem>, DomainError> {
        Ok(sqlx::query_as::<_, WorkItem>(
            "SELECT * FROM work_items
             WHERE workspace_id = ?1 AND section_id = ?2 AND deleted_at IS NULL
             ORDER BY created_at",
        )
        .bind(workspace_id)
        .bind(section_id)
        .fetch_all(pool)
        .await?)
    }

    /// Projenin tüm iş kalemleri (ağaç üstü sayaçlar ve toplu görünüm için).
    pub async fn list_by_project(
        pool: &SqlitePool,
        workspace_id: Uuid,
        project_id: Uuid,
    ) -> Result<Vec<WorkItem>, DomainError> {
        Ok(sqlx::query_as::<_, WorkItem>(
            "SELECT * FROM work_items
             WHERE workspace_id = ?1 AND project_id = ?2 AND deleted_at IS NULL
             ORDER BY created_at",
        )
        .bind(workspace_id)
        .bind(project_id)
        .fetch_all(pool)
        .await?)
    }

    pub async fn list_by_sections(
        pool: &SqlitePool,
        workspace_id: Uuid,
        section_ids: &[Uuid],
    ) -> Result<Vec<WorkItem>, DomainError> {
        if section_ids.is_empty() {
            return Ok(Vec::new());
        }
        // IN (...) listesi dinamik kurulur
        let mut query = String::from(
            "SELECT * FROM work_items WHERE workspace_id = ?1 AND section_id IN (",
        );
        for i in 0..section_ids.len() {
            if i > 0 {
                query.push(',');
            }
            query.push_str(&format!("?{}", i + 2));
        }
        query.push_str(") AND deleted_at IS NULL ORDER BY created_at");

        let mut q = sqlx::query_as::<_, WorkItem>(&query).bind(workspace_id);
        for id in section_ids {
            q = q.bind(id);
        }
        Ok(q.fetch_all(pool).await?)
    }

    pub async fn update(
        pool: &SqlitePool,
        item: &WorkItem,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE work_items SET name = ?2, priority = ?3, planned_start_at = ?4, planned_end_at = ?5, updated_at = ?6
             WHERE id = ?1",
        )
        .bind(item.id)
        .bind(&item.name)
        .bind(item.priority)
        .bind(item.planned_start_at)
        .bind(item.planned_end_at)
        .bind(item.updated_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn soft_delete(pool: &SqlitePool, item_id: Uuid) -> Result<(), DomainError> {
        sqlx::query("UPDATE work_items SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1 AND deleted_at IS NULL")
            .bind(item_id)
            .bind(now())
            .execute(pool)
            .await?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Özellik tanımları ve değerleri
// ---------------------------------------------------------------------------

pub struct PropertyRepository;

/// Typed değer — data_type'a göre biri dolar.
pub struct TypedValue {
    pub text: Option<String>,
    pub number: Option<f64>,
    pub boolean: Option<bool>,
}

impl PropertyRepository {
    pub async fn insert_definition(
        pool: &SqlitePool,
        workspace_id: Uuid,
        name: &str,
        key: &str,
        data_type: crate::domain::entities::PropertyDataType,
        unit: Option<&str>,
        required: bool,
        options: Option<&str>,
    ) -> Result<PropertyDefinition, DomainError> {
        let def = PropertyDefinition {
            id: new_id(),
            workspace_id,
            name: name.to_string(),
            key: key.to_string(),
            data_type,
            unit: unit.map(Into::into),
            required,
            options: options.map(Into::into),
            created_at: now(),
            updated_at: now(),
        };
        let result = sqlx::query(
            "INSERT INTO property_definitions (id, workspace_id, name, key, data_type, unit, required, options, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .bind(def.id)
        .bind(def.workspace_id)
        .bind(&def.name)
        .bind(&def.key)
        .bind(def.data_type)
        .bind(&def.unit)
        .bind(def.required)
        .bind(&def.options)
        .bind(def.created_at)
        .bind(def.updated_at)
        .execute(pool)
        .await;
        match result {
            Ok(_) => Ok(def),
            Err(sqlx::Error::Database(db)) if db.message().contains("UNIQUE") => {
                Err(DomainError::Conflict {
                    message: "Bu anahtar (key) ile bir özellik tanımı zaten var.".into(),
                })
            }
            Err(e) => Err(DomainError::Database(e)),
        }
    }

    pub async fn list_definitions(
        pool: &SqlitePool,
        workspace_id: Uuid,
    ) -> Result<Vec<PropertyDefinition>, DomainError> {
        Ok(sqlx::query_as::<_, PropertyDefinition>(
            "SELECT * FROM property_definitions WHERE workspace_id = ?1 ORDER BY name",
        )
        .bind(workspace_id)
        .fetch_all(pool)
        .await?)
    }

    pub async fn find_definition(
        pool: &SqlitePool,
        workspace_id: Uuid,
        def_id: Uuid,
    ) -> Result<PropertyDefinition, DomainError> {
        sqlx::query_as::<_, PropertyDefinition>(
            "SELECT * FROM property_definitions WHERE id = ?1 AND workspace_id = ?2",
        )
        .bind(def_id)
        .bind(workspace_id)
        .fetch_optional(pool)
        .await?
        .ok_or(DomainError::NotFound)
    }

    /// Değeri upsert et (varsa güncelle, yoksa ekle). null değer satırı temizler.
    pub async fn upsert_value(
        pool: &SqlitePool,
        work_item_id: Uuid,
        def_id: Uuid,
        value: &TypedValue,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO property_values (id, work_item_id, property_definition_id, value_text, value_number, value_boolean, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
             ON CONFLICT (work_item_id, property_definition_id) DO UPDATE SET
               value_text = excluded.value_text,
               value_number = excluded.value_number,
               value_boolean = excluded.value_boolean,
               updated_at = excluded.updated_at",
        )
        .bind(new_id())
        .bind(work_item_id)
        .bind(def_id)
        .bind(&value.text)
        .bind(value.number)
        .bind(value.boolean)
        .bind(now())
        .execute(pool)
        .await?;
        Ok(())
    }

    /// İş kaleminin özellik paneli: workspace'teki TÜM tanımlar + varsa değerleri
    /// (yeni tanımlar da değer girilebilir input olarak görünür).
    pub async fn list_definitions_with_values(
        pool: &SqlitePool,
        workspace_id: Uuid,
        work_item_id: Uuid,
    ) -> Result<Vec<PropertyValueRow>, DomainError> {
        Ok(sqlx::query_as::<_, PropertyValueRow>(
            "SELECT pd.id AS definition_id, pd.name AS definition_name, pd.key AS definition_key,
                    pd.data_type, pd.unit, pd.required, pd.options,
                    pv.value_text, pv.value_number, pv.value_boolean
             FROM property_definitions pd
             LEFT JOIN property_values pv
                    ON pv.property_definition_id = pd.id AND pv.work_item_id = ?2
             WHERE pd.workspace_id = ?1
             ORDER BY pd.name",
        )
        .bind(workspace_id)
        .bind(work_item_id)
        .fetch_all(pool)
        .await?)
    }
}

/// Tanım + değer birleşik satırı (property list endpoint çıktısı).
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct PropertyValueRow {
    pub definition_id: Uuid,
    pub definition_name: String,
    pub definition_key: String,
    pub data_type: crate::domain::entities::PropertyDataType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_number: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_boolean: Option<bool>,
}

/// Section yardımcıları (leaf hesabı service'te kullanılır).
pub fn leaves_under(sections: &[Section], root: &Section) -> Vec<Section> {
    use std::collections::{HashMap, VecDeque};
    let mut by_parent: HashMap<Uuid, Vec<&Section>> = HashMap::new();
    for s in sections {
        if let Some(p) = s.parent_id {
            by_parent.entry(p).or_default().push(s);
        }
    }
    let mut leaves = Vec::new();
    let mut queue = VecDeque::from(vec![root.id]);
    while let Some(current) = queue.pop_front() {
        match by_parent.get(&current) {
            Some(children) if !children.is_empty() => {
                for child in children {
                    queue.push_back(child.id);
                }
            }
            _ => {
                // yaprak: kendisi (root dahil — root'un çocuğu yoksa yapraktır)
                if let Some(s) = sections.iter().find(|s| s.id == current) {
                    leaves.push(s.clone());
                }
            }
        }
    }
    leaves
}
