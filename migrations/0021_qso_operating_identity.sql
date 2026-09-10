ALTER TABLE qso_logs
    ADD COLUMN operating_callsign TEXT,
    ADD COLUMN callsign_id UUID REFERENCES managed_callsigns(id) ON DELETE RESTRICT;

ALTER TABLE qso_logs
    ADD CONSTRAINT qso_logs_operating_callsign_uppercase
        CHECK (operating_callsign IS NULL OR operating_callsign = upper(operating_callsign));

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
    IF NEW.operating_callsign IS NULL OR NEW.callsign_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'operating callsign identity is required for event QSO submission';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM event_participants p
        JOIN managed_callsigns c ON c.id = p.callsign_id
        WHERE p.event_id = NEW.event_id AND p.user_id = NEW.user_id
          AND p.callsign_id = NEW.callsign_id
          AND p.operating_callsign = NEW.operating_callsign
          AND p.status IN ('active', 'completed')
          AND p.starts_at IS NOT NULL AND (p.ends_at IS NULL OR NEW.occurred_at < p.ends_at)
          AND NEW.occurred_at >= p.starts_at
          AND c.status = 'active' AND c.verification_status = 'verified'
    ) THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'operating callsign is not assigned to this event participant';
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
