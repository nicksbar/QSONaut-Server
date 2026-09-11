-- Keep personal and club identities synchronized after the initial registry seed.
CREATE FUNCTION sync_user_managed_callsign() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE' THEN
        UPDATE managed_callsigns
        SET callsign=upper(NEW.callsign), status='active', verification_status='verified', updated_at=now()
        WHERE identity_type='personal' AND owner_user_id=NEW.id;
        IF FOUND THEN RETURN NEW; END IF;
    END IF;
    INSERT INTO managed_callsigns
        (id, callsign, identity_type, owner_user_id, status, verification_status, authority)
    VALUES
        (gen_random_uuid(), upper(NEW.callsign), 'personal', NEW.id, 'active', 'verified', 'server-user');
    RETURN NEW;
END;
$$;

CREATE TRIGGER users_managed_callsign_sync
AFTER INSERT OR UPDATE OF callsign ON users
FOR EACH ROW EXECUTE FUNCTION sync_user_managed_callsign();

CREATE FUNCTION sync_club_managed_callsign() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    identity_id UUID;
BEGIN
    SELECT id INTO identity_id
    FROM managed_callsigns
    WHERE identity_type='club' AND club_id=NEW.id
    ORDER BY created_at
    LIMIT 1;
    IF NEW.callsign IS NULL OR trim(NEW.callsign) = '' THEN
        IF identity_id IS NOT NULL THEN
            UPDATE managed_callsigns SET status='revoked', updated_at=now() WHERE id=identity_id;
        END IF;
        RETURN NEW;
    END IF;
    IF identity_id IS NULL THEN
        INSERT INTO managed_callsigns
            (id, callsign, identity_type, club_id, status, verification_status, authority)
        VALUES
            (gen_random_uuid(), upper(NEW.callsign), 'club', NEW.id, 'active', 'verified', 'club-record');
    ELSE
        UPDATE managed_callsigns
        SET callsign=upper(NEW.callsign), status='active', verification_status='verified', updated_at=now()
        WHERE id=identity_id;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER clubs_managed_callsign_sync
AFTER INSERT OR UPDATE OF callsign ON clubs
FOR EACH ROW EXECUTE FUNCTION sync_club_managed_callsign();

CREATE FUNCTION propagate_managed_callsign_change() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    UPDATE event_participants
    SET operating_callsign=NEW.callsign, updated_at=now()
    WHERE callsign_id=NEW.id AND operating_callsign IS DISTINCT FROM NEW.callsign;
    RETURN NEW;
END;
$$;

CREATE TRIGGER managed_callsign_assignment_sync
AFTER UPDATE OF callsign ON managed_callsigns
FOR EACH ROW EXECUTE FUNCTION propagate_managed_callsign_change();

CREATE FUNCTION enforce_event_callsign_identity_match() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.event_id IS NULL OR NEW.callsign_id IS NULL OR NEW.operating_callsign IS NULL THEN
        RETURN NEW;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM managed_callsigns c
        WHERE c.id=NEW.callsign_id AND c.callsign=upper(trim(NEW.operating_callsign))
          AND (c.event_id IS NULL OR c.event_id=NEW.event_id)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'operating callsign does not match the managed event identity';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER ccc_event_callsign_identity_match
BEFORE INSERT OR UPDATE ON qso_logs
FOR EACH ROW EXECUTE FUNCTION enforce_event_callsign_identity_match();
