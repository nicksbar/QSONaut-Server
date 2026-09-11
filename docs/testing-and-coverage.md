# Testing and coverage

Coverage is measured with `cargo llvm-cov` across the Rust workspace. The CI
Rust job uploads the LCOV artifact and prints the summary for every build.

## Baseline

Measured after adding authenticated station, diagnostic, and event visibility
contracts on the server-console-boundaries feature branch:

- 43.92% region coverage
- 40.80% line coverage
- 36.50% function coverage

The distribution matters: the contest catalog is 100%, the protocol is about
69% by line, but the API and store contain many unexercised management paths.
The contest catalog is 100%, the protocol is about 75% by line, and the store
is now about 51% by line coverage. The remaining gap is concentrated in
authenticated API handlers and the broader store management surface.

## Path to 50%+

Build coverage in this order:

1. Add authenticated router tests for account, club, event, identity, station,
   log, diagnostic, and authorization responses.
2. Expand Postgres contract fixtures around membership lifecycle, visibility,
   diagnostics ownership, station ownership, event scope, and retry behavior.
3. Add negative tests for every administrator-only route and every
   cross-organization access path.
4. Add focused unit tests for validation and error mapping, then enforce the
   50% line threshold once the measured baseline is above it.

Do not count generated web output or database migration SQL as application
coverage. The web package currently has compile/build validation; component and
browser tests should be added when the console interaction surface stabilizes.
