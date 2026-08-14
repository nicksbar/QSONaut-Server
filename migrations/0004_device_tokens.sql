CREATE TABLE device_tokens (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_name TEXT NOT NULL,
    token_hash BYTEA NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT device_tokens_name_length CHECK (char_length(device_name) BETWEEN 1 AND 100)
);

CREATE INDEX device_tokens_user_idx ON device_tokens (user_id, created_at DESC);
CREATE INDEX device_tokens_expiry_idx ON device_tokens (expires_at);
