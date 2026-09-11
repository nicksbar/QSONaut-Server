# Changelog

## Unreleased — 2026-09-10

- Migration 0019 adds a database-level event QSO submission guard for active
  operating membership, event state and QSO occurrence time.
- API responses expose the guard's deliberate validation messages; other
  database failures retain generic error handling.
- Added PostgreSQL integration coverage for missing membership, observer and
  lapsed roles, schedule boundaries, cancelled events, completed-event delayed
  uploads, and accepted-log idempotent retries.
- Added managed callsign identities and explicit event participant/station
  assignments, with database-enforced operating-identity authorization.
- Added server-derived base points, duplicate decisions, and scoring
  explanations for event QSOs.
- Added event contest-definition snapshots, database configuration validation,
  stale-definition rejection, and multi-rule exchange multiplier awards.
- Added event-linked special-call registrations with manager permission checks
  and administrator verification.
- Added server-authoritative aggregate event scores to synchronized activity
  state and the QSONaut contest summary.
- Included the updated event score in each accepted event-QSO acknowledgment
  for immediate client refresh.
- Added append-only audit records for special-call registrations, including the
  authenticated actor and verification outcome.
- Added audited special-callsign lifecycle actions for administrator approval or
  rejection and manager/administrator suspension or revocation.
- Hardened contest authorization to re-check current membership, participant
  role, identity validity, station band/mode assignments, and every sensitive
  QSO update; corrected centimeter-band matching and made callsign audits
  database-immutable.
- Rejects event station band/mode assignments that conflict with the selected
  contest definition before operators can select them.
- Keeps newly created or renamed user/club callsigns synchronized with the
  managed identity registry and rejects stale identity-text substitutions.
- Validates managed identities on non-event logs and safely infers the
  authenticated user's active personal identity when the client omits one.
- Filters event discovery to active club access, synchronizes event special-call
  requests transactionally, exposes assignment management in the web console,
  records immutable actor-attributed assignment audits, and locks event
  configuration once assignments or logs depend on it.
