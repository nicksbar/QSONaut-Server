# Club operations

The club console gives every authenticated operator a club directory. Any
operator can register a club; creation and the initial owner membership happen
in one database transaction, so a club is never left unmanaged.

## Membership lifecycle

Non-members can request to join. A club owner or coordinator reviews the
durable request and may approve it as an operator or observer. Owners and global
administrators may also appoint coordinators or additional owners. Coordinators
cannot modify owner/coordinator records, and the final active owner cannot be
removed or demoted.

Roster records include:

- active, lapsed, or inactive membership status;
- current, due, overdue, waived, or untracked dues state;
- an optional local membership number;
- an optional renewal date.

These fields are deliberately independent. A club can track renewals without
tracking money, and a lapsed record remains available for history and follow-up.

## Governance and elections

Owners define officer and board positions with a seat count, term length, and
annual, even-year, or odd-year election cadence. Position assignments record
the member, seat, start/end dates, and whether the term was elected, appointed,
or acting. Multiple seats and overlapping terms are represented explicitly.

Election cycles record a year, workflow state, optional opening/closing times,
and the positions included in that cycle. This supports planning, nominations,
voting windows, closure, and certification calendars.

Secret ballot casting is intentionally a separate future layer. It needs
eligibility snapshots, one-ballot enforcement, ballot secrecy, recount rules,
and an auditable certification process; those guarantees should not be faked by
ordinary roster-edit permissions.
