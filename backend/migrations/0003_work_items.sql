-- 0003_work_items.sql — Faz 3: iş kalemleri, tipler ve dinamik özellikler
-- (MASTER PLAN §7, §8). Kesim/imalat gibi süreç kavramları burada YOKTUR;
-- iş kalemi tipleri ve özellikler kullanıcı tarafından tanımlanır.

CREATE TABLE work_item_types (
    id          TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces (id),
    name        TEXT NOT NULL,
    code        TEXT NOT NULL,
    active      BOOLEAN NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    UNIQUE (workspace_id, code)
);

CREATE TABLE work_items (
    id                TEXT PRIMARY KEY,
    workspace_id      TEXT NOT NULL REFERENCES workspaces (id),
    project_id        TEXT NOT NULL REFERENCES projects (id),
    section_id        TEXT NOT NULL REFERENCES sections (id),
    work_item_type_id TEXT NOT NULL REFERENCES work_item_types (id),
    name              TEXT NOT NULL,
    code              TEXT,
    status            TEXT NOT NULL DEFAULT 'PENDING'
                      CHECK (status IN ('PENDING', 'READY', 'IN_PROGRESS', 'PAUSED', 'BLOCKED', 'COMPLETED', 'CANCELLED')),
    priority          TEXT NOT NULL DEFAULT 'NORMAL'
                      CHECK (priority IN ('LOW', 'NORMAL', 'HIGH', 'URGENT')),
    planned_start_at  TEXT,
    planned_end_at    TEXT,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL,
    deleted_at        TEXT
);
CREATE INDEX idx_work_items_project ON work_items (project_id, status);
CREATE INDEX idx_work_items_section ON work_items (section_id);
CREATE INDEX idx_work_items_type ON work_items (work_item_type_id);

CREATE TABLE property_definitions (
    id          TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces (id),
    name        TEXT NOT NULL,
    key         TEXT NOT NULL,
    data_type   TEXT NOT NULL CHECK (data_type IN
                  ('TEXT', 'LONG_TEXT', 'NUMBER', 'DECIMAL', 'BOOLEAN', 'DATE', 'DATETIME', 'SELECT', 'MULTI_SELECT')),
    unit        TEXT,
    required    BOOLEAN NOT NULL DEFAULT 0,
    options     TEXT,   -- SELECT/MULTI_SELECT seçenek listesi (JSON array)
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    UNIQUE (workspace_id, key)
);

CREATE TABLE property_values (
    id                   TEXT PRIMARY KEY,
    work_item_id         TEXT NOT NULL REFERENCES work_items (id),
    property_definition_id TEXT NOT NULL REFERENCES property_definitions (id),
    value_text           TEXT,
    value_number         REAL,
    value_boolean        BOOLEAN,
    created_at           TEXT NOT NULL,
    updated_at           TEXT NOT NULL,
    UNIQUE (work_item_id, property_definition_id)
);
CREATE INDEX idx_property_values_item ON property_values (work_item_id);
