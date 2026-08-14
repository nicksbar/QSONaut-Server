# Scope boundary

QSONaut Server coordinates people and records. QSONaut operates radios.

## Server responsibilities

- Identity, authentication, sessions, and authorization
- Club membership and callsign profiles
- Event and contest setup
- Station registration and presence
- Offline-safe QSO synchronization and deduplication
- Event chat and operator coordination
- Shared logs, scoring, reports, and export
- N3FJP-compatible relay and ingestion
- Administrative management UI

## Explicitly outside the server

- Audio capture, transport, mixing, or playback
- Radio hardware discovery or control
- PTT and transmit authorization
- Digital-mode decoding or waveform generation
- QSO sequencing and automatic replies
- Spectrum and waterfall processing
- A browser replacement for the QSONaut operator console

No server command may imply transmit authority. A future station-edge service,
if one is ever introduced, requires a separate threat model and explicit local
arming. It is not part of the current product.

## Client independence

QSONaut must remain useful offline and without an account. Server integration is
an opt-in client capability built on the public API. Local decoding, logging,
radio control, and transmit safety must not depend on server availability.

