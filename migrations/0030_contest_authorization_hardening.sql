-- Close authorization and adjudication gaps discovered during cross-client review.
-- Existing migrations remain immutable because deployed SQLx checksums depend on them.

CREATE FUNCTION canonical_contest_band(band_value TEXT) RETURNS TEXT
LANGUAGE sql IMMUTABLE PARALLEL SAFE AS $$
    SELECT CASE
        WHEN upper(trim(band_value)) ~ '^[0-9]+([.][0-9]+)?M$'
            THEN left(upper(trim(band_value)), length(trim(band_value)) - 1)
        ELSE upper(trim(band_value))
    END
$$;

CREATE OR REPLACE FUNCTION validate_event_participant() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    selected_event events%ROWTYPE;
    selected_identity managed_callsigns%ROWTYPE;
    member_role TEXT;
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
    SELECT role INTO member_role
    FROM club_members
    WHERE club_id = NEW.club_id AND user_id = NEW.user_id AND membership_status = 'active'
    FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'active club membership is required';
    END IF;
    IF NEW.role <> 'observer' AND member_role NOT IN ('owner', 'coordinator', 'operator') THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'active operating membership is required';
    END IF;
    IF selected_identity.callsign <> NEW.operating_callsign THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'participant callsign does not match managed identity';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER event_participant_validation ON event_participants;
