# Deployment

QSONaut Server is designed to be built from source. The repository's `main`
branch is the deployment source of truth; releases are optional convenience
artifacts, not a runtime requirement.

## Docker Compose

On a Linux host with Docker Compose v2:

```text
git clone https://github.com/nicksbar/QSONaut-Server.git
cd QSONaut-Server
git switch main
cp .env.example .env
# Edit .env and replace QSONAUT_DATABASE_PASSWORD with a long random value.
docker compose --env-file .env -f deploy/compose.yaml up --build -d
docker compose -f deploy/compose.yaml ps
```

The service listens on port `8080` by default. Put a TLS reverse proxy in front
of it before exposing it outside a trusted LAN, and set
`QSONAUT_SECURE_COOKIES=true` in `.env` when the public endpoint uses HTTPS.

To update from source:

```text
git pull --ff-only origin main
docker compose --env-file .env -f deploy/compose.yaml up --build -d
```

PostgreSQL data is stored in the `qsonaut-data` named volume. Do not use
`down -v` unless you intentionally want to erase the database.

### Migrations and updates

The server applies any pending SQLx migrations when it starts. Migrations are
tracked in PostgreSQL and run once, in order; an existing database is not
reset when the container is rebuilt or restarted.

The first start after an update may take longer while new migrations are
applied before the HTTP listener starts. Keep the database volume and take a backup before
updates. If a migration fails, the server should not be treated as upgraded;
inspect the migration error and restore from backup only if necessary. Never
run `docker compose down -v` as part of a routine deployment.

Diagnostic reports are bounded at ingestion and retained for 30 days. Expired
diagnostics and share-link records are removed by the authenticated
administrator retention operation (`POST /api/v1/diagnostics/retention/purge`).
An administrator may also permanently delete every submitted diagnostic report
with `DELETE /api/v1/diagnostics`; startup and restart do not silently delete
data.

### Logs and health checks

For the Compose deployment, server startup, migration, request, and shutdown
messages are available with:

```text
docker compose --env-file .env -f deploy/compose.yaml logs -f --tail=200 qsonaut-server
docker compose --env-file .env -f deploy/compose.yaml logs -f --tail=200 postgres
curl -fsS http://127.0.0.1:8080/api/v1/health
```

For a native process, set `RUST_LOG=qsonaut_server=info,tower_http=info` and
keep the process supervisor's stdout/stderr. The desktop QSONaut client keeps
its local log at `~/.config/qsonaut/logs/qsonaut.log` on Linux.

## Proxmox

For Proxmox, a small Debian or Ubuntu VM is the recommended default: install
Docker Engine and the Compose plugin inside the VM, then follow the Docker
Compose steps above. This keeps the database and container runtime isolated
from the Proxmox host and avoids privileged-container requirements.

An unprivileged Debian LXC can also run the stack when the host policy permits
nesting and the required overlay/filesystem features. VM deployment is easier
to support; do not enable privileged nesting solely for QSONaut Server without
understanding the host security trade-off.

Recommended layout:

- private VM/LXC network address for the Compose stack;
- reverse proxy on the VM or an existing trusted gateway;
- persistent backups of the `qsonaut-data` volume or PostgreSQL data;
- firewall access limited to HTTPS and administration paths;
- outbound access for image builds and updates only when needed.

## Native source build

A native build needs Rust stable, Node.js/npm, and PostgreSQL. Build the web
console first, then build the Rust server:

```text
npm --prefix web ci
npm --prefix web run build
cargo build --release -p qsonaut-server
```

Set `QSONAUT_DATABASE_URL`, `QSONAUT_SERVER_BIND`, and
`QSONAUT_SERVER_WEB_ROOT=web/build` before starting the resulting binary.
