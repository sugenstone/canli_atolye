-- 0002_sections.sql — Faz 2: sınırsız iç içe bölüm yapısı (MASTER PLAN §6, §24)
-- Kural: blok/kat/daire gibi kavramlar KOLON OLMAZ; `type` yalnızca opsiyonel etiket,
-- `parent_id` recursive hiyerarşiyi sağlar. Soft delete: deleted_at.

CREATE TABLE sections (
    id          TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces (id),
    project_id  TEXT NOT NULL REFERENCES projects (id),
    parent_id   TEXT NULL REFERENCES sections (id),
    name        TEXT NOT NULL,
    code        TEXT,
    type        TEXT,                -- opsiyonel görsel/filtreleme etiketi (BLOCK/FLOOR/... serbest)
    sort_order  INTEGER NOT NULL DEFAULT 0,
    metadata    TEXT,                -- JSON (ileri kullanım için)
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    deleted_at  TEXT
);
CREATE INDEX idx_sections_project_parent ON sections (project_id, parent_id);
CREATE INDEX idx_sections_workspace ON sections (workspace_id);
