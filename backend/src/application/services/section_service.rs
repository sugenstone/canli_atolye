//! Section service (MASTER PLAN §24): ağaç, tekli oluşturma, bulk generator,
//! alt ağaç klonlama. Tüm değişiklikler RBAC + workspace/project izolasyonu
//! altındadır; bulk ve clone tek transaction'da atomiktir.

use crate::application::dto::{
    BulkSectionRequest, CloneSectionRequest, CreateSectionRequest, SectionTreeResponse,
    SectionTreeNode, UpdateSectionRequest,
};
use crate::domain::entities::{Section, User};
use crate::domain::errors::DomainError;
use crate::domain::permissions::{can, Action};
use crate::infrastructure::repositories::project_repository::ProjectRepository;
use crate::infrastructure::repositories::section_repository::{NewSection, SectionRepository};
use std::collections::HashMap;
use uuid::Uuid;

const MAX_BULK_COUNT: i64 = 200;
const MAX_CLONE_COUNT: i64 = 50;
const MAX_CREATED_ROWS: usize = 2_000;

fn ensure_can_view(actor: &User) -> Result<(), DomainError> {
    if !can(actor.role, Action::ViewSections) {
        return Err(DomainError::Forbidden);
    }
    Ok(())
}

fn ensure_can_manage(actor: &User) -> Result<(), DomainError> {
    if !can(actor.role, Action::CreateSection) {
        return Err(DomainError::Forbidden);
    }
    Ok(())
}

/// Proje aktörün workspace'inde mi? (izolasyon + anlamlı 404)
async fn ensure_project_access(
    pool: &sqlx::SqlitePool,
    actor: &User,
    project_id: Uuid,
) -> Result<(), DomainError> {
    ProjectRepository { pool }
        .find_in_workspace(actor.workspace_id, project_id)
        .await
        .map(|_| ())
}

/// `{n}` ve `{n:2}` / `{n:3}` (sıfır dolgu) yer tutucularını sıra numarasıyla değiştirir.
fn render_format(format: &str, n: i64) -> String {
    let mut out = format.to_string();
    for (pad, placeholder) in [(2, "{n:2}"), (3, "{n:3}")] {
        out = out.replace(placeholder, &format!("{n:0pad$}"));
    }
    out.replace("{n}", &n.to_string())
}

/// Klon adı üretimi: kaynak ad öndeki sayı ile başlıyorsa artır
/// ("1. Kat" + k=1 → "2. Kat"). Sayı yoksa kopya eki kullanılır.
fn derive_clone_name(source: &str, k: i64) -> String {
    let digits: String = source.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return format!("{source} (kopya {k})");
    }
    let rest = &source[digits.len()..];
    let num: i64 = digits.parse().unwrap_or(0);
    let width = digits.len();
    format!("{:0width$}{}", num + k, rest, width = width)
}

/// Flat listeden ağaç kur. Parent'ı silinmiş (orphan) düğümler kök seviyede
/// gösterilir — veri kaybı olmaz.
fn build_tree(sections: Vec<Section>) -> SectionTreeResponse {
    let project_id = sections.first().map(|s| s.project_id);
    let total = sections.len();
    let ids: std::collections::HashSet<Uuid> = sections.iter().map(|s| s.id).collect();

    let mut children_map: HashMap<Uuid, Vec<Section>> = HashMap::new();
    let mut roots: Vec<Section> = Vec::new();
    for section in sections {
        match section.parent_id {
            Some(parent_id) if ids.contains(&parent_id) => {
                children_map.entry(parent_id).or_default().push(section);
            }
            _ => roots.push(section),
        }
    }

    fn to_node(section: Section, children_map: &mut HashMap<Uuid, Vec<Section>>) -> SectionTreeNode {
        let children = children_map
            .remove(&section.id)
            .unwrap_or_default()
            .into_iter()
            .map(|child| to_node(child, children_map))
            .collect();
        SectionTreeNode {
            section: section.into(),
            children,
        }
    }

    let nodes = roots
        .into_iter()
        .map(|root| to_node(root, &mut children_map))
        .collect();

    SectionTreeResponse {
        project_id: project_id.unwrap_or_default(),
        total,
        nodes,
    }
}

