-- Reject station assignments that cannot be used under the selected contest.
CREATE FUNCTION participant_mode_group(mode_value TEXT) RETURNS TEXT
LANGUAGE sql IMMUTABLE PARALLEL SAFE AS $$
    SELECT CASE
        WHEN upper(trim(mode_value)) IN ('SSB','USB','LSB','VOICE','PHONE','AM','FM') THEN 'PHONE'
        WHEN upper(trim(mode_value)) IN ('FT4','FT8','JT9','JT65','Q65','PSK','PSK31','PSK63','RTTY','JS8','FST4','MSK144','DIGITAL') THEN 'DIGITAL'
        ELSE upper(trim(mode_value))
    END
$$;

CREATE FUNCTION validate_event_participant_catalog_assignment() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    selected_template contest_templates%ROWTYPE;
    assigned_mode TEXT;
    assigned_group TEXT;
BEGIN
    SELECT t.* INTO selected_template
    FROM events e
    JOIN contest_templates t ON t.id = e.contest_template_id AND t.is_active
    WHERE e.id = NEW.event_id;
    IF NOT FOUND THEN
        RETURN NEW;
    END IF;
    IF NEW.band IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM jsonb_array_elements_text(COALESCE(selected_template.validation_rules->'bands', '[]'::jsonb)) value
        WHERE canonical_contest_band(NEW.band) = canonical_contest_band(value)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'station assignment band is not allowed by the contest definition';
    END IF;
    IF NEW.mode IS NULL THEN
        RETURN NEW;
    END IF;
    assigned_mode := upper(trim(NEW.mode));
    assigned_group := participant_mode_group(assigned_mode);
    IF NOT EXISTS (
        SELECT 1 FROM jsonb_array_elements_text(COALESCE(selected_template.validation_rules->'modes', '[]'::jsonb)) value
        WHERE assigned_mode = upper(value) OR assigned_group = upper(value)
           OR (upper(value) = 'DIGITAL_NO_RTTY' AND assigned_group = 'DIGITAL' AND assigned_mode <> 'RTTY')
           OR (upper(value) = 'MIXED' AND assigned_group IN ('CW', 'PHONE', 'DIGITAL'))
    ) THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'station assignment mode is not allowed by the contest definition';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER event_participant_catalog_validation
BEFORE INSERT OR UPDATE OF event_id, band, mode ON event_participants
FOR EACH ROW EXECUTE FUNCTION validate_event_participant_catalog_assignment();
