ALTER TABLE access_challenges
    DROP CONSTRAINT access_challenges_attempts_remaining_check;

ALTER TABLE access_challenges
    ADD CONSTRAINT access_challenges_attempts_remaining_check
    CHECK (attempts_remaining BETWEEN 0 AND 3);
