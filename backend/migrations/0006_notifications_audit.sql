-- 0006_notifications_audit.sql — bildirimler (MASTER PLAN §34) ve
-- yönetimsel denetim kayıtları (§59).

CREATE TABLE notifications (
    id          TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces (id),
    user_id     TEXT NOT NULL REFERENCES users (id),
    type        TEXT NOT NULL,
    title       TEXT NOT NULL,
    message     TEXT NOT NULL,
    entity_type TEXT,
    entity_id   TEXT,
    read_at     TEXT,
    created_at  TEXT NOT NULL
);
CREATE INDEX idx_notifications_user_unread ON notifications (user_id, read_at);

CREATE TABLE admin_audit_logs (
    id          TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces (id),
    user_id     TEXT NOT NULL REFERENCES users (id),
    action      TEXT NOT NULL,
    entity_type TEXT,
    entity_id   TEXT,
    metadata    TEXT,
    timestamp   TEXT NOT NULL
);
CREATE INDEX idx_audit_ws_time ON admin_audit_logs (workspace_id, timestamp DESC);
