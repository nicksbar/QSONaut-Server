CREATE TABLE managed_callsign_audit (
    id UUID PRIMARY KEY,
    callsign_id UUID NOT NULL REFERENCES managed_callsigns(id) ON DELETE CASCADE,
    actor_user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    action TEXT NOT NULL CHECK (action IN ('registered', 'approved', 'suspended', 'revoked', 'expired')),
    details JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX managed_callsign_audit_identity_idx
    ON managed_callsign_audit (callsign_id, created_at DESC);
