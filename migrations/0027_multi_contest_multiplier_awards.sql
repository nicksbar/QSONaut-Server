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
    SELECT scoring_rules INTO selected_rules
    FROM contest_templates
    WHERE id = NEW.contest_template_id;

    FOR multiplier_rule IN
        SELECT value FROM jsonb_array_elements(COALESCE(selected_rules->'multipliers', '[]'::jsonb))
    LOOP
        multiplier_key := lower(coalesce(multiplier_rule->>'type', ''));
        IF multiplier_key = '' THEN
            CONTINUE;
        END IF;
        multiplier_value := trim(NEW.exchange->'fields_received'->>multiplier_key);
        IF multiplier_value IS NULL OR multiplier_value = '' THEN
            CONTINUE;
        END IF;
        per_band := coalesce((multiplier_rule->>'perBand')::boolean, false);
        per_mode := coalesce((multiplier_rule->>'perMode')::boolean, false);
        mode_group := CASE
            WHEN upper(NEW.mode) IN ('FT4','FT8','JT9','JT65','Q65','PSK','RTTY') THEN 'DIGITAL'
            WHEN upper(NEW.mode) IN ('SSB','FM','AM') THEN 'PHONE'
            ELSE upper(NEW.mode)
        END;
        IF NOT EXISTS (
            SELECT 1 FROM qso_logs q
            WHERE q.event_id = NEW.event_id
              AND NOT q.is_duplicate
              AND lower(trim(q.exchange->'fields_received'->>multiplier_key)) = lower(multiplier_value)
              AND (NOT per_band OR replace(upper(q.band), 'M', '') = replace(upper(NEW.band), 'M', ''))
              AND (NOT per_mode OR (
                  CASE
                      WHEN upper(q.mode) IN ('FT4','FT8','JT9','JT65','Q65','PSK','RTTY') THEN 'DIGITAL'
                      WHEN upper(q.mode) IN ('SSB','FM','AM') THEN 'PHONE'
                      ELSE upper(q.mode)
                  END
              ) = mode_group)
        ) THEN
            NEW.multipliers := NEW.multipliers || jsonb_build_object(multiplier_key, multiplier_value);
            NEW.scoring_explanation := concat_ws('; ', NULLIF(NEW.scoring_explanation, ''), format('%s multiplier earned: %s', multiplier_key, multiplier_value));
        END IF;
    END LOOP;
    RETURN NEW;
END;
$$;
