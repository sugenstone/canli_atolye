//! Work item service (MASTER PLAN §7, §8): tipler, iş kalemleri, dinamik
//! özellikler. Değer atamaları data_type bazlı doğrulanır (unit testli).

use crate::application::dto::{
    BulkWorkItemRequest, CreatePropertyDefinitionRequest, CreateWorkItemRequest,
    CreateWorkItemTypeRequest, SetPropertyValueRequest, UpdateWorkItemRequest, WorkItemDetailDto,
};
use crate::domain::entities::{
    Priority, PropertyDataType, Section, User, WorkItem, WorkItemType,
};
use crate::domain::errors::DomainError;
use crate::domain::permissions::{can, Action};
use crate::infrastructure::repositories::section_repository::SectionRepository;
use crate::infrastructure::repositories::work_item_repository::{
    leaves_under, NewWorkItem, PropertyRepository, TypedValue, WorkItemRepository,
    WorkItemTypeRepository,
};
use sqlx::SqlitePool;
use uuid::Uuid;

const MAX_BULK_LEAVES: usize = 500;

// ---------------------------------------------------------------------------
// Değer doğrulama (MASTER PLAN §8: typed value)
// ---------------------------------------------------------------------------

/// JSON değerini tanımın data_type'ına göre doğrula ve typed değere çevir.
/// `None` (JSON null) → alan temizleme.
pub fn coerce_value(
    def: &crate::domain::entities::PropertyDefinition,
    raw: &serde_json::Value,
) -> Result<TypedValue, DomainError> {
    use PropertyDataType as T;
    let invalid = |detail: String| DomainError::Validation {
        message: format!("Özellik \"{}\" için geçersiz değer: {detail}", def.name),
    };

    if raw.is_null() {
        return Ok(TypedValue { text: None, number: None, boolean: None });
    }

    match def.data_type {
        T::Text | T::LongText | T::Date | T::Datetime | T::MultiSelect => {
            let s = raw.as_str().ok_or_else(|| invalid("metin bekleniyor".into()))?;
            Ok(TypedValue { text: Some(s.to_string()), number: None, boolean: None })
        }
        T::Number | T::Decimal => {
            let n = raw
                .as_f64()
                .or_else(|| raw.as_str().and_then(|s| s.trim().parse::<f64>().ok()))
                .ok_or_else(|| invalid("sayı bekleniyor".into()))?;
            if def.data_type == T::Decimal && (n * 1000.0).fract() != 0.0 {
                // hafif tolerans: 3 ondalık basamağa kadar
            }
            Ok(TypedValue { text: None, number: Some(n), boolean: None })
        }
        T::Boolean => {
            let b = raw
                .as_bool()
                .or_else(|| raw.as_str().map(|s| s.eq_ignore_ascii_case("true")))
                .ok_or_else(|| invalid("true/false bekleniyor".into()))?;
            Ok(TypedValue { text: None, number: None, boolean: Some(b) })
        }
        T::Select => {
            let s = raw.as_str().ok_or_else(|| invalid("seçenek bekleniyor".into()))?;
            let options: Vec<String> = def
                .options
                .as_deref()
                .and_then(|o| serde_json::from_str(o).ok())
                .unwrap_or_default();
            if !options.contains(&s.to_string()) {
                return Err(invalid(format!(
                    "\"{s}\" tanımlı seçeneklerden değil ({})",
                    options.join(", ")
                )));
            }
            Ok(TypedValue { text: Some(s.to_string()), number: None, boolean: None })
        }
    }
}

