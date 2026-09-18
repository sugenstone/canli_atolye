//! Section repository. Bulk ve clone işlemleri service katmanının yönettiği
//! tek transaction içinde çalışır; bu yüzden insert fonksiyonları jenerik
//! executor alır (hem pool hem tx connection).

use crate::domain::entities::Section;
use crate::domain::errors::DomainError;
use crate::shared::{new_id, now};
use sqlx::{Sqlite, SqliteConnection};
use uuid::Uuid;

/// Yeni section satırı (insert öncesi bellekte hazırlanan değer).
/// `id: None` → repository üretir; klon işleminde belirli ID verilir
/// (alt düğümlerin parent FK'ları bu ID'lere bağlanır).
pub struct NewSection {
    pub project_id: Uuid,
    pub workspace_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub code: Option<String>,
    pub section_type: Option<String>,
    pub sort_order: i64,
    pub id: Option<Uuid>,
}

pub struct SectionPatch {
    pub name: Option<String>,
    pub code: Option<Option<String>>,
    pub section_type: Option<Option<String>>,
    pub sort_order: Option<i64>,
}

pub struct SectionRepository;

impl SectionRepository {
    pub fn new() -> Self {
        Self
    }

    async fn insert_row(
        executor: impl sqlx::Executor<'_, Database = Sqlite>,
        row: &NewSection,
    ) -> Result<Section, DomainError> {
        let section = Section {
            id: row.id.unwrap_or_else(new_id),
            workspace_id: row.workspace_id,
            project_id: row.project_id,
            parent_id: row.parent_id,
            name: row.name.clone(),
            code: row.code.clone(),
            section_type: row.section_type.clone(),
            sort_order: row.sort_order,
            created_at: now(),
            updated_at: now(),
            deleted_at: None,
        };
        sqlx::query(
            "INSERT INTO sections (id, workspace_id, project_id, parent_id, name, code, type, sort_order, metadata, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, ?10)",
        )
        .bind(section.id)
        .bind(section.workspace_id)
        .bind(section.project_id)
        .bind(section.parent_id)
        .bind(&section.name)
        .bind(&section.code)
        .bind(&section.section_type)
        .bind(section.sort_order)
        .bind(section.created_at)
        .bind(section.updated_at)
        .execute(executor)
        .await?;
        Ok(section)
    }

    /// Tek section ekle.
    pub async fn insert(&self, pool: &sqlx::SqlitePool, row: NewSection) -> Result<Section, DomainError> {
        Self::insert_row(pool, &row).await
    }

    /// Transaction içinde toplu ekle (bulk generator / clone için).
    pub async fn insert_many(
        tx: &mut SqliteConnection,
        rows: Vec<NewSection>,
    ) -> Result<Vec<Section>, DomainError> {
        let mut created = Vec::with_capacity(rows.len());
        for row in rows {
            created.push(Self::insert_row(&mut *tx, &row).await?);
        }
        Ok(created)
    }

    /// Workspace + project izolasyonlu lookup (silinmiş dahil edilmez).
    pub async fn find_in_workspace(
        &self,
        pool: &sqlx::SqlitePool,
        workspace_id: Uuid,
        section_id: Uuid,
    ) -> Result<Section, DomainError> {
        sqlx::query_as::<_, Section>(
            "SELECT id, workspace_id, project_id, parent_id, name, code, type AS section_type,
                    sort_order, created_at, updated_at, deleted_at
             FROM sections
             WHERE id = ?1 AND workspace_id = ?2 AND deleted_at IS NULL",
        )
        .bind(section_id)
        .bind(workspace_id)
        .fetch_optional(pool)
        .await?
        .ok_or(DomainError::NotFound)
    }

    /// Projenin tüm canlı section'ları (flat). Ağaç service katmanında kurulur —
    /// binlerce kayıt için tek sorgu + bellek içinde O(n) ağaç kurma yeterli.
    pub async fn list_by_project(
        &self,
        pool: &sqlx::SqlitePool,
        workspace_id: Uuid,
        project_id: Uuid,
    ) -> Result<Vec<Section>, DomainError> {
        Ok(sqlx::query_as::<_, Section>(
            "SELECT id, workspace_id, project_id, parent_id, name, code, type AS section_type,
                    sort_order, created_at, updated_at, deleted_at
             FROM sections
             WHERE workspace_id = ?1 AND project_id = ?2 AND deleted_at IS NULL
             ORDER BY sort_order, created_at",
        )
        .bind(workspace_id)
        .bind(project_id)
        .fetch_all(pool)
        .await?)
    }

    /// Bir section'ın canlı çocuk sayısı (silme ön koşulu).
    pub async fn count_children(
        &self,
        pool: &sqlx::SqlitePool,
        section_id: Uuid,
    ) -> Result<i64, DomainError> {
        let (count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM sections WHERE parent_id = ?1 AND deleted_at IS NULL")
                .bind(section_id)
                .fetch_one(pool)
                .await?;
        Ok(count)
    }

    /// Belirtilen parent'ın en yüksek sort_order'ı (yeni kayıtlar sona eklenir).
    pub async fn max_sort_order(
        &self,
        pool: &sqlx::SqlitePool,
        project_id: Uuid,
        parent_id: Option<Uuid>,
    ) -> Result<i64, DomainError> {
        let (max,): (Option<i64>,) = sqlx::query_as(
            "SELECT MAX(sort_order) FROM sections WHERE project_id = ?1 AND parent_id IS ?2 AND deleted_at IS NULL",
        )
        .bind(project_id)
        .bind(parent_id)
        .fetch_one(pool)
        .await?;
        Ok(max.unwrap_or(0))
    }

    pub async fn update(
        &self,
        pool: &sqlx::SqlitePool,
        section: &Section,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE sections SET name = ?2, code = ?3, type = ?4, sort_order = ?5, updated_at = ?6
             WHERE id = ?1",
        )
        .bind(section.id)
        .bind(&section.name)
        .bind(&section.code)
        .bind(&section.section_type)
        .bind(section.sort_order)
        .bind(section.updated_at)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Soft delete: geçmiş korunur (MASTER PLAN §42).
    pub async fn soft_delete(
        &self,
        pool: &sqlx::SqlitePool,
        section_id: Uuid,
    ) -> Result<(), DomainError> {
        sqlx::query("UPDATE sections SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1 AND deleted_at IS NULL")
            .bind(section_id)
            .bind(now())
            .execute(pool)
            .await?;
        Ok(())
    }
}

impl Default for SectionRepository {
    fn default() -> Self {
        Self::new()
    }
}
