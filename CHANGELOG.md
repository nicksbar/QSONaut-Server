# Changelog

## Unreleased — 2026-09-10

- Replaced submitter-scoped activity sharing with policy-owned identity, club,
  and event activity rules. Retired the unused per-QSO visibility columns and
  fields; maps now use the same effective permissions for personal, identity,
  club, and event scopes.
- Replaced the community map's coordinate-grid placeholder with a responsive,
  fully offline Leaflet map using bundled Natural Earth 110m country
  boundaries and U.S. state boundaries, with grid-centre markers, tooltips,
  zooming, and explicit approximate-location messaging.
- Added zoom-aware offline country and U.S. state labels to keep the basic map
  readable without introducing a hosted geocoder or map-label service.
- Added Activity map controls for country boundaries, U.S. state boundaries,
  labels, and fitting the view to the currently permitted activity.
- Added locally generated Maidenhead field and square overlays with independent
  Activity controls; no grid tiles or map service are required.
- Added offline U.S. county boundaries and zoom-aware labels, plus exact
  contact-cell outlines and full six-character grid labels where submitted
  exchanges provide them.
- Added zoom-triggered worldwide six-character Maidenhead grid detail and
  local grid-code labels, with an Activity toggle and automatic close-up
  framing for precise contacts.
- Added the administrator-only `DELETE /api/v1/diagnostics` operation and
  console action to permanently purge all submitted hardware-validation
  reports; retention cleanup remains a separate expired-data operation.
- Restricted club and event policy changes to owners/coordinators and exposed
  read-only policy ownership in the management console.

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
