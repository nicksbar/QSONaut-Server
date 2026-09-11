-- Validate managed operating identities for general as well as event QSOs.
CREATE FUNCTION validate_qso_managed_identity() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    personal_id UUID;
    personal_call TEXT;
BEGIN
    IF NEW.event_id IS NULL AND NEW.callsign_id IS NULL AND NEW.operating_callsign IS NULL THEN
        SELECT id,callsign INTO personal_id,personal_call
        FROM managed_callsigns
        WHERE identity_type='personal' AND owner_user_id=NEW.user_id
          AND status='active' AND verification_status='verified'
          AND effective_from <= NEW.occurred_at
          AND (expires_at IS NULL OR NEW.occurred_at < expires_at)
        ORDER BY created_at DESC LIMIT 1;
        IF FOUND THEN
            NEW.callsign_id := personal_id;
            NEW.operating_callsign := personal_call;
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.callsign_id IS NULL OR NEW.operating_callsign IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'operating callsign and managed identity must be supplied together';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM managed_callsigns c
        WHERE c.id=NEW.callsign_id AND c.callsign=upper(trim(NEW.operating_callsign))
          AND c.status='active' AND c.verification_status='verified'
          AND c.effective_from <= NEW.occurred_at
          AND (c.expires_at IS NULL OR NEW.occurred_at < c.expires_at)
          AND (c.event_id IS NULL OR c.event_id=NEW.event_id)
          AND (
              c.owner_user_id=NEW.user_id
              OR EXISTS (
                  SELECT 1 FROM club_members m
                  WHERE m.club_id=c.club_id AND m.user_id=NEW.user_id
                    AND m.membership_status='active'
                    AND m.role IN ('owner','coordinator','operator')
              )
          )
    ) THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'managed operating identity is not authorized for this operator';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER bb0_qso_managed_identity_validation
BEFORE INSERT OR UPDATE ON qso_logs
FOR EACH ROW EXECUTE FUNCTION validate_qso_managed_identity();
