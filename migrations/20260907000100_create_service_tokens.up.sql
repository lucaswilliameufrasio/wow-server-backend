CREATE TABLE IF NOT EXISTS auth_service_tokens (
    id BIGSERIAL PRIMARY KEY,
    token_hash CHAR(64) NOT NULL UNIQUE,
    name TEXT NOT NULL,
    created_by BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMPTZ NULL DEFAULT NULL,
    revoked_at TIMESTAMPTZ NULL DEFAULT NULL,
    last_used_at TIMESTAMPTZ NULL DEFAULT NULL
);

CREATE INDEX IF NOT EXISTS idx_service_tokens_created_by
    ON auth_service_tokens(created_by);

CREATE INDEX IF NOT EXISTS idx_service_tokens_expires_at
    ON auth_service_tokens(expires_at);
