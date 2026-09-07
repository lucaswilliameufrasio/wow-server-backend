CREATE TABLE IF NOT EXISTS admin_audit_log (
    id BIGSERIAL PRIMARY KEY,
    actor_account_id BIGINT NOT NULL,
    action VARCHAR(64) NOT NULL,
    target_type VARCHAR(32) NOT NULL,
    target_id VARCHAR(64) NULL DEFAULT NULL,
    details JSONB NULL DEFAULT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_audit_log_actor
    ON admin_audit_log(actor_account_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_audit_log_action
    ON admin_audit_log(action, created_at DESC);
