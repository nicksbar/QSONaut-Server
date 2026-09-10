# Changelog

## Unreleased — 2026-09-10

- Migration 0019 adds a database-level event QSO submission guard for active
  operating membership, event state and QSO occurrence time.
- API responses expose the guard's deliberate validation messages; other
  database failures retain generic error handling.
- Added PostgreSQL integration coverage for missing membership, observer and
  lapsed roles, schedule boundaries, cancelled events, completed-event delayed
  uploads, and accepted-log idempotent retries.
