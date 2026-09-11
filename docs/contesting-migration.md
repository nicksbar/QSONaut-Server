# Contest event authorization increment

Migration `0020_callsigns_and_event_participation.sql` adds the managed
callsign registry and explicit event participant/station assignments.
Migration `0021_qso_operating_identity.sql` stores the selected operating
callsign and managed identity on each QSO.
Migration `0026_event_contest_definition_snapshot.sql` records the event's
contest definition version and validates configuration at the database
boundary. Migration `0027_multi_contest_multiplier_awards.sql` evaluates all
configured exchange multiplier rules rather than only the first rule.
Migration `0028_special_callsign_registry.sql` links special/event callsigns
to their event and provides a permission-gated registration path. Club
managers create pending registrations; administrators create verified active
registrations.
Migration `0029_managed_callsign_audit.sql` records the authenticated actor and
registration and lifecycle outcomes in an append-only callsign audit stream.
The management API supports administrator approval/rejection and manager or
administrator suspension/revocation of event-linked special callsigns.
Migration `0030_contest_authorization_hardening.sql` revalidates current club
membership, operating participant role, identity lifetime, optional station
band/mode constraints, and all adjudication-sensitive QSO updates. It also
normalizes meter and centimeter band names consistently and enforces audit-row
immutability at the database boundary.
Migration `0031_participant_assignment_validation.sql` rejects station band or
mode assignments that are incompatible with the event's contest definition.
Migration `0032_callsign_registry_synchronization.sql` keeps personal and club
identities synchronized for users and clubs created after the initial seed and
binds every event QSO's callsign text to its managed identity ID.
Migration `0033_qso_managed_identity_validation.sql` validates optional managed
identities on general QSOs, infers the authenticated user's personal identity
when omitted, and prevents event-only or unauthorized club calls from being
used outside their permitted context.
Migration `0034_event_assignment_audit_and_locking.sql` adds immutable event
station-assignment history and prevents club, schedule, special-call, or
contest-rule changes after assignments or logs depend on the configuration.
Event create/update operations synchronize special-call requests and their
actor audit transactionally. Manager requests remain pending until an
administrator approves them; administrator changes are verified immediately.
The synchronized event snapshot also includes an aggregate score derived from
accepted QSO rows; clients must continue treating individual receipts and this
total as server-authoritative.
Accepted event QSO acknowledgments now include the updated aggregate score in
the same response, allowing clients to refresh score displays without waiting
for a reconnect.

Migration `0019_event_log_authorization.sql` installs a trigger on QSO inserts
and updates to actor, event, or occurrence time. It does not rewrite historical
records or create callsign/participation entitlements.

For event QSOs the authenticated actor must have an active owner, coordinator,
or operator membership in the owning club, an active event participant
assignment, and the assigned active/verified managed operating identity.
Observers, lapsed memberships, unrelated users, and arbitrary submitted
callsigns are rejected. Row locks protect the check against concurrent
membership, identity, and event changes. Draft and cancelled events reject
new submissions.
The QSO time must satisfy `starts_at <= occurred_at < ends_at`. Completed events
accept delayed uploads made inside their window. The existing store resolves
accepted idempotency keys before inserting, including retries after event closure.

Policy errors use SQLSTATE `P1001`, surfaced as a validation response over the
existing v1 API/WebSocket contract. General logs without an event are unchanged.
The v1 wire contract accepts operating_callsign and callsign_id. Missing
identity fields are rejected for event QSOs, so legacy clients cannot bypass
the new enforcement by omitting them. Contest context is server-stamped from
the event at insertion time; client metadata is not authoritative.
The desktop now includes captured operator/station/template/club context in its
exchange metadata and uses the event captured on the QSO for historical uploads.
This metadata is not an authorization source.

The web Events panel exposes special-identity status and event operator/station
assignment creation and editing. Event QSO authorization takes an event row
lock, serializing duplicate and multiplier adjudication for concurrent event
submissions. More specialized contest formulas and bonus handling remain
catalog-by-catalog work rather than an authorization limitation.
Event client-submitted points are ignored; base points, duplicate decisions,
multiplier awards, and the recorded scoring explanation are server-derived.

Apply through normal Store startup migrations. Validate against an isolated
PostgreSQL database with `QSONAUT_TEST_DATABASE_URL` and run:

```text
cargo test --locked -p qsonaut-store --test event_authorization -- --nocapture
```

When that variable is unset the test explicitly skips database work. A passing
workspace result alone therefore does not prove this migration was executed.
