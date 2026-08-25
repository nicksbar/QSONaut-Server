ALTER TABLE users
    ADD COLUMN hamdb_last_error TEXT NOT NULL DEFAULT '';

ALTER TABLE users
    ADD CONSTRAINT users_hamdb_error_length CHECK (char_length(hamdb_last_error) <= 240);