/// GET /projects/:id/sections/tree
pub async fn tree(
    pool: &sqlx::SqlitePool,
    actor: &User,
    project_id: Uuid,
) -> Result<SectionTreeResponse, DomainError> {
    ensure_can_view(actor)?;
    ensure_project_access(pool, actor, project_id).await?;
    let sections = SectionRepository::new()
        .list_by_project(pool, actor.workspace_id, project_id)
        .await?;
    Ok(build_tree(sections))
}

/// POST /projects/:id/sections — tek bölüm
pub async fn create(
    pool: &sqlx::SqlitePool,
    actor: &User,
    project_id: Uuid,
    req: CreateSectionRequest,
) -> Result<Section, DomainError> {
    ensure_can_manage(actor)?;
    ensure_project_access(pool, actor, project_id).await?;

    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(DomainError::Validation {
            message: "Bölüm adı zorunludur.".into(),
        });
    }

    let repo = SectionRepository::new();
    if let Some(parent_id) = req.parent_id {
        let parent = repo
            .find_in_workspace(pool, actor.workspace_id, parent_id)
            .await?;
        if parent.project_id != project_id {
            return Err(DomainError::Validation {
                message: "Üst bölüm aynı projede olmalıdır.".into(),
            });
        }
    }

    let next_sort = repo.max_sort_order(pool, project_id, req.parent_id).await? + 1;
    repo.insert(
        pool,
        NewSection {
            project_id,
            workspace_id: actor.workspace_id,
            parent_id: req.parent_id,
            name,
            code: req.code
                .map(|c| c.trim().to_uppercase())
                .filter(|c| !c.is_empty()),
            section_type: req
                .section_type
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty()),
            sort_order: next_sort,
            id: None,
        },
    )
    .await
}

/// PATCH /sections/:id
pub async fn update(
    pool: &sqlx::SqlitePool,
    actor: &User,
    section_id: Uuid,
    req: UpdateSectionRequest,
) -> Result<Section, DomainError> {
    ensure_can_manage(actor)?;
    let repo = SectionRepository::new();
    let existing = repo
        .find_in_workspace(pool, actor.workspace_id, section_id)
        .await?;

    let mut updated = existing.clone();
    if let Some(name) = req.name {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::Validation {
                message: "Bölüm adı boş olamaz.".into(),
            });
        }
        updated.name = name;
    }
    if let Some(code) = req.code {
        updated.code = code
            .map(|c| c.trim().to_uppercase())
            .filter(|c| !c.is_empty());
    }
    if let Some(section_type) = req.section_type {
        updated.section_type = section_type
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());
    }
    if let Some(sort_order) = req.sort_order {
        updated.sort_order = sort_order;
    }
    updated.updated_at = crate::shared::now();

    repo.update(pool, &updated).await?;
    Ok(updated)
}

/// DELETE /sections/:id — soft delete. Alt bölümü olan silinemez;
/// kullanıcı önce alt bölümleri temizler (ağaç bütünlüğü).
pub async fn delete(
    pool: &sqlx::SqlitePool,
    actor: &User,
    section_id: Uuid,
) -> Result<(), DomainError> {
    ensure_can_manage(actor)?;
    let repo = SectionRepository::new();
    let section = repo
        .find_in_workspace(pool, actor.workspace_id, section_id)
        .await?;

    let children = repo.count_children(pool, section.id).await?;
    if children > 0 {
        return Err(DomainError::Conflict {
            message: "Bu bölümün alt bölümleri var; önce onları silin.".into(),
        });
    }
    repo.soft_delete(pool, section.id).await
}

