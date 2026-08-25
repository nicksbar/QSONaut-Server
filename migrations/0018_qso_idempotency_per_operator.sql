ALTER TABLE qso_logs
    DROP CONSTRAINT qso_logs_idempotency_key_key;

ALTER TABLE qso_logs
    ADD CONSTRAINT qso_logs_user_id_idempotency_key_key
    UNIQUE (user_id, idempotency_key);
