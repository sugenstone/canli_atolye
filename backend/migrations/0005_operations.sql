-- 0005_operations.sql — Faz 5: bloke kataloğu, bloke kayıtları ve dosya ekleri
-- (MASTER PLAN §18, §19).

CREATE TABLE block_reasons (
    id          TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces (id),
    name        TEXT NOT NULL,
    description TEXT,
    active      BOOLEAN NOT NULL DEFAULT 1,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE process_blocks (
    id          TEXT PRIMARY KEY,
    process_execution_id TEXT NOT NULL REFERENCES process_executions (id),
    reason_id   TEXT NOT NULL REFERENCES block_reasons (id),
    description TEXT,
    created_by  TEXT NOT NULL REFERENCES users (id),
    created_at  TEXT NOT NULL,
    resolved_by TEXT NULL REFERENCES users (id),
    resolved_at TEXT NULL,
    resolution_note TEXT NULL
);
CREATE INDEX idx_pblocks_exec ON process_blocks (process_execution_id);

-- Polymorphic dosya eki (MASTER PLAN §19): entityType + entityId.
CREATE TABLE attachments (
    id          TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces (id),
    entity_type TEXT NOT NULL CHECK (entity_type IN
                ('PROJECT', 'SECTION', 'WORK_ITEM', 'PROCESS_EXECUTION', 'PROCESS_EVENT')),
    entity_id   TEXT NOT NULL,
    file_name   TEXT NOT NULL,
    storage_key TEXT NOT NULL,   -- storage abstraction anahtarı (URL değil)
    mime_type   TEXT NOT NULL,
    size        INTEGER NOT NULL,
    uploaded_by TEXT NOT NULL REFERENCES users (id),
    created_at  TEXT NOT NULL,
    deleted_at  TEXT
);
CREATE INDEX idx_attachments_entity ON attachments (entity_type, entity_id);
