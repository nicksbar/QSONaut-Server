-- Activity visibility belongs to the managed operating context, not to the
-- operator who happened to submit a record. This is intentionally a clean
-- replacement: the server is not deployed and the old policy/data columns
-- were never a durable public contract.
DROP TABLE IF EXISTS activity_visibility_policies;

ALTER TABLE qso_logs
    DROP COLUMN IF EXISTS visibility_club_id,
    DROP COLUMN IF EXISTS visibility;

CREATE TABLE activity_visibility_policies (
    id UUID PRIMARY KEY,
    identity_id UUID REFERENCES managed_callsigns(id) ON DELETE CASCADE,
    club_id UUID REFERENCES clubs(id) ON DELETE CASCADE,
    event_id UUID REFERENCES events(id) ON DELETE CASCADE,
    visibility TEXT NOT NULL DEFAULT 'private',
    updated_by_user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT activity_policy_one_target CHECK (
        (identity_id IS NOT NULL)::integer
        + (club_id IS NOT NULL)::integer
        + (event_id IS NOT NULL)::integer = 1
    ),
    CONSTRAINT activity_policy_visibility_kind
        CHECK (visibility IN ('private', 'members', 'global'))
);

CREATE UNIQUE INDEX activity_policy_identity_idx
    ON activity_visibility_policies (identity_id)
    WHERE identity_id IS NOT NULL;
CREATE UNIQUE INDEX activity_policy_club_idx
    ON activity_visibility_policies (club_id)
    WHERE club_id IS NOT NULL;
CREATE UNIQUE INDEX activity_policy_event_idx
    ON activity_visibility_policies (event_id)
    WHERE event_id IS NOT NULL;
