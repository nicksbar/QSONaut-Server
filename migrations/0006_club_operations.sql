ALTER TABLE club_members
    ADD COLUMN membership_status TEXT NOT NULL DEFAULT 'active'
        CHECK (membership_status IN ('active', 'lapsed', 'inactive')),
    ADD COLUMN dues_status TEXT NOT NULL DEFAULT 'not_tracked'
        CHECK (dues_status IN ('not_tracked', 'current', 'due', 'overdue', 'waived')),
    ADD COLUMN membership_number TEXT,
    ADD COLUMN renewal_due_on DATE,
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

CREATE TABLE club_join_requests (
    id UUID PRIMARY KEY,
    club_id UUID NOT NULL REFERENCES clubs(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'approved', 'rejected', 'cancelled')),
    requested_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    reviewed_at TIMESTAMPTZ,
    reviewed_by UUID REFERENCES users(id) ON DELETE SET NULL
);

CREATE UNIQUE INDEX club_join_requests_one_pending_idx
    ON club_join_requests (club_id, user_id)
    WHERE status = 'pending';
CREATE INDEX club_join_requests_club_status_idx
    ON club_join_requests (club_id, status, requested_at);
CREATE INDEX club_members_renewal_idx
    ON club_members (club_id, renewal_due_on)
    WHERE membership_status = 'active';

CREATE TABLE club_positions (
    id UUID PRIMARY KEY,
    club_id UUID NOT NULL REFERENCES clubs(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    position_type TEXT NOT NULL CHECK (position_type IN ('officer', 'board')),
    seats INTEGER NOT NULL DEFAULT 1 CHECK (seats BETWEEN 1 AND 100),
    term_years INTEGER NOT NULL DEFAULT 1 CHECK (term_years BETWEEN 1 AND 10),
    election_parity TEXT NOT NULL DEFAULT 'any'
        CHECK (election_parity IN ('any', 'even', 'odd')),
    description TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (club_id, name)
);

CREATE TABLE club_position_assignments (
    id UUID PRIMARY KEY,
    position_id UUID NOT NULL REFERENCES club_positions(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    seat_number INTEGER NOT NULL DEFAULT 1 CHECK (seat_number > 0),
    starts_on DATE NOT NULL,
    ends_on DATE NOT NULL,
    selection_method TEXT NOT NULL CHECK (selection_method IN ('elected', 'appointed', 'acting')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (ends_on >= starts_on),
    UNIQUE (position_id, seat_number, starts_on)
);
CREATE INDEX club_position_assignments_current_idx
    ON club_position_assignments (position_id, starts_on, ends_on);

CREATE TABLE club_elections (
    id UUID PRIMARY KEY,
    club_id UUID NOT NULL REFERENCES clubs(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    election_year INTEGER NOT NULL CHECK (election_year BETWEEN 2000 AND 2200),
    status TEXT NOT NULL DEFAULT 'planned'
        CHECK (status IN ('planned', 'nominations', 'voting', 'closed', 'certified', 'cancelled')),
    opens_at TIMESTAMPTZ,
    closes_at TIMESTAMPTZ,
    notes TEXT NOT NULL DEFAULT '',
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (closes_at IS NULL OR opens_at IS NULL OR closes_at > opens_at)
);

CREATE TABLE club_election_positions (
    election_id UUID NOT NULL REFERENCES club_elections(id) ON DELETE CASCADE,
    position_id UUID NOT NULL REFERENCES club_positions(id) ON DELETE CASCADE,
    PRIMARY KEY (election_id, position_id)
);
CREATE INDEX club_elections_club_year_idx ON club_elections (club_id, election_year DESC);
