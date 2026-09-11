ALTER TABLE managed_callsigns
    ADD COLUMN event_id UUID REFERENCES events(id) ON DELETE SET NULL;

INSERT INTO managed_callsigns
    (id, callsign, identity_type, club_id, event_id, status, verification_status, authority, effective_from, expires_at)
SELECT gen_random_uuid(), upper(e.special_callsign), 'special', e.club_id, e.id,
       'active', 'verified', 'event-record', e.starts_at, e.ends_at
FROM events e
WHERE e.special_callsign IS NOT NULL AND trim(e.special_callsign) <> ''
ON CONFLICT DO NOTHING;

CREATE UNIQUE INDEX managed_callsigns_event_special_idx
    ON managed_callsigns (event_id)
    WHERE identity_type = 'special' AND event_id IS NOT NULL;