CREATE TRIGGER event_participant_validation
BEFORE INSERT OR UPDATE ON event_participants
FOR EACH ROW EXECUTE FUNCTION validate_event_participant();

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
    mode_key := upper(trim(NEW.mode));
    mode_group := CASE
        WHEN mode_key IN ('FT4', 'FT8', 'JT9', 'JT65', 'Q65', 'PSK', 'PSK31', 'PSK63', 'RTTY', 'JS8', 'FST4', 'MSK144', 'DIGITAL') THEN 'DIGITAL'
        WHEN mode_key IN ('SSB', 'USB', 'LSB', 'VOICE', 'PHONE', 'FM', 'AM') THEN 'PHONE'
        ELSE mode_key
    END;
    SELECT p.operator_callsign INTO assigned_operator
    FROM event_participants p
    JOIN managed_callsigns c ON c.id = p.callsign_id
    WHERE p.event_id = NEW.event_id AND p.user_id = NEW.user_id
      AND p.club_id = selected_event.club_id
      AND p.callsign_id = NEW.callsign_id
      AND p.operating_callsign = NEW.operating_callsign
      AND p.role IN ('operator', 'coordinator', 'logger')
      AND p.status IN ('active', 'completed')
      AND p.starts_at IS NOT NULL AND (p.ends_at IS NULL OR NEW.occurred_at < p.ends_at)
      AND NEW.occurred_at >= p.starts_at
      AND (p.band IS NULL OR canonical_contest_band(p.band) = canonical_contest_band(NEW.band))
      AND (p.mode IS NULL OR upper(trim(p.mode)) IN (mode_key, mode_group))
      AND c.club_id = selected_event.club_id
      AND c.status = 'active' AND c.verification_status = 'verified'
      AND c.effective_from <= NEW.occurred_at
      AND (c.expires_at IS NULL OR NEW.occurred_at < c.expires_at);
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'active station assignment does not authorize this QSO';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM club_members
        WHERE club_id = selected_event.club_id AND user_id = NEW.user_id
          AND membership_status = 'active' AND role IN ('owner', 'coordinator', 'operator')
    ) THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'active operating membership is required for this event';
    END IF;
    IF selected_event.status NOT IN ('scheduled', 'active', 'completed') THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'event is not accepting QSO submissions';
    END IF;
    IF NEW.occurred_at < selected_event.starts_at OR NEW.occurred_at >= selected_event.ends_at THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'QSO time is outside the event window';
    END IF;

    NEW.operator_callsign := assigned_operator;
    IF selected_event.contest_template_id IS NULL THEN
        NEW.contest_template_id := NULL;
        NEW.contest_definition_version := NULL;
        NEW.contest_config := '{}';
        NEW.is_duplicate := false;
        NEW.multipliers := '{}';
        NEW.scoring_version := NULL;
        NEW.scoring_explanation := '';
        RETURN NEW;
    END IF;
    SELECT * INTO selected_template
    FROM contest_templates
    WHERE id = selected_event.contest_template_id AND is_active
    FOR SHARE;
    IF NOT FOUND THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'event contest definition is unavailable';
    END IF;
    IF selected_event.contest_definition_version IS DISTINCT FROM selected_template.definition_version THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'event contest definition is stale';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM jsonb_array_elements_text(COALESCE(selected_template.validation_rules->'bands', '[]'::jsonb)) value
        WHERE canonical_contest_band(NEW.band) = canonical_contest_band(value)
    ) AND jsonb_array_length(COALESCE(selected_template.validation_rules->'bands', '[]'::jsonb)) > 0 THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'QSO band is not allowed by the contest definition';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM jsonb_array_elements_text(COALESCE(selected_template.validation_rules->'modes', '[]'::jsonb)) value
        WHERE mode_key = upper(value) OR mode_group = upper(value)
           OR (upper(value) = 'DIGITAL_NO_RTTY' AND mode_group = 'DIGITAL' AND mode_key <> 'RTTY')
           OR (upper(value) = 'MIXED' AND mode_group IN ('CW', 'PHONE', 'DIGITAL'))
    ) AND jsonb_array_length(COALESCE(selected_template.validation_rules->'modes', '[]'::jsonb)) > 0 THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'QSO mode is not allowed by the contest definition';
    END IF;
    FOR required_key IN
        SELECT value FROM jsonb_array_elements_text(COALESCE(selected_template.validation_rules->'exchange'->'sent', '[]'::jsonb))
    LOOP
        IF coalesce(trim(NEW.exchange->'fields_sent'->>required_key), '') = '' THEN
            RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'required sent exchange field is missing';
        END IF;
    END LOOP;
    FOR required_key IN
        SELECT value FROM jsonb_array_elements_text(COALESCE(selected_template.validation_rules->'exchange'->'received', '[]'::jsonb))
    LOOP
        IF coalesce(trim(NEW.exchange->'fields_received'->>required_key), '') = '' THEN
            RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'required received exchange field is missing';
        END IF;
    END LOOP;
    duplicate_rule := coalesce(selected_template.validation_rules->>'duplicateRule', 'none');
    NEW.is_duplicate := CASE duplicate_rule
        WHEN 'band' THEN EXISTS (
            SELECT 1 FROM qso_logs q
            WHERE q.id <> NEW.id AND q.event_id = NEW.event_id
              AND q.callsign = upper(trim(NEW.callsign))
              AND canonical_contest_band(q.band) = canonical_contest_band(NEW.band)
              AND NOT q.is_duplicate
        )
        WHEN 'band-mode' THEN EXISTS (
            SELECT 1 FROM qso_logs q
            WHERE q.id <> NEW.id AND q.event_id = NEW.event_id
              AND q.callsign = upper(trim(NEW.callsign))
              AND canonical_contest_band(q.band) = canonical_contest_band(NEW.band)
              AND (CASE
                  WHEN upper(q.mode) IN ('FT4','FT8','JT9','JT65','Q65','PSK','PSK31','PSK63','RTTY','JS8','FST4','MSK144','DIGITAL') THEN 'DIGITAL'
                  WHEN upper(q.mode) IN ('SSB','USB','LSB','VOICE','PHONE','FM','AM') THEN 'PHONE'
                  ELSE upper(q.mode)
              END) = mode_group
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
    NEW.contest_template_id := selected_event.contest_template_id;
    NEW.contest_definition_version := selected_template.definition_version;
    NEW.contest_config := selected_event.contest_config;
    RETURN NEW;
END;
$$;

DROP TRIGGER qso_event_authorization ON qso_logs;
CREATE TRIGGER qso_event_authorization
BEFORE INSERT OR UPDATE ON qso_logs
FOR EACH ROW EXECUTE FUNCTION authorize_event_qso();

CREATE OR REPLACE FUNCTION award_contest_multiplier() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    selected_rules JSONB;
    multiplier_rule JSONB;
    multiplier_key TEXT;
    multiplier_value TEXT;
    per_band BOOLEAN;
    per_mode BOOLEAN;
    mode_group TEXT;
BEGIN
    IF NEW.event_id IS NULL OR NEW.is_duplicate OR NEW.contest_template_id IS NULL THEN
        RETURN NEW;
    END IF;
    SELECT scoring_rules INTO selected_rules FROM contest_templates WHERE id = NEW.contest_template_id;
    FOR multiplier_rule IN
        SELECT value FROM jsonb_array_elements(COALESCE(selected_rules->'multipliers', '[]'::jsonb))
    LOOP
        multiplier_key := lower(coalesce(multiplier_rule->>'type', ''));
        IF multiplier_key = '' THEN CONTINUE; END IF;
        multiplier_value := trim(NEW.exchange->'fields_received'->>multiplier_key);
        IF multiplier_value IS NULL OR multiplier_value = '' THEN CONTINUE; END IF;
        per_band := coalesce((multiplier_rule->>'perBand')::boolean, false);
        per_mode := coalesce((multiplier_rule->>'perMode')::boolean, false);
        mode_group := CASE
            WHEN upper(NEW.mode) IN ('FT4','FT8','JT9','JT65','Q65','PSK','PSK31','PSK63','RTTY','JS8','FST4','MSK144','DIGITAL') THEN 'DIGITAL'
            WHEN upper(NEW.mode) IN ('SSB','USB','LSB','VOICE','PHONE','FM','AM') THEN 'PHONE'
            ELSE upper(NEW.mode)
        END;
        IF NOT EXISTS (
            SELECT 1 FROM qso_logs q
            WHERE q.id <> NEW.id AND q.event_id = NEW.event_id AND NOT q.is_duplicate
              AND lower(trim(q.exchange->'fields_received'->>multiplier_key)) = lower(multiplier_value)
              AND (NOT per_band OR canonical_contest_band(q.band) = canonical_contest_band(NEW.band))
              AND (NOT per_mode OR (CASE
                  WHEN upper(q.mode) IN ('FT4','FT8','JT9','JT65','Q65','PSK','PSK31','PSK63','RTTY','JS8','FST4','MSK144','DIGITAL') THEN 'DIGITAL'
                  WHEN upper(q.mode) IN ('SSB','USB','LSB','VOICE','PHONE','FM','AM') THEN 'PHONE'
                  ELSE upper(q.mode)
              END) = mode_group)
        ) THEN
            NEW.multipliers := NEW.multipliers || jsonb_build_object(multiplier_key, multiplier_value);
            NEW.scoring_explanation := concat_ws('; ', NULLIF(NEW.scoring_explanation, ''), format('%s multiplier earned: %s', multiplier_key, multiplier_value));
        END IF;
    END LOOP;
    RETURN NEW;
END;
$$;

DROP TRIGGER zzz_contest_multiplier_award ON qso_logs;
CREATE TRIGGER zzz_contest_multiplier_award
BEFORE INSERT OR UPDATE ON qso_logs
FOR EACH ROW EXECUTE FUNCTION award_contest_multiplier();

ALTER TABLE managed_callsign_audit
    DROP CONSTRAINT managed_callsign_audit_callsign_id_fkey,
    ADD CONSTRAINT managed_callsign_audit_callsign_id_fkey
        FOREIGN KEY (callsign_id) REFERENCES managed_callsigns(id) ON DELETE RESTRICT;

CREATE FUNCTION prevent_managed_callsign_audit_mutation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'managed callsign audit records are immutable';
END;
$$;

CREATE TRIGGER managed_callsign_audit_immutable
BEFORE UPDATE OR DELETE ON managed_callsign_audit
FOR EACH ROW EXECUTE FUNCTION prevent_managed_callsign_audit_mutation();
