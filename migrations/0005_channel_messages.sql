CREATE TABLE channel_messages (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    event_id UUID REFERENCES events(id) ON DELETE SET NULL,
    channel TEXT NOT NULL,
    message TEXT NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT channel_messages_channel_length CHECK (char_length(channel) BETWEEN 1 AND 80),
    CONSTRAINT channel_messages_body_length CHECK (char_length(message) BETWEEN 1 AND 2000)
);

CREATE INDEX channel_messages_created_idx ON channel_messages (created_at DESC);
CREATE INDEX channel_messages_event_created_idx ON channel_messages (event_id, created_at DESC);
