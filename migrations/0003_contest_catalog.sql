ALTER TABLE contest_templates
    ADD COLUMN icon TEXT NOT NULL DEFAULT '🏁',
    ADD COLUMN rules_url TEXT NOT NULL DEFAULT '',
    ADD COLUMN definition_version INTEGER NOT NULL DEFAULT 1 CHECK (definition_version > 0),
    ADD COLUMN is_builtin BOOLEAN NOT NULL DEFAULT false;

UPDATE contest_templates
SET is_builtin = true
WHERE contest_type IN ('ARRL_FD', 'WINTER_FD', 'POTA', 'SOTA');