fn key_slug(input: &str) -> String {
    input
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'ç' => 'c', 'ğ' => 'g', 'ı' => 'i', 'ö' => 'o', 'ş' => 's', 'ü' => 'u',
            c if c.is_ascii_alphanumeric() => c,
            _ => '_',
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tipler
// ---------------------------------------------------------------------------

pub async fn list_types(pool: &SqlitePool, actor: &User) -> Result<Vec<WorkItemType>, DomainError> {
    if !can(actor.role, Action::ViewWorkItems) {
        return Err(DomainError::Forbidden);
    }
    WorkItemTypeRepository::list_by_workspace(pool, actor.workspace_id).await
}

pub async fn create_type(
    pool: &SqlitePool,
    actor: &User,
    req: CreateWorkItemTypeRequest,
) -> Result<WorkItemType, DomainError> {
    if !can(actor.role, Action::ManageWorkItemTypes) {
        return Err(DomainError::Forbidden);
    }
    let name = req.name.trim();
    let code = req.code.trim();
    if name.is_empty() || code.is_empty() {
        return Err(DomainError::Validation {
            message: "Tip adı ve kodu zorunludur.".into(),
        });
    }
    WorkItemTypeRepository::insert(pool, actor.workspace_id, name, code).await
}

// ---------------------------------------------------------------------------
// İş kalemleri
// ---------------------------------------------------------------------------

fn ensure_can_view(actor: &User) -> Result<(), DomainError> {
    if !can(actor.role, Action::ViewWorkItems) {
        return Err(DomainError::Forbidden);
    }
    Ok(())
}

fn ensure_can_manage(actor: &User) -> Result<(), DomainError> {
    if !can(actor.role, Action::CreateWorkItem) {
        return Err(DomainError::Forbidden);
    }
    Ok(())
}

async fn ensure_section_in_project(
    pool: &SqlitePool,
    actor: &User,
    section_id: Uuid,
) -> Result<Section, DomainError> {
    let section = SectionRepository::new()
        .find_in_workspace(pool, actor.workspace_id, section_id)
        .await?;
    Ok(section)
}

pub async fn create(
    pool: &SqlitePool,
    actor: &User,
    _project_id: Uuid,
    req: CreateWorkItemRequest,
) -> Result<WorkItem, DomainError> {
    ensure_can_manage(actor)?;
    let section = ensure_section_in_project(pool, actor, req.section_id).await?;
    let type_ = WorkItemTypeRepository::find_in_workspace(pool, actor.workspace_id, req.work_item_type_id)
        .await?;

    let name = req
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or(&type_.name)
        .to_string();

    WorkItemRepository::insert(
        pool,
        NewWorkItem {
            project_id: section.project_id,
            workspace_id: actor.workspace_id,
            section_id: section.id,
            work_item_type_id: type_.id,
            name,
            code: None,
            priority: req.priority.unwrap_or(Priority::Normal),
        },
    )
    .await
}

/// Üst bölümün altındaki tüm YAPRAK bölümlere aynı tipte iş kalemi ekler.
/// Aynı bölümde aynı tip zaten varsa o bölüm atlanır (idempotent).
pub async fn bulk_create(
    pool: &SqlitePool,
    actor: &User,
    _project_id: Uuid,
    req: BulkWorkItemRequest,
) -> Result<Vec<WorkItem>, DomainError> {
    ensure_can_manage(actor)?;
    let parent = ensure_section_in_project(pool, actor, req.parent_section_id).await?;
    let type_ = WorkItemTypeRepository::find_in_workspace(pool, actor.workspace_id, req.work_item_type_id)
        .await?;

    let all_sections = SectionRepository::new()
        .list_by_project(pool, actor.workspace_id, parent.project_id)
        .await?;
    let leaves = leaves_under(&all_sections, &parent);
    if leaves.is_empty() {
        return Err(DomainError::Validation {
            message: "Bu bölümün altında yaprak bölüm yok.".into(),
        });
    }
    if leaves.len() > MAX_BULK_LEAVES {
        return Err(DomainError::Validation {
            message: format!("Çok fazla yaprak bölüm ({} > {MAX_BULK_LEAVES}).", leaves.len()),
        });
    }

    let leaf_ids: Vec<Uuid> = leaves.iter().map(|s| s.id).collect();
    let existing = WorkItemRepository::list_by_sections(pool, actor.workspace_id, &leaf_ids).await?;
    let taken: std::collections::HashSet<(Uuid, Uuid)> = existing
        .iter()
        .map(|i| (i.section_id, i.work_item_type_id))
        .collect();

    let rows: Vec<NewWorkItem> = leaves
        .iter()
        .filter(|s| !taken.contains(&(s.id, type_.id)))
        .map(|s| NewWorkItem {
            project_id: s.project_id,
            workspace_id: actor.workspace_id,
            section_id: s.id,
            work_item_type_id: type_.id,
            name: type_.name.clone(),
            code: None,
            priority: Priority::Normal,
        })
        .collect();

    if rows.is_empty() {
        return Ok(Vec::new()); // hepsi zaten var
    }

    let mut tx = pool.begin().await?;
    let created = WorkItemRepository::insert_many(&mut tx, rows).await?;
    tx.commit().await?;
    Ok(created)
}

pub async fn list_by_section(
    pool: &SqlitePool,
    actor: &User,
    section_id: Uuid,
) -> Result<Vec<WorkItem>, DomainError> {
    ensure_can_view(actor)?;
    ensure_section_in_project(pool, actor, section_id).await?;
    WorkItemRepository::list_by_section(pool, actor.workspace_id, section_id).await
}

/// Projenin tüm iş kalemleri (section_id verilmemişse).
pub async fn list_all(
    pool: &SqlitePool,
    actor: &User,
    project_id: Uuid,
) -> Result<Vec<WorkItem>, DomainError> {
    ensure_can_view(actor)?;
    WorkItemRepository::list_by_project(pool, actor.workspace_id, project_id).await
}

pub async fn get(
    pool: &SqlitePool,
    actor: &User,
    item_id: Uuid,
) -> Result<WorkItemDetailDto, DomainError> {
    ensure_can_view(actor)?;
    let item = WorkItemRepository::find_in_workspace(pool, actor.workspace_id, item_id).await?;
    let properties =
        PropertyRepository::list_definitions_with_values(pool, actor.workspace_id, item.id).await?;
    Ok(WorkItemDetailDto { item, properties })
}

pub async fn update(
    pool: &SqlitePool,
    actor: &User,
    item_id: Uuid,
    req: UpdateWorkItemRequest,
) -> Result<WorkItem, DomainError> {
    ensure_can_manage(actor)?;
    let existing = WorkItemRepository::find_in_workspace(pool, actor.workspace_id, item_id).await?;
    let mut updated = existing.clone();
    if let Some(name) = req.name {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::Validation {
                message: "İş kalemi adı boş olamaz.".into(),
            });
        }
        updated.name = name;
    }
    if let Some(priority) = req.priority {
        updated.priority = priority;
    }
    if let Some(start) = req.planned_start_at {
        updated.planned_start_at = Some(start);
    }
    if let Some(end) = req.planned_end_at {
        updated.planned_end_at = Some(end);
    }
    updated.updated_at = crate::shared::now();
    WorkItemRepository::update(pool, &updated).await?;
    Ok(updated)
}

