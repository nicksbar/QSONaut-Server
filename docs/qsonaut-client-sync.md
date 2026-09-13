# QSONaut client synchronization

The management server persists and presents explicitly published QSONaut station activity, QSO records, and shared automation-channel traffic.

## Station presence

An authenticated QSONaut client may update `PUT /api/v1/stations/presence` with a per-installation UUID, client version, platform, online state, and operator-approved station context. Optional context includes a station label, radio manufacturer/model, band, mode, frequency, and bounded structured metadata.

Presence is a last-known coordination snapshot. It grants no server authority over CAT, PTT, audio, modem, or QSO sequencing. QSONaut should make each shared field visible and configurable locally, publish only while server connectivity is enabled, and send `offline` during a clean disconnect when practical.

## QSO collection

An authenticated QSONaut client may submit an idempotent record to `POST /api/v1/logs`. Each record has a client-generated UUID, operator identity from the authenticated session, a verified managed operating identity (`operating_callsign` and `callsign_id`), optional event, contact callsign, band, mode, optional frequency/RST/exchange fields, timestamp, points, and source.

The same `QsoLogInput` contract is used by the WebSocket `log` message. The
top-level shape is closed: unknown fields are rejected, contact and operating
callsigns, band, mode, source, timestamps, and exchange structure are
validated before storage, and every record must carry both
`operating_callsign` and `callsign_id`. Contest `fields_sent` and `fields_received` keys are canonical
lowercase catalog keys; the server normalizes legacy casing before applying
the authoritative contest definition and PostgreSQL event checks.

The server treats a repeated idempotency key from the same authenticated operator as a retry and returns the already-stored record without creating a duplicate. The management UI is intentionally read-only for QSO activity: operating and correction workflows remain in QSONaut until conflict-resolution and audit semantics are designed.

The management UI presents activity by permitted operating context. Activity
policies target a managed callsign, club, or event through
`/api/v1/activity/visibility`; event policies override callsign policies, and
callsign policies override club defaults. Personal callsigns may be controlled
by their owner. Club and special callsign policies, plus club and event
policies, require an owning club owner or coordinator. A policy does not grant
access by itself: `private` is submitter-only, `members` requires active
membership in the owning club, and `global` permits authenticated viewers.
Maps use this same effective visibility query for personal, identity, club,
and event scopes. QSO records remain operating records rather than individually
managed privacy objects. An authorized owner or administrator can create a short-lived
revocable share grant through
`POST /api/v1/logs/{log_id}/share`; the response contains a copyable path, and
the public read endpoint does not expose the underlying log ID. The copied
path opens a human-readable share page; `/api/v1/share/{token}` remains
available for JSON consumers.

Event and ordinary QSO submissions must include the selected
`operating_callsign` and `callsign_id`. Missing, stale, or unassigned
identities are rejected by both the request and database boundaries; clients
cannot bypass authorization by omitting them.
`GET /api/v1/identities` returns the caller's usable personal, club, and
event-linked special identities. Special identities are registered through
`POST /api/v1/identities/special`; administrators can approve or reject them,
and authorized club managers can suspend or revoke them. These lifecycle
actions are recorded with the authenticated actor.

For an authorized event, `GET /api/v1/events/{event_id}/score` and the event
snapshot expose the server aggregate. Accepted event-QSO acknowledgments also
carry the refreshed score and the QSO's authoritative duplicate, multiplier,
points, and scoring-explanation fields.

## Device authentication

QSONaut desktop uses a browser-based device authorization flow. It starts a
request with `POST /api/v1/auth/device/authorize`, receiving a short-lived
device code, human-readable user code, and an absolute `verification_uri`.
The operator opens the URI, signs in to the same server, and approves the named
installation. QSONaut polls `POST /api/v1/auth/device/token` until approval and
receives an access token; the browser never exposes its session cookie or the
native token.

The server serves the approval page at `/link`. Set
`QSONAUT_SERVER_PUBLIC_URL` to the externally reachable origin (for example
`https://www.qsonaut.com`) so the URI launched by QSONaut is correct. Local
development defaults to `http://localhost:8080`.

Signed-in operators may still use **Station link** in the management UI to
create a manually copyable token as a fallback for older clients. The token is
displayed once and the server stores only its hash.

Native clients exchange their callsign, password, and a local device name at
`POST /api/v1/auth/device`. The returned 90-day bearer token is shown once and
is stored only as a SHA-256 hash by the server. Password resets revoke all
browser sessions and device tokens for that operator. A client can revoke its
current token with `DELETE /api/v1/auth/device`.

The token records explicit `events:read`, `presence:write`, `logs:write`,
`diagnostics:write`, `messages:read`, and `messages:write` capabilities. The
WebSocket checks the required capability for each operation; browser cookies
are never copied into QSONaut configuration.

The **Station link** page lists the signed-in operator's issued tokens with
creation, expiry, and last-use timestamps. Operators may revoke a token or
reissue it; a replacement secret is again displayed only once.

## Operator profile and HamDB

`GET /api/v1/auth/profile` and `PATCH /api/v1/auth/profile` expose the signed-in
operator's editable profile. `POST /api/v1/auth/profile/hamdb` performs an
explicit HamDB lookup and stores the normalized license/name/location record
with a fetch timestamp. The server records provider failure state, does not
block log ingestion when HamDB is unavailable, and does not include address,
license, or coordinates in activity summaries, club rosters, or share links.

## Diagnostic snapshots

QSONaut may send a manually approved `diagnostic` WebSocket message containing
bounded structured radio, audio, decoder, and latest-error state. The server
retains these reports separately from QSO logs and exposes them only in the
administrator Activity view. Tokens, audio samples, and configured device names
are excluded from the client snapshot.

## Shared channels

Authenticated native clients can publish bounded text messages to named
channels over the WebSocket. The server records the authenticated author,
persists the message, includes recent traffic in snapshots, and broadcasts new
messages to connected clients. The management Activity view displays the most
recent 200 messages alongside station presence and collected logs.

## WebSocket transport

QSONaut connects to the exact path `GET /api/v1/ws` using the `qsonaut.v1` subprotocol and an
`Authorization: Bearer ...` header. Messages use versioned JSON envelopes with
client-generated event UUIDs. The current message set provides event/catalog
snapshots, presence publication, idempotent QSO submission, shared-channel
publication and broadcast, acknowledgements, and heartbeats.

This is an ordinary HTTP WebSocket upgrade. A hosted deployment exposes only
HTTPS/WSS on port 443; the reverse proxy forwards `/api/v1/ws` to the same
QSONaut Server process as the management API and web UI. No additional public
port or radio-specific proxy protocol is required.

Example nginx location:

```nginx
location / {
    proxy_pass http://127.0.0.1:8080;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
}
```
# Event submission validation increment

The v1 contract now enforces event membership, state and QSO schedule in the
database for all submission paths. Policy failures return validation messages.
Existing accepted idempotency keys remain successful retries. See
[migration and compatibility details](contesting-migration.md).
