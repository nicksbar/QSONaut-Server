ALTER TABLE events
    ADD COLUMN contest_definition_version INTEGER;

UPDATE events e
SET contest_definition_version = t.definition_version
FROM contest_templates t
WHERE e.contest_template_id = t.id;

ALTER TABLE events
    ADD CONSTRAINT events_definition_version_positive
        CHECK (contest_definition_version IS NULL OR contest_definition_version > 0),
    ADD CONSTRAINT events_definition_requires_template
        CHECK (contest_template_id IS NOT NULL OR contest_definition_version IS NULL);

CREATE OR REPLACE FUNCTION validate_event_contest_definition() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    selected_template contest_templates%ROWTYPE;
    field_key TEXT;
    field_definition JSONB;
    configured_value TEXT;
BEGIN
    IF NEW.contest_template_id IS NULL THEN
        NEW.contest_definition_version := NULL;
        NEW.contest_config := '{}';
        RETURN NEW;
    END IF;

    SELECT * INTO selected_template
    FROM contest_templates
    WHERE id = NEW.contest_template_id AND is_active
    FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'event contest definition is unavailable';
    END IF;
    IF jsonb_typeof(NEW.contest_config) <> 'object' THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'contest configuration must be an object';
    END IF;

    FOR field_key, field_definition IN
        SELECT key, value FROM jsonb_each(selected_template.required_fields)
    LOOP
        configured_value := trim(COALESCE(NEW.contest_config->>field_key, ''));
        IF COALESCE((field_definition->>'required')::boolean, false) AND configured_value = '' THEN
            RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = format('contest field %s is required', field_key);
        END IF;
        IF configured_value <> '' AND jsonb_typeof(field_definition->'options') = 'array'
           AND NOT EXISTS (
               SELECT 1 FROM jsonb_array_elements_text(field_definition->'options') option
               WHERE option = configured_value
           ) THEN
            RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = format('invalid value for contest field %s', field_key);
        END IF;
    END LOOP;

    NEW.contest_definition_version := selected_template.definition_version;
    RETURN NEW;
END;
$$;

CREATE TRIGGER event_contest_definition_validation
BEFORE INSERT OR UPDATE OF contest_template_id, contest_config ON events
FOR EACH ROW EXECUTE FUNCTION validate_event_contest_definition();

CREATE OR REPLACE FUNCTION enforce_event_definition_freshness() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    event_version INTEGER;
    template_version INTEGER;
BEGIN
    IF NEW.event_id IS NULL THEN
        RETURN NEW;
    END IF;
    SELECT e.contest_definition_version, t.definition_version
    INTO event_version, template_version
    FROM events e
    LEFT JOIN contest_templates t ON t.id = e.contest_template_id AND t.is_active
    WHERE e.id = NEW.event_id;
    IF event_version IS DISTINCT FROM template_version THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'event contest definition is stale';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER bbb_event_definition_freshness
BEFORE INSERT OR UPDATE OF event_id ON qso_logs
FOR EACH ROW EXECUTE FUNCTION enforce_event_definition_freshness();