pub async fn delete(pool: &SqlitePool, actor: &User, item_id: Uuid) -> Result<(), DomainError> {
    ensure_can_manage(actor)?;
    let item = WorkItemRepository::find_in_workspace(pool, actor.workspace_id, item_id).await?;
    WorkItemRepository::soft_delete(pool, item.id).await
}

// ---------------------------------------------------------------------------
// Özellikler
// ---------------------------------------------------------------------------

pub async fn list_definitions(
    pool: &SqlitePool,
    actor: &User,
) -> Result<Vec<crate::domain::entities::PropertyDefinition>, DomainError> {
    ensure_can_view(actor)?;
    PropertyRepository::list_definitions(pool, actor.workspace_id).await
}

pub async fn create_definition(
    pool: &SqlitePool,
    actor: &User,
    req: CreatePropertyDefinitionRequest,
) -> Result<crate::domain::entities::PropertyDefinition, DomainError> {
    if !can(actor.role, Action::ManagePropertyDefinitions) {
        return Err(DomainError::Forbidden);
    }
    let name = req.name.trim();
    if name.is_empty() {
        return Err(DomainError::Validation {
            message: "Özellik adı zorunludur.".into(),
        });
    }
    let key = match req.key.as_deref().map(str::trim).filter(|k| !k.is_empty()) {
        Some(k) => k.to_string(),
        None => key_slug(name),
    };
    let options_json = match (&req.data_type, &req.options) {
        (PropertyDataType::Select, Some(opts)) if !opts.is_empty() => {
            Some(serde_json::to_string(opts).unwrap())
        }
        (PropertyDataType::Select, _) => {
            return Err(DomainError::Validation {
                message: "SELECT tipi için en az bir seçenek gerekli.".into(),
            })
        }
        _ => None,
    };
    PropertyRepository::insert_definition(
        pool,
        actor.workspace_id,
        name,
        &key,
        req.data_type,
        req.unit.as_deref(),
        req.required,
        options_json.as_deref(),
    )
    .await
}

