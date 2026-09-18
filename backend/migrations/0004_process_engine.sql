-- 0004_process_engine.sql — Faz 4: süreç şablonları, gruplar, bağımlılıklar,
-- çalıştırmalar ve olaylar (MASTER PLAN §9-15).
-- KESİM/İMALAT/NAKLİYE/MONTAJ KODDA YOKTUR — bunlar seed/örnek veridir.

CREATE TABLE process_templates (
    id          TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces (id),
    name        TEXT NOT NULL,
    code        TEXT NOT NULL,
    description TEXT,
    default_duration INTEGER,
    color       TEXT,
    icon        TEXT,
    active      BOOLEAN NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    UNIQUE (workspace_id, code)
);

CREATE TABLE process_groups (
    id          TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces (id),
    name        TEXT NOT NULL,
    description TEXT,
    active      BOOLEAN NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE process_group_steps (
    id          TEXT PRIMARY KEY,
    process_group_id TEXT NOT NULL REFERENCES process_groups (id),
    process_template_id TEXT NOT NULL REFERENCES process_templates (id),
    sort_order  INTEGER NOT NULL,
    required    BOOLEAN NOT NULL DEFAULT 1,
    UNIQUE (process_group_id, sort_order)
);

CREATE TABLE process_dependencies (
    id          TEXT PRIMARY KEY,
    process_group_step_id TEXT NOT NULL REFERENCES process_group_steps (id),
    depends_on_process_group_step_id TEXT NOT NULL REFERENCES process_group_steps (id),
    required_status TEXT NOT NULL DEFAULT 'COMPLETED',
    CHECK (process_group_step_id <> depends_on_process_group_step_id)
);
CREATE INDEX idx_deps_step ON process_dependencies (process_group_step_id);

CREATE TABLE process_executions (
    id          TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces (id),
    work_item_id TEXT NOT NULL REFERENCES work_items (id),
    process_template_id TEXT NOT NULL REFERENCES process_templates (id),
    process_group_step_id TEXT NOT NULL REFERENCES process_group_steps (id),

    status      TEXT NOT NULL DEFAULT 'PENDING' CHECK (status IN
                ('PENDING', 'READY', 'IN_PROGRESS', 'PAUSED', 'BLOCKED', 'COMPLETED', 'CANCELLED')),
    status_before_block TEXT,
    version     INTEGER NOT NULL DEFAULT 0,   -- optimistic locking (MASTER PLAN §40)

    assigned_user_id TEXT NULL REFERENCES users (id),
    assigned_team_id TEXT NULL REFERENCES teams (id),
    planned_start_at TEXT,
    planned_end_at   TEXT,
    ready_at    TEXT,
    started_at  TEXT,
    completed_at TEXT,

    revision_no INTEGER NOT NULL DEFAULT 0,   -- revizyon zinciri (Faz 5+)
    parent_execution_id TEXT NULL REFERENCES process_executions (id),

    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    deleted_at  TEXT
);
CREATE INDEX idx_exec_work_item ON process_executions (work_item_id);
CREATE INDEX idx_exec_status ON process_executions (workspace_id, status);
CREATE INDEX idx_exec_assigned_user ON process_executions (assigned_user_id, status);
CREATE INDEX idx_exec_assigned_team ON process_executions (assigned_team_id, status);

-- IMMUTABLE audit trail: bu tabloya yalnız INSERT yapılır (MASTER PLAN §15, §42).
CREATE TABLE process_events (
    id          TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    process_execution_id TEXT NOT NULL REFERENCES process_executions (id),
    event_type  TEXT NOT NULL CHECK (event_type IN
                ('CREATED', 'READY', 'ASSIGNED', 'STARTED', 'PAUSED', 'RESUMED', 'BLOCKED',
                 'UNBLOCKED', 'COMPLETED', 'REOPENED', 'CANCELLED', 'NOTE_ADDED', 'FILE_ADDED',
                 'ASSIGNEE_CHANGED', 'PLANNED_DATE_CHANGED')),
    previous_status TEXT,
    new_status  TEXT,
    user_id     TEXT NOT NULL,
    team_id     TEXT,
    timestamp   TEXT NOT NULL,
    note        TEXT,
    metadata    TEXT,
    created_at  TEXT NOT NULL
);
CREATE INDEX idx_pevents_exec ON process_events (process_execution_id, timestamp);
