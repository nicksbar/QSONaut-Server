# Ownership and workspace boundaries

QSONaut Server is an operating system for amateur-radio activity, not a single
global dashboard. Every record and screen has an owning scope. The console
starts at the operator's active memberships; global visibility is an explicit
administrator action.

## Scope hierarchy

1. **Server/app** owns deployment configuration, authentication, capability
   discovery, migrations, health, and the technical boundary for station
   synchronization. It does not own a club's members, operating plan, or
   contest decisions.
2. **Global administrator** owns server recovery and cross-organization
   support: access requests, account repair, global identity review, abuse or
   diagnostics review, and exceptional operational repair. This scope is
   deliberately separated into Server oversight. It is not an operator's
   default workspace.
3. **Organization (club)** owns its identity, membership, join requests,
   roster maintenance, events, contest configuration, event callsigns, and
   operating assignments. An event belongs to exactly one organization.
4. **Operator** owns their account/profile, personal callsigns, station-link
   tokens, personal activity choices, and participation in organizations.
   Operators see organizations in which they have an active membership.

## Visibility rules

- The normal console shows only active memberships, identities reachable
  through those memberships or personal ownership, and events owned by those
  organizations.
- A lapsed or inactive membership remains a historical club record, but does
  not grant event, roster-management, or sidebar-workspace access.
- The organization directory is discovery, not the primary workspace. It may
  surface a small popular selection and search on demand; it must not turn a
  member's home screen into every organization on the server.
- Administrators may inspect global records only in Server oversight. Being an
  administrator does not make every club part of the administrator's personal
  operating workspace.
- Club roles govern club work: owners and coordinators manage roster and
  event setup; operators participate; observers do not transmit/log as event
  operators. Global authority is for recovery and support, not routine club
  governance.
- Submitted QSO logs and opt-in hardware-validation snapshots remain visible to
  their submitting operator as personal history. Cross-operator log,
  diagnostic, station, and channel review is an administrator-only Server
  oversight function.

## Product boundary

The public server keeps local, Field Day-capable club operations viable:
clubs, memberships, events, identity constraints, activity, and station
linking. Hosted extensions may add billing, third-party identity, governance,
email, and organization administration, but they must respect these scopes and
never make basic local operation depend on a provider.
