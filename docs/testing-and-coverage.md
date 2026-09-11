# Testing and coverage

Coverage is measured with `cargo llvm-cov` across the Rust workspace. The CI
Rust job uploads the LCOV artifact and prints the summary for every build.

## Baseline

Measured on the server-console-boundaries feature branch:

- 13.16% region coverage
- 13.57% line coverage
- 11.69% function coverage

The distribution matters: the contest catalog is 100%, the protocol is about
69% by line, but the API and store contain many unexercised management paths.
The store is currently below 1%, so a 50% total requires meaningful integration
coverage rather than superficial unit tests.

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
