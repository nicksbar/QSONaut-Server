# QSONaut client synchronization

The management server persists and presents explicitly published QSONaut station activity, QSO records, and shared automation-channel traffic.

## Station presence

An authenticated QSONaut client may update `PUT /api/v1/stations/presence` with a per-installation UUID, client version, platform, online state, and operator-approved station context. Optional context includes a station label, radio manufacturer/model, band, mode, frequency, and bounded structured metadata.

Presence is a last-known coordination snapshot. It grants no server authority over CAT, PTT, audio, modem, or QSO sequencing. QSONaut should make each shared field visible and configurable locally, publish only while server connectivity is enabled, and send `offline` during a clean disconnect when practical.

## QSO collection

An authenticated QSONaut client may submit an idempotent record to `POST /api/v1/logs`. Each record has a client-generated UUID, operator identity from the authenticated session, optional event, callsign, band, mode, optional frequency/RST/exchange fields, timestamp, points, and source.

The server rejects duplicate idempotency keys rather than silently duplicating contacts. The management UI is intentionally read-only for QSO activity: operating and correction workflows remain in QSONaut until conflict-resolution and audit semantics are designed.

## Device authentication

Native clients exchange their callsign, password, and a local device name at
`POST /api/v1/auth/device`. The returned 90-day bearer token is shown once and
is stored only as a SHA-256 hash by the server. Password resets revoke all
browser sessions and device tokens for that operator. A client can revoke its
current token with `DELETE /api/v1/auth/device`.

The token has fixed `events:read`, `presence:write`, `logs:write`,
`messages:read`, and `messages:write` capabilities. Browser cookies are never
copied into QSONaut configuration.

## Shared channels

Authenticated native clients can publish bounded text messages to named
channels over the WebSocket. The server records the authenticated author,
persists the message, includes recent traffic in snapshots, and broadcasts new
messages to connected clients. The management Activity view displays the most
recent 200 messages alongside station presence and collected logs.

## WebSocket transport

QSONaut connects to `GET /api/v1/ws` using the `qsonaut.v1` subprotocol and an
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
