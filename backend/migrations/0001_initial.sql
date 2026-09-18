-- 0001_initial.sql — Faz 1: kimlik, tenant, takım ve proje tabloları
-- Not: Development SQLite. PostgreSQL migration'ları staging öncesi ayrıca portlanır.
-- Kurallar (docs/architecture.md §11): UUIDv7 TEXT, timestamp'ler ISO-8601 UTC TEXT,
-- soft delete deleted_at, tüm sorgular workspace_id sınırlı.

CREATE TABLE workspaces (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    slug        TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE users (
    id            TEXT PRIMARY KEY,
    workspace_id  TEXT NOT NULL REFERENCES workspaces (id),
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    full_name     TEXT NOT NULL,
    role          TEXT NOT NULL CHECK (role IN ('ADMIN', 'PROJECT_MANAGER', 'TEAM_LEADER', 'WORKER', 'VIEWER')),
    active        BOOLEAN NOT NULL DEFAULT 1,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);
CREATE INDEX idx_users_workspace ON users (workspace_id);

CREATE TABLE sessions (
    id          TEXT PRIMARY KEY,               -- cookie token'ının SHA-256 hash'i
    user_id     TEXT NOT NULL REFERENCES users (id),
    workspace_id TEXT NOT NULL REFERENCES workspaces (id),
    created_at  TEXT NOT NULL,
    expires_at  TEXT NOT NULL,
    user_agent  TEXT,
    ip          TEXT
);
CREATE INDEX idx_sessions_user ON sessions (user_id);
CREATE INDEX idx_sessions_expires ON sessions (expires_at);

CREATE TABLE teams (
    id          TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces (id),
    name        TEXT NOT NULL,
    description TEXT,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
CREATE INDEX idx_teams_workspace ON teams (workspace_id);

CREATE TABLE team_members (
    team_id  TEXT NOT NULL REFERENCES teams (id),
    user_id  TEXT NOT NULL REFERENCES users (id),
    role     TEXT NOT NULL DEFAULT 'MEMBER' CHECK (role IN ('LEADER', 'MEMBER')),
    PRIMARY KEY (team_id, user_id)
);
CREATE INDEX idx_team_members_user ON team_members (user_id);

CREATE TABLE projects (
    id                 TEXT PRIMARY KEY,
    workspace_id       TEXT NOT NULL REFERENCES workspaces (id),
    name               TEXT NOT NULL,
    code               TEXT NOT NULL,
    description        TEXT,
    status             TEXT NOT NULL DEFAULT 'DRAFT'
                       CHECK (status IN ('DRAFT', 'ACTIVE', 'PAUSED', 'COMPLETED', 'CANCELLED', 'ARCHIVED')),
    planned_start_date TEXT,
    planned_end_date   TEXT,
    actual_start_date  TEXT,
    actual_end_date    TEXT,
    created_at         TEXT NOT NULL,
    updated_at         TEXT NOT NULL,
    deleted_at         TEXT
);
CREATE INDEX idx_projects_ws_status ON projects (workspace_id, status);
