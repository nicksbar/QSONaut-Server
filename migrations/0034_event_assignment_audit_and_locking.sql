-- Preserve event-definition consistency once operators or logs depend on it,
-- and retain an immutable audit trail for every station assignment mutation.
CREATE FUNCTION prevent_bound_event_reconfiguration() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF (
        NEW.club_id IS DISTINCT FROM OLD.club_id
        OR NEW.special_callsign IS DISTINCT FROM OLD.special_callsign
        OR NEW.starts_at IS DISTINCT FROM OLD.starts_at
        OR NEW.ends_at IS DISTINCT FROM OLD.ends_at
        OR NEW.contest_template_id IS DISTINCT FROM OLD.contest_template_id
        OR NEW.contest_config IS DISTINCT FROM OLD.contest_config
    ) AND (
        EXISTS (SELECT 1 FROM event_participants WHERE event_id=OLD.id)
        OR EXISTS (SELECT 1 FROM qso_logs WHERE event_id=OLD.id)
    ) THEN
        RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'event operating configuration cannot change after assignments or logs exist';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER aaa_bound_event_reconfiguration_guard
BEFORE UPDATE ON events
FOR EACH ROW EXECUTE FUNCTION prevent_bound_event_reconfiguration();

CREATE TABLE event_participant_audit (
    id UUID PRIMARY KEY,
    participant_id UUID NOT NULL,
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE RESTRICT,
    subject_user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    actor_user_id UUID REFERENCES users(id) ON DELETE RESTRICT,
    action TEXT NOT NULL CHECK (action IN ('created', 'changed', 'deleted')),
    details JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX event_participant_audit_event_idx
    ON event_participant_audit (event_id, created_at DESC);

INSERT INTO event_participant_audit
    (id, participant_id, event_id, subject_user_id, action, details, created_at)
SELECT gen_random_uuid(), id, event_id, user_id, 'created',
       jsonb_build_object('source', 'migration seed', 'assignment', to_jsonb(p)), created_at
FROM event_participants p;

CREATE FUNCTION audit_event_participant_change() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    actor UUID;
    assignment event_participants%ROWTYPE;
BEGIN
    actor := NULLIF(current_setting('qsonaut.actor_user_id', true), '')::UUID;
    assignment := CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
    INSERT INTO event_participant_audit
        (id, participant_id, event_id, subject_user_id, actor_user_id, action, details)
    VALUES
        (gen_random_uuid(), assignment.id, assignment.event_id, assignment.user_id, actor,
         CASE TG_OP WHEN 'INSERT' THEN 'created' WHEN 'UPDATE' THEN 'changed' ELSE 'deleted' END,
         jsonb_build_object('before', CASE WHEN TG_OP = 'INSERT' THEN NULL ELSE to_jsonb(OLD) END,
                            'after', CASE WHEN TG_OP = 'DELETE' THEN NULL ELSE to_jsonb(NEW) END));
    RETURN assignment;
END;
$$;

CREATE TRIGGER event_participant_audit_append
AFTER INSERT OR UPDATE OR DELETE ON event_participants
FOR EACH ROW EXECUTE FUNCTION audit_event_participant_change();

CREATE FUNCTION prevent_event_participant_audit_mutation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION USING ERRCODE = 'P1001', MESSAGE = 'event participant audit history is immutable';
END;
$$;

CREATE TRIGGER event_participant_audit_immutable
BEFORE UPDATE OR DELETE ON event_participant_audit
FOR EACH ROW EXECUTE FUNCTION prevent_event_participant_audit_mutation();
