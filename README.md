# QSONaut Server

QSONaut Server is the independent coordination service for QSONaut operators,
clubs, and group events. It provides management, synchronization, reporting,
shared channels, and protocol-integration services to native QSONaut clients.

## Capabilities

The server provides:

- authentication and authorization;
- club rosters, join approvals, renewals, governance, elections, events, and contest configuration;
- synchronized QSO records, live station presence, shared channels, and reports;
- N3FJP-compatible integration;
- the management web console.

See [docs/scope.md](docs/scope.md), [docs/architecture.md](docs/architecture.md),
the [QSONaut client synchronization boundary](docs/qsonaut-client-sync.md), and
the future opt-in [diagnostic bundle design](docs/diagnostic-bundles.md).
Club ownership, roster lifecycle, and election scheduling are described in
[docs/club-operations.md](docs/club-operations.md).

Native QSONaut clients connect through the same HTTPS endpoint as the web UI.
The `/api/v1/ws` route uses a standard WebSocket upgrade, so a reverse proxy can
publish everything on port 443 without a separate public service port.

## Repository layout

- `apps/qsonaut-server` - deployable Rust process
- `crates/qsonaut-api` - HTTP API and generated OpenAPI document
- `crates/qsonaut-contests` - typed, versioned built-in contest catalog
- `crates/qsonaut-protocol` - public, serializable wire contracts
- `crates/qsonaut-store` - PostgreSQL persistence and migrations
- `web` - static Svelte management console
- `deploy` - container deployment

This repository does not depend on the QSONaut source tree. QSONaut and other
clients integrate only through versioned network contracts.

The built-in catalog currently contains 19 contest and activation models. The
server synchronizes these definitions into PostgreSQL at startup; installations
do not need Yahaml or any external contest-definition package. See
[docs/contest-catalog.md](docs/contest-catalog.md) for the ownership and
accuracy boundaries.

## Development

### Start PostgreSQL for development

The simplest local setup uses Docker. From the repository root, start the
development-only PostgreSQL container:

```bash
docker compose -f deploy/postgres-dev.compose.yaml up -d
docker compose -f deploy/postgres-dev.compose.yaml ps
```

Wait until `postgres` reports `healthy`, then configure the server process:

```bash
export QSONAUT_DATABASE_URL=postgresql://qsonaut:qsonaut-dev-password@127.0.0.1:5432/qsonaut
export QSONAUT_SECURE_COOKIES=false
cargo run -p qsonaut-server
```

The Rust server creates or upgrades its tables automatically when it connects.
The password above is intentionally only for a database bound to the local
machine; do not use it for a LAN or hosted deployment.

If port `5432` is already occupied, select another host port and use that same
port in `QSONAUT_DATABASE_URL`:

```bash
QSONAUT_DEV_POSTGRES_PORT=5433 docker compose -f deploy/postgres-dev.compose.yaml up -d
export QSONAUT_DATABASE_URL=postgresql://qsonaut:qsonaut-dev-password@127.0.0.1:5433/qsonaut
```

Run the management console in another terminal. The install command is only
needed initially or after its dependencies change:

```bash
npm --prefix web ci
npm --prefix web run dev
```

Open the URL printed by Vite. Its development server proxies `/api` requests to
the Rust service at `127.0.0.1:8080`.

To stop PostgreSQL while preserving the development database:

```bash
docker compose -f deploy/postgres-dev.compose.yaml stop
```

Run the earlier `up -d` command to start it again. To remove the container and
network while retaining the database volume, use `down`. Add `-v` only when you
intentionally want to permanently erase all local QSONaut development records:

```bash
docker compose -f deploy/postgres-dev.compose.yaml down
docker compose -f deploy/postgres-dev.compose.yaml down -v
```

### Build and deployment

Build the complete service:

```bash
npm --prefix web run build
cargo build --release -p qsonaut-server
```

The Rust service serves `web/build` by default. Override this with
`QSONAUT_SERVER_WEB_ROOT`; override the listen address with
`QSONAUT_SERVER_BIND`.

For a complete local deployment, copy `.env.example` to `.env`, replace the
database password, and run `docker compose -f deploy/compose.yaml up --build`.
Set `QSONAUT_SECURE_COOKIES=true` when the browser-facing endpoint uses HTTPS.
