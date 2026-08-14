# Contributing

Thanks for helping improve QSONaut Server.

## Development setup

The supported development path is a local PostgreSQL container followed by the
Rust server and Svelte web console. See the [README](README.md) for the exact
commands and environment variables.

## Before opening a pull request

Run these checks from the repository root:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm --prefix web ci
npm --prefix web run check
npm --prefix web run build
```

If Docker is available, also build the deployable image with the repository's
`deploy/Dockerfile`. Do not commit build output, local `.env` files, database
volumes, or credentials.

## Pull requests

- Keep changes focused and explain the operational impact.
- Add or update tests for behavior changes.
- Update protocol or deployment documentation when contracts change.
- Call out migration changes explicitly; migration versions must be unique and
  strictly ordered.
- Do not include secrets or private station data in source, fixtures, or logs.
