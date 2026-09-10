-- Normalize the client spelling before the contest adjudication trigger.
-- This keeps the catalog comparison case-insensitive for values such as 20m.
CREATE FUNCTION normalize_event_qso_band() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    NEW.band = upper(trim(NEW.band));
    RETURN NEW;
END;
$$;

CREATE TRIGGER aaa_normalize_event_qso_band
BEFORE INSERT OR UPDATE OF band ON qso_logs
FOR EACH ROW EXECUTE FUNCTION normalize_event_qso_band();
