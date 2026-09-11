# Testing and coverage

Coverage is measured with `cargo llvm-cov` across the Rust workspace. The CI
Rust job uploads the LCOV artifact and prints the summary for every build.

## Baseline

Measured after the first ownership and visibility contract expansion on the
server-console-boundaries feature branch:

- 31.06% region coverage
- 30.02% line coverage
- 27.05% function coverage

The distribution matters: the contest catalog is 100%, the protocol is about
69% by line, but the API and store contain many unexercised management paths.
The store is now about 45% by region and line coverage after adding useful
membership, activity, log-visibility, diagnostic-ownership, and station-
ownership contracts. The remaining gap is concentrated in authenticated API
handlers and the broader store management surface.

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
