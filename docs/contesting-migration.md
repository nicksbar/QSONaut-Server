# Contest event authorization increment

Migration `0019_event_log_authorization.sql` installs a trigger on QSO inserts
and updates to actor, event, or occurrence time. It does not rewrite historical
records or create callsign/participation entitlements.

For event QSOs the authenticated actor must have an active owner, coordinator,
or operator membership in the owning club. Observers, lapsed memberships and
unrelated users are rejected. Row locks protect the check against concurrent
membership/event changes. Draft and cancelled events reject new submissions.
The QSO time must satisfy `starts_at <= occurred_at < ends_at`. Completed events
accept delayed uploads made inside their window. The existing store resolves
accepted idempotency keys before inserting, including retries after event closure.

Policy errors use SQLSTATE `P1001`, surfaced as a validation response over the
existing v1 API/WebSocket contract. General logs without an event are unchanged.
No new wire fields are required, so legacy clients receive the same enforcement.
The desktop now includes captured operator/station/template/club context in its
exchange metadata and uses the event captured on the QSO for historical uploads.
This metadata is not an authorization source.

This is a minimum event boundary, not the complete contest authorization model.
Explicit station assignments, managed/verified callsigns, immutable operator
snapshots, definition/config version enforcement, scoring, and server-wide
duplicate transactions remain to be implemented. Client-submitted points are
still stored; they must not be advertised as authoritative scores.

Apply through normal Store startup migrations. Validate against an isolated
PostgreSQL database with `QSONAUT_TEST_DATABASE_URL` and run:

```text
cargo test --locked -p qsonaut-store --test event_authorization -- --nocapture
```

When that variable is unset the test explicitly skips database work. A passing
workspace result alone therefore does not prove this migration was executed.
