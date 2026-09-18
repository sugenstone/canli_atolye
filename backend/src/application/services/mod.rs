//! Application service'leri. Her service çağrısı öncesinde RBAC + workspace
//! izolasyonu burada doğrulanır (handler'a güvenilmez).

pub mod auth_service;
pub mod insight_service;
pub mod operation_service;
pub mod process_service;
pub mod project_service;
pub mod section_service;
pub mod seed;
pub mod team_service;
pub mod user_service;
pub mod work_item_service;
