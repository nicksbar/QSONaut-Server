ALTER TABLE device_tokens
    ADD COLUMN scopes JSONB NOT NULL DEFAULT '["events:read", "messages:read", "messages:write", "presence:write", "logs:write", "diagnostics:write"]'::jsonb;

ALTER TABLE device_tokens
    ADD CONSTRAINT device_tokens_scopes_array CHECK (jsonb_typeof(scopes) = 'array');
