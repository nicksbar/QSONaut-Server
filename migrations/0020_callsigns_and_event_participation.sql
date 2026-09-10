-- Managed operating identities and explicit event participation.
-- A club membership is not, by itself, permission to operate a callsign.
CREATE TABLE managed_callsigns (
    id UUID PRIMARY KEY,
    callsign TEXT NOT NULL,
    identity_type TEXT NOT NULL CHECK (identity_type IN ('personal', 'club', 'special')),
    owner_user_id UUID REFERENCES users(id) ON DELETE RESTRICT,
    club_id UUID REFERENCES clubs(id) ON DELETE RESTRICT,
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('pending', 'active', 'suspended', 'expired', 'revoked')),
    effective_from TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ,
    verification_status TEXT NOT NULL DEFAULT 'pending'
        CHECK (verification_status IN ('pending', 'verified', 'rejected')),
    authority TEXT NOT NULL DEFAULT '',
    authority_reference TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT managed_callsigns_uppercase CHECK (callsign = upper(callsign)),
    CONSTRAINT managed_callsigns_window CHECK (expires_at IS NULL OR expires_at > effective_from),
    CONSTRAINT managed_callsigns_owner CHECK (
        (identity_type = 'personal' AND owner_user_id IS NOT NULL AND club_id IS NULL)
        OR (identity_type IN ('club', 'special') AND club_id IS NOT NULL)
    )
);
CREATE UNIQUE INDEX managed_callsigns_active_identity_idx
    ON managed_callsigns (callsign)
    WHERE status IN ('pending', 'active') AND verification_status <> 'rejected';
CREATE INDEX managed_callsigns_owner_idx ON managed_callsigns (owner_user_id, status);
CREATE INDEX managed_callsigns_club_idx ON managed_callsigns (club_id, status);

INSERT INTO managed_callsigns
    (id, callsign, identity_type, owner_user_id, status, verification_status, authority)
SELECT gen_random_uuid(), callsign, 'personal', id, 'active', 'verified', 'server-user'
FROM users
ON CONFLICT DO NOTHING;

INSERT INTO managed_callsigns
    (id, callsign, identity_type, club_id, status, verification_status, authority)
SELECT gen_random_uuid(), callsign, 'club', id, 'active', 'verified', 'club-record'
FROM clubs
WHERE callsign IS NOT NULL
ON CONFLICT DO NOTHING;

CREATE TABLE event_participants (
    id UUID PRIMARY KEY,
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    club_id UUID NOT NULL REFERENCES clubs(id) ON DELETE RESTRICT,
    callsign_id UUID NOT NULL REFERENCES managed_callsigns(id) ON DELETE RESTRICT,
    operator_callsign TEXT NOT NULL,
    operating_callsign TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('operator', 'coordinator', 'logger', 'observer')),
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'invited', 'active', 'suspended', 'withdrawn', 'completed')),
    starts_at TIMESTAMPTZ,
    ends_at TIMESTAMPTZ,
    station_label TEXT NOT NULL DEFAULT '',
    band TEXT,
    mode TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT event_participants_window CHECK (ends_at IS NULL OR starts_at IS NULL OR ends_at > starts_at),
    CONSTRAINT event_participants_callsigns CHECK (operator_callsign = upper(operator_callsign) AND operating_callsign = upper(operating_callsign))
);
CREATE UNIQUE INDEX event_participants_user_event_idx ON event_participants (event_id, user_id);
CREATE INDEX event_participants_event_status_idx ON event_participants (event_id, status);
CREATE INDEX event_participants_callsign_idx ON event_participants (callsign_id, status);

CREATE FUNCTION validate_event_participant() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    selected_event events%ROWTYPE;
    selected_identity managed_callsigns%ROWTYPE;
BEGIN
    SELECT * INTO selected_event FROM events WHERE id = NEW.event_id FOR SHARE;
    IF NOT FOUND OR selected_event.club_id <> NEW.club_id THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'event and participant club do not match';
    END IF;
    SELECT * INTO selected_identity FROM managed_callsigns WHERE id = NEW.callsign_id FOR SHARE;
    IF NOT FOUND OR selected_identity.club_id IS DISTINCT FROM NEW.club_id
       OR selected_identity.status <> 'active'
       OR selected_identity.verification_status <> 'verified'
       OR selected_identity.effective_from > COALESCE(NEW.starts_at, selected_event.starts_at)
       OR (selected_identity.expires_at IS NOT NULL AND selected_identity.expires_at <= COALESCE(NEW.ends_at, selected_event.ends_at)) THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'operating callsign is not an active verified club identity';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM club_members
        WHERE club_id = NEW.club_id AND user_id = NEW.user_id
          AND membership_status = 'active' AND role IN ('owner', 'coordinator', 'operator')
    ) THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'active operating membership is required';
    END IF;
    IF selected_identity.callsign <> NEW.operating_callsign THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'participant callsign does not match managed identity';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER event_participant_validation
BEFORE INSERT OR UPDATE OF event_id, user_id, club_id, callsign_id, operator_callsign, operating_callsign, starts_at, ends_at
ON event_participants FOR EACH ROW EXECUTE FUNCTION validate_event_participant();

CREATE OR REPLACE FUNCTION authorize_event_qso() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    selected_event events%ROWTYPE;
BEGIN
    IF NEW.event_id IS NULL THEN
        RETURN NEW;
    END IF;
    SELECT * INTO selected_event FROM events WHERE id = NEW.event_id FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'event is unavailable for QSO submission';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM event_participants p
        WHERE p.event_id = NEW.event_id AND p.user_id = NEW.user_id
          AND p.status IN ('active', 'completed')
          AND p.starts_at IS NOT NULL AND (p.ends_at IS NULL OR NEW.occurred_at < p.ends_at)
          AND NEW.occurred_at >= p.starts_at
    ) THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'explicit event participant assignment is required';
    END IF;
    IF selected_event.status NOT IN ('scheduled', 'active', 'completed') THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'event is not accepting QSO submissions';
    END IF;
    IF NEW.occurred_at < selected_event.starts_at OR NEW.occurred_at >= selected_event.ends_at THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'QSO time is outside the event window';
    END IF;
    RETURN NEW;
END;
$$;
