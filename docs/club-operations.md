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

## Hosted organizational governance

Board positions, officer assignments, election cycles, voting, and certification
are intentionally not part of the public server. They are supplied by the
private hosted companion. The public server keeps club identity, membership,
rosters, join requests, and activity setup.
# Event QSO authorization

New event submissions require active owner/coordinator/operator membership;
observers cannot contribute QSOs. Event state and occurrence time are checked
in the database. See [migration details](contesting-migration.md). Membership
does not itself authorize a club or special station callsign.
