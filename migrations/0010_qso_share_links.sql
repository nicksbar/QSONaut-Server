CREATE TABLE qso_share_links (
    id UUID PRIMARY KEY,
    qso_log_id UUID NOT NULL REFERENCES qso_logs(id) ON DELETE CASCADE,
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash BYTEA NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX qso_share_links_expiry_idx ON qso_share_links (expires_at)
    WHERE revoked_at IS NULL;
CREATE INDEX qso_share_links_owner_idx ON qso_share_links (created_by, created_at DESC);
