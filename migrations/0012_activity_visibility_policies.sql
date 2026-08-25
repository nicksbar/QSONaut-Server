CREATE TABLE activity_visibility_policies (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    scope TEXT NOT NULL,
    scope_id UUID,
    visibility TEXT NOT NULL DEFAULT 'private',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT activity_policy_scope_kind CHECK (scope IN ('overall', 'club', 'contest')),
    CONSTRAINT activity_policy_scope_target CHECK (
        (scope = 'overall' AND scope_id IS NULL) OR
        (scope IN ('club', 'contest') AND scope_id IS NOT NULL)
    ),
    CONSTRAINT activity_policy_visibility_kind CHECK (visibility IN ('private', 'members', 'global')),
    UNIQUE (user_id, scope, scope_id)
);

CREATE INDEX activity_policy_scope_idx
    ON activity_visibility_policies (scope, scope_id, visibility);
