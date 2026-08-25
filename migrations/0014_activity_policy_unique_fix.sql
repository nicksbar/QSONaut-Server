ALTER TABLE activity_visibility_policies
    DROP CONSTRAINT activity_visibility_policies_user_id_scope_scope_id_key;

CREATE UNIQUE INDEX activity_visibility_policies_unique_target_idx
    ON activity_visibility_policies (user_id, scope, COALESCE(scope_id, '00000000-0000-0000-0000-000000000000'::uuid));
