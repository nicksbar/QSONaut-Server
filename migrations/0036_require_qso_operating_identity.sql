-- Every QSO must identify the managed operating identity explicitly. The
-- previous trigger inferred a personal identity when fields were omitted;
-- that made policy ownership and reports ambiguous.
CREATE OR REPLACE FUNCTION validate_qso_managed_identity() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.callsign_id IS NULL OR NULLIF(trim(NEW.operating_callsign), '') IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'operating callsign and managed identity are required';
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