/// POST /projects/:id/sections/bulk — "Daire 1..Daire 10" üretici.
pub async fn bulk_create(
    pool: &sqlx::SqlitePool,
    actor: &User,
    project_id: Uuid,
    req: BulkSectionRequest,
) -> Result<Vec<Section>, DomainError> {
    ensure_can_manage(actor)?;
    ensure_project_access(pool, actor, project_id).await?;

    if req.count < 1 || req.count > MAX_BULK_COUNT {
        return Err(DomainError::Validation {
            message: format!("Adet 1 ile {MAX_BULK_COUNT} arasında olmalı."),
        });
    }
    if !req.name_format.contains("{n}") {
        return Err(DomainError::Validation {
            message: "İsim formatı {n} içermelidir. Örn: \"Daire {n}\"".into(),
        });
    }

    let repo = SectionRepository::new();
    if let Some(parent_id) = req.parent_id {
        let parent = repo
            .find_in_workspace(pool, actor.workspace_id, parent_id)
            .await?;
        if parent.project_id != project_id {
            return Err(DomainError::Validation {
                message: "Üst bölüm aynı projede olmalıdır.".into(),
            });
        }
    }

    let mut next_sort = repo.max_sort_order(pool, project_id, req.parent_id).await?;
    let rows: Vec<NewSection> = (0..req.count)
        .map(|i| {
            let n = req.start_index + i;
            next_sort += 1;
            NewSection {
                project_id,
                workspace_id: actor.workspace_id,
                parent_id: req.parent_id,
                name: render_format(&req.name_format, n),
                code: req.code_format.as_ref().map(|f| render_format(f, n)),
                section_type: req.section_type.clone(),
                sort_order: next_sort,
                id: None,
            }
        })
        .collect();

    let mut tx = pool.begin().await?;
    let created = SectionRepository::insert_many(&mut tx, rows).await?;
    tx.commit().await?;
    Ok(created)
}

