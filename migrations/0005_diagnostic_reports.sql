CREATE TABLE diagnostic_reports (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    instance_id UUID NOT NULL,
    category TEXT NOT NULL,
    summary TEXT NOT NULL,
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT diagnostic_category_length CHECK (char_length(category) BETWEEN 1 AND 40),
    CONSTRAINT diagnostic_summary_length CHECK (char_length(summary) BETWEEN 1 AND 240)
);

CREATE INDEX diagnostic_reports_created_idx ON diagnostic_reports (created_at DESC);
CREATE INDEX diagnostic_reports_station_idx ON diagnostic_reports (user_id, instance_id, created_at DESC);
