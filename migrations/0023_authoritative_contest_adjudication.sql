ALTER TABLE qso_logs
    ADD COLUMN is_duplicate BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN multipliers JSONB NOT NULL DEFAULT '{}',
    ADD COLUMN scoring_version TEXT,
    ADD COLUMN scoring_explanation TEXT NOT NULL DEFAULT '';

CREATE OR REPLACE FUNCTION authorize_event_qso() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    selected_event events%ROWTYPE;
    selected_template contest_templates%ROWTYPE;
    assigned_operator TEXT;
    required_key TEXT;
    mode_key TEXT;
    mode_group TEXT;
    duplicate_rule TEXT;
    base_points INTEGER;
BEGIN
    IF NEW.event_id IS NULL THEN
        RETURN NEW;
    END IF;
    SELECT * INTO selected_event FROM events WHERE id = NEW.event_id FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'event is unavailable for QSO submission';
    END IF;
    IF NEW.operating_callsign IS NULL OR NEW.callsign_id IS NULL THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'operating callsign identity is required for event QSO submission';
    END IF;
    SELECT p.operator_callsign INTO assigned_operator
    FROM event_participants p
    JOIN managed_callsigns c ON c.id = p.callsign_id
    WHERE p.event_id = NEW.event_id AND p.user_id = NEW.user_id
      AND p.callsign_id = NEW.callsign_id
      AND p.operating_callsign = NEW.operating_callsign
      AND p.status IN ('active', 'completed')
      AND p.starts_at IS NOT NULL AND (p.ends_at IS NULL OR NEW.occurred_at < p.ends_at)
      AND NEW.occurred_at >= p.starts_at
      AND c.status = 'active' AND c.verification_status = 'verified';
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'operating callsign is not assigned to this event participant';
    END IF;
    IF selected_event.status NOT IN ('scheduled', 'active', 'completed') THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'event is not accepting QSO submissions';
    END IF;
    IF NEW.occurred_at < selected_event.starts_at OR NEW.occurred_at >= selected_event.ends_at THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'QSO time is outside the event window';
    END IF;
    IF selected_event.contest_template_id IS NULL THEN
        RETURN NEW;
    END IF;
    SELECT * INTO selected_template
    FROM contest_templates
    WHERE id = selected_event.contest_template_id AND is_active
    FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'event contest definition is unavailable';
    END IF;
    mode_key := upper(trim(NEW.mode));
    mode_group := CASE
        WHEN mode_key IN ('FT4', 'FT8', 'JT9', 'JT65', 'Q65', 'PSK', 'RTTY') THEN 'DIGITAL'
        WHEN mode_key IN ('SSB', 'FM', 'AM') THEN 'PHONE'
        ELSE mode_key
    END;
    IF NOT EXISTS (
        SELECT 1 FROM jsonb_array_elements_text(selected_template.validation_rules->'bands') value
        WHERE upper(replace(trim(NEW.band), 'M', '')) = upper(value)
    ) AND jsonb_array_length(selected_template.validation_rules->'bands') > 0 THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'QSO band is not allowed by the contest definition';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM jsonb_array_elements_text(selected_template.validation_rules->'modes') value
        WHERE mode_key = upper(value) OR mode_group = upper(value)
    ) AND jsonb_array_length(selected_template.validation_rules->'modes') > 0 THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'QSO mode is not allowed by the contest definition';
    END IF;
    FOR required_key IN
        SELECT value FROM jsonb_array_elements_text(selected_template.validation_rules->'exchange'->'sent')
    LOOP
        IF coalesce(trim(NEW.exchange->'fields_sent'->>required_key), '') = '' THEN
            RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'required sent exchange field is missing';
        END IF;
    END LOOP;
    FOR required_key IN
        SELECT value FROM jsonb_array_elements_text(selected_template.validation_rules->'exchange'->'received')
    LOOP
        IF coalesce(trim(NEW.exchange->'fields_received'->>required_key), '') = '' THEN
            RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'required received exchange field is missing';
        END IF;
    END LOOP;
    duplicate_rule := coalesce(selected_template.validation_rules->>'duplicateRule', 'none');
    mode_group := CASE WHEN mode_group IN ('PHONE', 'DIGITAL') THEN mode_group ELSE mode_key END;
    NEW.is_duplicate := CASE duplicate_rule
        WHEN 'band' THEN EXISTS (
            SELECT 1 FROM qso_logs q
            WHERE q.event_id=NEW.event_id AND q.callsign=upper(trim(NEW.callsign))
              AND upper(replace(q.band, 'M', ''))=upper(replace(trim(NEW.band), 'M', ''))
              AND NOT q.is_duplicate
        )
        WHEN 'band-mode' THEN EXISTS (
            SELECT 1 FROM qso_logs q
            WHERE q.event_id=NEW.event_id AND q.callsign=upper(trim(NEW.callsign))
              AND upper(replace(q.band, 'M', ''))=upper(replace(trim(NEW.band), 'M', ''))
              AND (CASE WHEN q.mode IN ('FT4','FT8','JT9','JT65','Q65','PSK','RTTY') THEN 'DIGITAL' WHEN q.mode IN ('SSB','FM','AM') THEN 'PHONE' ELSE q.mode END)=mode_group
              AND NOT q.is_duplicate
        )
        ELSE false
    END;
    base_points := coalesce(
        (selected_template.scoring_rules->'pointsByMode'->>mode_key)::integer,
        (selected_template.scoring_rules->'pointsByMode'->>mode_group)::integer,
        (selected_template.scoring_rules->>'pointsPerQso')::integer,
        0
    );
    NEW.points := CASE WHEN NEW.is_duplicate THEN 0 ELSE base_points END;
    NEW.multipliers := '{}';
    NEW.scoring_version := selected_template.contest_type || ':' || selected_template.definition_version;
    NEW.scoring_explanation := CASE
        WHEN NEW.is_duplicate THEN 'duplicate contact; zero points'
        ELSE format('%s points from %s definition', base_points, NEW.scoring_version)
    END;
    NEW.operator_callsign = assigned_operator;
    NEW.contest_template_id = selected_event.contest_template_id;
    NEW.contest_definition_version = selected_template.definition_version;
    NEW.contest_config = selected_event.contest_config;
    RETURN NEW;
END;
$$;