/// POST /sections/:id/clone — kaynağın alt ağacını count kez kopyala
/// ("1. Kat" → "2. Kat", "3. Kat", ...; daireler altta aynen kopyalanır).
pub async fn clone(
    pool: &sqlx::SqlitePool,
    actor: &User,
    section_id: Uuid,
    req: CloneSectionRequest,
) -> Result<Vec<Section>, DomainError> {
    ensure_can_manage(actor)?;
    let repo = SectionRepository::new();
    let source = repo
        .find_in_workspace(pool, actor.workspace_id, section_id)
        .await?;

    if req.count < 1 || req.count > MAX_CLONE_COUNT {
        return Err(DomainError::Validation {
            message: format!("Kopya adedi 1 ile {MAX_CLONE_COUNT} arasında olmalı."),
        });
    }

    // Kaynak alt ağacı (kaynak dahil, üstten aşağı) — flat listeden BFS.
    // BFS sırası, bir düğümün ebeveyninin kopyasının her zaman önce
    // işlenmesini garanti eder (id_map hazır olur).
    let all = repo
        .list_by_project(pool, actor.workspace_id, source.project_id)
        .await?;
    let mut by_parent: HashMap<Uuid, Vec<&Section>> = HashMap::new();
    for s in &all {
        if let Some(p) = s.parent_id {
            by_parent.entry(p).or_default().push(s);
        }
    }

    let mut subtree: Vec<&Section> = vec![&source];
    let mut queue = std::collections::VecDeque::from(vec![source.id]);
    while let Some(current) = queue.pop_front() {
        if let Some(children) = by_parent.get(&current) {
            for child in children {
                subtree.push(child);
                queue.push_back(child.id);
            }
        }
    }

    if subtree.len() * req.count as usize > MAX_CREATED_ROWS {
        return Err(DomainError::Validation {
            message: format!(
                "İşlem çok büyük: {} bölüm × {} kopya. Parçalayarak deneyin.",
                subtree.len(),
                req.count
            ),
        });
    }

    let next_sort = repo
        .max_sort_order(pool, source.project_id, source.parent_id)
        .await?;

    // Her kopya: kök için yeni ID + türetilmiş ad; alt düğümler aynı adla,
    // ebeveyn bağları yeni kopyanın ID uzayına çevrilir.
    let mut rows: Vec<NewSection> = Vec::with_capacity(subtree.len() * req.count as usize);
    for k in 1..=req.count {
        let mut id_map: HashMap<Uuid, Uuid> = HashMap::new();
        let copy_root_id = crate::shared::new_id();
        id_map.insert(source.id, copy_root_id);

        for original in &subtree {
            let is_root = original.id == source.id;
            // Satırın kendi ID'si: klon zincirinde parent FK'lar bu ID'lere bağlanır
            let row_id = if is_root {
                copy_root_id
            } else {
                let id = crate::shared::new_id();
                id_map.insert(original.id, id);
                id
            };

            let parent_for_row = if is_root {
                source.parent_id
            } else {
                id_map
                    .get(&original.parent_id.expect("alt düğümün parent'ı olmalı"))
                    .copied()
            };

            let name = if is_root {
                match &req.name_format {
                    Some(fmt) if fmt.contains("{n}") => render_format(fmt, k),
                    _ => derive_clone_name(&original.name, k),
                }
            } else {
                original.name.clone()
            };
            let code = if is_root {
                req.code_format.as_ref().map(|f| render_format(f, k))
            } else {
                original.code.clone()
            };

            rows.push(NewSection {
                project_id: source.project_id,
                workspace_id: actor.workspace_id,
                parent_id: parent_for_row,
                name,
                code,
                section_type: original.section_type.clone(),
                sort_order: if is_root {
                    next_sort + k
                } else {
                    original.sort_order
                },
                id: Some(row_id),
            });
        }
    }

    let mut tx = pool.begin().await?;
    let created = SectionRepository::insert_many(&mut tx, rows).await?;
    tx.commit().await?;
    Ok(created)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_renders_sequence_and_padding() {
        assert_eq!(render_format("Daire {n}", 7), "Daire 7");
        assert_eq!(render_format("Daire {n:2}", 7), "Daire 07");
    }

    #[test]
    fn clone_name_increments_leading_number_with_padding() {
        assert_eq!(derive_clone_name("1. Kat", 1), "2. Kat");
        assert_eq!(derive_clone_name("08 Kat", 1), "09 Kat");
        assert_eq!(derive_clone_name("010", 5), "015");
        // Sayı önde DEĞİLSE otomatik artırma yapılamaz → kopya eki
        assert_eq!(derive_clone_name("Kat 01", 3), "Kat 01 (kopya 3)");
    }

    #[test]
    fn clone_name_falls_back_to_copy_suffix() {
        assert_eq!(derive_clone_name("A Blok", 1), "A Blok (kopya 1)");
        assert_eq!(derive_clone_name("Teras", 2), "Teras (kopya 2)");
    }

    #[test]
    fn tree_builds_hierarchy_and_surfaces_orphans() {
        let mk = |id: Uuid, parent: Option<Uuid>, sort: i64| Section {
            id,
            workspace_id: Uuid::nil(),
            project_id: Uuid::nil(),
            parent_id: parent,
            name: "x".into(),
            code: None,
            section_type: None,
            sort_order: sort,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            deleted_at: None,
        };
        let a = mk(Uuid::new_v4(), None, 1);
        let a1 = mk(Uuid::new_v4(), Some(a.id), 1);
        let deleted_parent = mk(Uuid::new_v4(), None, 2);
        let orphan = mk(Uuid::new_v4(), Some(deleted_parent.id), 2);

        let tree = build_tree(vec![a, a1, orphan]);
        assert_eq!(tree.total, 3);
        assert_eq!(tree.nodes.len(), 2, "orphan kök seviyesinde görünmeli");
        assert_eq!(tree.nodes[0].children.len(), 1);
        assert_eq!(tree.nodes[0].section.name, "x");
    }
}
