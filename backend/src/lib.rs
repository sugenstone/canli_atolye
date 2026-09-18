//! Canlı Atölye — Üretim ve Montaj Takip Sistemi backend.
//!
//! Katmanlar (docs/architecture.md §5):
//! ```text
//! api (Axum handlers)  →  application (services)  →  domain (kurallar)
//!                                              ↘  infrastructure (repo + db)
//! ```

pub mod api;
pub mod application;
pub mod config;
pub mod domain;
pub mod infrastructure;
pub mod shared;
