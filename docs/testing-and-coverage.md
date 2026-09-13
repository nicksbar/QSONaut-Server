# Testing and coverage

Coverage is measured with `cargo llvm-cov` across the Rust workspace. The CI
Rust job provisions Postgres so database-backed contracts execute, uploads the
LCOV artifact, prints the summary, and fails pull requests below the minimums.

## Baseline

Measured after adding authenticated profile, club creation, join-request,
station, diagnostic, event visibility, activity-sharing, device lifecycle, and
administrator-boundary contracts on the server-console-boundaries feature
branch:

- 57.98% region coverage (CI minimum)
- 53.87% line coverage (CI minimum)
- 51.48% function coverage (CI minimum)

The distribution matters: the contest catalog is 100%, the protocol is about
69% by line, but the API and store contain many unexercised management paths.
The contest catalog is 100%, the protocol is about 75% by line, and the store
is now about 53% by line coverage. The remaining gap is concentrated in
authenticated API handlers and the broader store management surface.

## Path to 50%+

Build coverage in this order:

1. Add authenticated router tests for account, club, event, identity, station,
   log, diagnostic, and authorization responses.
2. Expand Postgres contract fixtures around membership lifecycle, visibility,
   diagnostics ownership, station ownership, event scope, and retry behavior.
3. Add negative tests for every administrator-only route and every
   cross-organization access path.
4. Add focused unit tests for validation and error mapping, then raise the CI
   minimums only after a new measured baseline is repeatable.

Do not count generated web output or database migration SQL as application
coverage. The web package currently has compile/build validation; component and
browser tests should be added when the console interaction surface stabilizes.
