CREATE TABLE device_authorization_grants (
    id UUID PRIMARY KEY,
    client_id TEXT NOT NULL,
    device_name TEXT NOT NULL,
    client_version TEXT NOT NULL,
    device_code_hash BYTEA NOT NULL UNIQUE,
    user_code_hash BYTEA NOT NULL UNIQUE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'approved', 'denied', 'consumed')),
    expires_at TIMESTAMPTZ NOT NULL,
    interval_seconds INTEGER NOT NULL DEFAULT 5
        CHECK (interval_seconds BETWEEN 1 AND 60),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    approved_at TIMESTAMPTZ,
    consumed_at TIMESTAMPTZ,
    CONSTRAINT device_authorization_client_id_length
        CHECK (char_length(client_id) BETWEEN 1 AND 80),
    CONSTRAINT device_authorization_device_name_length
        CHECK (char_length(device_name) BETWEEN 1 AND 100),
    CONSTRAINT device_authorization_client_version_length
        CHECK (char_length(client_version) BETWEEN 1 AND 40)
);

CREATE INDEX device_authorization_expiry_idx
    ON device_authorization_grants (expires_at);
CREATE INDEX device_authorization_status_idx
    ON device_authorization_grants (status, expires_at);
