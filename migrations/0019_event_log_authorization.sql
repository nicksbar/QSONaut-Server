-- Apply the same event boundary to legacy and current QSO submission paths.
-- Row locks serialize submissions against membership and event changes.
CREATE FUNCTION authorize_event_qso() RETURNS trigger LANGUAGE plpgsql AS $$
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
    PERFORM 1 FROM club_members
        WHERE club_id = selected_event.club_id AND user_id = NEW.user_id
          AND membership_status = 'active' AND role IN ('owner', 'coordinator', 'operator')
        FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'active operating membership is required for this event';
    END IF;
    IF selected_event.status NOT IN ('scheduled', 'active', 'completed') THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'event is not accepting QSO submissions';
    END IF;
    -- Completed events accept delayed uploads of QSOs made within the window.
    IF NEW.occurred_at < selected_event.starts_at OR NEW.occurred_at >= selected_event.ends_at THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'QSO time is outside the event window';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER qso_event_authorization
BEFORE INSERT OR UPDATE OF event_id, user_id, occurred_at ON qso_logs
FOR EACH ROW EXECUTE FUNCTION authorize_event_qso();
