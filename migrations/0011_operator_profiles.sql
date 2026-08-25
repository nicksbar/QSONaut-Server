ALTER TABLE users
    ADD COLUMN grid TEXT NOT NULL DEFAULT '',
    ADD COLUMN qth TEXT NOT NULL DEFAULT '',
    ADD COLUMN first_name TEXT NOT NULL DEFAULT '',
    ADD COLUMN middle_name TEXT NOT NULL DEFAULT '',
    ADD COLUMN surname TEXT NOT NULL DEFAULT '',
    ADD COLUMN suffix TEXT NOT NULL DEFAULT '',
    ADD COLUMN license_class TEXT NOT NULL DEFAULT '',
    ADD COLUMN license_status TEXT NOT NULL DEFAULT '',
    ADD COLUMN license_expires_on DATE,
    ADD COLUMN address_line_1 TEXT NOT NULL DEFAULT '',
    ADD COLUMN address_line_2 TEXT NOT NULL DEFAULT '',
    ADD COLUMN state TEXT NOT NULL DEFAULT '',
    ADD COLUMN postal_code TEXT NOT NULL DEFAULT '',
    ADD COLUMN country TEXT NOT NULL DEFAULT '',
    ADD COLUMN latitude TEXT NOT NULL DEFAULT '',
    ADD COLUMN longitude TEXT NOT NULL DEFAULT '',
    ADD COLUMN hamdb_fetched_at TIMESTAMPTZ;

ALTER TABLE users
    ADD CONSTRAINT users_grid_length CHECK (char_length(grid) <= 16),
    ADD CONSTRAINT users_qth_length CHECK (char_length(qth) <= 160),
    ADD CONSTRAINT users_profile_name_lengths CHECK (
        char_length(first_name) <= 80 AND char_length(middle_name) <= 80
        AND char_length(surname) <= 120 AND char_length(suffix) <= 40
    );
