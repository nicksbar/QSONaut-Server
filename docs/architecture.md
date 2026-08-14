# Architecture

QSONaut Server is a Rust service with a statically built Svelte management UI.
The API is the product boundary; neither QSONaut nor the web console imports
server implementation code.

```text
QSONaut desktop ---- HTTPS / WebSocket ----+
                                             |
N3FJP clients ------ TCP / UDP adapter ------+-- QSONaut Server -- PostgreSQL
                                             |
Management browser - HTTPS -----------------+
```

## Contract policy

- HTTP routes are versioned under `/api/v1`.
- HTTP schemas are published as OpenAPI at `/api/v1/openapi.json`.
- WebSocket messages will use versioned JSON envelopes with stable event IDs.
- Clients generate idempotency IDs for offline-safe writes.
- Public DTOs live in `qsonaut-protocol`; persistence models do not.
- Breaking changes require a new API version.

QSONaut may explicitly publish station presence and idempotent QSO records for
coordination and reporting. These are client-originated observations, never
server commands. See [qsonaut-client-sync.md](qsonaut-client-sync.md).

## Technology decisions

- Axum and Tokio for HTTP, WebSocket, TCP, UDP, and process lifecycle
- SQLx with PostgreSQL for hosted deployments
- Svelte 5 and TypeScript for the management UI
- Static frontend output served by the Rust process
- Server-managed, revocable browser sessions with Argon2 password hashes

OIDC Authorization Code with PKCE remains a future option for installations
that already operate an identity provider; it is not required for a private-LAN
deployment.

SQLite may be added as a development or small-LAN profile. PostgreSQL remains
the production model so correctness is not designed around a single-process
database.

## Module growth rule

The root process assembles modules but contains no domain behavior. New features
belong in focused crates or modules with explicit inputs and outputs. Audio,
radio, modem, and transmit dependencies are prohibited from this repository.