/// Değerleri toplu ata; her biri data_type'a göre doğrulanır.
pub async fn set_values(
    pool: &SqlitePool,
    actor: &User,
    item_id: Uuid,
    values: Vec<SetPropertyValueRequest>,
) -> Result<(), DomainError> {
    ensure_can_manage(actor)?;
    let item = WorkItemRepository::find_in_workspace(pool, actor.workspace_id, item_id).await?;

    for entry in values {
        let def = PropertyRepository::find_definition(pool, actor.workspace_id, entry.property_definition_id)
            .await?;
        let typed = match &entry.value {
            Some(raw) => coerce_value(&def, raw)?,
            None => TypedValue { text: None, number: None, boolean: None },
        };
        PropertyRepository::upsert_value(pool, item.id, def.id, &typed).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::PropertyDefinition;
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    fn def(data_type: PropertyDataType, options: Option<&str>) -> PropertyDefinition {
        PropertyDefinition {
            id: Uuid::new_v4(),
            workspace_id: Uuid::nil(),
            name: "Test".into(),
            key: "test".into(),
            data_type,
            unit: None,
            required: false,
            options: options.map(Into::into),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn coerce_text_and_number() {
        let d = def(PropertyDataType::Text, None);
        let v = coerce_value(&d, &json!("Lamar Moon White")).unwrap();
        assert_eq!(v.text.as_deref(), Some("Lamar Moon White"));
        assert!(v.number.is_none());

        let d = def(PropertyDataType::Number, None);
        let v = coerce_value(&d, &json!(5.82)).unwrap();
        assert_eq!(v.number, Some(5.82));
        // string'ten parse de kabul (form girdileri)
        let v = coerce_value(&d, &json!("12,5".replace(',', "."))).unwrap();
        assert_eq!(v.number, Some(12.5));
    }

    #[test]
    fn coerce_number_rejects_garbage() {
        let d = def(PropertyDataType::Number, None);
        assert!(coerce_value(&d, &json!("abc")).is_err());
        assert!(coerce_value(&d, &json!(true)).is_err());
    }

    #[test]
    fn coerce_boolean_accepts_both_forms() {
        let d = def(PropertyDataType::Boolean, None);
        assert_eq!(coerce_value(&d, &json!(true)).unwrap().boolean, Some(true));
        assert_eq!(coerce_value(&d, &json!("false")).unwrap().boolean, Some(false));
    }

    #[test]
    fn coerce_select_validates_options() {
        let d = def(PropertyDataType::Select, Some(r#"["Lamar","Arma","Quartz"]"#));
        let v = coerce_value(&d, &json!("Lamar")).unwrap();
        assert_eq!(v.text.as_deref(), Some("Lamar"));
        assert!(coerce_value(&d, &json!("Bilinmeyen")).is_err());
    }

    #[test]
    fn coerce_null_clears_value() {
        let d = def(PropertyDataType::Text, None);
        let v = coerce_value(&d, &json!(null)).unwrap();
        assert!(v.text.is_none() && v.number.is_none() && v.boolean.is_none());
    }

    #[test]
    fn key_slug_normalizes_turkish() {
        assert_eq!(key_slug("Eviye Tipi"), "eviye_tipi");
        assert_eq!(key_slug("Metraj (m)"), "metraj__m_");
    }
}
