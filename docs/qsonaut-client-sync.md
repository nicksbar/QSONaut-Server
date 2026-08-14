# QSONaut client synchronization

The management server now has persistence and management views for two kinds of explicitly published QSONaut activity. The QSONaut desktop-side publisher is a separate integration and is not yet enabled by these server changes alone.

## Station presence

An authenticated QSONaut client may update `PUT /api/v1/stations/presence` with a per-installation UUID, client version, platform, online state, and operator-approved station context. Optional context includes a station label, radio manufacturer/model, band, mode, frequency, and bounded structured metadata.

Presence is a last-known coordination snapshot. It grants no server authority over CAT, PTT, audio, modem, or QSO sequencing. QSONaut should make each shared field visible and configurable locally, publish only while server connectivity is enabled, and send `offline` during a clean disconnect when practical.

## QSO collection

An authenticated QSONaut client may submit an idempotent record to `POST /api/v1/logs`. Each record has a client-generated UUID, operator identity from the authenticated session, optional event, callsign, band, mode, optional frequency/RST/exchange fields, timestamp, points, and source.

The server rejects duplicate idempotency keys rather than silently duplicating contacts. The management UI is intentionally read-only for QSO activity: operating and correction workflows remain in QSONaut until conflict-resolution and audit semantics are designed.

## Authentication follow-up

The current endpoints accept the existing authenticated session mechanism. Before the native QSONaut integration ships, add revocable device credentials with narrow `presence:write` and `logs:write` scopes. Browser administrator sessions must not be copied into desktop configuration.
