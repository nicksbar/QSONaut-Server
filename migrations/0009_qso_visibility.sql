ALTER TABLE qso_logs
    ADD COLUMN visibility TEXT NOT NULL DEFAULT 'private',
    ADD COLUMN visibility_club_id UUID REFERENCES clubs(id) ON DELETE SET NULL;

ALTER TABLE qso_logs
    ADD CONSTRAINT qso_logs_visibility_kind
        CHECK (visibility IN ('private', 'global', 'club', 'contest')),
    ADD CONSTRAINT qso_logs_visibility_target
        CHECK (
            (visibility = 'club' AND visibility_club_id IS NOT NULL)
            OR (visibility <> 'club' AND visibility_club_id IS NULL)
        ),
    ADD CONSTRAINT qso_logs_contest_target
        CHECK (visibility <> 'contest' OR event_id IS NOT NULL);

CREATE INDEX qso_logs_visibility_idx ON qso_logs (visibility, occurred_at DESC);
CREATE INDEX qso_logs_visibility_club_idx ON qso_logs (visibility_club_id, occurred_at DESC);
