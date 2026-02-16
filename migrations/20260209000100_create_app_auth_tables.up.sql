CREATE TABLE IF NOT EXISTS auth_refresh_tokens (
    id BIGSERIAL PRIMARY KEY,
    account_id BIGINT NOT NULL,
    token_hash CHAR(64) NOT NULL UNIQUE,
    family_id CHAR(36) NOT NULL,
    parent_token_id BIGINT NULL,
    issued_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ NULL DEFAULT NULL,
    replaced_by_token_id BIGINT NULL,
    created_ip VARCHAR(45) NOT NULL,
    user_agent VARCHAR(255) NULL,
    reason VARCHAR(64) NULL,
    CONSTRAINT fk_refresh_parent
        FOREIGN KEY (parent_token_id)
        REFERENCES auth_refresh_tokens(id)
        ON DELETE SET NULL,
    CONSTRAINT fk_refresh_replaced
        FOREIGN KEY (replaced_by_token_id)
        REFERENCES auth_refresh_tokens(id)
        ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_refresh_account_id
    ON auth_refresh_tokens(account_id);
CREATE INDEX IF NOT EXISTS idx_refresh_family_id
    ON auth_refresh_tokens(family_id);
CREATE INDEX IF NOT EXISTS idx_refresh_expires_at
    ON auth_refresh_tokens(expires_at);

CREATE TABLE IF NOT EXISTS auth_revoked_access_tokens (
    jti CHAR(36) NOT NULL PRIMARY KEY,
    account_id BIGINT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reason VARCHAR(64) NULL
);

CREATE INDEX IF NOT EXISTS idx_revoked_account_id
    ON auth_revoked_access_tokens(account_id);
CREATE INDEX IF NOT EXISTS idx_revoked_expires_at
    ON auth_revoked_access_tokens(expires_at);
