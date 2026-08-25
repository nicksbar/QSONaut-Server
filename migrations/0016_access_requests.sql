CREATE TABLE access_challenges (
    id UUID PRIMARY KEY,
    question_index INTEGER NOT NULL CHECK (question_index >= 0),
    attempts_remaining SMALLINT NOT NULL DEFAULT 3 CHECK (attempts_remaining BETWEEN 1 AND 3),
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX access_challenges_expiry_idx ON access_challenges (expires_at);

CREATE TABLE access_requests (
    id UUID PRIMARY KEY,
    callsign TEXT NOT NULL,
    email TEXT NOT NULL,
    club_name TEXT NOT NULL DEFAULT '',
    referral_source TEXT NOT NULL DEFAULT '',
    hamdb_display_name TEXT NOT NULL DEFAULT '',
    hamdb_grid TEXT NOT NULL DEFAULT '',
    hamdb_license_class TEXT NOT NULL DEFAULT '',
    hamdb_license_status TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'approved', 'rejected')),
    reviewed_at TIMESTAMPTZ,
    reviewed_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT access_requests_callsign_uppercase CHECK (callsign = upper(callsign)),
    CONSTRAINT access_requests_email_length CHECK (char_length(email) BETWEEN 3 AND 254),
    CONSTRAINT access_requests_club_length CHECK (char_length(club_name) <= 160),
    CONSTRAINT access_requests_referral_length CHECK (char_length(referral_source) <= 240)
);
CREATE INDEX access_requests_status_created_idx ON access_requests (status, created_at DESC);
CREATE INDEX access_requests_callsign_idx ON access_requests (callsign);
CREATE UNIQUE INDEX access_requests_pending_callsign_idx ON access_requests (callsign)
    WHERE status = 'pending';
