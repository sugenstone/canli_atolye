//! RBAC izin kuralları (docs/architecture.md §14 izin matrisinin kod karşılığı).
//!
//! Kural: yetki yalnızca frontend'de buton gizlemek değildir; her service çağrısı
//! öncesinde bu fonksiyonlarla backend kontrolü yapılır.
//!
//! Faz 1 notu: PROJECT_MANAGER'ın "atanmış projeler" kapsamı henüz yok (project_members
//! tablosu Faz 6'da). Şimdilik PM workspace'teki tüm projeleri görür/yönetir;
//! kapsam daraltması o fazda eklenir.

use crate::domain::entities::Role;

/// İzin verilen aksiyonlar. Her yeni aksiyon buraya eklenir ve matris testle sabitlenir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    // Workspace / kullanıcı yönetimi
    ManageWorkspace,
    ManageUsers,
    // Takımlar
    ManageTeams,
    ViewTeams,
    // Projeler
    CreateProject,
    UpdateProject,
    ArchiveProject,
    ViewProjects,
    // Bölümler (recursive sections)
    CreateSection,
    UpdateSection,
    DeleteSection,
    ViewSections,
    // İş kalemleri (proje içi yönetim ADMIN+PM)
    CreateWorkItem,
    UpdateWorkItem,
    DeleteWorkItem,
    ViewWorkItems,
    // Workspace seviyesi kataloglar: yalnızca ADMIN
    ManageWorkItemTypes,
    ManagePropertyDefinitions,
}

/// `actor` bu aksiyonu yapabilir mi? Kaynak bazlı ek kısıtlar (ör. WORKER yalnız
/// kendisine atanmış süreçler) service katmanında kaynak bilgisiyle kontrol edilir.
pub fn can(actor: Role, action: Action) -> bool {
    use Action::*;
    use Role::*;

    match action {
        // Yalnızca ADMIN
        ManageWorkspace | ManageUsers | ManageTeams | CreateProject => actor == Admin,

        // ADMIN + PM (PM kapsamı Faz 6'da atanmış projelere daraltılacak)
        UpdateProject | ArchiveProject => matches!(actor, Admin | ProjectManager),

        // Section yönetimi: ADMIN + PM (MASTER PLAN §5 — proje yapısını yönetme)
        CreateSection | UpdateSection | DeleteSection => matches!(actor, Admin | ProjectManager),

        // İş kalemi yönetimi (proje içi): ADMIN + PM
        CreateWorkItem | UpdateWorkItem | DeleteWorkItem => matches!(actor, Admin | ProjectManager),

        // Workspace katalogları (tip/özellik tanımları): yalnızca ADMIN
        ManageWorkItemTypes | ManagePropertyDefinitions => actor == Admin,

        // Tüm roller görebilir (WORKER kapsamı service katmanında daraltılır)
        ViewProjects | ViewTeams | ViewSections | ViewWorkItems => matches!(
            actor,
            Admin | ProjectManager | TeamLeader | Worker | Viewer
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_ROLES: [Role; 5] = [
        Role::Admin,
        Role::ProjectManager,
        Role::TeamLeader,
        Role::Worker,
        Role::Viewer,
    ];

    fn allowed_roles(action: Action) -> Vec<Role> {
        ALL_ROLES.into_iter().filter(|&r| can(r, action)).collect()
    }

    #[test]
    fn workspace_user_team_management_admin_only() {
        for action in [Action::ManageWorkspace, Action::ManageUsers, Action::ManageTeams] {
            assert_eq!(allowed_roles(action), vec![Role::Admin], "{action:?}");
        }
    }

    #[test]
    fn project_creation_admin_only() {
        assert_eq!(allowed_roles(Action::CreateProject), vec![Role::Admin]);
    }

    #[test]
    fn project_update_admin_and_pm() {
        assert_eq!(
            allowed_roles(Action::UpdateProject),
            vec![Role::Admin, Role::ProjectManager]
        );
        assert_eq!(
            allowed_roles(Action::ArchiveProject),
            vec![Role::Admin, Role::ProjectManager]
        );
    }

    #[test]
    fn work_item_management_admin_and_pm() {
        for action in [Action::CreateWorkItem, Action::UpdateWorkItem, Action::DeleteWorkItem] {
            assert_eq!(
                allowed_roles(action),
                vec![Role::Admin, Role::ProjectManager],
                "{action:?}"
            );
        }
        assert_eq!(allowed_roles(Action::ViewWorkItems).len(), 5);
    }

    #[test]
    fn workspace_catalogs_admin_only() {
        for action in [Action::ManageWorkItemTypes, Action::ManagePropertyDefinitions] {
            assert_eq!(allowed_roles(action), vec![Role::Admin], "{action:?}");
        }
    }

    #[test]
    fn section_management_admin_and_pm() {
        for action in [Action::CreateSection, Action::UpdateSection, Action::DeleteSection] {
            assert_eq!(
                allowed_roles(action),
                vec![Role::Admin, Role::ProjectManager],
                "{action:?}"
            );
        }
        // Görünürlük tüm rollere açık
        assert_eq!(allowed_roles(Action::ViewSections).len(), 5);
    }

    #[test]
    fn project_team_visibility_all_roles() {
        // Görünürlük: tüm roller; veri kapsamı service katmanında rol bazlı daraltılır.
        for action in [Action::ViewProjects, Action::ViewTeams] {
            assert_eq!(allowed_roles(action).len(), 5, "{action:?}");
        }
    }
}
