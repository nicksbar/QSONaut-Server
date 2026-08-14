CREATE TABLE contest_templates (
    id UUID PRIMARY KEY,
    contest_type TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    organization TEXT NOT NULL,
    scoring_rules JSONB NOT NULL DEFAULT '{}',
    required_fields JSONB NOT NULL DEFAULT '{}',
    validation_rules JSONB NOT NULL DEFAULT '{}',
    schedule JSONB NOT NULL DEFAULT '{}',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO contest_templates
    (id, contest_type, name, description, organization, scoring_rules, required_fields, validation_rules, schedule)
VALUES
    (
        '10000000-0000-4000-8000-000000000001', 'ARRL_FD', 'ARRL Field Day',
        'Annual emergency preparedness exercise held on the fourth full weekend of June', 'ARRL',
        '{"pointsPerQso":1,"pointsByMode":{"CW":2,"PHONE":1,"DIGITAL":2},"multipliers":[{"type":"section","perBand":false,"perMode":false,"description":"ARRL/RAC sections worked"}],"formula":"(QSO Points x Power Multiplier) + Bonuses"}',
        '{"class":{"required":true,"options":["1A","2A","3A","4A","5A","6A","1B","1C","1D","1E","2E","3E","1F","2F","3F"],"description":"Transmitter count and operating class"},"section":{"required":true,"description":"ARRL or RAC section"},"power":{"required":true,"options":["HIGH","LOW","QRP"],"description":"Power multiplier category"},"participants":{"required":false,"description":"Number of participants"}}',
        '{"bands":["160","80","40","20","15","10","6","2","1.25","70CM","SAT"],"modes":["CW","PHONE","DIGITAL","SSB","FM","FT8","FT4","PSK","RTTY"],"duplicateRule":"band-mode","exchange":{"sent":["class","section"],"received":["class","section"]}}',
        '{"type":"relative","summary":"Fourth full weekend of June","durationHours":27,"timezone":"UTC"}'
    ),
    (
        '10000000-0000-4000-8000-000000000002', 'WINTER_FD', 'Winter Field Day',
        'Emergency preparedness exercise held in winter conditions', 'Winter Field Day Association',
        '{"pointsPerQso":1,"pointsByMode":{"CW":2,"DIGITAL":2,"PHONE":1},"multipliers":[{"type":"section","description":"ARRL/RAC sections worked"}],"formula":"(QSO Points x Power Multiplier) + Bonuses"}',
        '{"class":{"required":true,"options":["1O","2O","3O","1I","2I","3I","1H"],"description":"O=Outdoor, I=Indoor, H=Home"},"section":{"required":true,"description":"ARRL or RAC section"}}',
        '{"bands":["160","80","40","20","15","10","6","2","1.25","70CM"],"modes":["CW","PHONE","DIGITAL","SSB","FM","FT8","FT4","RTTY"],"duplicateRule":"band-mode","exchange":{"sent":["class","section"],"received":["class","section"]}}',
        '{"type":"relative","summary":"Last full weekend of January","durationHours":24,"timezone":"UTC"}'
    ),
    (
        '10000000-0000-4000-8000-000000000003', 'POTA', 'Parks on the Air',
        'Activate parks and wildlife areas or contact park activators', 'POTA',
        '{"pointsPerQso":1,"bonuses":[{"name":"Park-to-Park","points":1}],"formula":"Ten QSOs minimum for an activator qualification"}',
        '{"park":{"required":true,"description":"Your park reference, for example K-4566"}}',
        '{"bands":["160","80","60","40","30","20","17","15","12","10","6","2","1.25","70CM"],"modes":["CW","SSB","FM","DIGITAL","FT8","FT4","PSK31","RTTY"],"exchange":{"sent":["rst","park"],"received":["rst","park"],"validation":{"park":"^[A-Z]{1,2}-[0-9]{4}$"}}}',
        '{"type":"year-round"}'
    ),
    (
        '10000000-0000-4000-8000-000000000004', 'SOTA', 'Summits on the Air',
        'Activate mountain summits or contact summit activators', 'SOTA',
        '{"pointsPerQso":1,"bonuses":[{"name":"Summit-to-Summit","points":2}],"formula":"Activator points by summit; chaser points by summit worked"}',
        '{"summit":{"required":true,"description":"Your summit reference, for example W7W/LC-001"}}',
        '{"bands":["160","80","60","40","30","20","17","15","12","10","6","2","1.25","70CM","23CM"],"modes":["CW","SSB","FM","DIGITAL","FT8","FT4"],"exchange":{"sent":["rst","summit"],"received":["rst","summit"],"validation":{"summit":"^[A-Z0-9]{2,3}/[A-Z]{2}-[0-9]{3}$"}}}',
        '{"type":"year-round"}'
    );

ALTER TABLE events
    ADD COLUMN contest_template_id UUID REFERENCES contest_templates(id) ON DELETE SET NULL,
    ADD COLUMN contest_config JSONB NOT NULL DEFAULT '{}';
CREATE INDEX events_contest_template_idx ON events (contest_template_id);

CREATE TABLE station_presence (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    instance_id UUID NOT NULL,
    station_label TEXT NOT NULL DEFAULT '',
    radio_manufacturer TEXT,
    radio_model TEXT,
    frequency_hz BIGINT CHECK (frequency_hz IS NULL OR frequency_hz >= 0),
    band TEXT,
    mode TEXT,
    qsonaut_version TEXT NOT NULL,
    platform TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('online', 'idle', 'offline')),
    metadata JSONB NOT NULL DEFAULT '{}',
    last_seen TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, instance_id)
);
CREATE INDEX station_presence_last_seen_idx ON station_presence (last_seen DESC);

CREATE TABLE qso_logs (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    event_id UUID REFERENCES events(id) ON DELETE SET NULL,
    idempotency_key UUID NOT NULL UNIQUE,
    callsign TEXT NOT NULL,
    band TEXT NOT NULL,
    mode TEXT NOT NULL,
    frequency_hz BIGINT CHECK (frequency_hz IS NULL OR frequency_hz >= 0),
    occurred_at TIMESTAMPTZ NOT NULL,
    rst_sent TEXT,
    rst_received TEXT,
    exchange JSONB NOT NULL DEFAULT '{}',
    points INTEGER NOT NULL DEFAULT 0,
    source TEXT NOT NULL DEFAULT 'qsonaut',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX qso_logs_event_time_idx ON qso_logs (event_id, occurred_at DESC);
CREATE INDEX qso_logs_user_time_idx ON qso_logs (user_id, occurred_at DESC);
