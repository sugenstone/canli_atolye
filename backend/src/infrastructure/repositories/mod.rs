//! Repository katmanı: tüm SQL burada izole edilir (docs/architecture.md §6).
//! Service katmanı SQL bilmez. Tüm sorgular workspace sınırındadır.

pub mod process_repository;
pub mod insight_repository;
pub mod my_work_repository;
pub mod notification_repository;
pub mod report_repository;
pub mod operation_repository;
pub mod project_repository;
pub mod section_repository;
pub mod session_repository;
pub mod team_repository;
pub mod user_repository;
pub mod work_item_repository;
pub mod workspace_repository;
